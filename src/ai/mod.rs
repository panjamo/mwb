//! AI module for integrating with Google Gemini API
//!
//! This module provides functionality for:
//! - Direct Gemini API integration via HTTP requests
//! - Web search capabilities
//! - Website content extraction
//! - Chronological episode sorting

pub mod tools;

use anyhow::Result;
use colored::Colorize;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub use tools::{perform_google_search, read_website_content};

#[derive(Debug, Serialize, Clone)]
struct GeminiRequest {
    contents: Vec<Content>,
    tools: Vec<Tool>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize, Clone)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
enum Part {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
}

#[derive(Debug, Serialize, Clone)]
struct FunctionCall {
    name: String,
    args: Value,
}

#[derive(Debug, Serialize, Clone)]
struct FunctionResponse {
    name: String,
    response: Value,
}

#[derive(Debug, Serialize, Clone)]
struct Tool {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize, Clone)]
struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: Parameters,
}

#[derive(Debug, Serialize, Clone)]
struct Parameters {
    r#type: String,
    properties: Value,
    required: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
    #[serde(rename = "finishReason")]
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ResponsePart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: ResponseFunctionCall,
    },
}

#[derive(Debug, Deserialize)]
struct ResponseFunctionCall {
    name: String,
    args: Value,
}

/// Main AI processor that handles the chronological sorting task
pub struct AIProcessor {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AIProcessor {
    /// Create a new AI processor
    pub async fn new() -> Result<Self> {
        let api_key = env::var("GOOGLE_API_KEY")
            .map_err(|_| {
                Self::handle_api_key_error();
                anyhow::anyhow!("GOOGLE_API_KEY environment variable not found. Please set it in a .env file or environment.")
            })?;

        let client = Client::builder()
            .user_agent("mwb-cli/1.0")
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let base_url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent".to_string();

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    /// Process TV show/series results with AI for chronological sorting and VLC playlist creation
    pub async fn process_episodes(
        &self,
        results: &[mediathekviewweb::models::Item],
    ) -> Result<String> {
        if results.is_empty() {
            return Err(anyhow::anyhow!("No results found to process with AI."));
        }

        println!(
            "🤖 Processing {} results with Gemini AI for chronological sorting...",
            results.len()
        );

        // Convert results to a more structured format for the AI
        let episodes_json = self.format_episodes_for_ai(results)?;

        let system_prompt = r#"You are an expert TV series analyst and VLC playlist creator. Your task is to:

1. Analyze the provided German TV episodes/shows
2. Use the available tools to search for chronological information about series if needed
3. Group episodes by series/show name
4. **INTELLIGENT DEDUPLICATION**: Carefully identify and remove duplicate episodes. Look for:
   - Episodes with identical or very similar titles (e.g., "Episode Title" vs "Episode Title (HD)")
   - Same content with different audio tracks (e.g., "Title" vs "Title (Audiodeskription)")
   - Different video qualities of the same episode (e.g., "Title" vs "Title (klare Sprache)")
   - Episodes with matching descriptions but slightly different titles
   - Same episode with different formatting or special versions
5. Sort remaining unique episodes in ASCENDING chronological order (oldest first, newest last - by air date, season/episode number, or story chronology)
6. **ALWAYS** call the create_vlc_playlist tool to create an XSPF playlist - this is mandatory!

DEDUPLICATION STRATEGY: When you find duplicates, keep the BEST version:
- Prefer standard version over audio description versions ("Audiodeskription")
- Prefer normal version over "klare Sprache" (simple language) versions
- Prefer higher quality when available
- Prefer complete/full versions over shortened versions
- When in doubt, keep the version with the most complete title/description

IMPORTANT: You MUST call the create_vlc_playlist function at the end with ONLY the deduplicated episodes, sorted in ASCENDING chronological order (oldest episodes first, newest episodes last). Be intelligent about deduplication - use your understanding of German TV naming conventions to identify duplicates that may have slightly different names.

The create_vlc_playlist function expects:
- episodes: array of {title, url, description, duration, channel, topic} objects (AFTER deduplication)
- playlist_name: a descriptive name for the playlist

Use the episode data provided in the input to create the playlist entries. Extract the title, url_video, description, duration, channel, and topic fields from each episode."#;

        let user_prompt = format!(
            "Please analyze and chronologically sort these German TV episodes:\n\n{}",
            episodes_json
        );

        let tools = self.create_tools();
        let mut conversation_history = vec![Content {
            role: "user".to_string(),
            parts: vec![Part::Text {
                text: format!("{}\n\n{}", system_prompt, user_prompt),
            }],
        }];

        // Main conversation loop with tool calling
        let max_iterations = 6; // Reduced to prevent quota issues
        for iteration in 1..=max_iterations {
            println!("🔄 Iteration {} - Sending request to Gemini...", iteration);

            let request = GeminiRequest {
                contents: conversation_history.clone(),
                tools: tools.clone(),
                generation_config: GenerationConfig {
                    temperature: 0.1,
                    max_output_tokens: 4096, // Reduced to save tokens
                },
            };

            let response = match self.call_gemini_api(&request).await {
                Ok(response) => response,
                Err(e) => {
                    Self::handle_api_error(&e);
                    return Err(e);
                }
            };

            if let Some(candidate) = response.candidates.first() {
                let content = &candidate.content;

                // Check if the model wants to call a function
                if let Some(part) = content.parts.first() {
                    match part {
                        ResponsePart::FunctionCall { function_call } => {
                            println!("🔧 Gemini is calling tool: {}", function_call.name);

                            let tool_result = self.execute_function_call(function_call).await?;

                            // Add the model's request to history
                            conversation_history.push(Content {
                                role: "model".to_string(),
                                parts: vec![Part::FunctionCall {
                                    function_call: FunctionCall {
                                        name: function_call.name.clone(),
                                        args: function_call.args.clone(),
                                    },
                                }],
                            });

                            // Add the tool's response to history
                            conversation_history.push(Content {
                                role: "user".to_string(),
                                parts: vec![Part::FunctionResponse {
                                    function_response: tool_result,
                                }],
                            });

                            // Continue the loop to send the tool result back to the model
                            continue;
                        }
                        ResponsePart::Text { text } => {
                            println!("✅ Received final response from Gemini");
                            return Ok(text.clone());
                        }
                    }
                }
            }

            if iteration == max_iterations {
                return Err(anyhow::anyhow!(
                    "Maximum iterations reached without final answer"
                ));
            }
        }

        Err(anyhow::anyhow!("Unexpected end of conversation loop"))
    }

    /// Make HTTP request to Gemini API
    async fn call_gemini_api(&self, request: &GeminiRequest) -> Result<GeminiResponse> {
        let url = format!("{}?key={}", self.base_url, self.api_key);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Gemini API error {}: {}",
                status,
                error_text
            ));
        }

