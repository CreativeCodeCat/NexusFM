// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Window layout remembered between runs.
//!
//! This is application state, not a setting, so it lives in its own JSON file
//! under `$XDG_STATE_HOME/nexusfm/window.json` (default `~/.local/state`). It
//! never touches `config.lua`.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Name of the layout file inside the state directory.
pub const WINDOW_FILE_NAME: &str = "window.json";

/// The window's size and which parts are visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    pub width: u32,
    pub height: u32,
    pub sidebar_visible: bool,
    pub sidebar_width: u32,
    pub toolbar_visible: bool,
    pub status_bar_visible: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 960,
            height: 600,
            sidebar_visible: true,
            sidebar_width: 180,
            toolbar_visible: true,
            status_bar_visible: true,
        }
    }
}

impl WindowState {
    /// Parses a layout from JSON. Missing fields keep their defaults; text that
    /// is not a layout gives the defaults.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Reads the saved layout. A missing or unreadable file gives the defaults.
    pub fn load_user(xdg_state_home: Option<&OsStr>, home: Option<&OsStr>) -> Self {
        state_dir(xdg_state_home, home)
            .and_then(|dir| fs::read_to_string(dir.join(WINDOW_FILE_NAME)).ok())
            .map(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    /// Saves the layout. Does nothing when no home folder is known.
    pub fn save_user(
        &self,
        xdg_state_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> io::Result<()> {
        let Some(dir) = state_dir(xdg_state_home, home) else {
            return Ok(());
        };
        fs::create_dir_all(&dir)?;
        let temp = dir.join(format!("{WINDOW_FILE_NAME}.tmp"));
        fs::write(&temp, self.to_json())?;
        fs::rename(&temp, dir.join(WINDOW_FILE_NAME))
    }
}

/// `$XDG_STATE_HOME/nexusfm`, or `$HOME/.local/state/nexusfm`.
pub fn state_dir(xdg_state_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    let base = match xdg_state_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".local/state"),
    };
    Some(base.join("nexusfm"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let state = WindowState {
            width: 1200,
            height: 800,
            sidebar_visible: false,
            sidebar_width: 240,
            toolbar_visible: true,
            status_bar_visible: false,
        };
        assert_eq!(WindowState::from_json(&state.to_json()), state);
    }

    #[test]
    fn bad_or_partial_json_gives_defaults_for_missing_fields() {
        assert_eq!(WindowState::from_json("not json"), WindowState::default());
        let partial = WindowState::from_json(r#"{"width": 1000}"#);
        assert_eq!(partial.width, 1000);
        assert_eq!(partial.height, WindowState::default().height);
    }

    #[test]
    fn saves_and_loads_from_the_state_folder() {
        let root = tempfile::tempdir().unwrap();
        let xdg = Some(root.path().as_os_str());
        assert_eq!(WindowState::load_user(xdg, None), WindowState::default());

        let state = WindowState {
            sidebar_visible: false,
            ..WindowState::default()
        };
        state.save_user(xdg, None).unwrap();
        assert_eq!(WindowState::load_user(xdg, None), state);
        assert!(root.path().join("nexusfm").join(WINDOW_FILE_NAME).is_file());
    }

    #[test]
    fn state_dir_follows_xdg_rules() {
        assert_eq!(
            state_dir(Some(OsStr::new("/s")), None),
            Some(PathBuf::from("/s/nexusfm"))
        );
        assert_eq!(
            state_dir(None, Some(OsStr::new("/h"))),
            Some(PathBuf::from("/h/.local/state/nexusfm"))
        );
    }
}
