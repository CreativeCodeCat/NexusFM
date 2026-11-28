// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

use cxx_qt_build::{CxxQtBuilder, QmlModule};
use qt_build_utils::{QResource, QResourceFile, QResources};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("NexusFM")
            .qml_file("qml/Main.qml")
            .qml_file("qml/ErrorPanel.qml")
            .qml_file("qml/Pane.qml"),
    )
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .files(["src/backend.rs"])
    // The application icon, available to QML as qrc:/qt/qml/NexusFM/icon.png.
    .qrc_resources(QResources::new().resource(
        QResource::new().file(QResourceFile::new("../../assets/icon.png").alias("icon.png")),
    ))
    .build();
}
