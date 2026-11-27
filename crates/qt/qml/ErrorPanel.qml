// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

import QtQuick
import QtQuick.Controls

// A dialog that shows an error message and closes on OK.
Dialog {
    id: panel

    property alias message: text.text

    title: qsTr("NexusFM")
    modal: true
    standardButtons: Dialog.Ok
    anchors.centerIn: Overlay.overlay

    Label {
        id: text
        width: 360
        wrapMode: Text.Wrap
    }
}
