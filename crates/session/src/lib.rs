// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Application state for the NexusFM window, with no GUI dependency.
//!
//! [`Session`] tracks the current folder and its history, sorting and hidden-file
//! settings, and the cut/copy clipboard. The Qt front end calls into it and shows
//! the results. Everything here can be tested without a display.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nexusfm_bookmarks::Bookmark;
use nexusfm_config::{Config, ViewMode};
use nexusfm_core::{Entry, EntryKind, Location, SortKey, SortOrder, sort_entries};
use nexusfm_filesystem::{
    Device, DeviceKind, FileProperties, is_uri, list_directory, list_uri, parent_location,
    read_properties,
};
use nexusfm_operations as ops;

/// A folder's contents, filtered and sorted according to the session settings.
#[derive(Debug, Clone)]
pub struct Listing {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

/// A sidebar entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
}

/// One tab: the folder it shows and its own back and forward history.
#[derive(Debug, Clone, Default)]
struct Tab {
    current: Option<PathBuf>,
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
}

impl Tab {
    /// The name shown on the tab: the folder name, or the whole location for a root.
    fn title(&self) -> String {
        let Some(path) = self.current.as_deref() else {
            return "New Tab".into();
        };
        let text = path.to_string_lossy();
        let trimmed = text.trim_end_matches('/');
        match trimmed.rsplit_once('/') {
            Some((_, name)) if !name.is_empty() => name.to_string(),
            _ if trimmed.is_empty() => "/".into(),
            _ => text.into_owned(),
        }
    }
}

/// The state of one NexusFM window.
#[derive(Debug, Clone)]
pub struct Session {
    home: PathBuf,
    config: Config,
    /// Open tabs. There is always at least one.
    tabs: Vec<Tab>,
    /// Index of the tab on screen.
    active: usize,
    clipboard: Vec<PathBuf>,
    clipboard_is_cut: bool,
    bookmarks: Vec<Bookmark>,
}

impl Session {
    /// Starts a session with `config`. `home` is the user's home folder.
    pub fn new(config: Config, home: PathBuf) -> Self {
        Self {
            home,
            config,
            tabs: vec![Tab::default()],
            active: 0,
            clipboard: Vec::new(),
            clipboard_is_cut: false,
            bookmarks: Vec::new(),
        }
    }

    /// The bookmarked folders, in sidebar order.
    pub fn bookmarks(&self) -> &[Bookmark] {
        &self.bookmarks
    }

    /// Replaces the bookmark list, for example after loading it from disk.
    pub fn set_bookmarks(&mut self, bookmarks: Vec<Bookmark>) {
        self.bookmarks = bookmarks;
    }

    /// Bookmarks the current folder. Returns `false` if it is already bookmarked
    /// or no folder is open.
    pub fn add_bookmark(&mut self) -> bool {
        let Some(current) = self.tabs[self.active].current.clone() else {
            return false;
        };
        self.add_bookmark_path(current)
    }

    /// Bookmarks the folder at `path`. Returns `false` if it is not a folder, is
    /// already bookmarked, or cannot be read.
    pub fn add_bookmark_path(&mut self, path: PathBuf) -> bool {
        let current = clean(&path);
        let is_folder = nexusfm_filesystem::read_metadata(&current)
            .is_ok_and(|meta| meta.kind == EntryKind::Directory);
        if !is_folder {
            return false;
        }
        if self
            .bookmarks
            .iter()
            .any(|bookmark| bookmark.path == current)
        {
            return false;
        }
        self.bookmarks.push(Bookmark::new(current));
        true
    }

    /// Removes the bookmark at `index`, if there is one.
    pub fn remove_bookmark(&mut self, index: usize) {
        if index < self.bookmarks.len() {
            self.bookmarks.remove(index);
        }
    }

    /// Gives the bookmark at `index` a custom name. An empty name restores the folder name.
    pub fn rename_bookmark(&mut self, index: usize, name: &str) {
        if let Some(bookmark) = self.bookmarks.get_mut(index) {
            let name = name.trim();
            bookmark.label = (!name.is_empty()).then(|| name.to_string());
        }
    }

    /// Moves the bookmark at `index` by `delta` places (negative is up).
    /// Inserts a separator at position `at` in the bookmark list (at most the end).
    pub fn insert_separator(&mut self, at: usize) {
        let at = at.min(self.bookmarks.len());
        self.bookmarks.insert(at, Bookmark::separator());
    }

    /// Moves the bookmark at `from` so that it ends up at position `to`.
    pub fn reorder_bookmark(&mut self, from: usize, to: usize) {
        if from >= self.bookmarks.len() {
            return;
        }
        let item = self.bookmarks.remove(from);
        let to = to.min(self.bookmarks.len());
        self.bookmarks.insert(to, item);
    }

