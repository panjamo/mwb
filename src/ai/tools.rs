//! Tools module for AI processing
//!
//! Provides web search and content extraction capabilities for the AI processor
//! 
//! ## Verbose Logging Implementation
//! 
//! When the application is run with the `--verbose` flag, detailed logging information
//! is displayed for all AI tool calls, including:
//! 
//! ### Function Call Logging:
//! - Function name being called by the LLM
//! - Input parameters (query, URL, etc.)
//! - Enhanced queries for search optimization
//! 
//! ### Search Process Logging:
//! - DuckDuckGo API attempts and results
//! - HTML scraping fallback attempts
//! - Search failure handling and fallback suggestions
//! 
//! ### Content Extraction Logging:
//! - Host detection for site-specific extraction
//! - CSS selector attempts and success rates
//! - Content filtering and extraction results
//! - Character count summaries
//! 
//! ### Usage:
//! The verbose logging is automatically enabled when `--verbose` flag is passed to the CLI.
//! The AIProcessor sets the VERBOSE environment variable which is read by these tool functions
//! to provide detailed tracing of LLM tool interactions.
//! 
//! ### Example Output:
//! ```
//! [VERBOSE] AI Tool Call: perform_google_search
//! [VERBOSE]   query: "Käthe und ich episodes"
//! [VERBOSE]   enhanced_query: "Käthe und ich episoden reihenfolge chronologisch wikipedia fernsehserien.de"
//! [VERBOSE]   DDG API success: 1247 chars returned
//! ```

use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;

use url::Url;

/// Performs a web search using DuckDuckGo's instant answer API
/// This is a free alternative to paid search APIs
/// Enhanced for German TV series episode information
pub async fn perform_google_search(query: &str) -> Result<String> {
    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE] AI Tool Call: perform_google_search");
        eprintln!("[VERBOSE]   query: \"{}\"", query);
    }
    // Enhance query for German TV series chronological information
    let enhanced_query = if query.to_lowercase().contains("käthe und ich") 
        || query.to_lowercase().contains("kathe und ich") {
        format!("{} episoden reihenfolge chronologisch wikipedia fernsehserien.de", query)
    } else {
        format!("{} episodes chronological order episode guide", query)
    };

    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE]   enhanced_query: \"{}\"", enhanced_query);
    }

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()?;

    // Try DuckDuckGo instant answer API first
    let ddg_url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding::encode(&enhanced_query)
    );

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
                    let result_summary = results.join("\n\n");
                    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
                        eprintln!("[VERBOSE]   DDG API success: {} chars returned", result_summary.len());
                    }
                    return Ok(result_summary);
                }
            }
        }
        Err(_) => {
            // DuckDuckGo API failed, try fallback
        }
    }

    // Fallback: Try to scrape DuckDuckGo search results directly
    let search_url = format!(
        "https://duckduckgo.com/html/?q={}",
        urlencoding::encode(&enhanced_query)
    );

    match client.get(&search_url).send().await {
        Ok(response) => {
            if let Ok(html) = response.text().await {
                if std::env::var("VERBOSE").unwrap_or_default() == "1" {
                    eprintln!("[VERBOSE]   Scraping DDG HTML results");
                }
                return scrape_duckduckgo_results(&html);
            }
        }
        Err(_) => {
            // DuckDuckGo search failed, try fallback
        }
    }

    // If all else fails, provide suggestions with German-specific sites
    let series_name = query.split_whitespace().take(3).collect::<Vec<&str>>().join("_");
    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE]   All search methods failed, returning fallback suggestions");
    }
    Ok(format!("Search failed for '{}'. Try these German TV resources:\n- Wikipedia DE: https://de.wikipedia.org/wiki/{}\n- Fernsehserien.de: https://www.fernsehserien.de/suche/{}\n- IMDB: https://www.imdb.com/find?q={}\n\nFor 'Käthe und ich' specifically, search for:\n- 'Käthe und ich episoden reihenfolge'\n- 'Käthe und ich chronologie'\n- Production years and air dates to determine correct order", 
              query, 
              urlencoding::encode(&series_name),
              urlencoding::encode(query),
              urlencoding::encode(query)))
}

/// Scrape DuckDuckGo search results from HTML
fn scrape_duckduckgo_results(html: &str) -> Result<String> {
    let document = Html::parse_document(html);
    let result_selector = Selector::parse("div.result").unwrap();
    let title_selector = Selector::parse("a.result__a").unwrap();
    let snippet_selector = Selector::parse("a.result__snippet").unwrap();

    let mut results = Vec::new();

    for (i, element) in document.select(&result_selector).take(5).enumerate() {
        let title = element
            .select(&title_selector)
            .next()
            .map(|e| e.inner_html())
            .unwrap_or_else(|| format!("Result {}", i + 1));

        let snippet = element
            .select(&snippet_selector)
            .next()
            .map(|e| e.inner_html())
            .unwrap_or_default();

        let url = element
            .select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();

        results.push(format!(
            "Title: {}\nURL: {}\nSnippet: {}",
            strip_html_tags(&title),
            url,
            strip_html_tags(&snippet)
        ));
    }

    if results.is_empty() {
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]   DDG scraping: No results found");
        }
        Ok("No search results found.".to_string())
    } else {
        let result_summary = results.join("\n\n---\n\n");
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]   DDG scraping success: {} results, {} chars", results.len(), result_summary.len());
        }
        Ok(result_summary)
    }
}

