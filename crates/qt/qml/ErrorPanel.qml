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

    // An explicit width keeps the dialog's size from depending on the wrapped text,
    // which would make the two bindings loop.
    width: 400
    title: qsTr("NexusFM")
    modal: true
    standardButtons: Dialog.Ok

    Label {
        id: text
        width: panel.availableWidth
        wrapMode: Text.Wrap
    }
}
