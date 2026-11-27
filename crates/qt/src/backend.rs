// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! The QObject the QML interface talks to. It holds the window state in a
//! [`Session`] and exposes the results as properties.

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use nexusfm_config::{Config, ViewMode};
use nexusfm_core::SortKey;
use nexusfm_session::{Listing, Session, listing_json, parse_paths, sidebar_json};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// An alias to the QString type
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, current_path, cxx_name = "currentPath")]
        #[qproperty(QString, entries_json, cxx_name = "entriesJson")]
        #[qproperty(QString, places_json, cxx_name = "placesJson")]
        #[qproperty(QString, home_path, cxx_name = "homePath")]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        #[qproperty(QString, error_text, cxx_name = "errorText")]
        #[qproperty(bool, show_hidden, cxx_name = "showHidden")]
        #[qproperty(i32, sort_key, cxx_name = "sortKey")]
        #[qproperty(bool, descending, cxx_name = "descending")]
        #[qproperty(bool, can_go_back, cxx_name = "canGoBack")]
        #[qproperty(bool, can_go_forward, cxx_name = "canGoForward")]
        #[qproperty(bool, can_paste, cxx_name = "canPaste")]
        #[qproperty(i32, view_mode, cxx_name = "viewMode")]
        #[qproperty(QString, properties_json, cxx_name = "propertiesJson")]
        type Backend = super::BackendRust;

        /// Opens the initial folder: `path` if given, otherwise the home folder.
        #[qinvokable]
        #[cxx_name = "start"]
        fn start(self: Pin<&mut Backend>, path: &QString);

        /// Opens text typed in the location bar: a path, `~/...` or a `file://` URI.
        #[qinvokable]
        #[cxx_name = "openLocation"]
        fn open_location(self: Pin<&mut Backend>, text: &QString);

        #[qinvokable]
        #[cxx_name = "navigate"]
        fn navigate(self: Pin<&mut Backend>, path: &QString);

        #[qinvokable]
        #[cxx_name = "goBack"]
        fn go_back(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "goForward"]
        fn go_forward(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "goUp"]
        fn go_up(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "reload"]
        fn reload(self: Pin<&mut Backend>);

        /// Sorts by a key: 0 name, 1 size, 2 modified. The active key reverses the order.
        #[qinvokable]
        #[cxx_name = "sortBy"]
        fn sort_by(self: Pin<&mut Backend>, key: i32);

        /// Sets the layout: 0 detailed, 1 compact, 2 icon.
        #[qinvokable]
        #[cxx_name = "chooseViewMode"]
        fn choose_view_mode(self: Pin<&mut Backend>, mode: i32);

        /// Reads the Properties details for `path` into `propertiesJson`.
        #[qinvokable]
        #[cxx_name = "inspect"]
        fn inspect(self: Pin<&mut Backend>, path: &QString);

        /// Bookmarks the current folder.
        #[qinvokable]
        #[cxx_name = "addBookmark"]
        fn add_bookmark(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "removeBookmark"]
        fn remove_bookmark(self: Pin<&mut Backend>, index: i32);

        #[qinvokable]
        #[cxx_name = "renameBookmark"]
        fn rename_bookmark(self: Pin<&mut Backend>, index: i32, name: &QString);

        /// Moves a bookmark by `delta` places; negative moves it up.
        #[qinvokable]
        #[cxx_name = "moveBookmark"]
        fn move_bookmark(self: Pin<&mut Backend>, index: i32, delta: i32);

        /// Reloads the folder if it changed on disk since the last check.
        #[qinvokable]
        #[cxx_name = "checkForChanges"]
        fn check_for_changes(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "toggleHiddenFiles"]
        fn toggle_hidden_files(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "createFolder"]
        fn create_folder(self: Pin<&mut Backend>, name: &QString);

        #[qinvokable]
        #[cxx_name = "renameItem"]
        fn rename_item(self: Pin<&mut Backend>, path: &QString, name: &QString);

        /// Each argument is a JSON array of paths.
        #[qinvokable]
        #[cxx_name = "moveToTrash"]
        fn move_to_trash(self: Pin<&mut Backend>, paths: &QString);

        #[qinvokable]
        #[cxx_name = "cut"]
        fn cut(self: Pin<&mut Backend>, paths: &QString);

        #[qinvokable]
        #[cxx_name = "copy"]
        fn copy(self: Pin<&mut Backend>, paths: &QString);

        #[qinvokable]
        #[cxx_name = "paste"]
        fn paste(self: Pin<&mut Backend>);

        #[qinvokable]
        #[cxx_name = "clearError"]
        fn clear_error(self: Pin<&mut Backend>);
    }
}

