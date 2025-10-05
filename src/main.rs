use anyhow::Result;
use chrono::DateTime;
use clap::{Parser, Subcommand};
use colored::Colorize;
use mediathekviewweb::{
    models::{SortField, SortOrder},
    Mediathek,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use std::fs::File;
use std::io::Write;

use std::process::Command;

mod ai;
mod logging;
use ai::AIProcessor;
use logging::init_tracing;

#[derive(Parser)]
#[command(name = "mwb")]
#[command(about = "MediathekViewWeb CLI - Search German public broadcasting content")]
#[command(version = "1.0")]
struct Cli {
    /// Enable verbose logging
    #[arg(long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug)]
struct SearchParams {
    query_terms: Vec<String>,
    exclude_patterns: Option<Vec<String>>,
    include_patterns: Option<Vec<String>>,
    size: u32,
    offset: u32,
    sort_by: String,
    sort_order: String,
    exclude_future: bool,
    format: String,
    vlc: Option<String>,
    vlc_ai: Option<String>,
    xspf_file: bool,
    count: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for content
    Search {
        /// Search query (supports `MediathekView` syntax: !channel #topic +title *description >duration <duration)
        /// Duration examples: ">90" (longer than 90min), "<30" (shorter than 30min), ">60 <120" (between 60-120min)
        #[arg(required = true)]
        query: Vec<String>,

        /// Exclude regex patterns (space-separated)
        #[arg(short, long)]
        exclude: Option<Vec<String>>,

        /// Include regex patterns - only show results matching these patterns (space-separated)
        #[arg(short, long)]
        include: Option<Vec<String>>,

        /// Maximum number of results
        #[arg(short, long, default_value = "15")]
        size: u32,

        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: u32,

        /// Sort by field (timestamp, duration, channel)
        #[arg(short = 'b', long, default_value = "timestamp")]
        sort_by: String,

        /// Sort order (asc or desc)
        #[arg(short = 'r', long, default_value = "desc")]
        sort_order: String,

        /// Exclude future content (default: include future content)
        #[arg(long = "no-future")]
        exclude_future: bool,

        /// Output format (table, json, csv, xspf, oneline, onelinetheme, theme-count)
        #[arg(short = 'f', long, default_value = "onelinetheme")]
        format: String,

        /// Show only the count of results
        #[arg(short = 'c', long)]
        count: bool,

        /// Save video links as XSPF playlist and launch VLC with quality option (l=low, m=medium/default, h=HD)
        #[arg(short = 'v', long, value_name = "QUALITY", require_equals = true, num_args = 0..=1, default_missing_value = "m")]
        vlc: Option<String>,

        /// Process results with AI (Gemini) for chronological sorting, deduplication, and VLC playlist creation
        /// If provided with a value, uses that information for AI web search to find the Wikipedia page
        #[arg(long = "vlc-ai", value_name = "SEARCH_INFO", require_equals = true, num_args = 0..=1, default_missing_value = "")]
        vlc_ai: Option<String>,

        /// Save XSPF playlist to file (use with -f xspf)
        #[arg(short = 'x', long)]
        xspf_file: bool,
    },
    /// List available channels
    Channels,
}

const USER_AGENT: &str = "mwb-cli/1.0";

fn get_search_info(search_info: Option<&str>) -> Result<Option<String>> {
    match search_info {
        Some("clipboard") => {
            tracing::info!("Detected clipboard parameter, attempting to read clipboard content");

            match arboard::Clipboard::new() {
                Ok(mut clipboard) => match clipboard.get_text() {
                    Ok(content) => {
                        let trimmed = content.trim();
                        if trimmed.is_empty() {
                            tracing::warn!("Clipboard is empty");
                            println!(
                                "{}",
                                "⚠️  Clipboard is empty, proceeding without search info".yellow()
                            );
                            Ok(None)
                        } else {
                            tracing::info!(clipboard_length = %trimmed.len(), "Successfully read clipboard content");
                            println!(
                                "{}",
                                format!(
                                    "📋 Using clipboard content: {}",
                                    if trimmed.len() > 50 {
                                        format!("{}...", &trimmed[..50])
                                    } else {
                                        trimmed.to_string()
                                    }
                                )
                                .cyan()
                            );
                            Ok(Some(trimmed.to_string()))
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to read clipboard content");
                        println!("{}", format!("❌ Failed to read clipboard: {}", e).red());
                        println!("{}", "📋 Proceeding without search info".yellow());
                        Ok(None)
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "Failed to initialize clipboard");
                    println!("{}", format!("❌ Failed to access clipboard: {}", e).red());
                    println!("{}", "📋 Proceeding without search info".yellow());
                    Ok(None)
                }
            }
        }
        other => Ok(other.map(|s| s.to_string())),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on global verbose flag
    init_tracing(cli.verbose);

    let client = Mediathek::new(USER_AGENT.parse()?)?;

    match cli.command {
        Commands::Search {
            query,
            exclude,
            include,
            size,
            offset,
            sort_by,
            sort_order,
            exclude_future,
            format,
            vlc,
            vlc_ai,
            xspf_file,
            count,
        } => {
            let params = SearchParams {
                query_terms: query,
                exclude_patterns: exclude,
                include_patterns: include,
                size,
                offset,
                sort_by,
                sort_order,
                exclude_future,
                format,
                vlc,
                vlc_ai,
                xspf_file,
                count,
            };
            search_content(&client, params).await?;
        }
        Commands::Channels => {
            list_channels(&client).await?;
        }
    }

    Ok(())
}

async fn search_content(client: &Mediathek, params: SearchParams) -> Result<()> {
    // Multi-search mode: perform separate searches for each query term
    if params.query_terms.len() > 1 {
        return multi_search_content(client, params).await;
    }

    let query_string = params.query_terms.join(" ");

    // Preprocess query to extract duration selectors and search terms
    let (search_terms_only, duration_filters) = extract_duration_selectors(&query_string);

    // Build the query using the mediathekviewweb crate
    // Use search terms without duration selectors for natural all-field search
    let mut query_builder = if search_terms_only.is_empty() {
        // Duration-only query
        client.query_string("", false)
    } else {
        // Let the API handle natural search across all fields
        client.query_string(&search_terms_only, false)
    };

    tracing::info!(
        original_query = %query_string,
        duration_filters = ?duration_filters,
        search_terms = %search_terms_only,
        size = %params.size,
        offset = %params.offset,
        sort_by = %params.sort_by,
        sort_order = %params.sort_order,
        exclude_future = %params.exclude_future,
        exclude_patterns = ?params.exclude_patterns,
        include_patterns = ?params.include_patterns,
        "Starting MediathekView search request"
    );

    // Apply duration filters extracted from the query
    for filter in duration_filters {
        if let Some(duration_str) = filter.strip_prefix('>') {
            if let Ok(min_duration) = duration_str.parse::<u64>() {
                query_builder =
                    query_builder.duration_min(std::time::Duration::from_secs(min_duration * 60));
            }
        } else if let Some(duration_str) = filter.strip_prefix('<') {
            if let Ok(max_duration) = duration_str.parse::<u64>() {
                query_builder =
                    query_builder.duration_max(std::time::Duration::from_secs(max_duration * 60));
            }
        }
    }

    // Apply other parameters
    query_builder = query_builder
        .include_future(!params.exclude_future)
        .size(params.size as usize)
        .offset(params.offset as usize);

    // Apply sorting
    let sort_field = match params.sort_by.as_str() {
        "duration" => SortField::Duration,
        "channel" => SortField::Channel,
        _ => SortField::Timestamp, // includes "timestamp" and default
    };

    let sort_direction = match params.sort_order.as_str() {
        "asc" => SortOrder::Ascending,
        _ => SortOrder::Descending,
    };

    query_builder = query_builder.sort_by(sort_field).sort_order(sort_direction);

    // Execute the query
    let start_time = Instant::now();

    tracing::info!("Executing MediathekView API request");

    let result = query_builder.send().await?;

    let duration = start_time.elapsed();
    tracing::info!(
        response_time_ms = %duration.as_millis(),
        results_found = %result.results.len(),
        total_available = %result.query_info.total_results,
        "MediathekView API request completed"
    );

    // Save original count before moving results
    let original_count = result.results.len();

    // Apply client-side regex filters
    let filtered_results = apply_regex_filters(
        result.results,
        params.exclude_patterns,
        params.include_patterns,
    )?;

    if filtered_results.len() != original_count {
        tracing::info!(
            before_count = %original_count,
            after_count = %filtered_results.len(),
            "Results filtered by regex patterns"
        );
    }

    if params.count {
        println!("{}", filtered_results.len());
    } else if params.vlc_ai.is_some() {
        let search_info = get_search_info(params.vlc_ai.as_deref())?;
        process_with_ai(&filtered_results, search_info.as_deref()).await?;
    } else if let Some(quality) = params.vlc {
        // Validate quality parameter and set default if invalid
        let validated_quality = match quality.as_str() {
            "l" | "low" => "l",
            "h" | "hd" | "high" => "h",
            "m" | "medium" | "" => "m",
            _ => {
                println!("{}", format!("Warning: Invalid quality '{quality}'. Using medium quality (m). Valid options: l (low), m (medium), h (HD)").yellow());
                "m"
            }
        };
        create_vlc_playlist_and_launch(&filtered_results, &params.query_terms, validated_quality)?;
    } else {
        match params.format.as_str() {
            "json" => {
                print_json(&filtered_results)?;
            }
            "csv" => {
                print_csv(&filtered_results);
            }
            "xspf" => {
                if params.xspf_file {
                    save_xspf_playlist(&filtered_results, &params.query_terms)?;
                } else {
                    print_xspf(&filtered_results, &params.query_terms.join(" "));
                }
            }
            "oneline" => {
                print_oneline(&filtered_results);
            }
            "onelinetheme" => {
                print_oneline_theme(&filtered_results);
            }
            "theme-count" => {
                print_theme_count_table(&filtered_results);
            }
            _ => {
                print_table(&filtered_results, &result.query_info);
            }
        }
    }

    Ok(())
}

async fn multi_search_content(client: &Mediathek, params: SearchParams) -> Result<()> {
    use std::collections::HashSet;

    tracing::info!(
        search_terms = ?params.query_terms,
        total_searches = %params.query_terms.len(),
        "Starting multi-search mode"
    );

    let mut all_results = Vec::new();
    let mut seen_urls = HashSet::new(); // For deduplication

    // Perform separate search for each query term
    for (index, query_term) in params.query_terms.iter().enumerate() {
        tracing::info!(
            query_term = %query_term,
            search_index = %(index + 1),
            total_searches = %params.query_terms.len(),
            "Executing individual search"
        );

        // Create params for individual search
        let individual_params = SearchParams {
            query_terms: vec![query_term.clone()],
            exclude_patterns: params.exclude_patterns.clone(),
            include_patterns: params.include_patterns.clone(),
            size: params.size,
            offset: params.offset,
            sort_by: params.sort_by.clone(),
            sort_order: params.sort_order.clone(),
            exclude_future: params.exclude_future,
            format: params.format.clone(),
            vlc: params.vlc.clone(),
            vlc_ai: params.vlc_ai.clone(),
            xspf_file: params.xspf_file,
            count: params.count,
        };

        // Perform individual search
        let query_string = query_term.clone();
        let (search_terms_only, duration_filters) = extract_duration_selectors(&query_string);

        let mut query_builder = if search_terms_only.is_empty() {
            client.query_string("", false)
        } else {
            client.query_string(&search_terms_only, false)
        };

        // Apply duration filters
        for filter in duration_filters {
            if let Some(duration_str) = filter.strip_prefix('>') {
                if let Ok(min_duration) = duration_str.parse::<u64>() {
                    query_builder = query_builder
                        .duration_min(std::time::Duration::from_secs(min_duration * 60));
                }
            } else if let Some(duration_str) = filter.strip_prefix('<') {
                if let Ok(max_duration) = duration_str.parse::<u64>() {
                    query_builder = query_builder
                        .duration_max(std::time::Duration::from_secs(max_duration * 60));
                }
            }
        }

        // Apply other parameters
        query_builder = query_builder
            .include_future(!individual_params.exclude_future)
            .size(individual_params.size as usize)
            .offset(individual_params.offset as usize);

        // Apply sorting
        let sort_field = match individual_params.sort_by.as_str() {
            "duration" => SortField::Duration,
            "channel" => SortField::Channel,
            _ => SortField::Timestamp,
        };

        let sort_direction = match individual_params.sort_order.as_str() {
            "asc" => SortOrder::Ascending,
            _ => SortOrder::Descending,
        };

        query_builder = query_builder.sort_by(sort_field).sort_order(sort_direction);

        // Execute the query
        let result = query_builder.send().await?;

        tracing::info!(
            query_term = %query_term,
            result_count = %result.results.len(),
            "Search completed"
        );

        // Add results with deduplication based on URL
        for item in result.results {
            if seen_urls.insert(item.url_video.clone()) {
                all_results.push(item);
            }
        }
    }

    tracing::info!(
        total_unique_results = %all_results.len(),
        "Multi-search completed"
    );

    // Sort unified results according to specified sort parameters
    all_results.sort_by(|a, b| {
        match params.sort_by.as_str() {
            "duration" => {
                let duration_a = a.duration.map(|d| d.as_secs()).unwrap_or(0);
                let duration_b = b.duration.map(|d| d.as_secs()).unwrap_or(0);
                match params.sort_order.as_str() {
                    "asc" => duration_a.cmp(&duration_b),
                    _ => duration_b.cmp(&duration_a),
                }
            }
            "channel" => match params.sort_order.as_str() {
                "asc" => a.channel.cmp(&b.channel),
                _ => b.channel.cmp(&a.channel),
            },
            _ => {
                // timestamp (default)
                match params.sort_order.as_str() {
                    "asc" => a.timestamp.cmp(&b.timestamp),
                    _ => b.timestamp.cmp(&a.timestamp),
                }
            }
        }
    });

    // Apply client-side regex filters to unified results
    // Save count before moving results
    let original_count = all_results.len();

    // Apply client-side regex filters
    let filtered_results = apply_regex_filters(
        all_results,
        params.exclude_patterns,
        params.include_patterns,
    )?;

    if filtered_results.len() != original_count {
        tracing::info!(
            before_count = %original_count,
            after_count = %filtered_results.len(),
            "Results filtered by regex patterns"
        );
    }

    // Output results using the same logic as single search
    if params.count {
        println!("{}", filtered_results.len());
    } else if params.vlc_ai.is_some() {
        let search_info = get_search_info(params.vlc_ai.as_deref())?;
        process_with_ai(&filtered_results, search_info.as_deref()).await?;
    } else if let Some(quality) = params.vlc {
        let validated_quality = match quality.as_str() {
            "l" | "low" => "l",
            "h" | "hd" | "high" => "h",
            "m" | "medium" | "" => "m",
            _ => {
                println!("{}", format!("Warning: Invalid quality '{quality}'. Using medium quality (m). Valid options: l (low), m (medium), h (HD)").yellow());
                "m"
            }
        };
        create_vlc_playlist_and_launch(&filtered_results, &params.query_terms, validated_quality)?;
    } else {
        match params.format.as_str() {
            "json" => {
                print_json(&filtered_results)?;
            }
            "csv" => {
                print_csv(&filtered_results);
            }
            "xspf" => {
                if params.xspf_file {
                    save_xspf_playlist(&filtered_results, &params.query_terms)?;
                } else {
                    print_xspf(&filtered_results, &params.query_terms.join(" "));
                }
            }
            "oneline" => {
                print_oneline(&filtered_results);
            }
            "onelinetheme" => {
                print_oneline_theme(&filtered_results);
            }
            "theme-count" => {
                print_theme_count_table(&filtered_results);
            }
            _ => {
                // Create a mock QueryInfo for table display
                let query_info = mediathekviewweb::models::QueryInfo {
                    filmliste_timestamp: 0,
                    result_count: filtered_results.len(),
                    search_engine_time: std::time::Duration::from_millis(0),
                    total_results: filtered_results.len() as u64,
                };
                print_table(&filtered_results, &query_info);
            }
        }
    }

    Ok(())
}

fn extract_duration_selectors(query: &str) -> (String, Vec<String>) {
    // Check if query contains duration selectors (>X or <X patterns)
    let duration_pattern = regex::Regex::new(r"[><]\d+").unwrap();

    if !duration_pattern.is_match(query) {
        // No duration selectors, return original query and empty filters
        return (query.to_string(), Vec::new());
    }

    // Split query into tokens
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut search_terms = Vec::new();
    let mut duration_selectors = Vec::new();

    for token in tokens {
        if duration_pattern.is_match(token) {
            duration_selectors.push(token.to_string());
        } else {
            // Keep all other tokens (search terms and selectors) as-is
            search_terms.push(token);
        }
    }

    // Return search terms and duration filters separately
    let search_query = search_terms.join(" ");
    (search_query, duration_selectors)
}

fn apply_regex_filters(
    results: Vec<mediathekviewweb::models::Item>,
    exclude_patterns: Option<Vec<String>>,
    include_patterns: Option<Vec<String>>,
) -> Result<Vec<mediathekviewweb::models::Item>> {
    let mut filtered_results = results;

    // Apply exclude regex patterns
    if let Some(exclude_terms) = exclude_patterns {
        if !exclude_terms.is_empty() {
            let exclude_regexes: Result<Vec<Regex>, _> = exclude_terms
                .iter()
                .map(|pattern| Regex::new(&format!("(?i){pattern}")))
                .collect();

            let exclude_regexes =
                exclude_regexes.map_err(|e| anyhow::anyhow!("Invalid exclude regex: {}", e))?;

            filtered_results.retain(|entry| {
                let text_fields = [
                    entry.channel.as_str(),
                    &entry.topic,
                    &entry.title,
                    entry.description.as_deref().unwrap_or(""),
                ];

                let combined_text = text_fields.join(" ");

                // Return true (keep) if none of the exclude patterns match
                !exclude_regexes
                    .iter()
                    .any(|pattern| pattern.is_match(&combined_text))
            });
        }
    }

    // Apply include regex patterns
    if let Some(include_terms) = include_patterns {
        if !include_terms.is_empty() {
            let include_regexes: Result<Vec<Regex>, _> = include_terms
                .iter()
                .map(|pattern| Regex::new(&format!("(?i){pattern}")))
                .collect();

            let include_regexes =
                include_regexes.map_err(|e| anyhow::anyhow!("Invalid include regex: {}", e))?;

            filtered_results.retain(|entry| {
                let text_fields = [
                    entry.channel.as_str(),
                    &entry.topic,
                    &entry.title,
                    entry.description.as_deref().unwrap_or(""),
                ];

                let combined_text = text_fields.join(" ");

                // Return true (keep) if any of the include patterns match
                include_regexes
                    .iter()
                    .any(|pattern| pattern.is_match(&combined_text))
            });
        }
    }

    Ok(filtered_results)
}

async fn list_channels(client: &Mediathek) -> Result<()> {
    // Get channels by making a wildcard query and extracting unique channels
    let result = client.query_string("", true).size(1000).send().await?;
    let mut channels: Vec<String> = result
        .results
        .iter()
        .map(|item| item.channel.clone())
        .collect();
    channels.sort();
    channels.dedup();

    println!("{}", "Available Channels:".bold().blue());
    println!();

    for (i, channel) in channels.iter().enumerate() {
        if i % 4 == 0 && i > 0 {
            println!();
        }
        print!("{:<20}", channel.green());
    }
    println!();
    println!();
    println!(
        "{}: Use {} to filter by channel",
        "Tip".yellow(),
        "!CHANNEL".cyan()
    );
    println!(
        "{}: Use {} for duration filtering",
        "Tip".yellow(),
        ">90 <120".cyan()
    );

    Ok(())
}

fn create_vlc_playlist_and_launch(
    results: &[mediathekviewweb::models::Item],
    query_terms: &[String],
    quality: &str,
) -> Result<()> {
    if results.is_empty() {
        println!("{}", "No results found to add to playlist.".yellow());
        return Ok(());
    }

    // Create playlist filename from query (now XSPF)
    let playlist_name = generate_vlc_playlist_filename(&query_terms.join(" "));

    // Generate XSPF content
    let xspf_content = generate_xspf_content(results, &query_terms.join(" "), quality);

    // Write to file
    let mut file = File::create(&playlist_name)?;
    writeln!(file, "{xspf_content}")?;

    println!(
        "{}",
        format!("Created XSPF playlist: {playlist_name}").green()
    );
    println!(
        "{}",
        format!("Added {} video(s) to playlist", results.len()).green()
    );

    // Try to launch VLC with the playlist
    println!("{}", "Launching VLC...".yellow());

    let vlc_result = if cfg!(target_os = "windows") {
        // Try common VLC paths on Windows
        Command::new("vlc")
            .arg(&playlist_name)
            .spawn()
            .or_else(|_| {
                Command::new("C:\\Program Files\\VideoLAN\\VLC\\vlc.exe")
                    .arg(&playlist_name)
                    .spawn()
            })
            .or_else(|_| {
                Command::new("C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe")
                    .arg(&playlist_name)
                    .spawn()
            })
    } else {
        // Try VLC on Unix-like systems
        Command::new("vlc").arg(&playlist_name).spawn()
    };

    match vlc_result {
        Ok(_) => {
            println!("{}", "VLC launched successfully!".green());
        }
        Err(e) => {
            println!("{}", format!("Failed to launch VLC: {e}").red());
            println!("{}", format!("Playlist saved as: {playlist_name}").yellow());
            println!("{}", "You can manually open this file with VLC.".yellow());
        }
    }

    Ok(())
}

fn generate_vlc_playlist_filename(query: &str) -> String {
    // Sanitize the query for use as filename
    let sanitized = query
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c,
            '>' | '<' => 'm', // Convert > to "more", < to "less" indicator
            _ => '_',         // includes spaces and all other characters
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    // Limit filename length and add timestamp suffix for uniqueness
    let max_len = 50;
    let truncated = if sanitized.len() > max_len {
        let partial = &sanitized[..max_len];
        format!("{partial}...")
    } else {
        sanitized
    };

    // Add short timestamp to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        % 10000; // Last 4 digits

    format!("mwb_{truncated}_{timestamp}.xspf")
}

async fn process_with_ai(
    results: &[mediathekviewweb::models::Item],
    search_info: Option<&str>,
) -> Result<()> {
    if results.is_empty() {
        println!("{}", "No results found to process with AI.".yellow());
        return Ok(());
    }

    // Load environment variables from .env file if it exists
    dotenvy::dotenv().ok();

    println!("{}", "🚀 Initializing Gemini AI processor...".yellow());

    let processor = match AIProcessor::new_with_verbose(search_info).await {
        Ok(processor) => processor,
        Err(e) => {
            println!(
                "{}",
                format!("❌ Failed to initialize AI processor: {}", e).red()
            );
            println!(
                "{}",
                "💡 Make sure you have set GOOGLE_API_KEY in your environment or .env file"
                    .yellow()
            );
            println!(
                "{}",
                "   You can get an API key from: https://aistudio.google.com/app/apikey".cyan()
            );
            return Ok(());
        }
    };

    match processor.process_episodes(results).await {
        Ok(response) => {
            println!("\n{}", "✅ AI Processing Results:".green().bold());
            println!("{}", "=".repeat(50).green());
            println!("{}", response);
            println!("{}", "=".repeat(50).green());

            // Optionally save the results to a file
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            let filename = format!("ai_sorted_episodes_{}.txt", timestamp);

            if let Ok(mut file) = File::create(&filename) {
                writeln!(
                    file,
                    "AI Sorted Episodes - Generated on {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                )?;
                writeln!(file, "{}", "=".repeat(70))?;
                writeln!(file, "{}", response)?;
                println!("\n{}", format!("📄 Results saved to: {}", filename).cyan());
            }
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();

            if error_msg.contains("401")
                || error_msg.contains("unauthorized")
                || error_msg.contains("api key")
            {
                println!("{}", "🔑 API Key Issue Detected!".yellow().bold());
                println!();
                println!("{}", "❌ There's a problem with your Google API key.".red());
                println!();
                println!("{}", "💡 To fix this:".cyan().bold());
                println!(
                    "{}",
                    "   1. Visit: https://aistudio.google.com/app/u/5/apikey".cyan()
                );
                println!("{}", "   2. Generate a new API key if needed".cyan());
                println!(
                    "{}",
                    "   3. Copy the key to your .env file as GOOGLE_API_KEY=your_key_here".cyan()
                );
                println!();
                println!("{}", "🌐 Opening API key page in your browser...".green());

                // Try to open the API key page in browser
                let url = "https://aistudio.google.com/app/u/5/apikey";
                let _ = open_browser_url(url);
            } else if error_msg.contains("429")
                || error_msg.contains("quota")
                || error_msg.contains("rate limit")
            {
                println!("{}", "⏱️  API Quota/Rate Limit Exceeded!".yellow().bold());
                println!();
                println!("{}", "❌ You've exceeded the API quota limits.".red());
                println!();
                println!("{}", "💡 Solutions:".cyan().bold());
                println!("{}", "   1. Wait a few minutes and try again".cyan());
                println!(
                    "{}",
                    "   2. Check your quota limits at the API console".cyan()
                );
                println!(
                    "{}",
                    "   3. Consider upgrading to a paid plan for higher limits".cyan()
                );
                println!();
                println!(
                    "{}",
                    "🌐 Opening Google AI Studio to check your usage...".green()
                );

                let url = "https://aistudio.google.com/app/u/5/apikey";
                let _ = open_browser_url(url);
            } else {
                println!("{}", format!("❌ AI processing failed: {}", e).red());
                println!("{}", "💡 The AI might need more specific episode information or the search tools might be having issues".yellow());
            }
        }
    }

    Ok(())
}

/// Open URL in the default browser
fn open_browser_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd").args(["/C", "start", url]).spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(url).spawn()?;
    }

    Ok(())
}

fn print_table(
    results: &[mediathekviewweb::models::Item],
    query_info: &mediathekviewweb::models::QueryInfo,
) {
    println!("{}", "Search Results".bold().blue());
    println!(
        "Total results: {}",
        query_info.total_results.to_string().green()
    );
    println!("Showing: {}", query_info.result_count.to_string().green());
    let search_time = query_info.search_engine_time.as_millis();
    println!("Search time: {}ms", format!("{search_time:.2}").yellow());
    println!();

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return;
    }

    for (i, entry) in results.iter().enumerate() {
        let entry_num = i + 1;
        println!(
            "{} {}",
            format!("{entry_num}.").blue().bold(),
            "─".repeat(60).blue()
        );

        println!("{}: {}", "Channel".bold(), entry.channel.green());
        println!("{}: {}", "Theme".bold(), entry.topic.cyan());
        println!("{}: {}", "Title".bold(), entry.title.bright_white());

        let duration_secs = entry.duration.map_or(0, |d| d.as_secs());
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let seconds = duration_secs % 60;

        if hours > 0 {
            println!(
                "{}: {}h {}m {}s",
                "Duration".bold(),
                hours,
                minutes,
                seconds
            );
        } else {
            println!("{}: {}m {}s", "Duration".bold(), minutes, seconds);
        }

        if let Some(dt) = DateTime::from_timestamp(entry.timestamp, 0) {
            println!(
                "{}: {}",
                "Date".bold(),
                dt.format("%Y-%m-%d %H:%M").to_string().yellow()
            );
        }

        println!("{}: {}", "Video URL".bold(), entry.url_video.bright_blue());

        if let Some(ref description) = entry.description {
            if !description.is_empty() && description.len() > 10 {
                let desc = if description.chars().count() > 200 {
                    let truncated: String = description.chars().take(200).collect();
                    format!("{truncated}...")
                } else {
                    description.clone()
                };
                println!("{}: {}", "Description".bold(), desc.bright_black());
            }
        }

        println!();
    }
}

fn print_csv(results: &[mediathekviewweb::models::Item]) {
    println!("Channel,Theme,Title,Duration,Date,URL,Description");

    for entry in results {
        let duration = entry
            .duration
            .map_or("0".to_string(), |d| d.as_secs().to_string());
        let date = DateTime::from_timestamp(entry.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        println!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
            entry.channel.replace('"', "\"\""),
            entry.topic.replace('"', "\"\""),
            entry.title.replace('"', "\"\""),
            duration,
            date,
            entry.url_video,
            entry
                .description
                .as_deref()
                .unwrap_or("")
                .replace('"', "\"\"")
        );
    }
}

#[derive(Serialize, Deserialize)]
struct JsonItem {
    channel: String,
    topic: String,
    title: String,
    timestamp: i64,
    date_human: String,
    duration_seconds: Option<u64>,
    duration_human: Option<String>,
    url_video: String,
    url_video_low: Option<String>,
    url_video_hd: Option<String>,
    description: Option<String>,
}

fn print_json(results: &[mediathekviewweb::models::Item]) -> Result<()> {
    let json_items: Vec<JsonItem> = results
        .iter()
        .map(|entry| {
            let date_human = DateTime::from_timestamp(entry.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            let duration_seconds = entry.duration.map(|d| d.as_secs());
            let duration_human = entry.duration.map(|d| {
                let total_secs = d.as_secs();
                let hours = total_secs / 3600;
                let minutes = (total_secs % 3600) / 60;
                let seconds = total_secs % 60;

                if hours > 0 {
                    format!("{}h {}m {}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            });

            JsonItem {
                channel: entry.channel.clone(),
                topic: entry.topic.clone(),
                title: entry.title.clone(),
                timestamp: entry.timestamp,
                date_human,
                duration_seconds,
                duration_human,
                url_video: entry.url_video.clone(),
                url_video_low: entry.url_video_low.clone(),
                url_video_hd: entry.url_video_hd.clone(),
                description: entry.description.clone(),
            }
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_items)?);
    Ok(())
}

fn print_oneline(results: &[mediathekviewweb::models::Item]) {
    for entry in results {
        let date = DateTime::from_timestamp(entry.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        let duration = entry
            .duration
            .map_or("".to_string(), |d| format!("{}min", d.as_secs() / 60));

        // Format: [Channel] Title (Date) [Duration] - URL
        println!(
            "[{}] {} ({}) {} - {}",
            entry.channel.bright_cyan(),
            entry.title.bright_white(),
            date.yellow(),
            if duration.is_empty() {
                "".to_string()
            } else {
                format!("[{}]", duration.green())
            },
            entry.url_video.bright_blue()
        );
    }
}

fn print_oneline_theme(results: &[mediathekviewweb::models::Item]) {
    for entry in results {
        let date = DateTime::from_timestamp(entry.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();

        let duration = entry
            .duration
            .map_or("".to_string(), |d| format!("{}min", d.as_secs() / 60));

        // Format: [Channel] Title (Date) [Duration] - Theme
        println!(
            "[{}] {} ({}) {} - {}",
            entry.channel.bright_cyan(),
            entry.title.bright_white(),
            date.yellow(),
            if duration.is_empty() {
                "".to_string()
            } else {
                format!("[{}]", duration.green())
            },
            entry.topic.bright_magenta()
        );
    }
}

fn print_theme_count_table(results: &[mediathekviewweb::models::Item]) {
    use std::collections::HashMap;

    // Count themes
    let mut theme_counts: HashMap<String, u32> = HashMap::new();
    for entry in results {
        *theme_counts.entry(entry.topic.clone()).or_insert(0) += 1;
    }

    // Convert to vector and sort by count (descending)
    let mut sorted_themes: Vec<(String, u32)> = theme_counts.into_iter().collect();
    sorted_themes.sort_by(|a, b| b.1.cmp(&a.1));

    if sorted_themes.is_empty() {
        println!("No themes found.");
        return;
    }

    // Calculate optimal column width based on longest theme name
    let max_theme_length = sorted_themes
        .iter()
        .map(|(theme, _)| theme.len())
        .max()
        .unwrap_or(10);
    let theme_width = std::cmp::max(max_theme_length + 2, 25); // Minimum 25 chars for "Theme" header
    let total_width = theme_width + 10; // +10 for count column and spacing

    // Print header
    println!("{}", "Theme Count Report".bold().underline());
    println!("{}", "─".repeat(total_width));
    println!(
        "{:<width$} {}",
        "Theme".bold(),
        "Count".bold(),
        width = theme_width
    );
    println!("{}", "─".repeat(total_width));

    // Print results
    for (theme, count) in &sorted_themes {
        println!(
            "{:<width$} {}",
            theme.cyan(),
            count.to_string().green().bold(),
            width = theme_width
        );
    }

    println!("{}", "─".repeat(total_width));
    println!(
        "Total unique themes: {}",
        sorted_themes.len().to_string().yellow().bold()
    );
}

fn print_xspf(results: &[mediathekviewweb::models::Item], query: &str) {
    let xspf_content = generate_xspf_content(results, query, "m");
    println!("{xspf_content}");
}

/// Generates complete XSPF playlist content as a string
///
/// This unified function creates XSPF (XML Shareable Playlist Format) content
/// with rich metadata including duration, broadcast dates, and descriptions.
///
/// # Arguments
/// * `results` - Array of `MediathekView` items to include in playlist
/// * `query` - Search query string used for playlist title
///
/// # Returns
/// * `Result<String>` - Complete XSPF XML content or error
fn generate_xspf_content(
    results: &[mediathekviewweb::models::Item],
    query: &str,
    quality: &str,
) -> String {
    // Pre-allocate capacity to reduce reallocations (header + ~512 chars per track)
    let mut content = String::with_capacity(1024 + results.len() * 512);

    content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    content.push_str("<playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n");
    content.push_str("  <title>MediathekView Search: ");
    content.push_str(&escape_xml(query));
    content.push_str("</title>\n");
    content.push_str("  <creator>MWB - MediathekViewWeb CLI</creator>\n");
    content.push_str("  <date>");
    content.push_str(&chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
    content.push_str("</date>\n");
    content.push_str("  <trackList>\n");

    // Generate track entries with metadata
    for entry in results {
        let duration_ms = entry
            .duration
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let date_readable = DateTime::from_timestamp(entry.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        content.push_str("    <track>\n");
        // Include date in title for VLC visibility
        let title_with_date = if date_readable.is_empty() {
            entry.title.clone()
        } else {
            format!("{} ({date_readable})", entry.title)
        };
        content.push_str("      <title>");
        content.push_str(&escape_xml(&title_with_date));
        content.push_str("</title>\n");
        // Use creator for channel, artist for date (VLC displays artist column)
        content.push_str("      <creator>");
        content.push_str(&escape_xml(&entry.channel));
        content.push_str("</creator>\n");
        content.push_str("      <artist>");
        content.push_str(&escape_xml(&date_readable));
        content.push_str("</artist>\n");
        content.push_str("      <album>");
        content.push_str(&escape_xml(&entry.topic));
        content.push_str("</album>\n");
        // Select video URL based on quality parameter
        let video_url = match quality {
            "l" | "low" => entry.url_video_low.as_ref().unwrap_or(&entry.url_video),
            "h" | "hd" | "high" => entry.url_video_hd.as_ref().unwrap_or(&entry.url_video),
            _ => &entry.url_video, // default to medium quality
        };
        content.push_str("      <location>");
        content.push_str(&escape_xml(video_url));
        content.push_str("</location>\n");
        if duration_ms > 0 {
            content.push_str("      <duration>");
            content.push_str(&duration_ms.to_string());
            content.push_str("</duration>\n");
        }
        if let Some(description) = &entry.description {
            if !description.is_empty() {
                content.push_str("      <annotation>");
                content.push_str(&escape_xml(description));
                content.push_str("</annotation>\n");
            }
        }
        content.push_str("    </track>\n");
    }

    content.push_str("  </trackList>\n");
    content.push_str("</playlist>\n");

    content
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn save_xspf_playlist(
    results: &[mediathekviewweb::models::Item],
    query_terms: &[String],
) -> Result<()> {
    if results.is_empty() {
        println!("{}", "No results found to save to playlist.".yellow());
        return Ok(());
    }

    // Create playlist filename from query (similar to VLC playlist naming)
    let playlist_name = generate_xspf_filename(&query_terms.join(" "));

    // Generate XSPF content
    let xspf_content = generate_xspf_content(results, &query_terms.join(" "), "m");

    // Write to file
    let mut file = File::create(&playlist_name)?;
    writeln!(file, "{xspf_content}")?;

    println!(
        "{}",
        format!("Created XSPF playlist: {playlist_name}").green()
    );
    println!(
        "{}",
        format!("Added {} track(s) to playlist", results.len()).green()
    );

    Ok(())
}

fn generate_xspf_filename(query: &str) -> String {
    // Similar to M3U playlist naming but with .xspf extension
    let sanitized_query = query
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .take(3) // Take first 3 words
        .collect::<Vec<_>>()
        .join("_");

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");

    if sanitized_query.is_empty() {
        format!("mwb_playlist_{timestamp}.xspf")
    } else {
        format!("mwb_{sanitized_query}_{timestamp}.xspf")
    }
}
