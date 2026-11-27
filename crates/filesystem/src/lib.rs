// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Filesystem backend for NexusFM.
//!
//! This is a plain `std::fs` implementation for now. It sits behind
//! `list_directory` so a GIO/GVFS backend can replace it without changing callers.

use std::fs;
use std::path::Path;

use nexusfm_core::{Entry, NexusError, Result};

mod applications;
mod metadata;
mod properties;
mod watch;
pub use applications::{default_application, default_in};
pub use metadata::{Metadata, read_metadata};
pub use properties::{FileProperties, permission_string, read_properties};
pub use watch::DirectoryWatcher;

/// Lists the immediate children of `dir`, unsorted.
///
/// Entries that cannot be read (for example, a file removed while listing) are
/// skipped rather than failing the whole listing.
pub fn list_directory(dir: &Path) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for item in fs::read_dir(dir).map_err(|err| NexusError::io(dir, err))? {
        let Ok(item) = item else { continue };
        let path = item.path();
        let Ok(fs_meta) = item.metadata() else {
            continue;
        };
        let meta = Metadata::from_fs(&path, &fs_meta);

        entries.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            path,
            kind: meta.kind,
            size: meta.size,
            modified: meta.modified,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusfm_core::EntryKind;

    #[test]
    fn lists_files_and_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.txt"), b"hi").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let mut entries = list_directory(dir.path()).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "hello.txt");
        assert_eq!(entries[0].kind, EntryKind::File);
        assert_eq!(entries[0].size, 2);
        assert_eq!(entries[1].name, "sub");
        assert_eq!(entries[1].kind, EntryKind::Directory);
    }

    #[test]
    fn missing_directory_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_directory(&dir.path().join("nope")).is_err());
    }
}