/// The Rust state behind the `Backend` QObject.
pub struct BackendRust {
    current_path: QString,
    entries_json: QString,
    places_json: QString,
    home_path: QString,
    status_text: QString,
    error_text: QString,
    show_hidden: bool,
    sort_key: i32,
    descending: bool,
    can_go_back: bool,
    can_go_forward: bool,
    can_paste: bool,
    view_mode: i32,
    properties_json: QString,
    session: Session,
    watcher: Option<nexusfm_filesystem::DirectoryWatcher>,
}

impl Default for BackendRust {
    fn default() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        let config = load_config();
        let session = Session::new(config, home.clone());
        let order = session.sort_order();

        Self {
            current_path: QString::default(),
            entries_json: QString::from("[]"),
            places_json: QString::from("[]"),
            home_path: QString::from(home.to_string_lossy().as_ref()),
            status_text: QString::default(),
            error_text: QString::default(),
            show_hidden: session.show_hidden(),
            sort_key: sort_key_to_i32(order.key),
            descending: order.descending,
            can_go_back: false,
            can_go_forward: false,
            can_paste: false,
            view_mode: view_mode_to_i32(session.view_mode()),
            properties_json: QString::from("{}"),
            session,
            watcher: None,
        }
    }
}

/// Loads the user's settings. A broken file is reported and the defaults are used.
fn load_config() -> Config {
    match Config::load_user(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    ) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("nexusfm: {err}; using default configuration");
            Config::default()
        }
    }
}

fn sort_key_to_i32(key: SortKey) -> i32 {
    match key {
        SortKey::Name => 0,
        SortKey::Size => 1,
        SortKey::Modified => 2,
    }
}

fn sort_key_from_i32(value: i32) -> SortKey {
    match value {
        1 => SortKey::Size,
        2 => SortKey::Modified,
        _ => SortKey::Name,
    }
}

fn view_mode_to_i32(mode: ViewMode) -> i32 {
    match mode {
        ViewMode::Detailed => 0,
        ViewMode::Compact => 1,
        ViewMode::Icon => 2,
    }
}

fn view_mode_from_i32(value: i32) -> ViewMode {
    match value {
        1 => ViewMode::Compact,
        2 => ViewMode::Icon,
        _ => ViewMode::Detailed,
    }
}

