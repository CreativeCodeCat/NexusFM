//! NexusFM entry point.
//!
//! For now this lists a directory in the terminal to prove the core and
//! filesystem layers work together. The Qt window replaces this later.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nexusfm_core::{EntryKind, SortOrder, sort_entries};
use nexusfm_filesystem::list_directory;

fn main() -> ExitCode {
    let dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut entries = match list_directory(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("nexusfm: cannot read {}: {err}", dir.display());
            return ExitCode::FAILURE;
        }
    };

    sort_entries(&mut entries, SortOrder::default());

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
