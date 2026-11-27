// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! One description of a filesystem object, shared by listings and properties.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use nexusfm_core::{EntryKind, NexusError, Result};

/// What NexusFM knows about a filesystem object. Symbolic links are described
/// themselves, never the file they point to.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub kind: EntryKind,
    /// Size in bytes. For a symbolic link, the length of the link itself.
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    /// The low nine bits of the file mode (permissions).
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// Where a symbolic link points.
    pub link_target: Option<PathBuf>,
}

impl Metadata {
    /// Describes `path` from metadata that does not follow symbolic links.
    pub fn from_fs(path: &Path, meta: &fs::Metadata) -> Self {
        let file_type = meta.file_type();
        let kind = if file_type.is_symlink() {
            EntryKind::Symlink
        } else if file_type.is_dir() {
            EntryKind::Directory
        } else if file_type.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        let link_target = if kind == EntryKind::Symlink {
            fs::read_link(path).ok()
        } else {
            None
        };

        Self {
            kind,
            size: meta.len(),
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
            created: meta.created().ok(),
            mode: meta.mode() & 0o777,
            uid: meta.uid(),
            gid: meta.gid(),
            link_target,
        }
    }
}

/// Reads [`Metadata`] for `path`, without following a symbolic link.
pub fn read_metadata(path: &Path) -> Result<Metadata> {
    let meta = fs::symlink_metadata(path).map_err(|err| NexusError::io(path, err))?;
    Ok(Metadata::from_fs(path, &meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn describes_files_directories_and_links() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, b"abc").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
        let link = dir.path().join("l");
        std::os::unix::fs::symlink(&file, &link).unwrap();

        let f = read_metadata(&file).unwrap();
        assert_eq!(f.kind, EntryKind::File);
        assert_eq!(f.size, 3);
        assert_eq!(f.mode, 0o640);
        assert!(f.link_target.is_none());

        assert_eq!(
            read_metadata(dir.path()).unwrap().kind,
            EntryKind::Directory
        );

        let l = read_metadata(&link).unwrap();
        assert_eq!(l.kind, EntryKind::Symlink);
        assert_eq!(l.link_target, Some(file));
    }
}