/// Where the bookmark list is stored, from the environment.
fn bookmarks_path() -> Option<PathBuf> {
    nexusfm_bookmarks::bookmarks_file(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

fn to_qstring(text: &str) -> QString {
    QString::from(text)
}

impl qobject::Backend {
    /// Runs `f` with the session. Results are returned as owned values, so the
    /// property setters can be called afterwards.
    fn with_session<R>(self: Pin<&mut Self>, f: impl FnOnce(&mut Session) -> R) -> R {
        f(&mut self.rust_mut().get_mut().session)
    }

    /// Copies the session's navigation and sort state into the QML-visible properties.
    fn sync_state(mut self: Pin<&mut Self>) {
        let (back, forward, paste, hidden, order) = self.as_mut().with_session(|s| {
            (
                s.can_go_back(),
                s.can_go_forward(),
                s.can_paste(),
                s.show_hidden(),
                s.sort_order(),
            )
        });
        self.as_mut().set_can_go_back(back);
        self.as_mut().set_can_go_forward(forward);
        self.as_mut().set_can_paste(paste);
        self.as_mut().set_show_hidden(hidden);
        self.as_mut().set_sort_key(sort_key_to_i32(order.key));
        self.as_mut().set_descending(order.descending);
        let view = self.as_mut().with_session(|s| s.view_mode());
        self.as_mut().set_view_mode(view_mode_to_i32(view));
    }

    /// Shows a folder's contents, or the error if it could not be read.
    fn show_listing(mut self: Pin<&mut Self>, result: Result<Listing, String>) {
        match result {
            Ok(listing) => {
                // Watch the folder now on screen so outside changes appear.
                let watcher = nexusfm_filesystem::DirectoryWatcher::new(&listing.path).ok();
                self.as_mut().rust_mut().get_mut().watcher = watcher;
                let count = listing.entries.len();
                let path = listing.path.to_string_lossy().into_owned();
                self.as_mut().set_current_path(to_qstring(&path));
                self.as_mut()
                    .set_entries_json(to_qstring(&listing_json(&listing)));
                let status = if count == 1 {
                    "1 item".to_string()
                } else {
                    format!("{count} items")
                };
                self.as_mut().set_status_text(to_qstring(&status));
            }
            Err(message) => self.as_mut().set_error_text(to_qstring(&message)),
        }
        self.sync_state();
    }

    /// Rebuilds the sidebar (places and bookmarks) for QML.
    fn refresh_sidebar(mut self: Pin<&mut Self>) {
        let json = self
            .as_mut()
            .with_session(|s| sidebar_json(&s.places(), s.bookmarks()));
        self.as_mut().set_places_json(to_qstring(&json));
    }

    /// Saves the bookmark list and refreshes the sidebar.
    fn persist_bookmarks(mut self: Pin<&mut Self>) {
        if let Some(path) = bookmarks_path() {
            let list = self.as_mut().with_session(|s| s.bookmarks().to_vec());
            if let Err(err) = nexusfm_bookmarks::save(&path, &list) {
                self.as_mut()
                    .set_error_text(to_qstring(&format!("cannot save bookmarks: {err}")));
            }
        }
        self.refresh_sidebar();
    }

    pub fn add_bookmark(mut self: Pin<&mut Self>) {
        if self.as_mut().with_session(|s| s.add_bookmark()) {
            self.persist_bookmarks();
        }
    }

    pub fn remove_bookmark(mut self: Pin<&mut Self>, index: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        self.as_mut().with_session(|s| s.remove_bookmark(index));
        self.persist_bookmarks();
    }

    pub fn rename_bookmark(mut self: Pin<&mut Self>, index: i32, name: &QString) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let name = String::from(name);
        self.as_mut()
            .with_session(|s| s.rename_bookmark(index, &name));
        self.persist_bookmarks();
    }

    pub fn move_bookmark(mut self: Pin<&mut Self>, index: i32, delta: i32) {
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        self.as_mut()
            .with_session(|s| s.move_bookmark(index, delta as isize));
        self.persist_bookmarks();
    }

    /// Saves the current settings to `config.lua`. A failure is shown as an error.
    fn persist_settings(mut self: Pin<&mut Self>) {
        let config = self.as_mut().with_session(|s| s.settings().clone());
        let xdg = std::env::var_os("XDG_CONFIG_HOME");
        let home = std::env::var_os("HOME");
        if let Err(err) = config.save_user(xdg.as_deref(), home.as_deref()) {
            self.as_mut().set_error_text(to_qstring(&err.to_string()));
        }
    }

    /// Shows the first error from a batch of operations, if any.
    fn show_errors(mut self: Pin<&mut Self>, errors: &[String]) {
        if !errors.is_empty() {
            self.as_mut().set_error_text(to_qstring(&errors.join("\n")));
        }
    }

    /// Reads the folder again after a change on disk.
    fn refresh(mut self: Pin<&mut Self>) {
        let result = self.as_mut().with_session(|s| s.reload());
        self.show_listing(result);
    }

    /// Paths selected in the interface, from the JSON argument. A bad argument
    /// shows an error and returns `None`.
    fn selection(mut self: Pin<&mut Self>, json: &QString) -> Option<Vec<PathBuf>> {
        match parse_paths(&String::from(json)) {
            Ok(paths) => Some(paths),
            Err(message) => {
                self.as_mut().set_error_text(to_qstring(&message));
                None
            }
        }
    }

    pub fn start(mut self: Pin<&mut Self>, path: &QString) {
        let text = String::from(path);
        let target = if text.trim().is_empty() {
            self.as_mut().with_session(|s| s.home().to_path_buf())
        } else {
            PathBuf::from(text)
        };

        let bookmarks =
            bookmarks_path().map(|path| nexusfm_bookmarks::load(&path).unwrap_or_default());
        self.as_mut()
            .with_session(|s| s.set_bookmarks(bookmarks.unwrap_or_default()));
        self.as_mut().refresh_sidebar();

        let result = self.as_mut().with_session(|s| s.navigate(&target));
        self.show_listing(result);
    }

    pub fn open_location(mut self: Pin<&mut Self>, text: &QString) {
        let text = String::from(text);
        let result = self.as_mut().with_session(|s| s.open_location(&text));
        self.show_listing(result);
    }

    pub fn navigate(mut self: Pin<&mut Self>, path: &QString) {
        let target = PathBuf::from(String::from(path));
        let result = self.as_mut().with_session(|s| s.navigate(&target));
        self.show_listing(result);
    }

    pub fn go_back(mut self: Pin<&mut Self>) {
        let result = self.as_mut().with_session(|s| s.go_back());
        self.show_listing(result);
    }

    pub fn go_forward(mut self: Pin<&mut Self>) {
        let result = self.as_mut().with_session(|s| s.go_forward());
        self.show_listing(result);
    }

    pub fn go_up(mut self: Pin<&mut Self>) {
        let result = self.as_mut().with_session(|s| s.go_up());
        self.show_listing(result);
    }

    pub fn reload(self: Pin<&mut Self>) {
        self.refresh();
    }

    pub fn sort_by(mut self: Pin<&mut Self>, key: i32) {
        self.as_mut()
            .with_session(|s| s.sort_by(sort_key_from_i32(key)));
        self.as_mut().persist_settings();
        self.refresh();
    }

    pub fn choose_view_mode(mut self: Pin<&mut Self>, mode: i32) {
        self.as_mut()
            .with_session(|s| s.set_view_mode(view_mode_from_i32(mode)));
        self.as_mut().persist_settings();
        self.sync_state();
    }

    pub fn inspect(mut self: Pin<&mut Self>, path: &QString) {
        let path = PathBuf::from(String::from(path));
        match nexusfm_session::inspect(&path) {
            Ok(json) => self.as_mut().set_properties_json(to_qstring(&json)),
            Err(message) => self.as_mut().set_error_text(to_qstring(&message)),
        }
    }

    pub fn check_for_changes(mut self: Pin<&mut Self>) {
        let changed = self
            .as_mut()
            .rust_mut()
            .get_mut()
            .watcher
            .as_ref()
            .is_some_and(|watcher| watcher.take_changes());
        if changed {
            self.refresh();
        }
    }

    pub fn toggle_hidden_files(mut self: Pin<&mut Self>) {
        self.as_mut().with_session(|s| s.toggle_hidden());
        self.as_mut().persist_settings();
        self.refresh();
    }

    pub fn create_folder(mut self: Pin<&mut Self>, name: &QString) {
        let name = String::from(name);
        let result = self.as_mut().with_session(|s| s.create_folder(&name));
        if let Err(message) = result {
            self.as_mut().set_error_text(to_qstring(&message));
        }
        self.refresh();
    }

    pub fn rename_item(mut self: Pin<&mut Self>, path: &QString, name: &QString) {
        let path = PathBuf::from(String::from(path));
        let name = String::from(name);
        let result = self.as_mut().with_session(|s| s.rename(&path, &name));
        if let Err(message) = result {
            self.as_mut().set_error_text(to_qstring(&message));
        }
        self.refresh();
    }

    pub fn move_to_trash(mut self: Pin<&mut Self>, paths: &QString) {
        let Some(paths) = self.as_mut().selection(paths) else {
            return;
        };
        let errors = self.as_mut().with_session(|s| s.trash(&paths));
        self.as_mut().show_errors(&errors);
        self.refresh();
    }

    pub fn cut(mut self: Pin<&mut Self>, paths: &QString) {
        let Some(paths) = self.as_mut().selection(paths) else {
            return;
        };
        self.as_mut().with_session(|s| s.set_clipboard(paths, true));
        self.sync_state();
    }

    pub fn copy(mut self: Pin<&mut Self>, paths: &QString) {
        let Some(paths) = self.as_mut().selection(paths) else {
            return;
        };
        self.as_mut()
            .with_session(|s| s.set_clipboard(paths, false));
        self.sync_state();
    }

    pub fn paste(mut self: Pin<&mut Self>) {
        let errors = self.as_mut().with_session(|s| s.paste());
        self.as_mut().show_errors(&errors);
        self.refresh();
    }

    pub fn clear_error(mut self: Pin<&mut Self>) {
        self.as_mut().set_error_text(QString::default());
    }
}
