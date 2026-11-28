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
    title: folderName() ? qsTr("%1 - NexusFM").arg(folderName()) : qsTr("NexusFM")
    flags: Qt.Window | Qt.FramelessWindowHint
    color: palette.window

    // ---- Backend and view state ------------------------------------------

    // The primary pane's backend: saves tabs, bookmarks and the window layout,
    // and feeds the sidebar. The second pane in split view has its own backend.
    Backend {
        id: backend
    }
    Backend {
        id: secondBackend
    }

    // Picks up changes made to the open folder by other programs.
    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: backend.checkForChanges()
    }

    property var places: []
    // Split view: a second pane beside the first.
    property bool splitView: false
    // True while an item from this window is being dragged, so a drop moves it.
    property bool internalDrag: false
    // The pane that shared actions act on: its entries, selection and backend.
    property var focusedPane: leftPane
    readonly property var focusedBackend: focusedPane.paneBackend
    readonly property var entries: focusedPane.entries
    readonly property var selected: focusedPane.selected
    readonly property int selectedCount: focusedPane.selectedCount
    // Items copied or cut in either pane, shared by the whole window.
    property var clipboardPaths: []
    property bool clipboardCut: false
    // The bookmark whose context menu is open, and its current name.
    // The device whose context menu is open.
    property string deviceKey: ""
    property bool deviceMounted: false
    property bool deviceCanUnmount: false
    property int bookmarkIndex: -1
    property string bookmarkName: ""
    property var properties: ({})

    // Layout, remembered between runs (see WindowState in the config crate).
    property bool sidebarVisible: true
    property int sidebarWidth: 180
    property bool toolbarVisible: true
    property bool statusBarVisible: true

    // file:// URIs to paths, and back. Other URIs (smb://, trash:///) stay as they are.
    function uriToPath(text) {
        if (text.indexOf("file://") === 0)
            return decodeURIComponent(text.substring(7));
        return text;
    }

    function pathToUri(path) {
        if (path.indexOf("://") > 0)
            return path;
        return "file://" + path.split("/").map(encodeURIComponent).join("/");
    }

    // Folders dropped on the sidebar are bookmarked; other items are ignored.
    function bookmarkDropped(drop) {
        if (!drop.hasUrls)
            return;
        Array.from(drop.urls).forEach(function (url) {
            backend.addBookmarkPath(uriToPath(String(url)));
        });
        drop.accept(Qt.CopyAction);
    }

    // A bookmark dragged onto a bookmark row moves to that row; a folder dropped there is bookmarked.
    function dropOnBookmark(drop, toIndex) {
        const from = drop.getDataAsString("application/x-nexusfm-bookmark");
        if (from.length > 0) {
            backend.moveBookmarkTo(parseInt(from), toIndex);
            drop.accept(Qt.MoveAction);
        } else {
            bookmarkDropped(drop);
        }
    }

    function focusPane(pane) {
        focusedPane = pane;
        location.text = focusedBackend.currentPath;
    }

    function toggleSplit(on) {
        if (on && secondBackend.currentPath === "") {
            secondBackend.setPersistent(false);
            secondBackend.start(backend.currentPath || backend.homePath);
        }
        splitView = on;
        if (!on)
            focusedPane = leftPane;
    }

    // Copy or cut the selection of the focused pane, for pasting in either pane.
    function setClipboard(paths, cut) {
        clipboardPaths = paths;
        clipboardCut = cut;
    }

    // Paste the shared clipboard into the focused pane. A cut keeps the items that failed.
    function pasteIntoFocused() {
        const failed = JSON.parse(focusedBackend.pasteItems(JSON.stringify(clipboardPaths), clipboardCut));
        if (clipboardCut)
            clipboardPaths = failed;
    }

    function folderName() {
        const path = focusedBackend.currentPath.replace(/\/+$/, "");
        if (path.length === 0)
            return "";
        const name = path.substring(path.lastIndexOf("/") + 1);
        return name.length > 0 ? name : "/";
    }

    function applyLayout(json) {
        const layout = JSON.parse(json);
        window.width = layout.width;
        window.height = layout.height;
        window.sidebarVisible = layout.sidebar_visible;
        window.sidebarWidth = layout.sidebar_width;
        window.toolbarVisible = layout.toolbar_visible;
        window.statusBarVisible = layout.status_bar_visible;
    }

    function saveLayout() {
        backend.saveWindowState(JSON.stringify({
            width: window.width,
            height: window.height,
            sidebar_visible: window.sidebarVisible,
            sidebar_width: window.sidebarWidth,
            toolbar_visible: window.toolbarVisible,
            status_bar_visible: window.statusBarVisible
        }));
    }

    // Save shortly after the layout changes, so it survives however the window is closed.
    Timer {
        id: layoutSaveTimer
        interval: 500
        onTriggered: window.saveLayout()
    }
    onWidthChanged: layoutSaveTimer.restart()
    onHeightChanged: layoutSaveTimer.restart()
    onSidebarVisibleChanged: layoutSaveTimer.restart()
    onSidebarWidthChanged: layoutSaveTimer.restart()
    onToolbarVisibleChanged: layoutSaveTimer.restart()
    onStatusBarVisibleChanged: layoutSaveTimer.restart()

    onClosing: saveLayout()

    Component.onCompleted: {
        const args = Qt.application.arguments;
        backend.start(args.length > 1 ? args[1] : "");
        window.applyLayout(backend.windowStateJson);
    }

    // Changes that belong to the primary backend: the sidebar and the window layout.
    Connections {
        target: backend

        function onPlacesJsonChanged() {
            window.places = JSON.parse(backend.placesJson);
        }

        function onErrorTextChanged() {
            if (backend.errorText.length > 0)
                errorPanel.open();
        }
    }

    // Errors from either pane are shown in the same panel.
    Connections {
        target: secondBackend

        function onErrorTextChanged() {
            if (secondBackend.errorText.length > 0)
                errorPanel.open();
        }
    }

    // The location bar follows the focused pane's folder.
    Connections {
        target: window.focusedBackend

        function onCurrentPathChanged() {
            location.text = window.focusedBackend.currentPath;
        }

        function onPropertiesJsonChanged() {
            window.properties = JSON.parse(window.focusedBackend.propertiesJson);
        }
    }

    // ---- Helpers ----------------------------------------------------------

    // Free space shown after a device name, such as "  4.00 GiB free".
    function capacityText(device) {
        if (device.total === null || device.total === undefined)
            return "";
        return "  " + window.formatSize(device.free ?? 0) + " free";
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
        focusedBackend.inspect(path);
        propertiesDialog.open();
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
        onTriggered: window.setClipboard(window.focusedPane.selectedPaths(), true)
    }
    Action {
        id: copyAction
        text: qsTr("Copy")
        shortcut: StandardKey.Copy
        enabled: window.selectedCount > 0
        onTriggered: window.setClipboard(window.focusedPane.selectedPaths(), false)
    }
    Action {
        id: pasteAction
        text: qsTr("Paste")
        shortcut: StandardKey.Paste
        enabled: window.clipboardPaths.length > 0
        onTriggered: window.pasteIntoFocused()
    }
    Action {
        id: renameAction
        text: qsTr("Rename...")
        shortcut: "F2"
        enabled: window.selectedCount === 1
        onTriggered: window.askText(qsTr("Rename"), qsTr("New name:"), window.focusedPane.entries[window.focusedPane.selected[0]].name, "rename")
    }
    Action {
        id: propertiesAction
        text: qsTr("Properties")
        shortcut: "Alt+Return"
        enabled: window.selectedCount === 1
        onTriggered: window.showProperties(window.focusedPane.entries[window.focusedPane.selected[0]].path)
    }
    Action {
        id: trashAction
        text: qsTr("Move to Trash")
        shortcut: StandardKey.Delete
        enabled: window.selectedCount > 0
        onTriggered: window.focusedBackend.moveToTrash(JSON.stringify(window.focusedPane.selectedPaths()))
    }
    Action {
        id: sidebarAction
        text: qsTr("Show Sidebar")
        shortcut: "F9"
        checkable: true
        checked: window.sidebarVisible
        onTriggered: window.sidebarVisible = checked
    }
    Action {
        id: toolbarAction
        text: qsTr("Show Toolbar")
        checkable: true
        checked: window.toolbarVisible
        onTriggered: window.toolbarVisible = checked
    }
    Action {
        id: statusBarAction
        text: qsTr("Show Status Bar")
        checkable: true
        checked: window.statusBarVisible
        onTriggered: window.statusBarVisible = checked
    }
    Action {
        id: hiddenAction
        text: qsTr("Show Hidden Files")
        shortcut: "Ctrl+H"
        checkable: true
        checked: window.focusedBackend.showHidden
        onTriggered: window.focusedBackend.toggleHiddenFiles()
    }
    Action {
        id: reloadAction
        text: qsTr("Reload")
        shortcut: StandardKey.Refresh
        onTriggered: window.focusedBackend.reload()
    }
    Action {
        id: backAction
        text: qsTr("Back")
        shortcut: "Alt+Left"
        enabled: window.focusedBackend.canGoBack
        onTriggered: window.focusedBackend.goBack()
    }
    Action {
        id: forwardAction
        text: qsTr("Forward")
        shortcut: "Alt+Right"
        enabled: window.focusedBackend.canGoForward
        onTriggered: window.focusedBackend.goForward()
    }
    Action {
        id: upAction
        text: qsTr("Up")
        shortcut: "Alt+Up"
        onTriggered: window.focusedBackend.goUp()
    }
    Action {
        id: homeAction
        text: qsTr("Home")
        shortcut: "Alt+Home"
        onTriggered: window.focusedBackend.navigate(window.focusedBackend.homePath)
    }
    ActionGroup {
        id: viewGroup
        exclusive: true
    }
    Action {
        id: detailedAction
        text: qsTr("Detailed List")
        checkable: true
        checked: window.focusedBackend.viewMode === 0
        ActionGroup.group: viewGroup
        onTriggered: window.focusedBackend.chooseViewMode(0)
    }
    Action {
        id: compactAction
        text: qsTr("Compact")
        checkable: true
        checked: window.focusedBackend.viewMode === 1
        ActionGroup.group: viewGroup
        onTriggered: window.focusedBackend.chooseViewMode(1)
    }
    Action {
        id: iconAction
        text: qsTr("Icons")
        checkable: true
        checked: window.focusedBackend.viewMode === 2
        ActionGroup.group: viewGroup
        onTriggered: window.focusedBackend.chooseViewMode(2)
    }
    Action {
        id: addBookmarkAction
        text: qsTr("Bookmark This Folder")
        shortcut: "Ctrl+D"
        onTriggered: backend.addBookmarkPath(window.focusedBackend.currentPath)
    }
    Action {
        id: newTabAction
        text: qsTr("New Tab")
        shortcut: "Ctrl+T"
        onTriggered: window.focusedBackend.newTab()
    }
    Action {
        id: closeTabAction
        text: qsTr("Close Tab")
        shortcut: "Ctrl+W"
        enabled: window.focusedPane.tabs.length > 1
        onTriggered: window.focusedBackend.closeTab(window.focusedBackend.activeTab)
    }
    Action {
        id: nextTabAction
        text: qsTr("Next Tab")
        shortcut: "Ctrl+Tab"
        enabled: window.focusedPane.tabs.length > 1
        onTriggered: window.focusedBackend.switchTab((window.focusedBackend.activeTab + 1) % window.focusedPane.tabs.length)
    }
    Action {
        id: previousTabAction
        text: qsTr("Previous Tab")
        shortcut: "Ctrl+Shift+Tab"
        enabled: window.focusedPane.tabs.length > 1
        onTriggered: window.focusedBackend.switchTab((window.focusedBackend.activeTab + window.focusedPane.tabs.length - 1) % window.focusedPane.tabs.length)
    }
    Action {
        id: openInNewTabAction
        text: qsTr("Open in New Tab")
        enabled: window.selectedCount === 1 && window.focusedPane.entries[window.focusedPane.selected[0]].kind === "directory"
        onTriggered: window.focusedBackend.openInNewTab(window.focusedPane.entries[window.focusedPane.selected[0]].path)
    }
    // Split view: show a second pane beside the first.
    Action {
        id: splitAction
        text: qsTr("Split View")
        shortcut: "F3"
        checkable: true
        checked: window.splitView
        onTriggered: window.toggleSplit(checked)
    }
    // Actions that the panes' menus use.
    readonly property var acts: ({
        newFolder: newFolderAction,
        cut: cutAction,
        copy: copyAction,
        paste: pasteAction,
        rename: renameAction,
        trash: trashAction,
        properties: propertiesAction,
        openInNewTab: openInNewTabAction
    })
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
                    MenuItem { action: newTabAction }
                    MenuItem { action: closeTabAction }
                    MenuSeparator {}
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
                    MenuItem { action: splitAction }
                    MenuSeparator {}
                    MenuItem { action: sidebarAction }
                    MenuItem { action: toolbarAction }
                    MenuItem { action: statusBarAction }
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
                    MenuSeparator {}
                    MenuItem {
                        text: qsTr("Import GTK Bookmarks")
                        onTriggered: backend.importGtkBookmarks()
                    }
                    MenuItem {
                        text: qsTr("Import Bookmarks...")
                        onTriggered: window.askText(qsTr("Import Bookmarks"), qsTr("File to import (GTK bookmarks format):"), "", "importBookmarks")
                    }
                    MenuItem {
                        text: qsTr("Export Bookmarks...")
                        onTriggered: window.askText(qsTr("Export Bookmarks"), qsTr("File to write (GTK bookmarks format):"), backend.homePath + "/nexusfm-bookmarks.txt", "exportBookmarks")
                    }
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
            visible: window.toolbarVisible

            ToolButton { action: backAction; text: "←"; display: AbstractButton.TextOnly }
            ToolButton { action: forwardAction; text: "→"; display: AbstractButton.TextOnly }
            ToolButton { action: upAction; text: "↑"; display: AbstractButton.TextOnly }
            ToolButton { action: reloadAction; text: "↻"; display: AbstractButton.TextOnly }

            TextField {
                id: location
                Layout.fillWidth: true
                placeholderText: qsTr("Location")
                text: window.focusedBackend.currentPath
                selectByMouse: true
                onAccepted: window.focusedBackend.openLocation(text)
            }
        }

        // ---- Sidebar and file panes ---------------------------------------

        RowLayout {
            id: sidebarRow
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            Item {
                id: sidebarBox
                Layout.preferredWidth: window.sidebarVisible ? window.sidebarWidth : 0
                Layout.fillHeight: true
                visible: window.sidebarVisible

                ListView {
                    id: sidebar
                anchors.fill: parent
                    clip: true
                    model: window.places
                    ScrollBar.vertical: ScrollBar {}

                    delegate: Item {
                        required property var modelData
                        width: sidebar.width
                        height: modelData.kind === "separator" ? 12 : 30

                        // A bookmark can be dragged onto another row to reorder the list.
                        Drag.active: bookmarkDrag.active
                        Drag.dragType: Drag.Automatic
                        Drag.supportedActions: Qt.MoveAction
                        Drag.mimeData: ({ "application/x-nexusfm-bookmark": String(modelData.index) })

                        DragHandler {
                            id: bookmarkDrag
                            enabled: modelData.kind === "bookmark"
                            target: null
                        }

                        // Dropping on a bookmark moves the dragged bookmark there, or bookmarks a dropped folder.
                        DropArea {
                            anchors.fill: parent
                            enabled: modelData.kind === "bookmark"
                            onDropped: function (drop) { window.dropOnBookmark(drop, modelData.index); }
                        }

                        Rectangle {
                            visible: modelData.kind === "separator"
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            height: 1
                            color: window.palette.mid
                        }

                        Label {
                            visible: modelData.kind === "header"
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.left: parent.left
                            anchors.leftMargin: 12
                            text: modelData.label
                            font.bold: true
                            color: window.palette.placeholderText
                        }

                        // Unavailable bookmarks stay listed, dimmed and not clickable.
                        ItemDelegate {
                            visible: modelData.kind !== "header" && modelData.kind !== "separator"
                            anchors.fill: parent
                            text: (modelData.kind === "device" ? modelData.label + window.capacityText(modelData) : modelData.label)
                                  + (modelData.available === false ? qsTr(" (unavailable)") : "")
                            icon.name: modelData.kind === "device" ? "drive-harddisk" : "folder"
                            enabled: modelData.available !== false
                            opacity: enabled ? 1 : 0.5
                            highlighted: modelData.kind !== "device" && backend.currentPath === modelData.path
                            onClicked: modelData.kind === "device" ? backend.openDevice(modelData.key) : window.focusedBackend.navigate(modelData.path)
                        }

                        // Right-click on a bookmark or device opens its menu; left clicks pass through.
                        MouseArea {
                            anchors.fill: parent
                            enabled: modelData.kind === "bookmark" || modelData.kind === "device"
                            acceptedButtons: Qt.RightButton
                            onClicked: {
                                if (modelData.kind === "device") {
                                    window.deviceKey = modelData.key;
                                    window.deviceMounted = modelData.mounted;
                                    window.deviceCanUnmount = modelData.canUnmount;
                                    deviceMenu.popup();
                                    return;
                                }
                                window.bookmarkIndex = modelData.index;
                                window.bookmarkName = modelData.label;
                                bookmarkMenu.popup();
                            }
                        }
                    }
                }

                // Folders dropped on the sidebar become bookmarks.
                DropArea {
                    anchors.fill: parent
                    onDropped: function (drop) { window.bookmarkDropped(drop); }
                }
            }

            // Divider between the sidebar and the file list; drag it to resize the sidebar.
            Rectangle {
                id: divider
                Layout.preferredWidth: window.sidebarVisible ? 1 : 0
                Layout.fillHeight: true
                visible: window.sidebarVisible
                color: window.palette.mid

                MouseArea {
                    anchors.fill: parent
                    anchors.leftMargin: -3
                    anchors.rightMargin: -3
                    cursorShape: Qt.SplitHCursor
                    onPositionChanged: function (mouse) {
                        if (pressed) {
                            const x = mapToItem(sidebarRow, mouse.x, 0).x;
                            window.sidebarWidth = Math.max(100, Math.min(400, Math.round(x)));
                        }
                    }
                }
            }

            // One pane, or two side by side in split view. Each has its own tabs and history.
            RowLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                Pane {
                    id: leftPane
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    paneBackend: backend
                    main: window
                    active: window.focusedPane === leftPane
                }

                Rectangle {
                    Layout.preferredWidth: window.splitView ? 1 : 0
                    Layout.fillHeight: true
                    visible: window.splitView
                    color: window.palette.mid
                }

                Pane {
                    id: rightPane
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    visible: window.splitView
                    paneBackend: secondBackend
                    main: window
                    active: window.focusedPane === rightPane
                }
            }
        }

        // ---- Status bar ---------------------------------------------------

        Label {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.bottomMargin: 2
            visible: window.statusBarVisible
            text: window.focusedBackend.statusText
            color: window.palette.placeholderText
        }
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

    // ---- Device context menu -----------------------------------------------

    Menu {
        id: deviceMenu
        MenuItem {
            text: qsTr("Mount")
            visible: !window.deviceMounted
            height: visible ? implicitHeight : 0
            onTriggered: backend.mountDevice(window.deviceKey)
        }
        MenuItem {
            text: qsTr("Unmount")
            visible: window.deviceCanUnmount
            height: visible ? implicitHeight : 0
            onTriggered: backend.unmountDevice(window.deviceKey)
        }
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
        MenuItem {
            text: qsTr("Add Separator Below")
            onTriggered: backend.addSeparator(window.bookmarkIndex)
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
            if (action === "importBookmarks") {
                if (name.length > 0)
                    backend.importBookmarks(name);
                return;
            }
            if (action === "exportBookmarks") {
                if (name.length > 0)
                    backend.exportBookmarks(name);
                return;
            }
            if (name.length === 0)
                return;
            if (action === "create")
                window.focusedBackend.createFolder(name);
            else if (action === "rename")
                window.focusedBackend.renameItem(window.focusedPane.entries[window.focusedPane.selected[0]].path, name);
        }
    }

    ErrorPanel {
        id: errorPanel
        message: backend.errorText || secondBackend.errorText
        onClosed: {
            backend.clearError();
            secondBackend.clearError();
        }
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
        width: 480

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            Image {
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: 8
                source: "qrc:/qt/qml/NexusFM/icon.png"
                sourceSize: Qt.size(96, 96)
                width: 96
                height: 96
                smooth: true
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: "NexusFM"
                font.pointSize: 18
                font.bold: true
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Version %1").arg("0.1.0")
                color: window.palette.placeholderText
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                text: qsTr("A lightweight file manager built with Rust and Qt Quick.")
            }

            Label {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                font.pointSize: 9
                color: window.palette.placeholderText
                text: qsTr("Licensed under the GNU General Public License v3.")
            }
        }
    }
}
