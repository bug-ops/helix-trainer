//! Logging initialization with security-focused configuration
//!
//! Configures tracing subscriber with:
//! - Environment-based filtering
//! - Sanitized output (no sensitive data)
//! - stderr output to avoid TUI interference

use anyhow::Result;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize secure logging
///
/// In release mode, logging is disabled by default to avoid interfering with TUI.
/// Set RUST_LOG environment variable to enable logging (e.g., RUST_LOG=debug).
///
/// # Security
///
/// - Filters out sensitive modules at high log levels
/// - Sanitizes output (no thread info, no file paths)
/// - Writes to stderr only
pub fn init_secure_logging() -> Result<()> {
    // Only enable logging if RUST_LOG is explicitly set
    // This prevents log output from interfering with TUI rendering
    if std::env::var("RUST_LOG").is_err() {
        // No logging configured - use a no-op subscriber
        return Ok(());
    }

    // Create filter that excludes sensitive modules at high log levels
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        // Never log PTY communication at debug level in production
        .add_directive("helix_trainer::helix::pty_controller=warn".parse()?)
        .add_directive("helix_trainer::config::scenarios=info".parse()?);

    // Configure formatter to sanitize output
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false) // Don't leak thread info
        .with_thread_names(false)
        .with_file(false) // Don't leak file paths in production
        .with_line_number(true)
        .with_writer(std::io::stderr); // Write to stderr, not stdout

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    Ok(())
}
