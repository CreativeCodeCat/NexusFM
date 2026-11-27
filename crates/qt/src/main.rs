// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! NexusFM: starts the Qt Quick window. The interface is in `qml/`, and the
//! state and file operations are in the Rust backend (`backend.rs`).

mod backend;

use std::path::PathBuf;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Wayland takes the window and taskbar icon from the installed desktop entry
    // (packaging/nexusfm.desktop). Only register the name when that entry exists:
    // otherwise the desktop portal refuses it and logs a warning.
    if desktop_entry_installed() {
        QGuiApplication::set_desktop_file_name(&QString::from(DESKTOP_ID));
    }

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/NexusFM/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

/// The desktop entry ID, matching `packaging/nexusfm.desktop`.
const DESKTOP_ID: &str = "nexusfm";

/// Whether `nexusfm.desktop` is installed in one of the XDG application folders.
fn desktop_entry_installed() -> bool {
    let file_name = format!("{DESKTOP_ID}.desktop");
    let mut dirs: Vec<PathBuf> = Vec::new();

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    dirs.extend(data_home);

    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(std::env::split_paths(&data_dirs));

    dirs.iter()
        .any(|dir| dir.join("applications").join(&file_name).is_file())
}
