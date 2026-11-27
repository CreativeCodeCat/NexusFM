// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! NexusFM: starts the Qt Quick window. The interface is in `qml/`, and the
//! state and file operations are in the Rust backend (`backend.rs`).

mod backend;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    // Wayland takes the window and taskbar icon from the desktop entry named
    // after this ID (packaging/nexusfm.desktop).
    QGuiApplication::set_desktop_file_name(&QString::from("nexusfm"));

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/NexusFM/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