        let gemini_response: GeminiResponse = response.json().await?;
        Ok(gemini_response)
    }

    /// Create tool definitions for the Gemini API
    fn create_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                function_declarations: vec![
                    FunctionDeclaration {
                        name: "perform_google_search".to_string(),
                        description: "Performs a web search to find information about TV series, episodes, chronological order, or air dates. Use this to find Wikipedia pages, episode guides, or other authoritative sources.".to_string(),
                        parameters: Parameters {
                            r#type: "object".to_string(),
                            properties: json!({
                                "query": {
                                    "type": "string",
                                    "description": "The search query. Include series name and terms like 'episodes chronological order', 'episode guide', 'air dates', etc."
                                }
                            }),
                            required: vec!["query".to_string()],
                        },
                    },
                    FunctionDeclaration {
                        name: "read_website_content".to_string(),
                        description: "Reads and extracts textual content from a website URL. Use this to get detailed episode information from Wikipedia, IMDB, or other sources found through search.".to_string(),
                        parameters: Parameters {
                            r#type: "object".to_string(),
                            properties: json!({
                                "url": {
                                    "type": "string",
                                    "description": "The URL of the website to read content from."
                                }
                            }),
                            required: vec!["url".to_string()],
                        },
                    },
                    FunctionDeclaration {
                        name: "create_vlc_playlist".to_string(),
                        description: "Creates and saves a VLC playlist file in XSPF format with the chronologically sorted episodes, then launches VLC with the playlist.".to_string(),
                        parameters: Parameters {
                            r#type: "object".to_string(),
                            properties: json!({
                                "episodes": {
                                    "type": "array",
                                    "description": "Array of episode objects with title, url, description, and duration",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "title": {"type": "string"},
                                            "url": {"type": "string"}, 
                                            "description": {"type": "string"},
                                            "duration": {"type": "number", "description": "Duration in seconds"},
                                            "channel": {"type": "string", "description": "TV channel name"},
                                            "topic": {"type": "string", "description": "Episode topic/theme"}
                                        }
                                    }
                                },
                                "playlist_name": {
                                    "type": "string",
                                    "description": "Name for the playlist file (without extension)"
                                }
                            }),
                            required: vec!["episodes".to_string(), "playlist_name".to_string()],
                        },
                    },
                ],
            }
        ]
    }

    /// Format episodes for AI processing
    fn format_episodes_for_ai(&self, results: &[mediathekviewweb::models::Item]) -> Result<String> {
        // Limit episodes to prevent token overflow
        let limited_results = if results.len() > 20 {
            &results[..20]
        } else {
            results
        };

        let formatted: Vec<Value> = limited_results
            .iter()
            .map(|item| {
                json!({
                    "title": item.title,
                    "topic": item.topic,
                    "description": if let Some(desc) = &item.description {
                        if desc.len() > 200 {
                            // Find a safe character boundary within 200 chars
                            let mut end = 200;
                            while end > 0 && !desc.is_char_boundary(end) {
                                end -= 1;
                            }
                            format!("{}...", &desc[..end])
                        } else {
                            desc.clone()
                        }
                    } else {
                        String::new()
                    },
                    "duration": item.duration,
                    "channel": item.channel,
                    "url": item.url_video,
                })
            })
            .collect();

        if results.len() > 20 {
            println!("ℹ️  Processing first 20 episodes to avoid API limits. Use smaller -s parameter for full dataset.");
        }

        serde_json::to_string_pretty(&formatted)
            .map_err(|e| anyhow::anyhow!("Failed to serialize episodes: {}", e))
    }

    /// Execute a function call from the AI
    async fn execute_function_call(&self, call: &ResponseFunctionCall) -> Result<FunctionResponse> {
        let function_name = &call.name;
        let args = &call.args;

        let result_string = match function_name.as_str() {
            "perform_google_search" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;
                perform_google_search(query).await?
            }
            "read_website_content" => {
                let url = args["url"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;
                read_website_content(url).await?
            }
            "create_vlc_playlist" => {
                let episodes = args["episodes"]
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'episodes' argument"))?;
                let playlist_name = args["playlist_name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'playlist_name' argument"))?;
                self.create_vlc_playlist(episodes, playlist_name).await?
            }
            _ => return Err(anyhow::anyhow!("Unknown function: {}", function_name)),
        };

        Ok(FunctionResponse {
            name: function_name.clone(),
            response: json!({ "result": result_string }),
        })
    }

    /// Create VLC playlist and launch VLC
    async fn create_vlc_playlist(&self, episodes: &[Value], playlist_name: &str) -> Result<String> {
        println!("🎵 Creating VLC playlist: {}", playlist_name);

        // Generate timestamp for unique filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.xspf", playlist_name, timestamp);

        // Create XSPF playlist content
        let mut playlist_content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        playlist_content.push_str("<playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n");
        playlist_content.push_str(&format!(
            "  <title>AI Sorted Playlist: {}</title>\n",
            self.escape_xml(playlist_name)
        ));
        playlist_content.push_str("  <creator>MWB - AI Episode Sorting</creator>\n");
        playlist_content.push_str(&format!(
            "  <date>{}</date>\n",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
        ));
        playlist_content.push_str("  <trackList>\n");

        for episode in episodes.iter() {
            if let (Some(title), Some(url)) = (episode["title"].as_str(), episode["url"].as_str()) {
                // Get duration if available (convert from seconds to milliseconds for XSPF)
                let duration_seconds = episode["duration"]
                    .as_i64()
                    .or_else(|| episode["duration"].as_str()?.parse().ok())
                    .unwrap_or(0);
                let duration_ms = duration_seconds * 1000;

                // Get other metadata
                let description = episode["description"].as_str().unwrap_or("");
                let clean_desc = self.clean_description(description);
                let channel = episode["channel"].as_str().unwrap_or("");
                let topic = episode["topic"].as_str().unwrap_or("");

                playlist_content.push_str("    <track>\n");
                playlist_content.push_str(&format!(
                    "      <title>{}</title>\n",
                    self.escape_xml(title)
                ));

                if !channel.is_empty() {
                    playlist_content.push_str(&format!(
                        "      <creator>{}</creator>\n",
                        self.escape_xml(channel)
                    ));
                }

                if !topic.is_empty() {
                    playlist_content.push_str(&format!(
                        "      <album>{}</album>\n",
                        self.escape_xml(topic)
                    ));
                }

                playlist_content.push_str(&format!(
                    "      <location>{}</location>\n",
                    self.escape_xml(url)
                ));

                if duration_ms > 0 {
                    playlist_content
                        .push_str(&format!("      <duration>{}</duration>\n", duration_ms));
                }

                if !clean_desc.is_empty() {
                    playlist_content.push_str(&format!(
                        "      <annotation>{}</annotation>\n",
                        self.escape_xml(&clean_desc)
                    ));
                }

                playlist_content.push_str("    </track>\n");
            }
        }

        playlist_content.push_str("  </trackList>\n");
        playlist_content.push_str("</playlist>\n");

        // Write playlist to file
        match File::create(&filename) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(playlist_content.as_bytes()) {
                    return Err(anyhow::anyhow!("Failed to write playlist file: {}", e));
                }
                println!("✅ Playlist saved as: {}", filename);
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to create playlist file: {}", e));
            }
        }

        // Try to launch VLC with the playlist
        self.launch_vlc(&filename)?;

        Ok(format!(
            "XSPF playlist '{}' created with {} episodes and VLC launched successfully!",
            filename,
            episodes.len()
        ))
    }

    /// Launch VLC with the playlist
    fn launch_vlc(&self, playlist_path: &str) -> Result<()> {
        println!("🚀 Launching VLC with playlist...");

        // Try different VLC executable names/paths
        let vlc_commands = vec![
            "vlc",
            "vlc.exe",
            "C:\\Program Files\\VideoLAN\\VLC\\vlc.exe",
            "C:\\Program Files (x86)\\VideoLAN\\VLC\\vlc.exe",
        ];

        for vlc_cmd in &vlc_commands {
            match Command::new(vlc_cmd).arg(playlist_path).spawn() {
                Ok(_) => {
                    println!("✅ VLC launched successfully with {}", vlc_cmd);
                    return Ok(());
                }
                Err(_) => continue,
            }
        }

        // If VLC launch failed, provide helpful message
        println!("⚠️  Could not auto-launch VLC. You can manually open the playlist:");
        println!("   📁 File: {}", playlist_path);
        println!("   💡 Tip: Add VLC to your PATH or install it to default location");

        Ok(())
    }

    /// Clean description text for XSPF format
    fn clean_description(&self, description: &str) -> String {
        // Remove line breaks, extra whitespace, and truncate to reasonable length
        let cleaned = description
            .replace(['\n', '\r'], " ")
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        // Truncate to 300 characters for XSPF annotations (longer than M3U since XML handles it better)
        if cleaned.len() > 300 {
            format!("{}...", &cleaned[..297])
        } else {
            cleaned
        }
    }

    /// Escape XML special characters
    fn escape_xml(&self, text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Handle API errors with helpful messages and browser opening
    fn handle_api_error(error: &anyhow::Error) {
        let error_msg = error.to_string().to_lowercase();

        if error_msg.contains("401")
            || error_msg.contains("unauthorized")
            || error_msg.contains("api key")
        {
            Self::handle_api_key_error();
        } else if error_msg.contains("429")
            || error_msg.contains("quota")
            || error_msg.contains("rate limit")
        {
            Self::handle_quota_error();
        } else if error_msg.contains("403") || error_msg.contains("forbidden") {
            Self::handle_permission_error();
        }
    }

    /// Handle API key errors with helpful messages and browser opening
    fn handle_api_key_error() {
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
        if Self::open_browser(url).is_err() {
            println!(
                "{}",
                "⚠️  Could not auto-open browser. Please visit the URL manually.".yellow()
            );
        }
    }

    /// Open URL in the default browser
    fn open_browser(url: &str) -> Result<()> {
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

    /// Handle quota/rate limit errors
    fn handle_quota_error() {
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
        if Self::open_browser(url).is_err() {
            println!(
                "{}",
                "⚠️  Could not auto-open browser. Please visit the URL manually.".yellow()
            );
        }
    }

    /// Handle permission errors
    fn handle_permission_error() {
        println!("{}", "🚫 API Permission Error!".red().bold());
        println!();
        println!(
            "{}",
            "❌ Your API key doesn't have the required permissions.".red()
        );
        println!();
        println!("{}", "💡 To fix this:".cyan().bold());
        println!(
            "{}",
            "   1. Visit: https://aistudio.google.com/app/u/5/apikey".cyan()
        );
        println!("{}", "   2. Check your API key permissions".cyan());
        println!("{}", "   3. Regenerate a new key if needed".cyan());
        println!();
        println!("{}", "🌐 Opening API key page...".green());

        let url = "https://aistudio.google.com/app/u/5/apikey";
        if Self::open_browser(url).is_err() {
            println!(
                "{}",
                "⚠️  Could not auto-open browser. Please visit the URL manually.".yellow()
            );
        }
    }
}