    /// Adds the bookmarks in `incoming` that are not already listed. Returns how many were added.
    pub fn import_bookmarks(&mut self, incoming: Vec<Bookmark>) -> usize {
        nexusfm_bookmarks::merge(&mut self.bookmarks, incoming)
    }

    pub fn move_bookmark(&mut self, index: usize, delta: isize) {
        let Some(target) = index.checked_add_signed(delta) else {
            return;
        };
        if index < self.bookmarks.len() && target < self.bookmarks.len() {
            self.bookmarks.swap(index, target);
        }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn current(&self) -> Option<&Path> {
        self.tabs[self.active].current.as_deref()
    }

    pub fn show_hidden(&self) -> bool {
        self.config.show_hidden
    }

    /// The settings as they stand now, including changes made in the window.
    pub fn settings(&self) -> &Config {
        &self.config
    }

    pub fn view_mode(&self) -> ViewMode {
        self.config.view
    }

    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.config.view = mode;
    }

    pub fn sort_order(&self) -> SortOrder {
        self.config.sort
    }

    pub fn can_go_back(&self) -> bool {
        !self.tabs[self.active].back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.tabs[self.active].forward.is_empty()
    }

    pub fn can_paste(&self) -> bool {
        !self.clipboard.is_empty()
    }

    /// Opens `path`, recording the previous folder in the back history.
    /// On failure the session stays where it was.
    pub fn navigate(&mut self, path: &Path) -> Result<Listing, String> {
        let target = clean(path);
        let listing = self.list(&target)?;
        let previous = self.tabs[self.active].current.take();
        if previous
            .as_ref()
            .is_some_and(|previous| *previous != target)
        {
            self.tabs[self.active].back.extend(previous);
            self.tabs[self.active].forward.clear();
        }
        self.tabs[self.active].current = Some(target);
        Ok(listing)
    }

    /// Returns to the previous folder.
    pub fn go_back(&mut self) -> Result<Listing, String> {
        let Some(previous) = self.tabs[self.active].back.last().cloned() else {
            return Err("There is no earlier folder".into());
        };
        let listing = self.list(&previous)?;
        self.tabs[self.active].back.pop();
        if let Some(current) = self.tabs[self.active].current.take() {
            self.tabs[self.active].forward.push(current);
        }
        self.tabs[self.active].current = Some(previous);
        Ok(listing)
    }

    /// Returns to the folder that was left by the last back step.
    pub fn go_forward(&mut self) -> Result<Listing, String> {
        let Some(next) = self.tabs[self.active].forward.last().cloned() else {
            return Err("There is no later folder".into());
        };
        let listing = self.list(&next)?;
        self.tabs[self.active].forward.pop();
        if let Some(current) = self.tabs[self.active].current.take() {
            self.tabs[self.active].back.push(current);
        }
        self.tabs[self.active].current = Some(next);
        Ok(listing)
    }

    /// Number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Index of the tab on screen.
    pub fn active_tab(&self) -> usize {
        self.active
    }

    /// The title of each tab, in order.
    pub fn tab_titles(&self) -> Vec<String> {
        self.tabs.iter().map(Tab::title).collect()
    }

    /// The tab titles as a JSON array of strings, for the QML front end.
    pub fn tabs_json(&self) -> String {
        serde_json::Value::from(self.tab_titles()).to_string()
    }

