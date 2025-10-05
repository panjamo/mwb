# MWB - MediathekViewWeb CLI

A Rust command-line interface for searching German public broadcasting content via the [MediathekViewWeb API](https://mediathekviewweb.de/).

Built using the official [mediathekviewweb](https://crates.io/crates/mediathekviewweb) Rust crate for robust and efficient API interaction.

## Features

- **Fast Search**: Search through thousands of German TV shows, documentaries, and news content
- **AI-Powered Playlists** ✨: Use Gemini AI for intelligent chronological sorting, deduplication, and smart VLC playlist creation
- **Advanced Filtering**: Use MediathekView's powerful search syntax with selectors
- **Duration Selectors**: Filter content by duration directly in the query (e.g., `>90` for content longer than 90 minutes)
- **Exclusion Filter**: Filter out unwanted content using regex patterns
- **Multiple Output Formats**: View results as formatted tables, JSON, CSV, one-line format, or VLC playlists
- **Count-Only Output**: Get just the number of matching results for quick statistics and scripting
- **VLC Integration**: Create playlists and launch VLC directly with search results
- **Channel Listing**: Browse all available broadcasting channels
- **Flexible Sorting**: Sort results by date, duration, or channel

## Installation

Make sure you have Rust installed, then build from source:

```bash
git clone <repository-url>
cd mwb
cargo build --release
```

The binary will be available at `target/release/mwb` (or `target/release/mwb.exe` on Windows).

### AI Feature Setup (Optional)

To use the `--vlc-ai` feature for intelligent episode sorting and analysis:

```bash
# 1. Get your Google AI Studio API key (free tier available)
#    Visit: https://aistudio.google.com/app/apikey

# 2. Create a .env file in the project directory
cp .env.example .env

# 3. Edit .env file and add your API key
# GOOGLE_API_KEY=your_api_key_here
```

**AI Feature Requirements:**
- Google AI Studio API key (free tier: 15 requests/minute)
- Active internet connection for web research
- No external CLI tools required (integrated directly)

**Setup Steps:**
1. **Get API Key**: Visit [Google AI Studio](https://aistudio.google.com/app/apikey) and generate a free API key
2. **Environment File**: Copy `.env.example` to `.env` and add your key
3. **Ready to Go**: The `--vlc-ai` flag will now work with intelligent web research

The AI features use direct API integration with built-in web search tools. All other functionality works without AI setup.

## Shell Completion

MWB supports shell completion for all major shells. Generate completion files to enable auto-completion of commands, options, and values.

### Generate Completion Files

```bash
# Generate completion for your shell
mwb completion bash > ~/.bash_completion.d/mwb
mwb completion zsh > ~/.zsh/completions/_mwb
mwb completion fish > ~/.config/fish/completions/mwb.fish
mwb completion nushell > ~/.config/nushell/completions/mwb.nu
mwb completion powershell > $PROFILE.d/mwb.ps1
mwb completion elvish > ~/.config/elvish/completions/mwb.elv
```

### Setup Instructions

**Bash:**
```bash
# Add to ~/.bashrc or ~/.bash_profile
source ~/.bash_completion.d/mwb
```

**Zsh:**
```bash
# Ensure ~/.zsh/completions is in your fpath
# Add to ~/.zshrc:
fpath=(~/.zsh/completions $fpath)
autoload -U compinit && compinit
```

**Fish:**
```bash
# Fish automatically loads completions from ~/.config/fish/completions/
# No additional setup required
```

**Nushell:**
```bash
# Add to your Nushell config file:
use ~/.config/nushell/completions/mwb.nu
```

**Features:**
- Command completion (`search`, `channels`, `completion`)
- Option completion (`--format`, `--size`, `--exclude`, etc.)
- Value completion for format options and shells
- Help text integration

## Usage

### Basic Search

```bash
# Search for content containing "Tatort"
mwb search Tatort

# Search with multiple terms (single search)
mwb search "climate change documentary"

# Multi-Search: Perform separate searches for each term and unify results
mwb search Tatort ZDF ARD
mwb search "climate change" "documentary" "environment"
```

**Multi-Search Mode**: When you provide multiple separate arguments to the search command, MWB automatically performs individual searches for each term and unifies the results:

- Each search term is processed separately
- Results are automatically deduplicated based on video URL
- All results are combined and sorted according to your specified criteria
- Perfect for finding content across different topics or channels

```bash
# Example: Find content from multiple channels
mwb search ARD ZDF BR --size 10

# Example: Multi-topic search with deduplication
mwb search "Tatort" "Krimi" "Polizeiruf" --count

# Example: Combine with other options
mwb search "Dokumentation" "Reportage" --format json --size 20
```

### Advanced Search with Selectors

MWB supports MediathekViewWeb's selector syntax:

| Selector | Field       | Example           | Description                              |
|----------|-------------|-------------------|------------------------------------------|
| `!`      | Channel     | `!ARD`            | Find content from ARD channel           |
| `#`      | Topic       | `#Tatort`         | Find content with "Tatort" topic        |
| `+`      | Title       | `+Schokolade`     | Find content with "Schokolade" in title |
| `*`      | Description | `*Berlin`         | Find content mentioning Berlin          |
| `>`      | Duration    | `>80`             | Content longer than 80 minutes          |
| `<`      | Duration    | `<10`             | Content shorter than 10 minutes         |

### Duration Selector Examples

The mediathekviewweb crate supports duration filtering directly in the search query:

```bash
# Find content longer than 90 minutes
mwb search ">90"

# Find content shorter than 30 minutes
mwb search "<30"

# Find content between 60 and 120 minutes
mwb search ">60 <120"

# Combine duration with other selectors - ARD documentaries longer than 45 minutes
mwb search "!ARD #Dokumentation >45"

# Short news segments from ZDF (less than 15 minutes)
mwb search "!ZDF #Nachrichten <15"

# Find feature-length content (movies, long documentaries)
mwb search ">80 Dokumentation"
```

### Examples

```bash
# Find all Tatort episodes from ARD
mwb search "!ARD #Tatort"

# Find documentaries about climate change longer than 30 minutes
mwb search "dokumentation climate change >30"

# Find news content from multiple channels
mwb search "!ARD !ZDF !NDR #Nachrichten"

# Search with regex exclusion (removes results matching patterns)
mwb search "documentary" --exclude "sport.*ball" "weather.*report"

# Search with regex inclusion (only show results matching patterns)
mwb search "Nachrichten" -i "Politik|Wirtschaft" -e "Sport|Wetter"

# Find recent short clips from Arte using short forms
mwb search "!Arte <20" -b timestamp -r desc -s 20
```

### Multi-Search Functionality ✨

MWB automatically detects when you provide multiple search terms and performs separate searches for each term, then unifies the results:

```bash
# Multi-channel search - searches for "ARD" and "ZDF" separately, then combines results
mwb search ARD ZDF --size 10 --verbose

# Multi-topic search with automatic deduplication
mwb search Tatort Krimi Polizeiruf --count

# Mixed search types - channels, topics, and keywords
mwb search "!ARD" "#Dokumentation" "climate change" --format table

# Complex multi-search with filtering
mwb search "Reportage" "Dokumentation" "Investigation" --size 20 --exclude "Sport.*" --include "Politik|Umwelt"
```

**How Multi-Search Works:**

1. **Separate Searches**: Each term triggers an individual search request
2. **Deduplication**: Results are automatically deduplicated based on video URL
3. **Unified Sorting**: All results are combined and sorted by your specified criteria
4. **Same Options**: All filtering, formatting, and output options work the same way

**Benefits:**

- Find content across multiple channels or topics in one command
- Automatic deduplication prevents duplicate entries
- More comprehensive results than single combined searches
- Perfect for discovering related content across different categories

**Verbose Mode Example:**

```bash
mwb search ARD ZDF --size 5 --verbose
# Output shows:
# === Multi-Search Mode ===
# Search terms: ARD, ZDF
# Total searches: 2
# ========================
# Searching: ARD (1/2)
# Found: 5 results
# Searching: ZDF (2/2)  
# Found: 5 results
# === Multi-Search Results ===
# Total unique results: 10
# ===========================
```

### Output Formats

```bash
# Default onelinetheme format (compact with colors, shows theme)
mwb search "Tatort"

# Table format (detailed human-readable)
mwb search "Tatort" -f table

# Get only the count of results
mwb search "Tatort" -c
# Output: 42

# JSON output for scripting using short form
mwb search "Tatort" -f json

# CSV output for spreadsheets using short form
mwb search "Tatort" -f csv > results.csv

# One-line format (compact output with colors) - shows URL
mwb search "Tatort" -f oneline

# One-line theme format (shows theme/topic instead of URL) - default
mwb search "Tatort" -f onelinetheme

# Compare the two oneline formats:
# oneline:      [WDR] Kollaps (2015) (2025-10-14 18:15) [88min] - https://wdrmedien-a.akamaihd.net/medp/...
# onelinetheme: [WDR] Kollaps (2015) (2025-10-14 18:15) [88min] - Tatort

# XSPF playlist output (XML Shareable Playlist Format) to stdout
mwb search "Tatort" -f xspf

# Save XSPF playlist to file with duration and date/time metadata
# Creates file: mwb_Tatort_80_20250912_092818.xspf
mwb search "Tatort >80" -f xspf -x

# Create XSPF playlist and launch VLC directly (medium quality by default)
# Creates file: mwb_Tatort_m80_1234.xspf
mwb search "Tatort >80" -v

# VLC with low quality video links (smaller file sizes, faster streaming)
# Creates file: mwb_dokumentation_m60_1234.xspf
mwb search "dokumentation >60" -s 10 -v=l

# VLC with HD quality video links (when available)
# Creates file: mwb_dokumentation_m60_1234.xspf
mwb search "dokumentation >60" -s 10 --vlc=h
```

#### Format Descriptions

| Format | Description | Best For |
|--------|-------------|----------|
| `onelinetheme` | Compact single-line format: `[Channel] Title (Date) [Duration] - Theme` | Content discovery and topic browsing *(default)* |
| `oneline` | Compact single-line format: `[Channel] Title (Date) [Duration] - URL` | Quick scanning and terminal output |
| `table` | Human-readable formatted output with colors and full details | Interactive browsing and viewing |
| `json` | Machine-readable JSON format with all metadata | Scripting and programmatic processing |
| `csv` | Comma-separated values for spreadsheet import | Data analysis and Excel/LibreOffice |
| `xspf` | XML playlist format compatible with VLC and other media players | Creating playlists for media players |

### Count-Only Output

Use the `--count` (or `-c`) flag when you only need to know how many results match your search criteria:

```bash
# Basic count
mwb search "Tatort" -c
# Output: 42

# Count with filters
mwb search "Tatort" -c --exclude "WDR" --size 100
# Output: 67

# Count for scripting
if [ $(mwb search "live stream" -c) -gt 0 ]; then
    echo "Live streams are available"
fi

# Automation example: Check for new Tatort episodes
#!/bin/bash
TATORT_COUNT=$(mwb search "Tatort" --no-future -c)
if [ $TATORT_COUNT -gt 50 ]; then
    echo "Found $TATORT_COUNT Tatort episodes - creating playlist"
    mwb search "Tatort" --no-future -v
else
    echo "Only $TATORT_COUNT Tatort episodes found"
fi
```

This is especially useful for:
- **Monitoring**: Check if new content matching criteria is available
- **Analytics**: Get quick statistics about content availability
- **Scripting**: Use the count in conditional logic or automation
- **Performance**: Much faster than fetching full results when you only need the count

### When to Use Each One-Line Format

**Use `oneline` when:**
- You need the video URLs for direct access or scripting
- Building playlists or downloading content
- Working with automation that processes video links

**Use `onelinetheme` (default) when:**
- Browsing content by topic or category
- You want cleaner, more readable output
- Exploring what types of content are available
- The theme/topic is more relevant than the URL

### XSPF Playlist Format

The XSPF (XML Shareable Playlist Format) is a standardized playlist format that includes rich metadata:

- **Duration**: Track duration in milliseconds
- **Date/Time**: Original broadcast date displayed in VLC's Artist column and track titles
- **Creator**: TV channel name
- **Artist**: Broadcast date (YYYY-MM-DD format) - shows in VLC's Artist column
- **Album**: Topic/theme of the content
- **Annotation**: Full description of the content
- **Location**: Direct video URL

Example XSPF output structure:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
  <title>MediathekView Search: Tatort >80</title>
  <creator>MWB - MediathekViewWeb CLI</creator>
  <date>2025-09-12T07:28:18Z</date>
  <trackList>
    <track>
      <title>Kollaps (2015) (2025-10-14)</title>
      <creator>WDR</creator>
      <artist>2025-10-14</artist>
      <album>Tatort</album>
      <location>https://example.com/video.mp4</location>
      <duration>5310000</duration>
      <annotation>Episode description...</annotation>
    </track>
  </trackList>
</playlist>
```

### VLC Playlist Integration

The VLC integration now uses XSPF format instead of M3U for richer metadata support. VLC fully supports XSPF playlists and can display the additional information like duration, broadcast date, and descriptions. Broadcast dates are displayed in VLC's Artist column and also included in track titles for maximum visibility.

```bash
# Create XSPF playlist and launch VLC with search results (medium quality default)
# Creates file: mwb_tatort_m85_1234.xspf
mwb search "tatort >85" --vlc

# Use short form with low quality for faster streaming
# Creates file: mwb_dokumentation_climate_change_m30_1234.xspf  
mwb search "dokumentation climate change >30" -s 20 -e "weather" -v=l

# VLC integration with HD quality (when available)
# Creates file: mwb__Arte_m60_m120_1234.xspf
mwb search "!Arte >60 <120" --vlc=h

# Quality options: l=low, m=medium (default), h=HD
# If no quality specified, medium quality is used
mwb search "documentary" -v      # medium quality (default)
mwb search "documentary" -v=m    # medium quality (explicit)
```

The VLC feature:
- Creates an XSPF playlist file with query-based naming (e.g., `mwb_tatort_m85_1234.xspf`)
- Filename reflects search terms and duration filters for easy identification
- Includes rich metadata: duration (milliseconds), broadcast date/time (ISO 8601), channel, topic, descriptions
- Broadcast dates displayed in VLC's Artist column and track titles for optimal visibility
- Full XSPF format with proper XML structure and metadata tags
- Automatically launches VLC with the playlist
- Works on Windows (tries common VLC installation paths) and Unix-like systems
- Falls back gracefully if VLC cannot be launched - playlist file is still created

#### Playlist Filename Format

Playlist files are named based on your search query for easy identification:

- **Format**: `mwb_<search_terms>_<duration>_<timestamp>.xspf`
- **Examples**:
  - `"tatort >85"` → `mwb_tatort_m85_1234.xspf`
  - `"dokumentation klima >30 <90"` → `mwb_dokumentation_klima_m30_m90_1234.xspf`
  - `"!Arte >60"` → `mwb__Arte_m60_1234.xspf`
  - `">120"` → `mwb_m120_1234.xspf`

**Character conversion**:
- Spaces → `_` (underscore)
- `>` → `m` (more than)
- `<` → `m` (less than)  
- Special chars (`!`, `#`, `+`, `*`) → `_`
- Long queries are truncated to 50 characters
- 4-digit timestamp suffix prevents filename conflicts

### AI-Powered Episode Sorting ✨

The `--vlc-ai` option uses Google's Gemini API with intelligent web search tools to automatically research, sort, and analyze TV episodes chronologically.

```bash
# Process results with AI for chronological sorting
mwb search "Ostfriesenkrimis >85" -e Audio --vlc-ai

# Works with any search query - AI researches episode chronology
mwb search "Tatort" --vlc-ai

# AI processes and analyzes complex series with web research
mwb search "documentary climate" -s 50 --vlc-ai
```

**Example Output:**
```bash
C:\Users\user> mwb search "Ostfriesenkrimis >85" -e Audio --vlc-ai
🚀 Initializing Gemini AI processor...
🤖 Processing 23 results with Gemini AI for chronological sorting...
🔄 Iteration 1 - Sending request to Gemini...
🔧 Gemini is calling tool: perform_google_search
🔍 Searching for: 'Ostfriesenkrimis episodes chronological order Wikipedia'
🔧 Gemini is calling tool: read_website_content
📖 Reading content from: 'https://de.wikipedia.org/wiki/Ostfriesenkrimis'
🔄 Iteration 3 - Sending request to Gemini...
✅ Received final response from Gemini

✅ AI Processing Results:
==================================================
# Ostfriesenkrimis - Chronologically Sorted Episodes

## Series Overview
Based on Klaus-Peter Wolf's novels, aired chronologically:

1. **Ostfriesenfluch** (2017)
   - Air Date: 2017-09-18
   - Summary: Ann Kathrin Klaasen investigates her first case...

[Detailed chronological listing continues...]
==================================================

📄 Results saved to: ai_sorted_episodes_20231201_143022.txt
```

**Key Features**:
- **Integrated Web Research**: Built-in tools search Wikipedia, fernsehserien.de, and other sources
- **Chronological Analysis**: AI determines proper episode order using multiple sources
- **German TV Expertise**: Optimized for German broadcasting content and episode guides
- **Duplicate Detection**: Automatically identifies and removes duplicate episodes
- **Detailed Summaries**: Provides episode descriptions and air dates when available
- **Series Grouping**: Organizes episodes by series and seasons
- **Source Attribution**: Cites research sources for transparency

**Technical Implementation**:
- **Direct API Integration**: Uses Gemini API directly (no external CLI required)
- **Tool-Calling Architecture**: AI can dynamically search and read websites
- **Intelligent Prompting**: Specialized prompts for German TV content analysis
- **Robust Error Handling**: Graceful fallbacks if web sources are unavailable
- **Progress Feedback**: Real-time updates on AI reasoning process

**Requirements**:
- Google AI Studio API key (free tier available)
- Internet connection for web research
- `.env` file with `GOOGLE_API_KEY` configured

**Research Sources Used**:
- Wikipedia (de.wikipedia.org)
- Fernsehserien.de
- IMDB episode guides
- Official broadcaster websites
- TVButler.de

The AI conducts thorough research to ensure accurate chronological ordering, making it perfect for binge-watching series in the correct sequence.

### List Available Channels

```bash
mwb channels
```

### Search Options

```bash
mwb search [QUERY...] [OPTIONS]

OPTIONS:
    -e, --exclude <EXCLUDE>...     Exclude regex patterns (space-separated)
    -i, --include <INCLUDE>...     Include regex patterns - only show matching results (space-separated)
    -s, --size <SIZE>             Maximum number of results [default: 15]
    -o, --offset <OFFSET>         Offset for pagination [default: 0]
    -b, --sort-by <SORT_BY>       Sort by field (timestamp, duration, channel) [default: timestamp]
    -r, --sort-order <SORT_ORDER> Sort order (asc or desc) [default: desc]
        --no-future               Exclude future content (default: include future content)
    -c, --count                   Show only the count of results
    -f, --format <FORMAT>         Output format (table, json, csv, oneline, onelinetheme, xspf) [default: onelinetheme]
    -v, --vlc[=<QUALITY>]         Save video links as VLC playlist and launch VLC
                                  Quality options: l (low), m (medium, default), h (HD)
        --vlc-ai                  Process results with AI (Gemini) for chronological sorting,
                                  deduplication, and VLC playlist creation
```

## Search Syntax Details

### Combining Selectors

You can combine multiple selectors to refine your search:

```bash
# All Tatort episodes from ARD or ZDF
mwb search "!ARD !ZDF #Tatort"

# Documentaries about nature longer than 45 minutes (explicit selectors)
mwb search "#Dokumentation +Natur >45"

# Or use simplified syntax (automatically processed)
mwb search "dokumentation natur >45"

# Short news segments from specific channels
mwb search "!ARD !ZDF nachrichten <15"
```

### Automatic Query Processing

The CLI automatically processes queries that mix search terms with duration selectors:

- **Search terms + duration**: `"tatort >85"` → searches all fields (title, topic, description, channel) with duration filter applied server-side
- **Multiple terms + duration**: `"dokumentation klima >30"` → searches all fields for content containing both terms, filtered by duration
- **Explicit selectors**: `"#tatort >85"` → unchanged (user knows what they want)
- **Duration only**: `">90"` → unchanged (duration-only search)

The improved processing allows natural all-field search while extracting duration selectors for server-side filtering, providing comprehensive results across all content fields.

### Duration Filtering Best Practices

Duration selectors are processed server-side by the mediathekviewweb API. The CLI automatically handles mixed search terms and duration selectors:

```bash
# Feature-length documentaries (90+ minutes) - searches all fields naturally
mwb search "dokumentation >90"

# Quick news updates (under 5 minutes) - searches all fields naturally
mwb search "nachrichten <5"

# Standard TV program length (between 45-90 minutes)
mwb search ">45 <90"

# Long-form investigative reports (over 60 minutes)
mwb search "reportage >60"

# Multiple search terms with duration - automatically uses description search
mwb search "climate change documentary >30"

# Combine duration with explicit selectors
mwb search "!Arte #Dokumentation Klima >30 <120"
```

### Multiple Values (OR Search)

Using the same selector multiple times creates an OR condition:

```bash
# Content from ARD OR ZDF with topic Reportage
mwb search "!ARD !ZDF #Reportage"
```

### Regex Filtering

The `--exclude` and `--include` options use regular expressions for powerful content filtering:

#### Exclusion Filtering
```bash
# Exclude sports content using regex patterns
mwb search "#Dokumentation" --exclude "Sport|Fußball|Tennis"

# Exclude weather reports with word boundaries
mwb search "#Nachrichten" --exclude "\bWetter\b" "Wettervorhersage"

# Exclude multiple patterns with advanced regex
mwb search "documentary" --exclude "sport.*ball" "weather.*forecast"
```

#### Inclusion Filtering
```bash
# Only show content about climate or environment
mwb search "Dokumentation" --include "Klima|Umwelt|Environment"

# Find content with specific actors or directors
mwb search "!Arte" --include "Tatort.*Münster" "Regie.*Fatih Akin"

# Combine include and exclude for precise filtering
mwb search "#Nachrichten" --include "Politik|Wirtschaft" --exclude "Sport|Wetter"
```

#### Regex Syntax Examples
- `word1|word2` - Match either word1 OR word2
- `\bword\b` - Match whole word only (word boundaries)
- `word.*` - Match "word" followed by any characters
- `^word` - Match lines starting with "word"
- `word$` - Match lines ending with "word"
- `[Tt]atort` - Match "Tatort" or "tatort"
- `\d{4}` - Match exactly 4 digits (for years)
- `(?i)munich|münchen` - Case-insensitive match for Munich (German/English)

## Practical Examples

### Finding Recent Documentaries

```bash
# Recent long-form documentaries, newest first (using short forms)
mwb search "#Dokumentation >45" -b timestamp -r desc -s 10
```

### Channel-specific Content with Duration

```bash
# Everything from Arte longer than 60 minutes (using short form)
mwb search "!Arte >60" -s 20

# Short segments from public broadcasters (using short form)
mwb search "!ARD !ZDF !NDR <20" -s 30
```

### Topic-based Duration Searches

```bash
# Long investigative reports
mwb search "Reportage Investigation >90"

# Quick news summaries
mwb search "#Nachrichten Zusammenfassung <10"

# Feature-length crime shows
mwb search "#Tatort #Polizeiruf >80 <100"
```

### Complex Duration and Content Filtering

```bash
# Long documentaries about climate, excluding weather reports (using short form)
mwb search "#Dokumentation Klima >60" -e "\bWetter\b|Wettervorhersage"

# Short educational content for children
mwb search "!KiKA Lernen Schule <30"

# Medium-length cultural programs from Arte
mwb search "!Arte Kultur >30 <90"
```

### Multi-Search Examples ✨

```bash
# Compare content across public broadcasters
mwb search ARD ZDF BR --size 15 --format table

# Find crime content across different series
mwb search Tatort Krimi "Polizeiruf 110" --count

# Multi-topic documentary search
mwb search "#Dokumentation" "#Reportage" "#Investigation" --size 20 --exclude "Sport.*"

# Channel and topic combination search  
mwb search "!Arte" "#Kultur" "Dokumentation" --format json

# Quick news comparison across channels
mwb search "!ARD Nachrichten" "!ZDF heute" "!NDR aktuell" --size 10 --verbose
```

### Exporting Data

```bash
# Export long-form content to CSV (using short forms)
mwb search ">90" -s 50 -f csv > long_content.csv

# Get JSON for documentaries over an hour (using short form)
mwb search "#Dokumentation >60" -f json | jq '.[] | {title, duration, url_video}'

# Create VLC playlist with long documentaries (using short form, low quality)
mwb search "#Dokumentation >90" -s 25 -v=l
```

## Output Fields

The table format displays:
- **Channel**: Broadcasting channel (ARD, ZDF, etc.)
- **Topic**: Program topic or series name
- **Title**: Episode or program title
- **Duration**: Length in hours, minutes, and seconds
- **Date**: Broadcast date and time
- **Video URL**: Direct link to video stream
- **Description**: Program description (truncated to 200 characters)

## API Information

This tool uses the public MediathekViewWeb API at `https://mediathekviewweb.de/api/` through the official [mediathekviewweb](https://crates.io/crates/mediathekviewweb) Rust crate. The API provides access to content from German public broadcasters including ARD, ZDF, and their regional stations.

### Benefits of Using the Official Crate

- **Built-in Duration Selectors**: Native support for `>90`, `<30` syntax
- **Robust Error Handling**: Built-in error handling and retry logic
- **Type Safety**: Strongly-typed Rust structs for all API responses
- **Efficient Parsing**: Optimized JSON parsing and data structures
- **Future-Proof**: Automatic updates when the API changes
- **Built-in Query Builder**: Native support for MediathekView search syntax

## Real-World Examples

### Finding Recent Crime Shows
```bash
# Find all standard-length Tatort episodes (typically 90 minutes, using short forms)
mwb search "#Tatort >80 <100" -b timestamp -r desc -s 10

# Find crime shows of any length but exclude short clips and trailers (using short form)
mwb search "Krimi Tatort Polizeiruf >20" -e "Trailer|Preview|Vorschau"

# Only show Tatort episodes from specific cities, full length (using short form)
mwb search "#Tatort >75" -i "Münster|Stuttgart|Bremen"
```

### Researching Specific Topics
```bash
# Find substantial documentaries about climate change (using short forms)
mwb search "Klima Klimawandel >30" -e "\bWetter\b|Wettervorhersage"

# Search for in-depth news analysis (longer segments, using short forms)
mwb search "!ARD !ZDF Analyse >15" -i "Politik|Wirtschaft"

# Find comprehensive science documentaries (using short forms)
mwb search "Wissenschaft >60" -i "Physik|Chemie|Astronomie" -e "Kurz|Short"
```

### Media Analysis and Export
```bash
# Export all substantial Arte content (over 45 minutes) to CSV (using short forms)
mwb search "!ARTE.DE >45" -s 100 -f csv > arte_longform.csv

# Find and analyze duration patterns in documentaries (using short forms)
mwb search "#Dokumentation" -s 200 -f json | jq '.[] | {title, duration_seconds: .duration}'
```

### Educational Content Discovery
```bash
# Find comprehensive educational programs for adults (using short forms)
mwb search "Bildung Wissen >30" -e "Kinder|Children"

# Short educational clips for quick learning (using short form)
mwb search "!KiKA Lernen <15" -s 20

# University-level lectures and discussions
mwb search "Universität Vorlesung >45"

# Create VLC playlist with educational content (HD quality)
# Creates file: mwb_Bildung_Wissenschaft_m30_1234.xspf
mwb search "Bildung Wissenschaft >30" --vlc=h
```

### VLC Playlist for Binge Watching
```bash
# Create a playlist of all Tatort episodes over 80 minutes for weekend viewing
# Creates file: mwb_tatort_m80_1234.m3u with chronological dates
mwb search "tatort >80" -s 50 -v

# Arte documentaries for educational viewing session with broadcast dates
# Creates file: mwb__Arte_dokumentation_m45_1234.m3u  
mwb search "!Arte dokumentation >45" -s 20 -e "trailer|preview" -v

# Crime series marathon - exclude short clips and audio descriptions
# Creates file: mwb_krimi_investigation_m70_1234.m3u with episode dates
mwb search "krimi investigation >70" -s 30 -e "audio|kurz|short" -v

# International content playlist from specific channels with chronological info
# Creates file: mwb__Arte__3Sat_m60_1234.m3u
mwb search "!Arte !3Sat >60" -s 25 -i "deutsch|german|english" -v
```

### AI-Powered Smart Playlists ✨
```bash
# Let AI sort Ostfriesenkrimis chronologically and create optimized VLC playlist
# AI removes duplicates and sorts by episode order from Wikipedia
mwb search "Ostfriesenkrimis >85" -e Audio --vlc-ai

# Smart Tatort playlist - AI sorts by air date and removes duplicate broadcasts
mwb search "Tatort" -s 100 --vlc-ai

# AI-curated documentary collection with intelligent sorting and deduplication
mwb search "dokumentation climate change >30" -s 50 --vlc-ai

# Crime series with AI chronological ordering - perfect for binge watching
mwb search "krimi investigation >60" -e "audio|trailer" --vlc-ai

# Educational content with AI-powered organization
mwb search "wissenschaft >30" -s 25 --vlc-ai
```

**AI Benefits over Regular VLC Integration:**
- **Chronological Order**: Episodes sorted by actual air date, not just broadcast timestamp
- **Duplicate Detection**: Removes repeat broadcasts and identical content
- **Series Recognition**: Groups and orders episodes from the same series correctly
- **Real-time Progress**: Live streaming of AI processing steps and progress updates
- **Manual Fallback**: Provides exact command if Gemini is unavailable
- **Smart Filtering**: AI can identify and exclude trailers, previews, and audio descriptions

### Curated Content Collections
```bash
# Create themed playlists for specific interests with broadcast dates
# Creates file: mwb_wissenschaft_physik_astronomie_m40_1234.m3u
mwb search "wissenschaft physik astronomie >40" -s 15 -v

# Historical documentaries playlist - dates help avoid duplicates
# Creates file: mwb_geschichte_dokumentation_m50_1234.m3u
mwb search "geschichte dokumentation >50" -s 20 -e "wiederholung|repeat" -v

# Nature and environment content for relaxing viewing with chronological order
# Creates file: mwb_natur_umwelt_tiere_m30_1234.m3u
mwb search "natur umwelt tiere >30" -s 25 -i "wild|forest|ocean" -v
```

## Tips and Tricks

1. **Use Duration Selectors**: The `>` and `<` selectors are processed server-side and are more efficient than client-side filtering.

2. **Combine Duration with Topics**: `#Dokumentation >60` is more precise than searching for documentaries and filtering duration later.

3. **Master Regex Filtering**: The `--exclude` and `--include` options complement duration selectors for precise content discovery.

4. **Standard Duration Ranges**:
   - News segments: `<20` minutes
   - Talk shows: `>30 <60` minutes
   - Standard TV programs: `>45 <90` minutes
   - Feature-length: `>90` minutes
   - Short clips/trailers: `<5` minutes

5. **Export for Analysis**: Use `-f json` or `-f csv` to export data for further processing.

6. **Pagination**: Use `-o` and `-s` for browsing through large result sets.

7. **Future Content**: By default, the CLI includes future/scheduled content. Use `--no-future` to exclude it.

8. **Short Forms**: All options have short forms for faster typing: `-s` (size), `-o` (offset), `-b` (sort-by), `-r` (sort-order), `-f` (format), `-e` (exclude), `-i` (include).

9. **AI-Powered Organization**: Use `--vlc-ai` for series and episodic content to get chronologically ordered, duplicate-free playlists optimized for binge-watching. Especially useful for crime series like Tatort or documentary series.

10. **Smart Query Processing**: You can mix search terms with duration selectors naturally:
   - `"tatort >85"` searches all fields (title, topic, description, channel) with duration filtering
   - `"climate change documentary >60"` finds content containing all terms across all fields
   - Provides comprehensive results without being limited to specific fields
   - No need to remember selector syntax for simple searches

11. **VLC Playlist Features**:
    - Quality selection: `-v` (medium), `-v=l` (low), `-v=m` (medium), `-v=h` (HD)
    - Filenames generated from search query (e.g., `mwb_tatort_m85_1234.xspf`)  
    - Include broadcast dates in YYYY-MM-DD format for chronological identification
    - Query-based naming makes playlist management easy
    - Automatic fallback to medium quality if HD not available

11. **Duration Query Examples**:
    - `>90 <180` - Feature films and long documentaries
    - `>20 <45` - Standard program segments  
    - `<10` - News updates and short clips
    - `>60` - In-depth content and investigations

### Quick Reference - Short Forms

| Long Form | Short | Description |
|-----------|-------|-------------|
| `--exclude` | `-e` | Exclude regex patterns |
| `--include` | `-i` | Include regex patterns |
| `--size` | `-s` | Maximum results |
| `--offset` | `-o` | Pagination offset |
| `--sort-by` | `-b` | Sort field |
| `--sort-order` | `-r` | Sort order (asc/desc) |
| `--count` | `-c` | Show only count of results |
| `--format` | `-f` | Output format |
| `--vlc[=QUALITY]` | `-v[=QUALITY]` | Create VLC playlist with quality option |
| `--no-future` | - | Exclude future content |

## Troubleshooting

- **No Results Found**: Try broader search terms or check selector syntax
- **Duration Not Working**: Make sure to use `>` and `<` with numbers (minutes)
- **API Errors**: The service might be temporarily unavailable
- **Slow Responses**: Try reducing `--size` or using more specific selectors
- **VLC Not Found**: If VLC doesn't launch, check your VLC installation path or manually open the created `.xspf` file
- **Invalid Quality**: Invalid quality parameters default to medium with a warning message

## Contributing

[Add contribution guidelines here]

## License

[Add your license information here]