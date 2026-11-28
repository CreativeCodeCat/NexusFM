// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! One description of a filesystem object, shared by listings and properties.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gio::prelude::*;
use nexusfm_core::{EntryKind, NexusError, Result};

/// Attributes read for one object's Properties.
const METADATA_ATTRIBUTES: &str = "standard::type,standard::size,standard::symlink-target,\
    time::modified,time::access,time::created,unix::mode,unix::uid,unix::gid,\
    owner::user,owner::group";

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
    /// Account and group names, when the system knows them.
    pub owner: Option<String>,
    pub group: Option<String>,
    /// Where a symbolic link points.
    pub link_target: Option<PathBuf>,
}

impl Metadata {
    fn from_info(info: &gio::FileInfo) -> Self {
        let kind = match info.file_type() {
            gio::FileType::Directory => EntryKind::Directory,
            gio::FileType::Regular => EntryKind::File,
            gio::FileType::SymbolicLink => EntryKind::Symlink,
            _ => EntryKind::Other,
        };
        let time = |attribute: &str| match info.attribute_uint64(attribute) {
            0 => None,
            secs => Some(UNIX_EPOCH + Duration::from_secs(secs)),
        };

        Self {
            kind,
            size: u64::try_from(info.size()).unwrap_or(0),
            modified: time("time::modified"),
            accessed: time("time::access"),
            created: time("time::created"),
            mode: info.attribute_uint32("unix::mode") & 0o777,
            uid: info.attribute_uint32("unix::uid"),
            gid: info.attribute_uint32("unix::gid"),
            owner: info
                .attribute_string("owner::user")
                .map(|name| name.to_string()),
            group: info
                .attribute_string("owner::group")
                .map(|name| name.to_string()),
            link_target: info.symlink_target(),
        }
    }
}

/// Reads [`Metadata`] for a local path or URI, without following a symbolic link.
pub fn read_metadata(path: &Path) -> Result<Metadata> {
    let target = if crate::is_uri(path) {
        gio::File::for_uri(&path.to_string_lossy())
    } else {
        gio::File::for_path(path)
    };
    let info = target
        .query_info(
            METADATA_ATTRIBUTES,
            gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
            None::<&gio::Cancellable>,
        )
        .map_err(|err| NexusError::Message(format!("{}: {}", path.display(), err.message())))?;
    Ok(Metadata::from_info(&info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