    /// The folder of each tab, in order. Tabs that have not opened a folder are left out.
    pub fn tab_paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter_map(|tab| tab.current.clone())
            .collect()
    }

    /// Replaces the tabs with one per readable folder in `paths`, and shows the
    /// one at `active` (or the nearest remaining one). Folders that cannot be read
    /// are skipped. Returns an error, and leaves the tabs alone, if none can be read.
    pub fn restore_tabs(&mut self, paths: &[PathBuf], active: usize) -> Result<Listing, String> {
        let mut tabs = Vec::new();
        let mut new_active = 0;
        for (index, path) in paths.iter().enumerate() {
            let target = clean(path);
            if self.list(&target).is_err() {
                continue;
            }
            if index <= active {
                new_active = tabs.len();
            }
            tabs.push(Tab {
                current: Some(target),
                ..Tab::default()
            });
        }
        if tabs.is_empty() {
            return Err("None of the saved tabs could be opened".into());
        }
        self.tabs = tabs;
        self.active = new_active;
        self.reload()
    }

    /// Opens a new tab showing the current folder, and switches to it.
    pub fn new_tab(&mut self) -> Result<Listing, String> {
        let folder = self
            .current()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.home.clone());
        self.open_in_new_tab(&folder)
    }

    /// Opens `path` in a new tab after the current one, and switches to it.
    /// If the folder cannot be read, no tab is added.
    pub fn open_in_new_tab(&mut self, path: &Path) -> Result<Listing, String> {
        let target = clean(path);
        let listing = self.list(&target)?;
        let tab = Tab {
            current: Some(target),
            ..Tab::default()
        };
        self.active += 1;
        self.tabs.insert(self.active, tab);
        Ok(listing)
    }

    /// Shows the tab at `index`.
    pub fn switch_tab(&mut self, index: usize) -> Result<Listing, String> {
        if index >= self.tabs.len() {
            return Err("There is no such tab".into());
        }
        self.active = index;
        self.reload()
    }

    /// Closes the tab at `index` and shows its neighbour. The last tab cannot be
    /// closed; that returns an error.
    pub fn close_tab(&mut self, index: usize) -> Result<Listing, String> {
        if self.tabs.len() <= 1 {
            return Err("The last tab cannot be closed".into());
        }
        if index >= self.tabs.len() {
            return Err("There is no such tab".into());
        }
        self.tabs.remove(index);
        if index < self.active || self.active >= self.tabs.len() {
            self.active = self.active.saturating_sub(1);
        }
        self.reload()
    }

    /// Opens a location typed by the user: a path, `~/...`, or a `file://` URI.
    /// Other URI schemes are not supported yet.
    pub fn open_location(&mut self, text: &str) -> Result<Listing, String> {
        match Location::parse(text, &self.home).map_err(|err| err.to_string())? {
            Location::Local(path) => self.navigate(&path),
            Location::Remote(uri) => self.navigate(Path::new(&uri)),
        }
    }

    /// Opens the parent of the current folder.
    pub fn go_up(&mut self) -> Result<Listing, String> {
        let Some(parent) = self.tabs[self.active]
            .current
            .as_deref()
            .and_then(parent_location)
        else {
            return Err("This folder has no parent".into());
        };
        self.navigate(&parent)
    }

    /// Reads the current folder again, e.g. after a change on disk.
    pub fn reload(&mut self) -> Result<Listing, String> {
        let path = self.tabs[self.active]
            .current
            .clone()
            .ok_or("No folder is open")?;
        self.list(&path)
    }

    /// Sorts by `key`. Choosing the key that is already active reverses the order.
    pub fn sort_by(&mut self, key: SortKey) {
        if self.config.sort.key == key {
            self.config.sort.descending = !self.config.sort.descending;
        } else {
            self.config.sort = SortOrder {
                key,
                descending: false,
            };
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.config.show_hidden = !self.config.show_hidden;
    }

    /// Lists `path` with the current hidden-file and sort settings.
    pub fn list(&self, path: &Path) -> Result<Listing, String> {
        let listed = if is_uri(path) {
            list_uri(&path.to_string_lossy()).map_err(|err| err.to_string())
        } else {
            list_directory(path).map_err(|err| format!("Cannot read {}: {err}", path.display()))
        };
        let mut entries = listed?;
        if !self.config.show_hidden {
            entries.retain(|entry| !entry.name.starts_with('.'));
        }
        sort_entries(&mut entries, self.config.sort);
        Ok(Listing {
            path: path.to_path_buf(),
            entries,
        })
    }

    /// Creates a folder in the current folder.
    pub fn create_folder(&self, name: &str) -> Result<(), String> {
        let parent = self.require_current()?;
        ops::create_directory(parent, name)
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    /// Renames `path` within its folder.
    pub fn rename(&self, path: &Path, new_name: &str) -> Result<(), String> {
        ops::rename(path, new_name)
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    /// Moves each path to the trash. Returns one message per failure.
    pub fn trash(&self, paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .filter_map(|path| ops::move_to_trash(path).err().map(|err| err.to_string()))
            .collect()
    }

    /// Puts `paths` on the clipboard for a later paste. A cut replaces a copy and
    /// vice versa.
    pub fn set_clipboard(&mut self, paths: Vec<PathBuf>, cut: bool) {
        self.clipboard = paths;
        self.clipboard_is_cut = cut;
    }

    /// Pastes the clipboard into the current folder. A copy keeps the clipboard.
    /// A cut keeps only the items that failed, so they can be retried.
    /// Returns one message per failure.
    pub fn paste(&mut self) -> Vec<String> {
        let cut = self.clipboard_is_cut;
        let paths = self.clipboard.clone();
        let (remaining, errors) = self.paste_items(&paths, cut);
        if cut {
            self.clipboard = remaining;
        }
        errors
    }

    /// Copies or moves `paths` into the current folder. Returns the paths that
    /// failed, so a cut can keep them, and one message per failure.
    /// Items are never overwritten.
    pub fn paste_items(&self, paths: &[PathBuf], cut: bool) -> (Vec<PathBuf>, Vec<String>) {
        let Ok(dest) = self.require_current().map(Path::to_path_buf) else {
            return (paths.to_vec(), vec!["No folder is open".into()]);
        };
        self.transfer_items(paths, &dest, cut)
    }

    /// Copies or moves `paths` into the folder `dest`, which may be a local path
    /// or a URI. Returns the paths that failed and one message per failure. Items
    /// are never overwritten.
    pub fn transfer_items(
        &self,
        paths: &[PathBuf],
        dest: &Path,
        cut: bool,
    ) -> (Vec<PathBuf>, Vec<String>) {
        let mut errors = Vec::new();
        let mut failed = Vec::new();
        for path in paths {
            let result = if cut {
                ops::move_into(path, dest)
            } else {
                ops::copy(path, dest)
            };
            if let Err(err) = result {
                errors.push(err.to_string());
                failed.push(path.clone());
            }
        }
        (failed, errors)
    }

    /// Sidebar entries: the standard user folders that exist, then the file system root.
    pub fn places(&self) -> Vec<Place> {
        let mut places = vec![Place {
            label: "Home".into(),
            path: self.home.clone(),
        }];

        let user_dirs = user_dirs(&self.home);
        for (label, key) in [
            ("Desktop", "XDG_DESKTOP_DIR"),
            ("Documents", "XDG_DOCUMENTS_DIR"),
            ("Downloads", "XDG_DOWNLOAD_DIR"),
            ("Music", "XDG_MUSIC_DIR"),
            ("Pictures", "XDG_PICTURES_DIR"),
            ("Videos", "XDG_VIDEOS_DIR"),
        ] {
            let path = user_dirs
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, path)| path.clone())
                .unwrap_or_else(|| self.home.join(label));
            if path.is_dir() {
                places.push(Place {
                    label: label.into(),
                    path,
                });
            }
        }

        // Virtual locations provided by GVFS.
        places.push(Place {
            label: "Trash".into(),
            path: PathBuf::from("trash:///"),
        });
        places.push(Place {
            label: "Recent".into(),
            path: PathBuf::from("recent:///"),
        });

        places.push(Place {
            label: "File System".into(),
            path: PathBuf::from("/"),
        });
        places
    }

    fn require_current(&self) -> Result<&Path, String> {
        self.tabs[self.active]
            .current
            .as_deref()
            .ok_or_else(|| "No folder is open".into())
    }
}

