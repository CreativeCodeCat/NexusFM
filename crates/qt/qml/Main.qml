// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import NexusFM

ApplicationWindow {
    id: window

    width: 960
    height: 600
    minimumWidth: 480
    minimumHeight: 320
    visible: true
    title: qsTr("NexusFM")
    flags: Qt.Window | Qt.FramelessWindowHint
    color: palette.window

    // ---- Backend and view state ------------------------------------------

    Backend {
        id: backend
    }

    property var entries: []
    property var places: []
    // Selected row indexes. Reassigned, never mutated, so bindings update.
    property var selected: []
    property int anchorRow: -1
    readonly property int selectedCount: selected.length

    readonly property var columnWidths: ({ name: 0, size: 110, type: 90, modified: 170 })

    Component.onCompleted: {
        const args = Qt.application.arguments;
        backend.start(args.length > 1 ? args[1] : "");
    }

    Connections {
        target: backend

        function onEntriesJsonChanged() {
            window.entries = JSON.parse(backend.entriesJson);
            window.selected = [];
            window.anchorRow = -1;
        }

        function onPlacesJsonChanged() {
            window.places = JSON.parse(backend.placesJson);
        }

        function onCurrentPathChanged() {
            location.text = backend.currentPath;
        }

        function onErrorTextChanged() {
            if (backend.errorText.length > 0)
                errorPanel.open();
        }
    }

    // ---- Helpers ----------------------------------------------------------

    function selectedPaths() {
        return selected.map(function (row) { return entries[row].path; });
    }

    function selectedEntries() {
        return selected.map(function (row) { return entries[row]; });
    }

    function selectOnly(row) {
        selected = [row];
        anchorRow = row;
    }

    function toggleRow(row) {
        const index = selected.indexOf(row);
        selected = index >= 0
            ? selected.filter(function (r) { return r !== row; })
            : selected.concat([row]);
        anchorRow = row;
    }

    function selectRange(row) {
        const from = Math.min(anchorRow < 0 ? row : anchorRow, row);
        const to = Math.max(anchorRow < 0 ? row : anchorRow, row);
        const rows = [];
        for (let r = from; r <= to; r++)
            rows.push(r);
        selected = rows;
    }

    function formatSize(bytes) {
        if (bytes < 1024) return bytes + (bytes === 1 ? " byte" : " bytes");
        const units = ["KiB", "MiB", "GiB", "TiB"];
        let value = bytes / 1024;
        let unit = 0;
        while (value >= 1024 && unit < units.length - 1) {
            value /= 1024;
            unit++;
        }
        return value.toFixed(2) + " " + units[unit];
    }

    function formatDate(seconds) {
        if (seconds === null || seconds === undefined) return "";
        return Qt.formatDateTime(new Date(seconds * 1000), "yyyy-MM-dd HH:mm");
    }

    function typeName(kind) {
        switch (kind) {
        case "directory": return qsTr("Folder");
        case "symlink": return qsTr("Link");
        case "file": return qsTr("File");
        default: return qsTr("Other");
        }
    }

    function openEntry(entry) {
        if (entry.kind === "directory")
            backend.navigate(entry.path);
        else
            Qt.openUrlExternally("file://" + encodeURI(entry.path));
    }

    function askText(title, label, initial, action) {
        inputDialog.title = title;
        inputDialog.label = label;
        inputField.text = initial;
        inputDialog.action = action;
        inputDialog.open();
    }

    // ---- Actions (keyboard shortcuts and menus) --------------------------

    Action {
        id: newFolderAction
        text: qsTr("New Folder...")
        shortcut: "Ctrl+Shift+N"
        onTriggered: window.askText(qsTr("New Folder"), qsTr("Folder name:"), qsTr("New Folder"), "create")
    }
    Action {
        id: cutAction
        text: qsTr("Cut")
        shortcut: StandardKey.Cut
        enabled: window.selectedCount > 0
        onTriggered: backend.cut(JSON.stringify(window.selectedPaths()))
    }
    Action {
        id: copyAction
        text: qsTr("Copy")
        shortcut: StandardKey.Copy
        enabled: window.selectedCount > 0
        onTriggered: backend.copy(JSON.stringify(window.selectedPaths()))
    }
    Action {
        id: pasteAction
        text: qsTr("Paste")
        shortcut: StandardKey.Paste
        enabled: backend.canPaste
        onTriggered: backend.paste()
    }
    Action {
        id: renameAction
        text: qsTr("Rename...")
        shortcut: "F2"
        enabled: window.selectedCount === 1
        onTriggered: window.askText(qsTr("Rename"), qsTr("New name:"), window.entries[window.selected[0]].name, "rename")
    }
    Action {
        id: trashAction
        text: qsTr("Move to Trash")
        shortcut: StandardKey.Delete
        enabled: window.selectedCount > 0
        onTriggered: backend.moveToTrash(JSON.stringify(window.selectedPaths()))
    }
    Action {
        id: hiddenAction
        text: qsTr("Show Hidden Files")
        shortcut: "Ctrl+H"
        checkable: true
        checked: backend.showHidden
        onTriggered: backend.toggleHiddenFiles()
    }
    Action {
        id: reloadAction
        text: qsTr("Reload")
        shortcut: StandardKey.Refresh
        onTriggered: backend.reload()
    }
    Action {
        id: backAction
        text: qsTr("Back")
        shortcut: "Alt+Left"
        enabled: backend.canGoBack
        onTriggered: backend.goBack()
    }
    Action {
        id: forwardAction
        text: qsTr("Forward")
        shortcut: "Alt+Right"
        enabled: backend.canGoForward
        onTriggered: backend.goForward()
    }
    Action {
        id: upAction
        text: qsTr("Up")
        shortcut: "Alt+Up"
        onTriggered: backend.goUp()
    }
    Action {
        id: homeAction
        text: qsTr("Home")
        shortcut: "Alt+Home"
        onTriggered: backend.navigate(backend.homePath)
    }
    Action {
        id: quitAction
        text: qsTr("Quit")
        shortcut: StandardKey.Quit
        onTriggered: window.close()
    }

    // ---- Title row: menus, drag area and window buttons -------------------

    header: Rectangle {
        height: 34
        color: window.palette.window

        MouseArea {
            anchors.fill: parent
            onPressed: window.startSystemMove()
            onDoubleClicked: window.toggleMaximized()
        }

        RowLayout {
            anchors.fill: parent
            spacing: 0

            Image {
                Layout.alignment: Qt.AlignVCenter
                Layout.leftMargin: 6
                source: "qrc:/qt/qml/NexusFM/icon.png"
                sourceSize: Qt.size(20, 20)
                width: 20
                height: 20
                smooth: true
            }

            MenuBar {
                Layout.alignment: Qt.AlignVCenter
                Menu {
                    title: qsTr("&File")
                    MenuItem { action: quitAction }
                }
                Menu {
                    title: qsTr("&Edit")
                    MenuItem { action: cutAction }
                    MenuItem { action: copyAction }
                    MenuItem { action: pasteAction }
                    MenuSeparator {}
                    MenuItem { action: renameAction }
                    MenuItem { action: trashAction }
                    MenuSeparator {}
                    MenuItem { action: newFolderAction }
                }
                Menu {
                    title: qsTr("&View")
                    MenuItem { action: hiddenAction }
                    MenuSeparator {}
                    MenuItem { action: reloadAction }
                }
                Menu {
                    title: qsTr("&Go")
                    MenuItem { action: backAction }
                    MenuItem { action: forwardAction }
                    MenuItem { action: upAction }
                    MenuItem { action: homeAction }
                }
                Menu {
                    title: qsTr("&Help")
                    MenuItem {
                        text: qsTr("About NexusFM")
                        onTriggered: aboutDialog.open()
                    }
                }
            }

            Item { Layout.fillWidth: true }

            ToolButton {
                text: "–"
                ToolTip.text: qsTr("Minimize")
                ToolTip.visible: hovered
                onClicked: window.showMinimized()
            }
            ToolButton {
                text: window.visibility === Window.Maximized ? "❐" : "□"
                ToolTip.text: window.visibility === Window.Maximized ? qsTr("Restore") : qsTr("Maximize")
                ToolTip.visible: hovered
                onClicked: window.toggleMaximized()
            }
            ToolButton {
                text: "×"
                ToolTip.text: qsTr("Close")
                ToolTip.visible: hovered
                onClicked: window.close()
            }
        }
    }

    function toggleMaximized() {
        window.visibility = window.visibility === Window.Maximized ? Window.Windowed : Window.Maximized;
    }

    // ---- Navigation toolbar and location bar ------------------------------

    footer: ToolBar {
        visible: false
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 4
            spacing: 4

            ToolButton { action: backAction; text: "←"; display: AbstractButton.TextOnly }
            ToolButton { action: forwardAction; text: "→"; display: AbstractButton.TextOnly }
            ToolButton { action: upAction; text: "↑"; display: AbstractButton.TextOnly }
            ToolButton { action: reloadAction; text: "↻"; display: AbstractButton.TextOnly }

            TextField {
                id: location
                Layout.fillWidth: true
                placeholderText: qsTr("Location")
                text: backend.currentPath
                selectByMouse: true
                onAccepted: backend.navigate(text)
            }
        }

        // ---- Sidebar and file list ----------------------------------------

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            ListView {
                id: sidebar
                Layout.preferredWidth: 180
                Layout.fillHeight: true
                clip: true
                model: window.places
                ScrollBar.vertical: ScrollBar {}

                delegate: ItemDelegate {
                    required property var modelData
                    width: sidebar.width
                    text: modelData.label
                    highlighted: backend.currentPath === modelData.path
                    onClicked: backend.navigate(modelData.path)
                }
            }

            Rectangle {
                Layout.preferredWidth: 1
                Layout.fillHeight: true
                color: window.palette.mid
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                // Column headers. Clicking one sorts by that column.
                Row {
                    id: headerRow
                    Layout.fillWidth: true
                    height: 26

                    Repeater {
                        model: [
                            { title: qsTr("Name"), key: 0, width: -1 },
                            { title: qsTr("Size"), key: 1, width: window.columnWidths.size },
                            { title: qsTr("Type"), key: -1, width: window.columnWidths.type },
                            { title: qsTr("Modified"), key: 2, width: window.columnWidths.modified }
                        ]

                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: modelData.width < 0 ? headerRow.width - (headerRow.children.length > 1 ? 0 : 0)
                                                       - window.columnWidths.size - window.columnWidths.type - window.columnWidths.modified
                                                       : modelData.width
                            height: headerRow.height
                            color: window.palette.button

                            Label {
                                anchors.fill: parent
                                anchors.leftMargin: 6
                                verticalAlignment: Text.AlignVCenter
                                elide: Text.ElideRight
                                text: modelData.title + (backend.sortKey === modelData.key
                                                         ? (backend.descending ? " ▾" : " ▴") : "")
                            }

                            MouseArea {
                                anchors.fill: parent
                                enabled: modelData.key >= 0
                                onClicked: backend.sortBy(modelData.key)
                            }
                        }
                    }
                }

                ListView {
                    id: fileView
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: window.entries
                    focus: true
                    keyNavigationEnabled: true
                    ScrollBar.vertical: ScrollBar {}

                    delegate: Rectangle {
                        required property var modelData
                        required property int index

                        width: fileView.width
                        height: 26
                        color: window.selected.indexOf(index) >= 0
                               ? window.palette.highlight
                               : (index % 2 === 0 ? "transparent" : Qt.rgba(window.palette.text.r, window.palette.text.g, window.palette.text.b, 0.05))

                        RowLayout {
                            anchors.fill: parent
                            spacing: 0

                            Label {
                                Layout.fillWidth: true
                                Layout.leftMargin: 6
                                text: modelData.name
                                elide: Text.ElideRight
                                font.bold: modelData.kind === "directory"
                                color: window.selected.indexOf(index) >= 0 ? window.palette.highlightedText : window.palette.text
                            }
                            Label {
                                Layout.preferredWidth: window.columnWidths.size
                                horizontalAlignment: Text.AlignRight
                                rightPadding: 8
                                text: modelData.kind === "directory" ? "" : window.formatSize(modelData.size)
                                color: window.selected.indexOf(index) >= 0 ? window.palette.highlightedText : window.palette.text
                            }
                            Label {
                                Layout.preferredWidth: window.columnWidths.type
                                leftPadding: 6
                                text: window.typeName(modelData.kind)
                                color: window.selected.indexOf(index) >= 0 ? window.palette.highlightedText : window.palette.text
                            }
                            Label {
                                Layout.preferredWidth: window.columnWidths.modified
                                leftPadding: 6
                                text: window.formatDate(modelData.modified)
                                color: window.selected.indexOf(index) >= 0 ? window.palette.highlightedText : window.palette.text
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton | Qt.RightButton
                            onClicked: function (mouse) {
                                if (mouse.button === Qt.RightButton) {
                                    if (window.selected.indexOf(index) < 0)
                                        window.selectOnly(index);
                                    contextMenu.popup();
                                } else if (mouse.modifiers & Qt.ControlModifier) {
                                    window.toggleRow(index);
                                } else if (mouse.modifiers & Qt.ShiftModifier) {
                                    window.selectRange(index);
                                } else {
                                    window.selectOnly(index);
                                }
                            }
                            onDoubleClicked: window.openEntry(modelData)
                        }
                    }

                    MouseArea {
                        // Clicking empty space clears the selection.
                        anchors.fill: parent
                        z: -1
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function (mouse) {
                            if (mouse.button === Qt.RightButton)
                                contextMenu.popup();
                            else
                                window.selected = [];
                        }
                    }
                }
            }
        }

        // ---- Status bar ---------------------------------------------------

        Label {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.bottomMargin: 2
            text: backend.statusText
            color: window.palette.placeholderText
        }
    }

    // ---- Context menu for files and empty space ---------------------------

    Menu {
        id: contextMenu
        MenuItem { action: newFolderAction }
        MenuSeparator {}
        MenuItem { action: cutAction }
        MenuItem { action: copyAction }
        MenuItem { action: pasteAction }
        MenuSeparator {}
        MenuItem { action: renameAction }
        MenuItem { action: trashAction }
    }


    // ---- Edge handles for resizing the frameless window -------------------

    component EdgeHandle: MouseArea {
        required property int edges
        property int grip: 6
        enabled: window.visibility !== Window.Maximized
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton
        z: 10
        onPressed: window.startSystemResize(edges)
    }

    EdgeHandle {
        edges: Qt.LeftEdge
        cursorShape: Qt.SizeHorCursor
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: 34
        width: grip
    }
    EdgeHandle {
        edges: Qt.RightEdge
        cursorShape: Qt.SizeHorCursor
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.topMargin: 34
        width: grip
    }
    EdgeHandle {
        edges: Qt.BottomEdge
        cursorShape: Qt.SizeVerCursor
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: grip
    }
    EdgeHandle {
        edges: Qt.LeftEdge | Qt.BottomEdge
        cursorShape: Qt.SizeBDiagCursor
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        width: grip * 2
        height: grip * 2
    }
    EdgeHandle {
        edges: Qt.RightEdge | Qt.BottomEdge
        cursorShape: Qt.SizeFDiagCursor
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: grip * 2
        height: grip * 2
    }

    // ---- Dialogs -----------------------------------------------------------

    Dialog {
        id: inputDialog
        property string label: ""
        property string action: ""
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        anchors.centerIn: Overlay.overlay

        ColumnLayout {
            Label { text: inputDialog.label }
            TextField {
                id: inputField
                Layout.preferredWidth: 300
                selectByMouse: true
            }
        }

        onAccepted: {
            const name = inputField.text.trim();
            if (name.length === 0)
                return;
            if (action === "create")
                backend.createFolder(name);
            else if (action === "rename")
                backend.renameItem(window.entries[window.selected[0]].path, name);
        }
    }

    ErrorPanel {
        id: errorPanel
        message: backend.errorText
        onClosed: backend.clearError()
    }

    Dialog {
        id: aboutDialog
        title: qsTr("About NexusFM")
        modal: true
        standardButtons: Dialog.Ok
        anchors.centerIn: Overlay.overlay

        Label {
            width: 320
            wrapMode: Text.Wrap
            text: qsTr("NexusFM is a lightweight file manager built with Rust and Qt Quick.")
        }
    }
}