/// Reads and extracts content from a website
pub async fn read_website_content(url: &str) -> Result<String> {
    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE] AI Tool Call: read_website_content");
        eprintln!("[VERBOSE]   url: \"{}\"", url);
    }

    // Validate URL
    let parsed_url = Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid URL: {}", url))?;
    
    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE]   validated_url: \"{}\"", parsed_url);
    }

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
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]   Content truncated: {} -> {} chars", content.len(), MAX_LENGTH);
        }
        Ok(format!(
            "{}...\n\n[Content truncated to {} characters]",
            &content[..MAX_LENGTH],
            MAX_LENGTH
        ))
    } else {
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]   Content extraction success: {} chars", content.len());
        }
        Ok(content)
    }
}

/// Extract main content from HTML document based on the website
fn extract_main_content(document: &Html, url: &Url) -> Result<String> {
    let host = url.host_str().unwrap_or("");

    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE]   Extracting content from host: \"{}\"", host);
    }

    let selectors = match host {
        h if h.contains("wikipedia.org") => vec![
            "div.mw-parser-output p",
            "div.mw-parser-output li", 
            "table.infobox tr",
            ".episode-list td",
            "table.wikitable tr",
            ".filmography tr",
            "div.mw-parser-output table tr",
        ],
        h if h.contains("fernsehserien.de") => {
            vec![
                "div.serie-info p", 
                "div.episoden-liste tr", 
                "div.content p",
                ".episode-guide tr",
                ".staffel-info tr",
                ".film-info p"
            ]
        }
        h if h.contains("imdb.com") => vec![
            "[data-testid='plot-xl']",
            ".ipc-html-content-inner-div",
            "li[data-testid='title-episode-item']",
            ".episode-item-wrapper",
            ".titleColumn",
        ],
        h if h.contains("tvbutler.de") => {
            vec![".episode-info", ".episode-description", ".show-info p"]
        }
        h if h.contains("filmstarts.de") => {
            vec![".episode-list tr", ".film-synopsis p", ".cast-info p"]
        }
        _ => vec![
            "article p",
            "main p", 
            ".content p",
            ".post p",
            ".entry-content p",
            "div.text p",
            ".article-body p",
            "table tr",
            ".episode-guide tr",
        ],
    };

    let mut extracted_text = Vec::new();

    // Try each selector until we find content
    for selector_str in &selectors {
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]     Trying selector: \"{}\"", selector_str);
        }
        if let Ok(selector) = Selector::parse(selector_str) {
            let elements: Vec<String> = document
                .select(&selector)
                .map(|el| {
                    let text = el.text().collect::<String>();
                    // Preserve table structure and episode information
                    if selector_str.contains("tr") || selector_str.contains("table") {
                        // For table rows, try to preserve structure with pipe separators
                        let cells: Vec<String> = if let Ok(cell_selector) = Selector::parse("td, th") {
                            el.select(&cell_selector)
                                .map(|cell| clean_text(&cell.text().collect::<String>()))
                                .filter(|cell| !cell.is_empty())
                                .collect()
                        } else {
                            Vec::new()
                        };
                        if !cells.is_empty() {
                            cells.join(" | ")
                        } else {
                            clean_text(&text)
                        }
                    } else {
                        clean_text(&text)
                    }
                })
                .filter(|text| {
                    text.len() > 15 && (
                        // Look for episode-related keywords
                        text.to_lowercase().contains("episode") ||
                        text.to_lowercase().contains("folge") ||
                        text.to_lowercase().contains("staffel") ||
                        text.to_lowercase().contains("season") ||
                        text.to_lowercase().contains("erstausstrahlung") ||
                        text.to_lowercase().contains("ausgestrahlt") ||
                        text.contains("2019") || text.contains("2020") || text.contains("2021") || 
                        text.contains("2022") || text.contains("2023") || text.contains("2024") ||
                        text.len() > 30 // General content fallback
                    )
                })
                .take(60) // Increased limit for episode information
                .collect();

            if !elements.is_empty() {
                if std::env::var("VERBOSE").unwrap_or_default() == "1" {
                    eprintln!("[VERBOSE]       Found {} elements with selector \"{}\"", elements.len(), selector_str);
                }
                extracted_text.extend(elements);
                if extracted_text.len() > 30 { // Increased threshold
                    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
                        eprintln!("[VERBOSE]     Content threshold reached, stopping selector search");
                    }
                    break; // We have enough content
                }
            }
        }
    }

    // If specific selectors didn't work, try general paragraph extraction
    if extracted_text.is_empty() {
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]     Specific selectors failed, trying general paragraph extraction");
        }
        let p_selector = Selector::parse("p").unwrap();
        extracted_text = document
            .select(&p_selector)
            .map(|el| clean_text(&el.text().collect::<String>()))
            .filter(|text| {
                text.len() > 25 && (
                    // Prioritize episode-related content
                    text.to_lowercase().contains("käthe") ||
                    text.to_lowercase().contains("episode") ||
                    text.to_lowercase().contains("folge") ||
                    text.to_lowercase().contains("film") ||
                    text.to_lowercase().contains("reihenfolge") ||
                    text.to_lowercase().contains("chronolog") ||
                    text.contains("201") || text.contains("202") ||
                    text.len() > 40
                )
            })
            .take(40)
            .collect();
    }

    if extracted_text.is_empty() {
        if std::env::var("VERBOSE").unwrap_or_default() == "1" {
            eprintln!("[VERBOSE]     No meaningful content extracted from webpage");
        }
        return Err(anyhow::anyhow!(
            "Could not extract meaningful content from the webpage"
        ));
    }

    if std::env::var("VERBOSE").unwrap_or_default() == "1" {
        eprintln!("[VERBOSE]     Successfully extracted {} text blocks", extracted_text.len());
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