/// Removes trailing separators and `.` components so equal folders compare equal.
/// URIs are kept as they are, since their `//` is not a path separator.
fn clean(path: &Path) -> PathBuf {
    if is_uri(path) {
        return path.to_path_buf();
    }
    path.components().collect()
}

/// Reads `user-dirs.dirs` into `(KEY, path)` pairs, expanding `$HOME`.
fn user_dirs(home: &Path) -> Vec<(String, PathBuf)> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    let Ok(text) = fs::read_to_string(config_home.join("user-dirs.dirs")) else {
        return Vec::new();
    };
    parse_user_dirs(&text, home)
}

/// Parses lines of the form `XDG_NAME_DIR="$HOME/path"`.
fn parse_user_dirs(text: &str, home: &Path) -> Vec<(String, PathBuf)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"');
            let path = match value.strip_prefix("$HOME") {
                Some(rest) => home.join(rest.trim_start_matches('/')),
                None => PathBuf::from(value),
            };
            Some((key.trim().to_string(), path))
        })
        .collect()
}

/// Serializes a listing as a JSON array for the QML front end.
///
/// Each element has `name`, `path`, `kind` (`directory`, `file`, `symlink` or
/// `other`), `size` in bytes, and `modified` as seconds since the Unix epoch
/// (or `null` when unknown).
pub fn listing_json(listing: &Listing) -> String {
    let items: Vec<serde_json::Value> = listing
        .entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "name": entry.name,
                "path": entry.path.to_string_lossy(),
                "kind": kind_name(entry.kind),
                "size": entry.size,
                "modified": entry.modified.and_then(|time| {
                    time.duration_since(UNIX_EPOCH).ok().map(|after| after.as_secs())
                }),
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

/// Serializes the sidebar as a JSON array: the places, then a `header` item and
/// the bookmarks. Each item has `kind` (`place`, `header` or `bookmark`) and
/// `label`. Places and bookmarks have `path`, and bookmarks also have `index`
/// (their position in the bookmark list, for rename, move and remove).
pub fn sidebar_json(places: &[Place], devices: &[Device], bookmarks: &[Bookmark]) -> String {
    let mut items: Vec<serde_json::Value> = places
        .iter()
        .map(|place| {
            serde_json::json!({
                "kind": "place",
                "label": place.label,
                "path": place.path.to_string_lossy(),
            })
        })
        .collect();
    if !devices.is_empty() {
        items.push(serde_json::json!({ "kind": "header", "label": "Devices" }));
    }
    for device in devices {
        items.push(serde_json::json!({
            "kind": "device",
            "key": device.key,
            "label": device.name,
            "path": device.root.clone().unwrap_or_default(),
            "mounted": device.mounted,
            "canUnmount": device.can_unmount,
            "deviceKind": device_kind_name(device.kind),
            "free": device.free_bytes,
            "total": device.total_bytes,
        }));
    }
    if !bookmarks.is_empty() {
        items.push(serde_json::json!({ "kind": "header", "label": "Bookmarks" }));
    }
    for (index, bookmark) in bookmarks.iter().enumerate() {
        if bookmark.is_separator() {
            items.push(serde_json::json!({ "kind": "separator", "index": index }));
            continue;
        }
        items.push(serde_json::json!({
            "kind": "bookmark",
            "index": index,
            "label": bookmark.display_name(),
            "path": bookmark.path.to_string_lossy(),
            "available": bookmark_available(bookmark),
        }));
    }
    serde_json::Value::Array(items).to_string()
}

/// Reads the Properties details for `path` and serializes them as a JSON object.
///
/// Keys: `name`, `path`, `kind`, `size`, `modified`, `accessed`, `created`
/// (seconds since the Unix epoch, or `null`), `permissions` (as `rwxr-x---`),
/// `owner`, `group`, `linkTarget` (or `null`), `mimeType`, and
/// `defaultApplication` (or `null`).
pub fn inspect(path: &Path) -> Result<String, String> {
    let props = read_properties(path).map_err(|err| err.to_string())?;
    Ok(properties_json(&props))
}

/// Serializes already-read properties as a JSON object. See [`inspect`].
pub fn properties_json(props: &FileProperties) -> String {
    let secs = |time: Option<SystemTime>| {
        time.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|after| after.as_secs())
    };
    serde_json::json!({
        "name": props.name,
        "path": props.path.to_string_lossy(),
        "kind": kind_name(props.kind),
        "size": props.size,
        "modified": secs(props.modified),
        "accessed": secs(props.accessed),
        "created": secs(props.created),
        "permissions": props.permission_string(),
        "owner": props.owner,
        "group": props.group,
        "linkTarget": props.link_target.as_ref().map(|target| target.to_string_lossy().into_owned()),
        "mimeType": props.mime_type,
        "defaultApplication": props.default_application,
    })
    .to_string()
}

