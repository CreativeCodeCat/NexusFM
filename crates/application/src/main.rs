//! NexusFM entry point.
//!
//! For now this lists a directory in the terminal to prove the core, config and
//! filesystem layers work together. The Qt window replaces this later.
//!
//! Log output is controlled by `RUST_LOG` (for example `RUST_LOG=debug`).

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nexusfm_config::{Config, config_file_path};
use nexusfm_core::{EntryKind, sort_entries};
use nexusfm_filesystem::list_directory;
use tracing::{debug, error, warn};
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = load_config();

    let dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    debug!(path = %dir.display(), "listing directory");

    let mut entries = match list_directory(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            error!(path = %dir.display(), %err, "cannot read directory");
            eprintln!("nexusfm: cannot read {}: {err}", dir.display());
            return ExitCode::FAILURE;
        }
    };

    if !config.show_hidden {
        entries.retain(|entry| !entry.name.starts_with('.'));
    }
    sort_entries(&mut entries, config.sort);

    println!("{}", dir.display());
    for entry in &entries {
        let marker = match entry.kind {
            EntryKind::Directory => "/",
            EntryKind::Symlink => "@",
            _ => "",
        };
        println!("  {}{marker}  ({} bytes)", entry.name, entry.size);
    }

    ExitCode::SUCCESS
}

/// Loads the user configuration. Problems fall back to the defaults so a bad
/// file never stops the application from starting.
fn load_config() -> Config {
    let Some(path) = config_file_path(
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    ) else {
        warn!("no HOME or XDG_CONFIG_HOME set; using default configuration");
        return Config::default();
    };
    debug!(path = %path.display(), "loading configuration");

    match Config::load(&path) {
        Ok(config) => config,
        Err(err) => {
            warn!(%err, "using default configuration");
            Config::default()
        }
    }
}
