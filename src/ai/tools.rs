//! Tools module for AI processing
//!
//! Provides web search and content extraction capabilities for the AI processor
//! 
//! ## Structured Logging Implementation
//! 
//! Uses the tracing crate for structured logging throughout all AI tool operations:
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
//! Logging is automatically configured based on the `--verbose` flag passed to the CLI.
//! All trace events use structured fields for better observability and debugging.
//! 

use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::Value;

use url::Url;

/// Performs a web search using DuckDuckGo's instant answer API
/// This is a free alternative to paid search APIs
/// Enhanced for German TV series episode information
pub async fn perform_google_search(query: &str) -> Result<String> {
    tracing::info!(query = %query, "Starting web search");

    let enhanced_query = format!("{} wikipedia", query);

    tracing::debug!(enhanced_query = %enhanced_query, "Enhanced search query");

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
                    tracing::info!(
                        result_length = %result_summary.len(),
                        "DuckDuckGo API search successful"
                    );
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
                tracing::debug!("Attempting to scrape DuckDuckGo HTML results");
                return scrape_duckduckgo_results(&html);
            }
        }
        Err(_) => {
            // DuckDuckGo search failed, try fallback
        }
    }

    // If all else fails, provide suggestions with German-specific sites
    let series_name = query.split_whitespace().take(3).collect::<Vec<&str>>().join("_");
    tracing::warn!("All search methods failed, returning fallback suggestions");
    Ok(format!("Search failed for '{}'. Try these German TV resources:\n- Wikipedia DE: https://de.wikipedia.org/wiki/{}\n- Fernsehserien.de: https://www.fernsehserien.de/suche/{}\n- IMDB: https://www.imdb.com/find?q={}\n\n", 
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
        tracing::warn!("DuckDuckGo scraping found no results");
        return Ok("No search results found".to_string());
    } else {
        let result_summary = results.join("\n\n---\n\n");
        tracing::info!(
            result_count = %results.len(),
            total_length = %result_summary.len(),
            "DuckDuckGo scraping successful"
        );
        return Ok(result_summary);
    }
}

/// Reads and extracts content from a website
pub async fn read_website_content(url: &str) -> Result<String> {
    tracing::info!(url = %url, "Starting website content extraction");

    // Validate URL
    let parsed_url = Url::parse(url).map_err(|_| anyhow::anyhow!("Invalid URL: {}", url))?;
    
    tracing::debug!(validated_url = %parsed_url, "URL validation successful");

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
        tracing::info!(
            original_length = %content.len(),
            truncated_length = %MAX_LENGTH,
            "Content truncated due to size limit"
        );
        Ok(format!(
            "{}...\n\n[Content truncated to {} characters]",
            &content[..MAX_LENGTH],
            MAX_LENGTH
        ))
    } else {
        tracing::info!(content_length = %content.len(), "Content extraction successful");
        Ok(content)
    }
}

/// Extract main content from HTML document based on the website
fn extract_main_content(document: &Html, url: &Url) -> Result<String> {
    let host = url.host_str().unwrap_or("");

    tracing::debug!(host = %host, "Extracting content from website");

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
        tracing::debug!(selector = %selector_str, "Trying CSS selector");
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
                tracing::debug!(
                    element_count = %elements.len(),
                    selector = %selector_str,
                    "Found elements with CSS selector"
                );
                extracted_text.extend(elements);
                if extracted_text.len() > 30 { // Increased threshold
                    tracing::debug!("Content threshold reached, stopping selector search");
                    break;
                }
            }
        }
    }

    // If specific selectors didn't work, try general paragraph extraction
    if extracted_text.is_empty() {
        tracing::debug!("Specific selectors failed, trying general paragraph extraction");
        let p_selector = Selector::parse("p").unwrap();
        extracted_text = document
            .select(&p_selector)
            .map(|el| clean_text(&el.text().collect::<String>()))
            .filter(|text| {
                text.len() > 25 && (
                    // Prioritize episode-related content
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
        tracing::warn!("No meaningful content extracted from webpage");
        return Err(anyhow::anyhow!(
            "Could not extract meaningful content from webpage"
        ));
    }

    tracing::info!(
        text_blocks = %extracted_text.len(),
        "Successfully extracted content blocks"
    );

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
