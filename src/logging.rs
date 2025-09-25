//! Logging configuration using the tracing crate
//! 
//! This module provides structured logging functionality to replace the verbose
//! flag-based logging throughout the application. It uses the tracing crate
//! for structured, hierarchical logging with different levels.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the tracing subscriber based on verbosity level
/// 
/// # Arguments
/// * `verbose` - If true, enables debug-level logging. If false, enables info-level logging.
/// 
/// This function sets up a tracing subscriber that:
/// - Uses structured logging with spans and events
/// - Filters based on log level (debug when verbose, info otherwise)
/// - Outputs to stderr with colored formatting
/// - Includes module paths and line numbers in verbose mode
pub fn init_tracing(verbose: bool) {
    if !verbose {
        // When not verbose, don't initialize any tracing subscriber
        // This completely disables all tracing output
        return;
    }

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mwb=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_line_number(true)
        .with_file(true)
        .with_ansi(true)
        .with_writer(std::io::stderr);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Convenience macros for common logging patterns used throughout the application
/// These replace the previous eprintln! verbose logging patterns

/// Log AI tool calls with structured data
#[macro_export]
macro_rules! log_ai_tool_call {
    ($tool_name:expr) => {
        tracing::debug!(tool = %$tool_name, "AI tool call started");
    };
    ($tool_name:expr, $($key:ident = $value:expr),+) => {
        tracing::debug!(tool = %$tool_name, $($key = %$value),+, "AI tool call started");
    };
}

/// Log AI tool responses with result information
#[macro_export]
macro_rules! log_ai_tool_response {
    ($tool_name:expr, $result_length:expr) => {
        tracing::debug!(
            tool = %$tool_name, 
            result_length = %$result_length, 
            "AI tool call completed"
        );
    };
}

/// Log search operations with timing and result counts
#[macro_export]
macro_rules! log_search_operation {
    ($operation:expr, $duration:expr, $count:expr) => {
        tracing::info!(
            operation = %$operation,
            duration_ms = %$duration.as_millis(),
            result_count = %$count,
            "Search operation completed"
        );
    };
}

/// Log API requests with timing
#[macro_export]
macro_rules! log_api_request {
    ($api:expr, $duration:expr) => {
        tracing::info!(
            api = %$api,
            duration_ms = %$duration.as_millis(),
            "API request completed"
        );
    };
}

/// Log content extraction operations
#[macro_export]
macro_rules! log_content_extraction {
    ($source:expr, $content_length:expr) => {
        tracing::debug!(
            source = %$source,
            content_length = %$content_length,
            "Content extraction completed"
        );
    };
}

/// Log web scraping operations with selector information
#[macro_export]
macro_rules! log_web_scraping {
    ($url:expr, $selector:expr, $elements_found:expr) => {
        tracing::debug!(
            url = %$url,
            selector = %$selector,
            elements_found = %$elements_found,
            "Web scraping selector attempt"
        );
    };
}

/// Log filtering operations showing before/after counts
#[macro_export]
macro_rules! log_filtering {
    ($filter_type:expr, $before:expr, $after:expr) => {
        tracing::info!(
            filter_type = %$filter_type,
            before_count = %$before,
            after_count = %$after,
            "Results filtered"
        );
    };
}