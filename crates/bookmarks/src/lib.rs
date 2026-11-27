// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Bookmarked folders.
//!
//! The file uses the GTK bookmarks format, one `file://` URI per line with an
//! optional label after a space, so other file managers can read it. It lives at
//! `$XDG_CONFIG_HOME/nexusfm/bookmarks`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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

/// Parses the bookmarks file. Blank lines, comments and lines that are not
/// `file://` URIs are skipped.
pub fn parse(text: &str) -> Vec<Bookmark> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_end();
            let (uri, label) = match line.split_once(' ') {
                Some((uri, label)) => (uri, Some(label.trim().to_string())),
                None => (line, None),
            };
            let path = decode_uri(uri.strip_prefix("file://")?)?;
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
        text.push_str("file://");
        text.push_str(&encode_path(&bookmark.path));
        if let Some(label) = bookmark.label.as_deref().filter(|label| !label.is_empty()) {
            text.push(' ');
            text.push_str(&label.replace(['\n', '\r'], " "));
        }
        text.push('\n');
    }
    text
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

/// Percent-encodes a path for use in a `file://` URI, keeping `/` and unreserved characters.
fn encode_path(path: &Path) -> String {
    let mut out = String::new();
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Decodes percent-escapes in a URI path. Returns `None` for malformed escapes.
fn decode_uri(encoded: &str) -> Option<PathBuf> {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8(out).ok()?))
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
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, PathBuf::from("/home/u/Projects"));
        assert_eq!(list[1].path, PathBuf::from("/tmp/x y"));
        assert_eq!(list[1].label.as_deref(), Some("Shared"));
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
}
