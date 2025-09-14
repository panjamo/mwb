//! Tools module for AI processing
//! 
//! Provides web search and content extraction capabilities for the AI processor

use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;

use url::Url;

/// Performs a web search using DuckDuckGo's instant answer API
/// This is a free alternative to paid search APIs
pub async fn perform_google_search(query: &str) -> Result<String> {
    println!("🔍 Searching for: '{}'", query);
    
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;

    // Try DuckDuckGo instant answer API first
    let ddg_url = format!("https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1", 
                         urlencoding::encode(query));
    
    match client.get(&ddg_url).send().await {
        Ok(response) => {
            if let Ok(json) = response.json::<Value>().await {
                let mut results = Vec::new();
                
                // Extract abstract if available
                if let Some(abstract_text) = json["Abstract"].as_str() {
                    if !abstract_text.is_empty() {
                        results.push(format!("Abstract: {}", abstract_text));
                        if let Some(abstract_url) = json["AbstractURL"].as_str() {
                            results.push(format!("Source: {}", abstract_url));
                        }
                    }
                }
                
                // Extract related topics
                if let Some(related_topics) = json["RelatedTopics"].as_array() {
                    for (i, topic) in related_topics.iter().take(3).enumerate() {
                        if let Some(text) = topic["Text"].as_str() {
                            results.push(format!("Related {}: {}", i + 1, text));
                        }
                        if let Some(first_url) = topic["FirstURL"].as_str() {
                            results.push(format!("URL: {}", first_url));
                        }
                    }
                }
                
                if !results.is_empty() {
                    return Ok(results.join("\n\n"));
                }
            }
        }
        Err(e) => {
            println!("⚠️ DuckDuckGo API failed: {}", e);
        }
    }
    
    // Fallback: Try to scrape DuckDuckGo search results directly
    let search_url = format!("https://duckduckgo.com/html/?q={}", urlencoding::encode(query));
    
    match client.get(&search_url).send().await {
        Ok(response) => {
            if let Ok(html) = response.text().await {
                return scrape_duckduckgo_results(&html);
            }
        }
        Err(e) => {
            println!("⚠️ DuckDuckGo search failed: {}", e);
        }
    }
    
    // If all else fails, provide suggestions
    Ok(format!("Search failed for '{}'. Try searching manually on:\n- Wikipedia: https://de.wikipedia.org/wiki/{}\n- Fernsehserien.de\n- IMDB", 
              query, urlencoding::encode(query)))
}

/// Scrape DuckDuckGo search results from HTML
fn scrape_duckduckgo_results(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let result_selector = Selector::parse("div.result").unwrap();
    let title_selector = Selector::parse("a.result__a").unwrap();
    let snippet_selector = Selector::parse("a.result__snippet").unwrap();
    
    let mut results = Vec::new();
    
    for (i, element) in document.select(&result_selector).take(5).enumerate() {
        let title = element.select(&title_selector)
            .next()
            .map(|e| e.inner_html())
            .unwrap_or_else(|| format!("Result {}", i + 1));
            
        let snippet = element.select(&snippet_selector)
            .next()
            .map(|e| e.inner_html())
            .unwrap_or_default();
            
        let url = element.select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();
            
        results.push(format!("Title: {}\nURL: {}\nSnippet: {}", 
                           strip_html_tags(&title), url, strip_html_tags(&snippet)));
    }
    
    if results.is_empty() {
        Ok("No search results found.".to_string())
    } else {
        Ok(results.join("\n\n---\n\n"))
    }
}

/// Reads and extracts content from a website
pub async fn read_website_content(url: &str) -> Result<String> {
    println!("📖 Reading content from: '{}'", url);
    
    // Validate URL
    let parsed_url = Url::parse(url)
        .map_err(|_| anyhow::anyhow!("Invalid URL: {}", url))?;
    
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error {}: {}", response.status(), url));
    }
    
    let html_content = response.text().await?;
    let document = Html::parse_document(&html_content);
    
    // Extract content using multiple selectors for different sites
    let content = extract_main_content(&document, &parsed_url)?;
    
    // Limit content size to avoid overwhelming the AI
    const MAX_LENGTH: usize = 8000;
    if content.len() > MAX_LENGTH {
        Ok(format!("{}...\n\n[Content truncated to {} characters]", 
                  &content[..MAX_LENGTH], MAX_LENGTH))
    } else {
        Ok(content)
    }
}

/// Extract main content from HTML document based on the website
fn extract_main_content(document: &Html, url: &Url) -> Result<String> {
    let host = url.host_str().unwrap_or("");
    
    let selectors = match host {
        h if h.contains("wikipedia.org") => vec![
            "div.mw-parser-output p",
            "div.mw-parser-output li", 
            "table.infobox tr",
            ".episode-list td"
        ],
        h if h.contains("fernsehserien.de") => vec![
            "div.serie-info p",
            "div.episoden-liste tr",
            "div.content p"
        ],
        h if h.contains("imdb.com") => vec![
            "[data-testid='plot-xl']",
            ".ipc-html-content-inner-div",
            "li[data-testid='title-episode-item']"
        ],
        h if h.contains("tvbutler.de") => vec![
            ".episode-info",
            ".episode-description",
            ".show-info p"
        ],
        _ => vec![
            "article p", "main p", ".content p", ".post p", 
            ".entry-content p", "div.text p", ".article-body p"
        ]
    };
    
    let mut extracted_text = Vec::new();
    
    // Try each selector until we find content
    for selector_str in &selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let elements: Vec<String> = document
                .select(&selector)
                .map(|el| clean_text(&el.text().collect::<String>()))
                .filter(|text| text.len() > 20) // Filter out very short text
                .take(50) // Limit number of elements
                .collect();
                
            if !elements.is_empty() {
                extracted_text.extend(elements);
                if extracted_text.len() > 20 {
                    break; // We have enough content
                }
            }
        }
    }
    
    // If specific selectors didn't work, try general paragraph extraction
    if extracted_text.is_empty() {
        let p_selector = Selector::parse("p").unwrap();
        extracted_text = document
            .select(&p_selector)
            .map(|el| clean_text(&el.text().collect::<String>()))
            .filter(|text| text.len() > 30)
            .take(30)
            .collect();
    }
    
    if extracted_text.is_empty() {
        return Err(anyhow::anyhow!("Could not extract meaningful content from the webpage"));
    }
    
    Ok(extracted_text.join("\n\n"))
}

/// Clean extracted text by removing extra whitespace and HTML artifacts
fn clean_text(text: &str) -> String {
    text.trim()
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Remove HTML tags from text
fn strip_html_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").to_string()
}

/// URL encoding helper
mod urlencoding {
    pub fn encode(input: &str) -> String {
        url::form_urlencoded::byte_serialize(input.as_bytes()).collect()
    }
}