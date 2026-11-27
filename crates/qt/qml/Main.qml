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
    // The bookmark whose context menu is open, and its current name.
    property int bookmarkIndex: -1
    property string bookmarkName: ""
    property var properties: ({})
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

        function onPropertiesJsonChanged() {
            window.properties = JSON.parse(backend.propertiesJson);
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

    function rowColor(row) {
        if (selected.indexOf(row) >= 0)
            return window.palette.highlight;
        return row % 2 === 0 ? "transparent"
                             : Qt.rgba(window.palette.text.r, window.palette.text.g, window.palette.text.b, 0.05);
    }

    function textColor(row) {
        return selected.indexOf(row) >= 0 ? window.palette.highlightedText : window.palette.text;
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

    function showProperties(path) {
        backend.inspect(path);
        propertiesDialog.open();
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
        id: propertiesAction
        text: qsTr("Properties")
        shortcut: "Alt+Return"
        enabled: window.selectedCount === 1
        onTriggered: window.showProperties(window.entries[window.selected[0]].path)
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
    ActionGroup {
        id: viewGroup
        exclusive: true
    }
    Action {
        id: detailedAction
        text: qsTr("Detailed List")
        checkable: true
        checked: backend.viewMode === 0
        ActionGroup.group: viewGroup
        onTriggered: backend.chooseViewMode(0)
    }
    Action {
        id: compactAction
        text: qsTr("Compact")
        checkable: true
        checked: backend.viewMode === 1
        ActionGroup.group: viewGroup
        onTriggered: backend.chooseViewMode(1)
    }
    Action {
        id: iconAction
        text: qsTr("Icons")
        checkable: true
        checked: backend.viewMode === 2
        ActionGroup.group: viewGroup
        onTriggered: backend.chooseViewMode(2)
    }
    Action {
        id: addBookmarkAction
        text: qsTr("Bookmark This Folder")
        shortcut: "Ctrl+D"
        onTriggered: backend.addBookmark()
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
                    MenuItem { action: detailedAction }
                    MenuItem { action: compactAction }
                    MenuItem { action: iconAction }
                    MenuSeparator {}
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
                    title: qsTr("&Bookmarks")
                    MenuItem { action: addBookmarkAction }
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

                delegate: Item {
                    required property var modelData
                    width: sidebar.width
                    height: 30

                    Label {
                        visible: modelData.kind === "header"
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: 12
                        text: modelData.label
                        font.bold: true
                        color: window.palette.placeholderText
                    }

                    ItemDelegate {
                        visible: modelData.kind !== "header"
                        anchors.fill: parent
                        text: modelData.label
                        highlighted: backend.currentPath === modelData.path
                        onClicked: backend.navigate(modelData.path)
                    }

                    // Right-click on a bookmark opens its menu; left clicks pass through.
                    MouseArea {
                        anchors.fill: parent
                        enabled: modelData.kind === "bookmark"
                        acceptedButtons: Qt.RightButton
                        onClicked: {
                            window.bookmarkIndex = modelData.index;
                            window.bookmarkName = modelData.label;
                            bookmarkMenu.popup();
                        }
                    }
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
                    visible: backend.viewMode === 0
                    height: visible ? 26 : 0

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

                // The three layouts share one model and one selection.
                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    MouseArea {
                        // Clicking empty space clears the selection.
                        anchors.fill: parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        onClicked: function (mouse) {
                            if (mouse.button === Qt.RightButton)
                                contextMenu.popup();
                            else
                                window.selected = [];
                        }
                    }

                    ListView {
                        id: fileView
                        anchors.fill: parent
                        visible: backend.viewMode === 0
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
                            color: window.rowColor(index)

                            RowLayout {
                                anchors.fill: parent
                                spacing: 0

                                Label {
                                    Layout.fillWidth: true
                                    Layout.leftMargin: 6
                                    text: modelData.name
                                    elide: Text.ElideRight
                                    font.bold: modelData.kind === "directory"
                                    color: window.textColor(index)
                                }
                                Label {
                                    Layout.preferredWidth: window.columnWidths.size
                                    horizontalAlignment: Text.AlignRight
                                    rightPadding: 8
                                    text: modelData.kind === "directory" ? "" : window.formatSize(modelData.size)
                                    color: window.textColor(index)
                                }
                                Label {
                                    Layout.preferredWidth: window.columnWidths.type
                                    leftPadding: 6
                                    text: window.typeName(modelData.kind)
                                    color: window.textColor(index)
                                }
                                Label {
                                    Layout.preferredWidth: window.columnWidths.modified
                                    leftPadding: 6
                                    text: window.formatDate(modelData.modified)
                                    color: window.textColor(index)
                                }
                            }

                            ItemClick {
                                anchors.fill: parent
                                modelData: parent.modelData
                                index: parent.index
                            }
                        }
                    }

                    GridView {
                        // Compact: names only, filling each column top to bottom.
                        id: compactView
                        anchors.fill: parent
                        visible: backend.viewMode === 1
                        clip: true
                        cellWidth: 240
                        cellHeight: 26
                        flow: GridView.FlowTopToBottom
                        model: window.entries
                        ScrollBar.horizontal: ScrollBar {}

                        delegate: Rectangle {
                            required property var modelData
                            required property int index

                            width: compactView.cellWidth
                            height: compactView.cellHeight
                            color: window.rowColor(index)

                            Label {
                                anchors.fill: parent
                                leftPadding: 6
                                rightPadding: 6
                                verticalAlignment: Text.AlignVCenter
                                text: modelData.name
                                elide: Text.ElideRight
                                font.bold: modelData.kind === "directory"
                                color: window.textColor(index)
                            }

                            ItemClick {
                                anchors.fill: parent
                                modelData: parent.modelData
                                index: parent.index
                            }
                        }
                    }

                    GridView {
                        // Icons: a placeholder tile per entry, with the name beneath.
                        id: iconView
                        anchors.fill: parent
                        visible: backend.viewMode === 2
                        clip: true
                        cellWidth: 120
                        cellHeight: 104
                        model: window.entries
                        ScrollBar.vertical: ScrollBar {}

                        delegate: Rectangle {
                            required property var modelData
                            required property int index

                            width: iconView.cellWidth
                            height: iconView.cellHeight
                            radius: 4
                            color: window.rowColor(index)

                            Column {
                                anchors.horizontalCenter: parent.horizontalCenter
                                anchors.top: parent.top
                                anchors.topMargin: 8
                                width: parent.width - 12
                                spacing: 6

                                Rectangle {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    width: 48
                                    height: 40
                                    radius: 4
                                    color: modelData.kind === "directory"
                                           ? Qt.rgba(0.35, 0.6, 0.9, 1)
                                           : window.palette.mid
                                }
                                Label {
                                    width: parent.width
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.Wrap
                                    maximumLineCount: 2
                                    elide: Text.ElideRight
                                    text: modelData.name
                                    color: window.textColor(index)
                                }
                            }

                            ItemClick {
                                anchors.fill: parent
                                modelData: parent.modelData
                                index: parent.index
                            }
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
        MenuSeparator {}
        MenuItem { action: propertiesAction }
    }


    // ---- Edge handles for resizing the frameless window -------------------

    // Click handling shared by all file layouts: select, extend the selection,
    // open on double-click, or show the context menu on right-click.
    component ItemClick: MouseArea {
        required property var modelData
        required property int index

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

    // ---- Bookmark context menu ---------------------------------------------

    Menu {
        id: bookmarkMenu
        MenuItem {
            text: qsTr("Rename...")
            onTriggered: window.askText(qsTr("Rename Bookmark"), qsTr("Bookmark name (empty for the folder name):"), window.bookmarkName, "renameBookmark")
        }
        MenuItem {
            text: qsTr("Move Up")
            onTriggered: backend.moveBookmark(window.bookmarkIndex, -1)
        }
        MenuItem {
            text: qsTr("Move Down")
            onTriggered: backend.moveBookmark(window.bookmarkIndex, 1)
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Remove Bookmark")
            onTriggered: backend.removeBookmark(window.bookmarkIndex)
        }
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
            if (action === "renameBookmark") {
                // An empty name restores the folder's own name.
                backend.renameBookmark(window.bookmarkIndex, inputField.text);
                return;
            }
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
        id: propertiesDialog
        title: qsTr("Properties")
        modal: true
        standardButtons: Dialog.Close
        anchors.centerIn: Overlay.overlay

        GridLayout {
            columns: 2
            columnSpacing: 16
            rowSpacing: 6

            Label { text: qsTr("Name") }
            Label { text: window.properties.name ?? ""; Layout.fillWidth: true; elide: Text.ElideMiddle }

            Label { text: qsTr("Type") }
            Label { text: window.typeName(window.properties.kind ?? "") }

            Label { text: qsTr("MIME type") }
            Label { text: window.properties.mimeType ?? "" }

            Label { text: qsTr("Opens with"); visible: !!window.properties.defaultApplication }
            Label {
                visible: !!window.properties.defaultApplication
                text: window.properties.defaultApplication ?? ""
            }

            Label { text: qsTr("Location") }
            Label {
                text: (window.properties.path ?? "").replace(/\/[^\/]*$/, "") || "/"
                Layout.fillWidth: true
                elide: Text.ElideMiddle
            }

            Label { text: qsTr("Size"); visible: window.properties.kind !== "directory" }
            Label {
                visible: window.properties.kind !== "directory"
                text: window.formatSize(window.properties.size ?? 0)
            }

            Label { text: qsTr("Link target"); visible: !!window.properties.linkTarget }
            Label {
                visible: !!window.properties.linkTarget
                text: window.properties.linkTarget ?? ""
                Layout.fillWidth: true
                elide: Text.ElideMiddle
            }

            Label { text: qsTr("Modified") }
            Label { text: window.formatDate(window.properties.modified) }

            Label { text: qsTr("Accessed") }
            Label { text: window.formatDate(window.properties.accessed) }

            Label { text: qsTr("Created") }
            Label { text: window.formatDate(window.properties.created) }

            Label { text: qsTr("Permissions") }
            Label { text: window.properties.permissions ?? "" }

            Label { text: qsTr("Owner") }
            Label { text: window.properties.owner ?? "" }

            Label { text: qsTr("Group") }
            Label { text: window.properties.group ?? "" }
        }
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
