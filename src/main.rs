use anyhow::Result;
use chrono::DateTime;
use clap::{Parser, Subcommand};
use colored::*;
use mediathekviewweb::{Mediathek, models::{SortField, SortOrder}};
use regex::Regex;
use serde_json;
use std::fs::File;
use std::io::Write;
use std::process::Command;


#[derive(Parser)]
#[command(name = "mwb")]
#[command(about = "MediathekViewWeb CLI - Search German public broadcasting content")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for content
    Search {
        /// Search query (supports MediathekView syntax: !channel #topic +title *description >duration <duration)
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
        
        /// Output format (table, json, csv)
        #[arg(short = 'f', long, default_value = "table")]
        format: String,
        
        /// Save video links as VLC playlist and launch VLC
        #[arg(short = 'v', long)]
        vlc: bool,
        

    },
    /// List available channels
    Channels,
}

const USER_AGENT: &str = "mwb-cli/1.0";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
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
        } => {
            search_content(
                &client,
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
            ).await?;
        }
        Commands::Channels => {
            list_channels(&client).await?;
        }
    }

    Ok(())
}

async fn search_content(
    client: &Mediathek,
    query_terms: Vec<String>,
    exclude_patterns: Option<Vec<String>>,
    include_patterns: Option<Vec<String>>,
    size: u32,
    offset: u32,
    sort_by: String,
    sort_order: String,
    exclude_future: bool,
    format: String,
    vlc: bool,
) -> Result<()> {
    let query_string = query_terms.join(" ");
    
    // Preprocess query to handle duration selectors mixed with search terms
    let processed_query = preprocess_query(&query_string);
    

    
    // Build the query using the mediathekviewweb crate
    // Use the built-in query_string method to parse MediathekView syntax including duration selectors
    let mut query_builder = client.query_string(&processed_query, false);
    
    // Apply other parameters
    query_builder = query_builder
        .include_future(!exclude_future)
        .size(size as usize)
        .offset(offset as usize);
    
    // Apply sorting
    let sort_field = match sort_by.as_str() {
        "timestamp" => SortField::Timestamp,
        "duration" => SortField::Duration,
        "channel" => SortField::Channel,
        _ => SortField::Timestamp,
    };
    
    let sort_direction = match sort_order.as_str() {
        "asc" => SortOrder::Ascending,
        _ => SortOrder::Descending,
    };
    
    query_builder = query_builder
        .sort_by(sort_field)
        .sort_order(sort_direction);
    
    // Execute the query
    let result = query_builder.send().await?;
    
    // Apply client-side regex filters
    let filtered_results = apply_regex_filters(result.results, exclude_patterns, include_patterns)?;

    if vlc {
        create_vlc_playlist_and_launch(&filtered_results)?;
    } else {
        match format.as_str() {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&filtered_results)?);
            }
            "csv" => {
                print_csv(&filtered_results)?;
            }
            _ => {
                print_table(&filtered_results, &result.query_info)?;
            }
        }
    }

    Ok(())
}

fn preprocess_query(query: &str) -> String {
    // Check if query contains duration selectors (>X or <X patterns)
    let duration_pattern = regex::Regex::new(r"[><]\d+").unwrap();
    
    if !duration_pattern.is_match(query) {
        // No duration selectors, return as-is
        return query.to_string();
    }
    
    // Split query into tokens
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut search_terms = Vec::new();
    let mut duration_selectors = Vec::new();
    let mut has_selectors = false;
    
    for token in tokens {
        if duration_pattern.is_match(token) {
            duration_selectors.push(token.to_string());
        } else if token.starts_with('!') || token.starts_with('#') || 
                 token.starts_with('+') || token.starts_with('*') {
            // This is already a proper selector
            has_selectors = true;
            search_terms.push(token.to_string());
        } else {
            // Regular search term
            search_terms.push(token.to_string());
        }
    }
    
    // If we have regular search terms without selectors, and we have duration selectors,
    // convert search terms to make them work better with duration filtering
    if !has_selectors && !search_terms.is_empty() && !duration_selectors.is_empty() {
        // For single terms, use title selector for broader matching than topic
        if search_terms.len() == 1 {
            if let Some(first_term) = search_terms.first_mut() {
                *first_term = format!("+{}", first_term);
            }
        } else {
            // For multiple terms, use description search which is most inclusive
            for term in &mut search_terms {
                *term = format!("*{}", term);
            }
        }
    }
    
    // Reconstruct query
    let mut result_terms = search_terms;
    result_terms.extend(duration_selectors);
    result_terms.join(" ")
}