/// Parses a JSON array of path strings, such as a selection from the front end.
pub fn parse_paths(json: &str) -> Result<Vec<PathBuf>, String> {
    let values: Vec<String> =
        serde_json::from_str(json).map_err(|err| format!("Invalid selection: {err}"))?;
    Ok(values.into_iter().map(PathBuf::from).collect())
}

/// Whether a bookmarked folder is there. Remote locations are not checked, since
/// asking a server would hold up the sidebar, so they count as available.
fn bookmark_available(bookmark: &Bookmark) -> bool {
    is_uri(&bookmark.path) || bookmark.path.exists()
}

fn device_kind_name(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Drive => "drive",
        DeviceKind::Removable => "removable",
        DeviceKind::Network => "network",
    }
}

fn kind_name(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::File => "file",
        EntryKind::Directory => "directory",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(home: &Path) -> Session {
        Session::new(Config::default(), home.to_path_buf())
    }

    #[test]
    fn navigate_and_history() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        let mut s = session(dir.path());
        s.navigate(&a).unwrap();
        s.navigate(&b).unwrap();
        assert!(s.can_go_back());
        assert!(!s.can_go_forward());

        s.go_back().unwrap();
        assert_eq!(s.current(), Some(a.as_path()));
        assert!(s.can_go_forward());

        s.go_forward().unwrap();
        assert_eq!(s.current(), Some(b.as_path()));
    }

    #[test]
    fn tabs_have_their_own_folder_and_history() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        let mut s = session(dir.path());
        s.navigate(&a).unwrap();
        s.new_tab().unwrap();
        assert_eq!(s.tab_count(), 2);
        assert_eq!(s.active_tab(), 1);
        assert_eq!(
            s.current(),
            Some(a.as_path()),
            "a new tab starts in the same folder"
        );
        assert!(!s.can_go_back(), "a new tab has no history");

        s.navigate(&b).unwrap();
        assert_eq!(s.tab_titles(), vec!["a".to_string(), "b".to_string()]);

        s.switch_tab(0).unwrap();
        assert_eq!(s.current(), Some(a.as_path()));
        s.switch_tab(1).unwrap();
        assert!(s.can_go_back());
    }

    #[test]
    fn closing_tabs_keeps_a_sensible_active_tab() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        assert!(s.close_tab(0).is_err(), "the last tab stays open");

        s.new_tab().unwrap();
        s.new_tab().unwrap();
        assert_eq!(s.tab_count(), 3);
        assert_eq!(s.active_tab(), 2);

        s.close_tab(2).unwrap();
        assert_eq!(
            s.active_tab(),
            1,
            "closing the active last tab shows the one before"
        );
        s.close_tab(0).unwrap();
        assert_eq!(
            s.active_tab(),
            0,
            "closing a tab before the active one shifts it left"
        );
        assert_eq!(s.tab_count(), 1);
        assert!(s.switch_tab(5).is_err());
    }

    #[test]
    fn paste_items_copies_into_the_current_folder_and_reports_failures() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.txt");
        fs::write(&src, b"data").unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("taken.txt"), b"old").unwrap();
        let taken = dir.path().join("taken.txt");
        fs::write(&taken, b"new").unwrap();

        let mut s = session(dir.path());
        s.navigate(&dest).unwrap();
        let (failed, errors) = s.paste_items(&[src.clone(), taken.clone()], false);
        assert!(dest.join("src.txt").exists());
        assert_eq!(failed, vec![taken]);
        assert_eq!(errors.len(), 1);
        assert!(src.exists(), "a copy leaves the source in place");
    }

    #[test]
    fn transfer_items_copies_and_moves_into_any_folder() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let file = a.join("note.txt");
        fs::write(&file, b"x").unwrap();

        let s = session(dir.path());
        let (failed, errors) = s.transfer_items(std::slice::from_ref(&file), &b, false);
        assert!(failed.is_empty() && errors.is_empty());
        assert!(
            b.join("note.txt").exists() && file.exists(),
            "copy keeps the source"
        );

        let (failed, _) = s.transfer_items(std::slice::from_ref(&file), &b, true);
        assert_eq!(
            failed,
            vec![file.clone()],
            "the copy already exists, so the move fails"
        );
        fs::remove_file(b.join("note.txt")).unwrap();
        let (failed, _) = s.transfer_items(std::slice::from_ref(&file), &b, true);
        assert!(failed.is_empty());
        assert!(
            !file.exists() && b.join("note.txt").exists(),
            "move removes the source"
        );
    }

    #[test]
    fn separators_and_reordering() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let mut s = session(dir.path());
        s.navigate(&a).unwrap();
        s.add_bookmark();
        s.navigate(&b).unwrap();
        s.add_bookmark();

        s.insert_separator(1);
        assert!(s.bookmarks()[1].is_separator());
        assert_eq!(s.bookmarks().len(), 3);

        // Move the last bookmark to the front.
        s.reorder_bookmark(2, 0);
        assert_eq!(s.bookmarks()[0].path, b);
        assert!(s.bookmarks()[2].is_separator() || s.bookmarks()[1].is_separator());
        let value: serde_json::Value =
            serde_json::from_str(&sidebar_json(&s.places(), &[], s.bookmarks())).unwrap();
        assert!(
            value
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["kind"] == "separator")
        );
    }

    #[test]
    fn missing_local_bookmarks_are_marked_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone");
        let list = vec![
            Bookmark::new(dir.path().to_path_buf()),
            Bookmark::new(gone),
            Bookmark::new(PathBuf::from("smb://server/share")),
        ];
        let value: serde_json::Value =
            serde_json::from_str(&sidebar_json(&[], &[], &list)).unwrap();
        // The first sidebar item is the "Bookmarks" heading, so pick out the bookmarks.
        let items: Vec<&serde_json::Value> = value
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["kind"] == "bookmark")
            .collect();
        assert_eq!(items[0]["available"], true);
        assert_eq!(items[1]["available"], false);
        assert_eq!(
            items[2]["available"], true,
            "remote locations are not checked"
        );
    }

    #[test]
    fn imports_add_only_new_folders() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        s.add_bookmark();
        let incoming = vec![
            Bookmark::new(dir.path().to_path_buf()),
            Bookmark::new(PathBuf::from("/elsewhere")),
        ];
        assert_eq!(s.import_bookmarks(incoming), 1);
        assert_eq!(s.bookmarks().len(), 2);
    }

    #[test]
    fn only_folders_can_be_bookmarked() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plain.txt");
        fs::write(&file, b"x").unwrap();
        let mut s = session(dir.path());
        assert!(!s.add_bookmark_path(file));
        assert!(!s.add_bookmark_path(dir.path().join("missing")));
        assert!(s.add_bookmark_path(dir.path().to_path_buf()));
    }

    #[test]
    fn bookmarks_can_be_added_for_any_folder() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        let mut s = session(dir.path());
        assert!(s.add_bookmark_path(other.clone()));
        assert!(!s.add_bookmark_path(other.clone()));
        assert_eq!(s.bookmarks()[0].path, other);
    }

    #[test]
    fn restore_tabs_skips_missing_folders_and_keeps_the_active_one() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let gone = dir.path().join("gone");

        let mut s = session(dir.path());
        s.restore_tabs(&[a.clone(), gone.clone(), b.clone()], 2)
            .unwrap();
        assert_eq!(s.tab_paths(), vec![a.clone(), b.clone()]);
        assert_eq!(
            s.current(),
            Some(b.as_path()),
            "the saved active tab is shown"
        );

        // When the active tab's folder is gone, the one before it is shown.
        s.restore_tabs(&[a.clone(), gone.clone()], 1).unwrap();
        assert_eq!(s.current(), Some(a.as_path()));

        assert!(s.restore_tabs(&[gone], 0).is_err());
        assert_eq!(
            s.tab_paths(),
            vec![a],
            "a failed restore leaves the tabs alone"
        );
    }

    #[test]
    fn open_in_new_tab_does_not_add_a_tab_for_a_missing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        assert!(s.open_in_new_tab(&dir.path().join("missing")).is_err());
        assert_eq!(s.tab_count(), 1);
    }

    #[test]
    fn navigating_to_new_folder_clears_forward_history() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        let c = dir.path().join("c");
        for p in [&a, &b, &c] {
            fs::create_dir_all(p).unwrap();
        }

        let mut s = session(dir.path());
        s.navigate(&a).unwrap();
        s.navigate(&b).unwrap();
        s.go_back().unwrap();
        s.navigate(&c).unwrap();
        assert!(!s.can_go_forward());
    }

    #[test]
    fn failed_navigation_keeps_position() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        assert!(s.navigate(&dir.path().join("missing")).is_err());
        assert_eq!(s.current(), Some(dir.path()));
        assert!(!s.can_go_back());
    }

    #[test]
    fn file_operations_accept_uris() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("a.txt");
        fs::write(&original, b"x").unwrap();
        let uri = PathBuf::from(format!(
            "file://{}",
            nexusfm_core::location::encode(&original.to_string_lossy())
        ));
        let s = session(dir.path());
        s.rename(&uri, "b.txt").unwrap();
        assert!(dir.path().join("b.txt").exists());
        assert!(!original.exists());
    }

    #[test]
    fn open_location_accepts_paths_and_uris() {
        let dir = tempfile::tempdir().unwrap();
        let docs = dir.path().join("My Docs");
        fs::create_dir(&docs).unwrap();
        let mut s = session(dir.path());

        s.open_location(&format!(
            "file://{}",
            nexusfm_core::location::encode(&docs.to_string_lossy())
        ))
        .unwrap();
        assert_eq!(s.current(), Some(docs.as_path()));

        assert!(s.open_location("smb://server/share").is_err());
        assert!(s.open_location("not a path").is_err());
    }

    #[test]
    fn go_up_stops_at_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(Path::new("/")).unwrap();
        assert!(s.go_up().is_err());
    }

    #[test]
    fn sorting_the_same_key_reverses_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.sort_by(SortKey::Size);
        assert_eq!(
            s.sort_order(),
            SortOrder {
                key: SortKey::Size,
                descending: false
            }
        );
        s.sort_by(SortKey::Size);
        assert!(s.sort_order().descending);
        s.sort_by(SortKey::Name);
        assert_eq!(
            s.sort_order(),
            SortOrder {
                key: SortKey::Name,
                descending: false
            }
        );
    }

    #[test]
    fn hidden_files_are_filtered_unless_shown() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".hidden"), b"").unwrap();
        fs::write(dir.path().join("visible"), b"").unwrap();

        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        let names: Vec<_> = s
            .reload()
            .unwrap()
            .entries
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["visible"]);

        s.toggle_hidden();
        assert_eq!(s.reload().unwrap().entries.len(), 2);
    }

    #[test]
    fn copy_paste_keeps_clipboard_and_cut_clears_it() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dest).unwrap();
        fs::write(src.join("f.txt"), b"x").unwrap();

        let mut s = session(dir.path());
        s.navigate(&dest).unwrap();
        s.set_clipboard(vec![src.join("f.txt")], false);
        assert!(s.paste().is_empty());
        assert!(dest.join("f.txt").exists());
        assert!(s.can_paste(), "copy keeps the clipboard");

        s.set_clipboard(vec![src.join("f.txt")], true);
        fs::remove_file(dest.join("f.txt")).unwrap();
        assert!(s.paste().is_empty());
        assert!(!src.join("f.txt").exists());
        assert!(!s.can_paste(), "cut empties the clipboard after success");
    }

    #[test]
    fn failed_cut_stays_on_clipboard() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f.txt");
        fs::write(&src, b"x").unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("f.txt"), b"existing").unwrap();

        let mut s = session(dir.path());
        s.navigate(&dest).unwrap();
        s.set_clipboard(vec![src.clone()], true);
        assert_eq!(s.paste().len(), 1);
        assert!(s.can_paste());
    }

    #[test]
    fn listing_json_has_expected_fields() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let mut s = session(dir.path());
        let listing = s.navigate(dir.path()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&listing_json(&listing)).unwrap();
        let items = value.as_array().unwrap();
        assert_eq!(items[0]["name"], "sub");
        assert_eq!(items[0]["kind"], "directory");
        assert_eq!(items[1]["name"], "a.txt");
        assert_eq!(items[1]["size"], 5);
        assert!(items[1]["modified"].is_u64());
    }

    #[test]
    fn parses_user_dirs_file() {
        let home = Path::new("/home/u");
        let text = "# comment\nXDG_DESKTOP_DIR=\"$HOME/Desk\"\nXDG_MUSIC_DIR=\"/data/music\"\n";
        let parsed = parse_user_dirs(text, home);
        assert!(parsed.contains(&("XDG_DESKTOP_DIR".into(), PathBuf::from("/home/u/Desk"))));
        assert!(parsed.contains(&("XDG_MUSIC_DIR".into(), PathBuf::from("/data/music"))));
    }

    #[test]
    fn view_mode_can_be_changed() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        assert_eq!(s.view_mode(), ViewMode::Detailed);
        s.set_view_mode(ViewMode::Compact);
        assert_eq!(s.view_mode(), ViewMode::Compact);
    }

    #[test]
    fn inspect_describes_a_file_as_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, b"hello").unwrap();

        let value: serde_json::Value = serde_json::from_str(&inspect(&file).unwrap()).unwrap();
        assert_eq!(value["name"], "a.txt");
        assert_eq!(value["kind"], "file");
        assert_eq!(value["size"], 5);
        assert_eq!(value["permissions"].as_str().unwrap().len(), 9);
        assert!(value["linkTarget"].is_null());
        assert!(inspect(&dir.path().join("missing")).is_err());
    }

    #[test]
    fn parses_selection_json() {
        let paths = parse_paths(r#"["/a/b", "/c"]"#).unwrap();
        assert_eq!(paths, vec![PathBuf::from("/a/b"), PathBuf::from("/c")]);
        assert!(parse_paths("not json").is_err());
    }

    #[test]
    fn bookmarks_can_be_added_renamed_moved_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        let mut s = session(dir.path());
        assert!(!s.add_bookmark(), "no folder is open yet");
        s.navigate(&a).unwrap();
        assert!(s.add_bookmark());
        assert!(!s.add_bookmark(), "a folder is only bookmarked once");
        s.navigate(&b).unwrap();
        assert!(s.add_bookmark());

        s.rename_bookmark(0, "Alpha");
        assert_eq!(s.bookmarks()[0].display_name(), "Alpha");
        s.move_bookmark(1, -1);
        assert_eq!(s.bookmarks()[0].path, b);
        s.remove_bookmark(0);
        assert_eq!(s.bookmarks().len(), 1);
        assert_eq!(s.bookmarks()[0].path, a);
    }

    #[test]
    fn sidebar_lists_places_then_bookmarks() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        s.navigate(dir.path()).unwrap();
        s.add_bookmark();
        let value: serde_json::Value =
            serde_json::from_str(&sidebar_json(&s.places(), &[], s.bookmarks())).unwrap();
        let items = value.as_array().unwrap();
        let header = items
            .iter()
            .position(|item| item["kind"] == "header")
            .unwrap();
        assert_eq!(items[header + 1]["kind"], "bookmark");
        assert_eq!(items[header + 1]["index"], 0);
        assert_eq!(items[0]["kind"], "place");
    }

    #[test]
    fn places_include_existing_home_and_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Downloads")).unwrap();
        let s = session(dir.path());
        let labels: Vec<_> = s.places().into_iter().map(|p| p.label).collect();
        assert!(labels.iter().any(|label| label == "Trash"));
        assert!(labels.iter().any(|label| label == "Recent"));
        assert_eq!(labels.first().map(String::as_str), Some("Home"));
        assert!(labels.iter().any(|label| label == "Downloads"));
        assert!(!labels.iter().any(|label| label == "Music"));
        assert_eq!(labels.last().map(String::as_str), Some("File System"));
    }
}
