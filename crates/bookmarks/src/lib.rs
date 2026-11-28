// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Bookmarked folders.
//!
//! The file uses the GTK bookmarks format, one URI per line with an optional
//! label after a space, so other file managers can read it. A line `#separator`
//! marks a separator in the sidebar (GTK ignores it as a comment). The file lives
//! at `$XDG_CONFIG_HOME/nexusfm/bookmarks`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nexusfm_core::location::{decode, encode, is_uri};

/// The path that stands for a separator in the list. It is not a real path.
pub const SEPARATOR_PATH: &str = "separator:";

/// The line that stores a separator in the file.
const SEPARATOR_LINE: &str = "#separator";

/// A bookmarked folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    pub path: PathBuf,
    /// A custom name. When absent, the folder's own name is shown.
    pub label: Option<String>,
}

impl Bookmark {
    pub fn new(path: PathBuf) -> Self {
        Self { path, label: None }
    }

    /// A separator line in the sidebar.
    pub fn separator() -> Self {
        Self {
            path: PathBuf::from(SEPARATOR_PATH),
            label: None,
        }
    }

    pub fn is_separator(&self) -> bool {
        self.path.as_os_str() == SEPARATOR_PATH
    }

    /// The name to show: the custom label, else the folder name, else the path.
    pub fn display_name(&self) -> String {
        if let Some(label) = self.label.as_deref().filter(|label| !label.is_empty()) {
            return label.to_string();
        }
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

/// Parses a bookmarks file. Blank lines and other comments are skipped.
/// `file://` URIs become local paths; other URIs (such as `smb://`) are kept as written.
pub fn parse(text: &str) -> Vec<Bookmark> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end();
            if line == SEPARATOR_LINE {
                return Some(Bookmark::separator());
            }
            if line.starts_with('#') || line.is_empty() {
                return None;
            }
            let (uri, label) = match line.split_once(' ') {
                Some((uri, label)) => (uri, Some(label.trim().to_string())),
                None => (line, None),
            };
            let path = match uri.strip_prefix("file://") {
                Some(encoded) => PathBuf::from(decode(encoded)?),
                None if is_uri(uri) => PathBuf::from(uri),
                None => return None,
            };
            Some(Bookmark {
                path,
                label: label.filter(|label| !label.is_empty()),
            })
        })
        .collect()
}

/// Formats bookmarks in the same format [`parse`] reads.
pub fn format(bookmarks: &[Bookmark]) -> String {
    let mut text = String::new();
    for bookmark in bookmarks {
        if bookmark.is_separator() {
            text.push_str(SEPARATOR_LINE);
            text.push('\n');
            continue;
        }
        let path = bookmark.path.to_string_lossy();
        if is_uri(&path) {
            text.push_str(&path);
        } else {
            text.push_str("file://");
            text.push_str(&encode(&path));
        }
        if let Some(label) = bookmark.label.as_deref().filter(|label| !label.is_empty()) {
            text.push(' ');
            text.push_str(&label.replace(['\n', '\r'], " "));
        }
        text.push('\n');
    }
    text
}

/// Adds the entries of `incoming` that are not already in `list`. Separators are
/// not imported. Returns how many were added.
pub fn merge(list: &mut Vec<Bookmark>, incoming: Vec<Bookmark>) -> usize {
    let mut added = 0;
    for bookmark in incoming
        .into_iter()
        .filter(|bookmark| !bookmark.is_separator())
    {
        if !list.iter().any(|existing| existing.path == bookmark.path) {
            list.push(bookmark);
            added += 1;
        }
    }
    added
}

/// Reads the bookmarks file. A missing file gives an empty list.
pub fn load(path: &Path) -> io::Result<Vec<Bookmark>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

/// Writes the bookmarks file, replacing it atomically.
pub fn save(path: &Path, bookmarks: &[Bookmark]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("tmp");
    fs::write(&temp, format(bookmarks))?;
    fs::rename(&temp, path)
}

/// GTK's own bookmarks file, which file managers on GNOME-style desktops use.
pub fn gtk_bookmarks_file(
    xdg_config_home: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("gtk-3.0").join("bookmarks"))
}

/// Where the bookmarks file lives for the given environment values.
pub fn bookmarks_file(
    xdg_config_home: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Option<PathBuf> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("nexusfm").join("bookmarks"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_paths_with_spaces_and_labels() {
        let list = vec![
            Bookmark::new(PathBuf::from("/home/u/My Docs")),
            Bookmark {
                path: PathBuf::from("/data/100%/music"),
                label: Some("Music library".into()),
            },
        ];
        assert_eq!(parse(&format(&list)), list);
    }

    #[test]
    fn reads_gtk_style_lines() {
        let text =
            "# comment\nfile:///home/u/Projects\nfile:///tmp/x%20y Shared\nsmb://server/share\n\n";
        let list = parse(text);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].path, PathBuf::from("/home/u/Projects"));
        assert_eq!(list[1].path, PathBuf::from("/tmp/x y"));
        assert_eq!(list[1].label.as_deref(), Some("Shared"));
        assert_eq!(list[2].path, PathBuf::from("smb://server/share"));
    }

    #[test]
    fn display_name_prefers_label() {
        let plain = Bookmark::new(PathBuf::from("/a/b/Photos"));
        assert_eq!(plain.display_name(), "Photos");
        let named = Bookmark {
            label: Some("Pics".into()),
            ..plain
        };
        assert_eq!(named.display_name(), "Pics");
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nexusfm").join("bookmarks");
        assert!(load(&path).unwrap().is_empty());
        let list = vec![Bookmark::new(PathBuf::from("/srv/files"))];
        save(&path, &list).unwrap();
        assert_eq!(load(&path).unwrap(), list);
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn bookmarks_file_follows_xdg_rules() {
        use std::ffi::OsStr;
        assert_eq!(
            bookmarks_file(Some(OsStr::new("/x")), None),
            Some(PathBuf::from("/x/nexusfm/bookmarks"))
        );
        assert_eq!(
            bookmarks_file(None, Some(OsStr::new("/h"))),
            Some(PathBuf::from("/h/.config/nexusfm/bookmarks"))
        );
    }

    #[test]
    fn separators_round_trip_and_are_not_comments_to_us() {
        let list = vec![
            Bookmark::new(PathBuf::from("/a")),
            Bookmark::separator(),
            Bookmark::new(PathBuf::from("/b")),
        ];
        let text = format(&list);
        assert!(text.contains("#separator\n"));
        assert_eq!(parse(&text), list);
    }

    #[test]
    fn network_uris_are_kept_as_written() {
        let list = parse("smb://server/share Team files\n");
        assert_eq!(list[0].path, PathBuf::from("smb://server/share"));
        assert_eq!(list[0].label.as_deref(), Some("Team files"));
        assert_eq!(parse(&format(&list)), list);
    }

    #[test]
    fn merge_adds_only_new_folders() {
        let mut list = vec![Bookmark::new(PathBuf::from("/a"))];
        let incoming = vec![
            Bookmark::new(PathBuf::from("/a")),
            Bookmark::separator(),
            Bookmark::new(PathBuf::from("/b")),
        ];
        assert_eq!(merge(&mut list, incoming), 1);
        assert_eq!(list.len(), 2);
        assert!(!list.iter().any(Bookmark::is_separator));
    }

    #[test]
    fn gtk_file_lives_in_gtk3_folder() {
        use std::ffi::OsStr;
        assert_eq!(
            gtk_bookmarks_file(Some(OsStr::new("/x")), None),
            Some(PathBuf::from("/x/gtk-3.0/bookmarks"))
        );
    }
}
