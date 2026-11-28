// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Details shown in a file's Properties dialog.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use nexusfm_core::{EntryKind, Result};

use crate::metadata::read_metadata;

/// What a file's Properties dialog shows.
#[derive(Debug, Clone, PartialEq)]
pub struct FileProperties {
    pub path: PathBuf,
    pub name: String,
    pub kind: EntryKind,
    /// Size in bytes. For a symbolic link, the length of the link itself.
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    /// Permission bits, the low nine bits of the file mode.
    pub mode: u32,
    pub owner: String,
    pub group: String,
    /// Where a symbolic link points, if this is one.
    pub link_target: Option<PathBuf>,
    /// MIME type guessed from the file name, such as `text/plain`.
    pub mime_type: String,
    /// The desktop entry that opens this type by default, if one is set.
    pub default_application: Option<String>,
}

impl FileProperties {
    /// Permissions as `rwxr-x---`.
    pub fn permission_string(&self) -> String {
        permission_string(self.mode)
    }
}

/// Reads the Properties details for `path`. A symbolic link is described
/// itself, not the file it points to.
pub fn read_properties(path: &Path) -> Result<FileProperties> {
    let meta = read_metadata(path)?;
    let kind = meta.kind;
    let link_target = meta.link_target.clone();

    let mime_type = if kind == EntryKind::Directory {
        "inode/directory".to_string()
    } else {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_string()
    };
    let default_application = crate::default_application(&mime_type);

    Ok(FileProperties {
        path: path.to_path_buf(),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        kind,
        size: meta.size,
        modified: meta.modified,
        accessed: meta.accessed,
        created: meta.created,
        mode: meta.mode,
        owner: meta.owner.clone().unwrap_or_else(|| meta.uid.to_string()),
        group: meta.group.clone().unwrap_or_else(|| meta.gid.to_string()),
        link_target,
        mime_type,
        default_application,
    })
}

/// Formats permission bits as `rwxr-x---`.
pub fn permission_string(mode: u32) -> String {
    let flags = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    flags
        .iter()
        .map(|(bit, ch)| if mode & bit != 0 { *ch } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn formats_permission_bits() {
        assert_eq!(permission_string(0o755), "rwxr-xr-x");
        assert_eq!(permission_string(0o640), "rw-r-----");
        assert_eq!(permission_string(0o000), "---------");
    }

    #[test]
    fn describes_a_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("note.txt");
        fs::write(&file, b"hello").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();

        let props = read_properties(&file).unwrap();
        assert_eq!(props.name, "note.txt");
        assert_eq!(props.kind, EntryKind::File);
        assert_eq!(props.size, 5);
        assert_eq!(props.mode, 0o640);
        assert_eq!(props.permission_string(), "rw-r-----");
        assert!(props.modified.is_some());
        assert!(props.link_target.is_none());
        assert!(!props.owner.is_empty());
    }

    #[test]
    fn guesses_mime_type_from_name() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        fs::write(&file, b"x").unwrap();
        assert_eq!(read_properties(&file).unwrap().mime_type, "text/plain");
    }

    #[test]
    fn describes_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let props = read_properties(dir.path()).unwrap();
        assert_eq!(props.kind, EntryKind::Directory);
    }

    #[test]
    fn describes_a_symlink_without_following_it() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::write(&target, b"x").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let props = read_properties(&link).unwrap();
        assert_eq!(props.kind, EntryKind::Symlink);
        assert_eq!(props.link_target, Some(target));
    }

    #[test]
    fn missing_path_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_properties(&dir.path().join("nope")).is_err());
    }
}
