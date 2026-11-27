// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Filesystem backend for NexusFM.
//!
//! This is a plain `std::fs` implementation for now. It sits behind
//! `list_directory` so a GIO/GVFS backend can replace it without changing callers.

use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use gio::prelude::*;
use nexusfm_core::{Entry, EntryKind, NexusError, Result};

mod applications;
mod metadata;
mod properties;
mod watch;
pub use applications::{default_application, default_in};
pub use metadata::{Metadata, read_metadata};
pub use properties::{FileProperties, permission_string, read_properties};
pub use watch::DirectoryWatcher;

/// Attributes requested for each entry when a folder is listed.
const LIST_ATTRIBUTES: &str = "standard::name,standard::type,standard::size,time::modified";

/// Lists the immediate children of `dir`, unsorted, through GIO.
///
/// Entries that cannot be read (for example, a file removed while listing) are
/// skipped rather than failing the whole listing.
pub fn list_directory(dir: &Path) -> Result<Vec<Entry>> {
    list_file(&gio::File::for_path(dir)).map_err(|err| {
        NexusError::Message(format!("cannot read {}: {}", dir.display(), err.message()))
    })
}

/// Lists a folder given by URI, such as `file:///...`, `smb://server/share` or
/// `trash:///`. Children of remote folders have URI paths.
pub fn list_uri(uri: &str) -> Result<Vec<Entry>> {
    list_file(&gio::File::for_uri(uri))
        .map_err(|err| NexusError::Message(format!("cannot read {uri}: {}", err.message())))
}

/// Whether `path` is a URI rather than a local path.
pub fn is_uri(path: &Path) -> bool {
    nexusfm_core::location::is_uri(&path.to_string_lossy())
}

/// The folder above `path`. For URIs this is computed by GIO, so `smb://a/b`
/// gives `smb://a`. Returns `None` at the top of the location.
pub fn parent_location(path: &Path) -> Option<PathBuf> {
    if is_uri(path) {
        let parent = gio::File::for_uri(&path.to_string_lossy()).parent()?;
        Some(PathBuf::from(parent.uri().as_str()))
    } else {
        path.parent().map(Path::to_path_buf)
    }
}

fn list_file(folder: &gio::File) -> std::result::Result<Vec<Entry>, gio::glib::Error> {
    let enumerator = folder.enumerate_children(
        LIST_ATTRIBUTES,
        gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
        None::<&gio::Cancellable>,
    )?;

    let mut entries = Vec::new();
    for info in enumerator {
        let Ok(info) = info else { continue };
        let name = info.name().to_string_lossy().into_owned();
        let child = folder.child(&name);
        let path = child
            .path()
            .unwrap_or_else(|| PathBuf::from(child.uri().as_str()));

        let kind = match info.file_type() {
            gio::FileType::Directory => EntryKind::Directory,
            gio::FileType::Regular => EntryKind::File,
            gio::FileType::SymbolicLink => EntryKind::Symlink,
            _ => EntryKind::Other,
        };
        let modified = match info.attribute_uint64("time::modified") {
            0 => None,
            secs => Some(UNIX_EPOCH + Duration::from_secs(secs)),
        };

        entries.push(Entry {
            name,
            path,
            kind,
            size: u64::try_from(info.size()).unwrap_or(0),
            modified,
        });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusfm_core::EntryKind;
    use std::fs;

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

    #[test]
    fn lists_a_file_uri_like_a_local_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"x").unwrap();
        let uri = format!(
            "file://{}",
            nexusfm_core::location::encode(&dir.path().to_string_lossy())
        );

        let entries = list_uri(&uri).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "a.txt");
        assert_eq!(entries[0].path, dir.path().join("a.txt"));
    }

    #[test]
    fn uri_parents_are_computed_by_gio() {
        assert!(is_uri(Path::new("smb://server/share/dir")));
        assert!(!is_uri(Path::new("/home/u")));
        assert_eq!(
            parent_location(Path::new("file:///srv/a/b")),
            Some(PathBuf::from("file:///srv/a"))
        );
        assert_eq!(
            parent_location(Path::new("/srv/a")),
            Some(PathBuf::from("/srv"))
        );
    }
}