fn apply_regex_filters(
    results: Vec<mediathekviewweb::models::Item>,
    exclude_patterns: Option<Vec<String>>,
    include_patterns: Option<Vec<String>>
) -> Result<Vec<mediathekviewweb::models::Item>> {
    let mut filtered_results = results;
    
    // Apply exclude regex patterns
    if let Some(exclude_terms) = exclude_patterns {
        if !exclude_terms.is_empty() {
            let exclude_regexes: Result<Vec<Regex>, _> = exclude_terms
                .iter()
                .map(|pattern| Regex::new(&format!("(?i){}", pattern)))
                .collect();
            
            let exclude_regexes = exclude_regexes.map_err(|e| anyhow::anyhow!("Invalid exclude regex: {}", e))?;
            
            filtered_results = filtered_results
                .into_iter()
                .filter(|entry| {
                    let text_fields = vec![
                        entry.channel.as_str(),
                        &entry.topic,
                        &entry.title,
                        entry.description.as_deref().unwrap_or(""),
                    ];
                    
                    let combined_text = text_fields.join(" ");
                    
                    // Return true (keep) if none of the exclude patterns match
                    !exclude_regexes.iter().any(|pattern| pattern.is_match(&combined_text))
                })
                .collect();
        }
    }
    
    // Apply include regex patterns
    if let Some(include_terms) = include_patterns {
        if !include_terms.is_empty() {
            let include_regexes: Result<Vec<Regex>, _> = include_terms
                .iter()
                .map(|pattern| Regex::new(&format!("(?i){}", pattern)))
                .collect();
            
            let include_regexes = include_regexes.map_err(|e| anyhow::anyhow!("Invalid include regex: {}", e))?;
            
            filtered_results = filtered_results
                .into_iter()
                .filter(|entry| {
                    let text_fields = vec![
                        entry.channel.as_str(),
                        &entry.topic,
                        &entry.title,
                        entry.description.as_deref().unwrap_or(""),
                    ];
                    
                    let combined_text = text_fields.join(" ");
                    
                    // Return true (keep) if any of the include patterns match
                    include_regexes.iter().any(|pattern| pattern.is_match(&combined_text))
                })
                .collect();
        }
    }
    
    Ok(filtered_results)
}

async fn list_channels(client: &Mediathek) -> Result<()> {
    // Get channels by making a wildcard query and extracting unique channels
    let result = client.query_string("", true).size(1000).send().await?;
    let mut channels: Vec<String> = result.results.iter().map(|item| item.channel.clone()).collect();
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
    println!("{}: Use {} to filter by channel", "Tip".yellow(), "!CHANNEL".cyan());
    println!("{}: Use {} for duration filtering", "Tip".yellow(), ">90 <120".cyan());

    Ok(())
}

fn create_vlc_playlist_and_launch(results: &[mediathekviewweb::models::Item]) -> Result<()> {
    if results.is_empty() {
        println!("{}", "No results found to add to playlist.".yellow());
        return Ok(());
    }

    // Create playlist filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let playlist_name = format!("mwb_playlist_{}.m3u", timestamp);
    
    // Create M3U playlist file
    let mut file = File::create(&playlist_name)?;
    writeln!(file, "#EXTM3U")?;
    
    for entry in results.iter() {
        // Write entry info as comment
        writeln!(file, "#EXTINF:-1,{} - {}", entry.channel, entry.title)?;
        // Write video URL
        writeln!(file, "{}", entry.url_video)?;
    }
    
    println!("{}", format!("Created playlist: {}", playlist_name).green());
    println!("{}", format!("Added {} video(s) to playlist", results.len()).green());
    
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
        Command::new("vlc")
            .arg(&playlist_name)
            .spawn()
    };
    
    match vlc_result {
        Ok(_) => {
            println!("{}", "VLC launched successfully!".green());
        }
        Err(e) => {
            println!("{}", format!("Failed to launch VLC: {}", e).red());
            println!("{}", format!("Playlist saved as: {}", playlist_name).yellow());
            println!("{}", "You can manually open this file with VLC.".yellow());
        }
    }
    
    Ok(())
}

fn print_table(results: &[mediathekviewweb::models::Item], query_info: &mediathekviewweb::models::QueryInfo) -> Result<()> {
    println!("{}", "Search Results".bold().blue());
    println!("Total results: {}", query_info.total_results.to_string().green());
    println!("Showing: {}", query_info.result_count.to_string().green());
    println!("Search time: {}ms", format!("{:.2}", query_info.search_engine_time.as_millis()).yellow());
    println!();

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    for (i, entry) in results.iter().enumerate() {
        println!("{} {}", format!("{}.", i + 1).blue().bold(), "─".repeat(60).blue());
        
        println!("{}: {}", "Channel".bold(), entry.channel.green());
        println!("{}: {}", "Theme".bold(), entry.topic.cyan());
        println!("{}: {}", "Title".bold(), entry.title.bright_white());
        
        let duration_secs = entry.duration.map_or(0, |d| d.as_secs());
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let seconds = duration_secs % 60;
        
        if hours > 0 {
            println!("{}: {}h {}m {}s", "Duration".bold(), hours, minutes, seconds);
        } else {
            println!("{}: {}m {}s", "Duration".bold(), minutes, seconds);
        }
        
        if let Some(dt) = DateTime::from_timestamp(entry.timestamp, 0) {
            println!("{}: {}", "Date".bold(), dt.format("%Y-%m-%d %H:%M").to_string().yellow());
        }
        
        println!("{}: {}", "Video URL".bold(), entry.url_video.bright_blue());
        
        if let Some(ref description) = entry.description {
            if !description.is_empty() && description.len() > 10 {
                let desc = if description.chars().count() > 200 {
                    let truncated: String = description.chars().take(200).collect();
                    format!("{}...", truncated)
                } else {
                    description.clone()
                };
                println!("{}: {}", "Description".bold(), desc.bright_black());
            }
        }
        
        println!();
    }

    Ok(())
}

fn print_csv(results: &[mediathekviewweb::models::Item]) -> Result<()> {
    println!("Channel,Theme,Title,Duration,Date,URL,Description");
    
    for entry in results {
        let duration = entry.duration.map_or("0".to_string(), |d| d.as_secs().to_string());
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
            entry.description.as_deref().unwrap_or("").replace('"', "\"\"")
        );
    }
    
    Ok(())
}