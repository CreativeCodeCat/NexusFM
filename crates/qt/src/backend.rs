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
use nexusfm_config::{Config, config_file_path};
use nexusfm_core::SortKey;
use nexusfm_session::{Listing, Session, listing_json, parse_paths, places_json};

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
        type Backend = super::BackendRust;

        /// Opens the initial folder: `path` if given, otherwise the home folder.
        #[qinvokable]
        #[cxx_name = "start"]
        fn start(self: Pin<&mut Backend>, path: &QString);

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
    session: Session,
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
            session,
        }
    }
}

/// Loads the user's settings. A broken file is reported and the defaults are used.
fn load_config() -> Config {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    match config_file_path(xdg.as_deref(), home.as_deref()) {
        Some(path) => Config::load(&path).unwrap_or_default(),
        None => Config::default(),
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
    }

    /// Shows a folder's contents, or the error if it could not be read.
    fn show_listing(mut self: Pin<&mut Self>, result: Result<Listing, String>) {
        match result {
            Ok(listing) => {
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

        let places = self.as_mut().with_session(|s| places_json(&s.places()));
        self.as_mut().set_places_json(to_qstring(&places));

        let result = self.as_mut().with_session(|s| s.navigate(&target));
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
        self.refresh();
    }

    pub fn toggle_hidden_files(mut self: Pin<&mut Self>) {
        self.as_mut().with_session(|s| s.toggle_hidden());
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
