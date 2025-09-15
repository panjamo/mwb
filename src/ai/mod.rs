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
    verbose: bool,
}

impl AIProcessor {
    /// Create a new AI processor with verbose flag
    pub async fn new_with_verbose(verbose: bool) -> Result<Self> {
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
            verbose,
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

        let system_prompt = r#"Sie sind ein Experte für TV-Serien-Analyse und VLC-Playlist-Erstellung. Ihre Aufgabe ist es:

* Die bereitgestellten deutschen TV-Episoden/Sendungen zu analysieren
* Kennungen im Titel wie "(S2/E10)" haben die höchste Priorität, (S2/E10) bedeutet Staffel 2, Episode 10, sortieren nach Staffel und Episoden
* Zahlen am Ende der Titel wie zum Beispiel "(234)" bedeuten Episode 234 in Staffel 1
* ansonsten verfügbaren Tools zu nutzen, um bei Bedarf chronologische Informationen über Serien zu suchen
* **IMPORTENT** such auf jeden Fall mit "perform_google_search" nach der Episodenreihenfolge bei wikipedia.de
* **INTELLIGENTE DEDUPLIZIERUNG**: Sorgfältig Duplikate von Episoden identifizieren und entfernen. Achten Sie auf:
   - Episoden mit identischen oder sehr ähnlichen Titeln (z.B. "Episodentitel" vs "Episodentitel (HD)")
   - Gleicher Inhalt mit verschiedenen Tonspuren (z.B. "Titel" vs "Titel (Audiodeskription)")
   - Verschiedene Videoqualitäten derselben Episode (z.B. "Titel" vs "Titel (klare Sprache)")
   - Episoden mit übereinstimmenden Beschreibungen aber leicht unterschiedlichen Titeln
   - Gleiche Episode mit unterschiedlicher Formatierung oder Spezialversionen
* Verbleibende einzigartige Episoden in AUFSTEIGENDER chronologischer Reihenfolge sortieren (älteste zuerst, neueste zuletzt - nach Ausstrahlungsdatum, Staffel/Episodennummer oder Story-Chronologie)
* **IMMER** die create_vlc_playlist Funktion aufrufen, um eine XSPF-Playlist zu erstellen - dies ist zwingend erforderlich!

DEDUPLIZIERUNGS-STRATEGIE: Bei Duplikaten die BESTE Version behalten:
- Standardversion gegenüber Audiodeskriptionsversionen bevorzugen
- Normale Version gegenüber "klare Sprache"-Versionen bevorzugen
- Höhere Qualität wenn verfügbar bevorzugen
- Vollständige Versionen gegenüber gekürzten Versionen bevorzugen
- Im Zweifelsfall die Version mit dem vollständigsten Titel/Beschreibung behalten

WICHTIG: Sie MÜSSEN die create_vlc_playlist Funktion am Ende mit NUR den deduplizierten Episoden aufrufen, sortiert in AUFSTEIGENDER chronologischer Reihenfolge (älteste Episoden zuerst, neueste zuletzt). Gehen Sie intelligent bei der Deduplizierung vor - nutzen Sie Ihr Verständnis deutscher TV-Namenskonventionen, um Duplikate mit leicht unterschiedlichen Namen zu identifizieren.

Die create_vlc_playlist Funktion erwartet:
- episodes: Array von {title, url, description, duration, channel, topic} Objekten (NACH Deduplizierung)
- playlist_name: ein beschreibender Name für die Playlist

Verwenden Sie die in der Eingabe bereitgestellten Episodendaten, um die Playlist-Einträge zu erstellen. Extrahieren Sie die Felder title, url_video, description, duration, channel und topic aus jeder Episode."#;

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

        // Debug: Print tool definitions
        if self.verbose {
            eprintln!("[VERBOSE] Registered {} tools:", tools.len());
            for tool in &tools {
                for func in &tool.function_declarations {
                    eprintln!("[VERBOSE]   - {}: {}", func.name, func.description);
                }
            }
        }

        // Main conversation loop with tool calling
        let max_iterations = 8; // Increased to allow for proper tool usage
        for iteration in 1..=max_iterations {
            if iteration == 1 {
                println!("🔄 Iteration {} - Initial request (expecting search tool call)...", iteration);
            } else {
                println!("🔄 Iteration {} - Continuing conversation...", iteration);
            }

            let request = GeminiRequest {
                contents: conversation_history.clone(),
                tools: tools.clone(),
                generation_config: GenerationConfig {
                    temperature: 0.1,
                    max_output_tokens: 4096, // Reduced to save tokens
                },
            };

            // Debug: Log request details
            if self.verbose {
                eprintln!("[VERBOSE] Sending request with {} tools", request.tools.len());
                eprintln!("[VERBOSE] Request has {} conversation turns", request.contents.len());
            }

            let response = match self.call_gemini_api(&request).await {
                Ok(response) => response,
                Err(e) => {
                    Self::handle_api_error(&e);
                    return Err(e);
                }
            };

            if let Some(candidate) = response.candidates.first() {
                let content = &candidate.content;

                // Debug: Log response type
                if self.verbose {
                    eprintln!("[VERBOSE] Response received with {} parts", content.parts.len());
                    for (i, part) in content.parts.iter().enumerate() {
                        match part {
                            ResponsePart::FunctionCall { function_call } => {
                                eprintln!("[VERBOSE]   Part {}: Function call to {}", i, function_call.name);
                            }
                            ResponsePart::Text { text } => {
                                eprintln!("[VERBOSE]   Part {}: Text response ({} chars)", i, text.len());
                                if text.len() < 200 {
                                    eprintln!("[VERBOSE]     Preview: {}", text.trim());
                                }
                            }
                        }
                    }
                }

                // Check if the model wants to call a function
                if let Some(part) = content.parts.first() {
                    match part {
                        ResponsePart::FunctionCall { function_call } => {
                            println!("🔧 ✅ Gemini is calling tool: {}", function_call.name);
                            
                            // Encourage continued tool usage if this is the first search
                            if function_call.name == "perform_google_search" && iteration <= 2 {
                                println!("💡 Good! AI is searching for episode information as required.");
                            }

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
                            // Check if the AI tried to provide a final answer without using required tools
                            if iteration == 1 {
                                println!("❌ AI provided text response instead of calling perform_google_search first!");
                                
                                // Add the model's response to history
                                conversation_history.push(Content {
                                    role: "model".to_string(),
                                    parts: vec![Part::Text { text: text.clone() }],
                                });
                                
                                // Force the AI to use the search tool
                                conversation_history.push(Content {
                                    role: "user".to_string(),
                                    parts: vec![Part::Text {
                                        text: "STOP! You MUST use the perform_google_search tool first. Do not provide any analysis or sorting until you have searched for chronological information. Call perform_google_search now with a query about the series episode order.".to_string(),
                                    }],
                                });
                                
                                continue; // Continue the conversation loop
                            } else if iteration <= 4 && !text.to_lowercase().contains("playlist") {
                                println!("⚠️  AI provided text response without completing required steps - prompting for tool usage...");
                                
                                // Add the model's response to history
                                conversation_history.push(Content {
                                    role: "model".to_string(),
                                    parts: vec![Part::Text { text: text.clone() }],
                                });
                                
                                // Prompt the AI to use tools
                                conversation_history.push(Content {
                                    role: "user".to_string(),
                                    parts: vec![Part::Text {
                                        text: "Continue following the mandatory workflow: Search → Read Sources → Deduplicate → Sort → Create Playlist. What is your next step?".to_string(),
                                    }],
                                });
                                
                                continue; // Continue the conversation loop
                            } else {
                                println!("✅ Received final response from Gemini");
                                return Ok(text.clone());
                            }
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
                        description: "MANDATORY FIRST TOOL: Search the web for TV series information, episodes, chronological order, or broadcast dates. Use this IMMEDIATELY when you receive episode data to find authoritative sources in Wikipedia pages, Example queries: '[series name] wikipedia.de'.".to_string(),
                        parameters: Parameters {
                            r#type: "object".to_string(),
                            properties: json!({
                                "query": {
                                    "type": "string",
                                    "description": "Die Suchanfrage. Enthalten Sie den Seriennamen und Begriffe wie 'Episoden chronologische Reihenfolge', 'Episodenführer', 'Ausstrahlungsdaten', etc."
                                }
                            }),
                            required: vec!["query".to_string()],
                        },
                    },
                    FunctionDeclaration {
                        name: "read_website_content".to_string(),
                        description: "MANDATORY SECOND TOOL: Read and extract text content from a website URL. Use this IMMEDIATELY after perform_google_search to get detailed episode information from Wikipedia You find a table with episode details. The sequence of episodes is listed in chronological order.".to_string(),
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
                        description: "MANDATORY FINAL TOOL: Create and save a VLC playlist in XSPF format with chronologically sorted episodes, then launch VLC with this playlist. Use this ONLY AFTER you have searched for and gathered chronological information using the other tools, deduplicated episodes, and sorted them in ascending chronological order (oldest first). This tool is REQUIRED to complete the task.".to_string(),
                        parameters: Parameters {
                            r#type: "object".to_string(),
                            properties: json!({
                                "episodes": {
                                    "type": "array",
                                    "description": "Array von Episoden-Objekten mit Titel, URL, Beschreibung und Dauer",
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
                                    "description": "Name der Wiedergabelisten-Datei (ohne Erweiterung)"
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

        if self.verbose {
            eprintln!("[VERBOSE] AI Tool Call: {}", function_name);
            eprintln!("[VERBOSE]   args: {}", serde_json::to_string_pretty(args).unwrap_or_else(|_| "invalid JSON".to_string()));
        }

        // Enforce tool usage order - read_website_content cannot be called before perform_google_search
        if function_name == "read_website_content" {
            let search_tool_used = std::env::var("SEARCH_TOOL_USED").unwrap_or_default() == "1";
            if !search_tool_used {
                return Err(anyhow::anyhow!("ERROR: You must use perform_google_search BEFORE using read_website_content. Please search for information first, then read the discovered URLs."));
            }
        }

        let result_string = match function_name.as_str() {
            "perform_google_search" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;
                
                // Set environment variable so tools.rs can read it
                if self.verbose {
                    std::env::set_var("VERBOSE", "1");
                }
                
                // Mark that search tool has been used
                std::env::set_var("SEARCH_TOOL_USED", "1");
                
                perform_google_search(query).await?
            }
            "read_website_content" => {
                let url = args["url"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;
                
                // Set environment variable so tools.rs can read it
                if self.verbose {
                    std::env::set_var("VERBOSE", "1");
                }
                
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

        let response = FunctionResponse {
            name: function_name.clone(),
            response: json!({ "result": result_string }),
        };

        if self.verbose {
            eprintln!("[VERBOSE] AI Tool Response: {}", function_name);
            eprintln!("[VERBOSE]   result length: {} chars", result_string.len());
        }

        Ok(response)
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
