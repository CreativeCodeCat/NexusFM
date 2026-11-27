// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Where a folder is: a local path, or a URI for another kind of location.

use std::path::{Path, PathBuf};

use crate::NexusError;

/// A place a folder can be opened from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A folder on this computer.
    Local(PathBuf),
    /// A URI with a scheme NexusFM does not open yet, such as `smb://` or `sftp://`.
    Remote(String),
}

impl Location {
    /// Interprets text typed in the location bar or stored elsewhere.
    ///
    /// Accepts absolute paths, `~` and `~/...` (relative to `home`), and
    /// `file://` URIs. Any other `scheme://` text becomes a [`Location::Remote`].
    pub fn parse(text: &str, home: &Path) -> Result<Self, NexusError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(NexusError::Message("Enter a location".into()));
        }
        if let Some(rest) = text.strip_prefix("file://") {
            let path =
                decode(rest).ok_or_else(|| NexusError::Message(format!("invalid URI: {text}")))?;
            return Ok(Self::Local(PathBuf::from(path)));
        }
        if has_scheme(text) {
            return Ok(Self::Remote(text.to_string()));
        }
        if text == "~" {
            return Ok(Self::Local(home.to_path_buf()));
        }
        if let Some(rest) = text.strip_prefix("~/") {
            return Ok(Self::Local(home.join(rest)));
        }
        if text.starts_with('/') {
            return Ok(Self::Local(PathBuf::from(text)));
        }
        Err(NexusError::Message(format!(
            "\"{text}\" is not a path or a URI"
        )))
    }

    /// The URI for this location.
    pub fn to_uri(&self) -> String {
        match self {
            Self::Local(path) => format!("file://{}", encode(&path.to_string_lossy())),
            Self::Remote(uri) => uri.clone(),
        }
    }
}

/// Whether `text` starts with a URI scheme such as `smb:` (letters, digits, `+`, `-`, `.`, then `://`).
fn has_scheme(text: &str) -> bool {
    match text.split_once("://") {
        Some((scheme, _)) => {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        None => false,
    }
}

/// Percent-encodes text for a URI path, keeping `/` and unreserved characters.
pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Decodes percent-escapes in URI text. Returns `None` for malformed escapes or
/// bytes that are not UTF-8.
pub fn decode(encoded: &str) -> Option<String> {
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
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        PathBuf::from("/home/u")
    }

    #[test]
    fn parses_paths_and_home_shortcuts() {
        assert_eq!(
            Location::parse("/srv/data", &home()).unwrap(),
            Location::Local("/srv/data".into())
        );
        assert_eq!(
            Location::parse("~", &home()).unwrap(),
            Location::Local("/home/u".into())
        );
        assert_eq!(
            Location::parse("~/Music", &home()).unwrap(),
            Location::Local("/home/u/Music".into())
        );
    }

    #[test]
    fn parses_file_uris() {
        assert_eq!(
            Location::parse("file:///tmp/My%20Files", &home()).unwrap(),
            Location::Local("/tmp/My Files".into())
        );
        assert!(Location::parse("file:///bad%zz", &home()).is_err());
    }

    #[test]
    fn other_schemes_are_remote() {
        assert_eq!(
            Location::parse("smb://server/share", &home()).unwrap(),
            Location::Remote("smb://server/share".into())
        );
    }

    #[test]
    fn rejects_empty_and_relative_text() {
        assert!(Location::parse("   ", &home()).is_err());
        assert!(Location::parse("Documents", &home()).is_err());
    }

    #[test]
    fn local_locations_round_trip_through_uri() {
        let location = Location::Local("/tmp/My Files/a+b".into());
        let uri = location.to_uri();
        assert_eq!(uri, "file:///tmp/My%20Files/a%2Bb");
        assert_eq!(Location::parse(&uri, &home()).unwrap(), location);
    }
}
