// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Default application lookup, following the XDG `mimeapps.list` rules.

use std::fs;
use std::path::PathBuf;

/// The desktop entry that opens `mime` by default, such as `org.gnome.gedit.desktop`.
///
/// Files are read in XDG priority order: the user's config folder, then the
/// system config folders, then the user's and system data folders. The first
/// file that names a default wins.
pub fn default_application(mime: &str) -> Option<String> {
    mimeapps_files()
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .find_map(|text| default_in(&text, mime))
}

/// Looks up `mime` in the `[Default Applications]` section of a `mimeapps.list` file.
pub fn default_in(text: &str, mime: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line == "[Default Applications]";
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && key.trim() == mime
        {
            return value
                .split(';')
                .map(str::trim)
                .find(|app| !app.is_empty())
                .map(str::to_string);
        }
    }
    None
}

/// The `mimeapps.list` files to consult, most important first.
fn mimeapps_files() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".config")));
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
    let config_dirs = list_env("XDG_CONFIG_DIRS", "/etc/xdg");
    let data_dirs = list_env("XDG_DATA_DIRS", "/usr/local/share:/usr/share");

    let mut files = Vec::new();
    files.extend(config_home.map(|dir| dir.join("mimeapps.list")));
    files.extend(config_dirs.iter().map(|dir| dir.join("mimeapps.list")));
    files.extend(data_home.map(|dir| dir.join("applications/mimeapps.list")));
    files.extend(
        data_dirs
            .iter()
            .map(|dir| dir.join("applications/mimeapps.list")),
    );
    files
}

fn list_env(name: &str, default: &str) -> Vec<PathBuf> {
    let value = std::env::var(name)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string());
    std::env::split_paths(&value).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
[Added Associations]
text/plain=wrong.desktop;

[Default Applications]
text/plain=org.gnome.gedit.desktop;code.desktop;
image/png=eog.desktop
";

    #[test]
    fn finds_first_default_in_default_section() {
        assert_eq!(
            default_in(LIST, "text/plain").as_deref(),
            Some("org.gnome.gedit.desktop")
        );
        assert_eq!(
            default_in(LIST, "image/png").as_deref(),
            Some("eog.desktop")
        );
    }

    #[test]
    fn ignores_other_sections_and_unknown_types() {
        assert_eq!(default_in(LIST, "application/pdf"), None);
        assert_eq!(
            default_in("[Added Associations]\nimage/png=x.desktop\n", "image/png"),
            None
        );
    }
}
