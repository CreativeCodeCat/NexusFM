//! UI-independent types and rules for NexusFM.
//!
//! Nothing in this crate touches the filesystem or the GUI. The Qt front end
//! and the filesystem backend both build on these types.

use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// What kind of object a directory entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

/// One item shown in a file view.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Directory
    }
}

/// The column a file view is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Modified,
}

/// How a file view is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SortOrder {
    pub key: SortKey,
    pub descending: bool,
}

/// Sorts entries in place: directories always come first, then by `order`.
/// Names break ties and compare case-insensitively.
pub fn sort_entries(entries: &mut [Entry], order: SortOrder) {
    entries.sort_by(|a, b| {
        let dirs_first = b.is_dir().cmp(&a.is_dir());
        if dirs_first != Ordering::Equal {
            return dirs_first;
        }

        let by_key = match order.key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Size => a.size.cmp(&b.size),
            SortKey::Modified => a.modified.cmp(&b.modified),
        };
        let by_key = if order.descending {
            by_key.reverse()
        } else {
            by_key
        };

        by_key.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind,
            size,
            modified: None,
        }
    }

    #[test]
    fn directories_sort_before_files() {
        let mut entries = vec![
            entry("a.txt", EntryKind::File, 1),
            entry("zdir", EntryKind::Directory, 0),
        ];
        sort_entries(&mut entries, SortOrder::default());
        assert_eq!(entries[0].name, "zdir");
    }

    #[test]
    fn names_sort_case_insensitively() {
        let mut entries = vec![
            entry("b", EntryKind::File, 0),
            entry("A", EntryKind::File, 0),
            entry("c", EntryKind::File, 0),
        ];
        sort_entries(&mut entries, SortOrder::default());
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["A", "b", "c"]);
    }

    #[test]
    fn size_descending_keeps_directories_first() {
        let mut entries = vec![
            entry("small", EntryKind::File, 1),
            entry("big", EntryKind::File, 100),
            entry("dir", EntryKind::Directory, 0),
        ];
        let order = SortOrder {
            key: SortKey::Size,
            descending: true,
        };
        sort_entries(&mut entries, order);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["dir", "big", "small"]);
    }
}
