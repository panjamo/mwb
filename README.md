# MWB - MediathekViewWeb CLI

A Rust command-line interface for searching German public broadcasting content via the [MediathekViewWeb API](https://mediathekviewweb.de/).

Built using the official [mediathekviewweb](https://crates.io/crates/mediathekviewweb) Rust crate for robust and efficient API interaction.

## Features

- **Fast Search**: Search through thousands of German TV shows, documentaries, and news content
- **Advanced Filtering**: Use MediathekView's powerful search syntax with selectors
- **Duration Selectors**: Filter content by duration directly in the query (e.g., `>90` for content longer than 90 minutes)
- **Exclusion Filter**: Filter out unwanted content using regex patterns
- **Multiple Output Formats**: View results as formatted tables, JSON, or CSV
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

## Usage

### Basic Search

```bash
# Search for content containing "Tatort"
mwb search Tatort

# Search with multiple terms
mwb search "climate change documentary"
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
mwb search "#Dokumentation climate change >30"

# Find news content from multiple channels
mwb search "!ARD !ZDF !NDR #Nachrichten"

# Search with regex exclusion (removes results matching patterns)
mwb search "documentary" --exclude "sport.*ball" "weather.*report"

# Search with regex inclusion (only show results matching patterns)
mwb search "Nachrichten" --include "Politik|Wirtschaft" --exclude "Sport|Wetter"

# Find recent short clips from Arte
mwb search "!Arte <20" --sort-by timestamp --sort-order desc --size 20
```

### Output Formats

```bash
# Default table format (human-readable)
mwb search "Tatort"

# JSON output for scripting
mwb search "Tatort" --format json

# CSV output for spreadsheets
mwb search "Tatort" --format csv > results.csv
```

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
        --sort-by <SORT_BY>       Sort by field (timestamp, duration, channel) [default: timestamp]
        --sort-order <SORT_ORDER> Sort order (asc or desc) [default: desc]
        --include-future          Include future content
        --format <FORMAT>         Output format (table, json, csv) [default: table]
```

## Search Syntax Details

### Combining Selectors

You can combine multiple selectors to refine your search:

```bash
# All Tatort episodes from ARD or ZDF
mwb search "!ARD !ZDF #Tatort"

# Documentaries about nature longer than 45 minutes
mwb search "#Dokumentation +Natur >45"

# Short news segments from specific channels
mwb search "!ARD !ZDF #Nachrichten <15"
```

### Duration Filtering Best Practices

Duration selectors are processed server-side by the mediathekviewweb API:

```bash
# Feature-length documentaries (90+ minutes)
mwb search "#Dokumentation >90"

# Quick news updates (under 5 minutes)
mwb search "#Nachrichten <5"

# Standard TV program length (between 45-90 minutes)
mwb search ">45 <90"

# Long-form investigative reports (over 60 minutes)
mwb search "Reportage >60"

# Combine duration with topic and channel
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
# Recent long-form documentaries, newest first
mwb search "#Dokumentation >45" --sort-by timestamp --sort-order desc --size 10
```

### Channel-specific Content with Duration

```bash
# Everything from Arte longer than 60 minutes
mwb search "!Arte >60" --size 20

# Short segments from public broadcasters
mwb search "!ARD !ZDF !NDR <20" --size 30
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
# Long documentaries about climate, excluding weather reports
mwb search "#Dokumentation Klima >60" --exclude "\bWetter\b|Wettervorhersage"

# Short educational content for children
mwb search "!KiKA Lernen Schule <30"

# Medium-length cultural programs from Arte
mwb search "!Arte Kultur >30 <90"
```

### Exporting Data

```bash
# Export long-form content to CSV
mwb search ">90" --size 50 --format csv > long_content.csv

# Get JSON for documentaries over an hour
mwb search "#Dokumentation >60" --format json | jq '.[] | {title, duration, url_video}'
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
# Find all standard-length Tatort episodes (typically 90 minutes)
mwb search "#Tatort >80 <100" --sort-by timestamp --sort-order desc --size 10

# Find crime shows of any length but exclude short clips and trailers
mwb search "Krimi Tatort Polizeiruf >20" --exclude "Trailer|Preview|Vorschau"

# Only show Tatort episodes from specific cities, full length
mwb search "#Tatort >75" --include "Münster|Stuttgart|Bremen"
```

### Researching Specific Topics
```bash
# Find substantial documentaries about climate change
mwb search "Klima Klimawandel >30" --exclude "\bWetter\b|Wettervorhersage"

# Search for in-depth news analysis (longer segments)
mwb search "!ARD !ZDF Analyse >15" --include "Politik|Wirtschaft"

# Find comprehensive science documentaries
mwb search "Wissenschaft >60" --include "Physik|Chemie|Astronomie" --exclude "Kurz|Short"
```

### Media Analysis and Export
```bash
# Export all substantial Arte content (over 45 minutes) to CSV
mwb search "!ARTE.DE >45" --size 100 --format csv > arte_longform.csv

# Find and analyze duration patterns in documentaries
mwb search "#Dokumentation" --size 200 --format json | jq '.[] | {title, duration_seconds: .duration}'
```

### Educational Content Discovery
```bash
# Find comprehensive educational programs for adults
mwb search "Bildung Wissen >30" --exclude "Kinder|Children"

# Short educational clips for quick learning
mwb search "!KiKA Lernen <15" --size 20

# University-level lectures and discussions
mwb search "Universität Vorlesung >45"
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

5. **Export for Analysis**: Use `--format json` or `--format csv` to export data for further processing.

6. **Pagination**: Use `--offset` and `--size` for browsing through large result sets.

7. **Duration Query Examples**:
   - `>90 <180` - Feature films and long documentaries
   - `>20 <45` - Standard program segments
   - `<10` - News updates and short clips
   - `>60` - In-depth content and investigations

## Troubleshooting

- **No Results Found**: Try broader search terms or check selector syntax
- **Duration Not Working**: Make sure to use `>` and `<` with numbers (minutes)
- **API Errors**: The service might be temporarily unavailable
- **Slow Responses**: Try reducing `--size` or using more specific selectors

## Contributing

[Add contribution guidelines here]

## License

[Add your license information here]