// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import NexusFM

// One file area: its own backend, tabs, history, selection and views. The main
// window shows one pane, or two side by side in split view.
Item {
    id: pane

    // The backend that owns this pane's folder, tabs and history.
    required property var paneBackend
    // The main window, for shared actions, menus and formatting helpers.
    required property var main
    // Whether this pane has the focus. Shared actions act on the focused pane.
    property bool active: false

    // Entries of the folder shown in this pane.
    property var entries: []
    // Titles of this pane's tabs.
    property var tabs: []
    // The folder the current entries belong to.
    property string shownPath: ""
    // Selected row indexes. Reassigned, never mutated, so bindings update.
    property var selected: []
    property int anchorRow: -1
    readonly property int selectedCount: selected.length

    readonly property var columnWidths: ({ name: 0, size: 110, type: 90, modified: 170 })
    // In a narrow pane (split view) the Type column is hidden so names stay readable.
    readonly property int typeWidth: width < 560 ? 0 : columnWidths.type

    function selectedPaths() {
        return selected
            .filter(function (row) { return row < entries.length; })
            .map(function (row) { return entries[row].path; });
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
            return main.palette.highlight;
        return row % 2 === 0 ? "transparent"
                             : Qt.rgba(main.palette.text.r, main.palette.text.g, main.palette.text.b, 0.05);
    }

    function textColor(row) {
        return selected.indexOf(row) >= 0 ? main.palette.highlightedText : main.palette.text;
    }

    function openEntry(entry) {
        if (entry.kind === "directory")
            paneBackend.navigate(entry.path);
        else
            // Remote entries already carry a URI; local paths need the file:// prefix.
            Qt.openUrlExternally(entry.path.indexOf("://") > 0 ? entry.path : "file://" + encodeURI(entry.path));
    }

    // Focus this pane when the user touches it, without taking the click.
    TapHandler {
        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        onPressedChanged: if (pressed) pane.main.focusPane(pane)
    }

    // Keep this pane's listing in step with its backend.
    Connections {
        target: pane.paneBackend

        function onEntriesJsonChanged() {
            // A refresh of the same folder keeps the selected items; a new folder clears it.
            const previous = pane.selectedPaths();
            const samePlace = pane.shownPath === pane.paneBackend.currentPath;
            pane.entries = JSON.parse(pane.paneBackend.entriesJson);
            pane.shownPath = pane.paneBackend.currentPath;
            if (samePlace && previous.length > 0) {
                pane.selected = pane.entries
                    .map(function (entry, row) { return previous.indexOf(entry.path) >= 0 ? row : -1; })
                    .filter(function (row) { return row >= 0; });
            } else {
                pane.selected = [];
                pane.anchorRow = -1;
            }
        }

        function onTabsJsonChanged() {
            pane.tabs = JSON.parse(pane.paneBackend.tabsJson);
            tabBar.currentIndex = pane.paneBackend.activeTab;
        }

        function onActiveTabChanged() {
            tabBar.currentIndex = pane.paneBackend.activeTab;
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Tabs, shown once there is more than one.
        TabBar {
            id: tabBar
            Layout.fillWidth: true
            visible: pane.tabs.length > 1

            Repeater {
                model: pane.tabs

                delegate: TabButton {
                    required property string modelData
                    required property int index

                    onClicked: pane.paneBackend.switchTab(index)

                    contentItem: RowLayout {
                        spacing: 4
                        Label {
                            Layout.fillWidth: true
                            text: modelData
                            elide: Text.ElideRight
                            horizontalAlignment: Text.AlignHCenter
                        }
                        ToolButton {
                            text: "×"
                            implicitWidth: 22
                            implicitHeight: 22
                            ToolTip.text: qsTr("Close Tab")
                            ToolTip.visible: hovered
                            onClicked: pane.paneBackend.closeTab(index)
                        }
                    }

                    // Middle-click closes a tab, as in most tabbed apps.
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.MiddleButton
                        onClicked: pane.paneBackend.closeTab(index)
                    }
                }
            }
        }

        // Column headers. Clicking one sorts by that column.
        Row {
            id: headerRow
            Layout.fillWidth: true
            visible: pane.paneBackend.viewMode === 0
            height: visible ? 26 : 0

            Repeater {
                model: [
                    { title: qsTr("Name"), key: 0, width: -1 },
                    { title: qsTr("Size"), key: 1, width: pane.columnWidths.size },
                    { title: qsTr("Type"), key: -1, width: pane.typeWidth },
                    { title: qsTr("Modified"), key: 2, width: pane.columnWidths.modified }
                ]

                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: modelData.width < 0
                           ? headerRow.width - pane.columnWidths.size - pane.typeWidth - pane.columnWidths.modified
                           : modelData.width
                    height: headerRow.height
                    color: pane.main.palette.button

                    Label {
                        anchors.fill: parent
                        anchors.leftMargin: 6
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                        text: modelData.title + (pane.paneBackend.sortKey === modelData.key
                                                 ? (pane.paneBackend.descending ? " ▾" : " ▴") : "")
                    }

                    MouseArea {
                        anchors.fill: parent
                        enabled: modelData.key >= 0
                        onClicked: pane.paneBackend.sortBy(modelData.key)
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
                        pane.selected = [];
                }
            }

            ListView {
                id: fileView
                anchors.fill: parent
                visible: pane.paneBackend.viewMode === 0
                clip: true
                model: pane.entries
                keyNavigationEnabled: true
                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    required property var modelData
                    required property int index

                    width: fileView.width
                    height: 26
                    color: pane.rowColor(index)

                    RowLayout {
                        anchors.fill: parent
                        spacing: 0

                        Label {
                            Layout.fillWidth: true
                            Layout.leftMargin: 6
                            text: modelData.name
                            elide: Text.ElideRight
                            font.bold: modelData.kind === "directory"
                            color: pane.textColor(index)
                        }
                        Label {
                            Layout.preferredWidth: pane.columnWidths.size
                            horizontalAlignment: Text.AlignRight
                            rightPadding: 8
                            text: modelData.kind === "directory" ? "" : pane.main.formatSize(modelData.size)
                            color: pane.textColor(index)
                        }
                        Label {
                            Layout.preferredWidth: pane.typeWidth
                            visible: pane.typeWidth > 0
                            leftPadding: 6
                            text: pane.main.typeName(modelData.kind)
                            color: pane.textColor(index)
                        }
                        Label {
                            Layout.preferredWidth: pane.columnWidths.modified
                            leftPadding: 6
                            text: pane.main.formatDate(modelData.modified)
                            color: pane.textColor(index)
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
                visible: pane.paneBackend.viewMode === 1
                clip: true
                cellWidth: 240
                cellHeight: 26
                flow: GridView.FlowTopToBottom
                model: pane.entries
                ScrollBar.horizontal: ScrollBar {}

                delegate: Rectangle {
                    required property var modelData
                    required property int index

                    width: compactView.cellWidth
                    height: compactView.cellHeight
                    color: pane.rowColor(index)

                    Label {
                        anchors.fill: parent
                        leftPadding: 6
                        rightPadding: 6
                        verticalAlignment: Text.AlignVCenter
                        text: modelData.name
                        elide: Text.ElideRight
                        font.bold: modelData.kind === "directory"
                        color: pane.textColor(index)
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
                visible: pane.paneBackend.viewMode === 2
                clip: true
                cellWidth: 120
                cellHeight: 104
                model: pane.entries
                ScrollBar.vertical: ScrollBar {}

                delegate: Rectangle {
                    required property var modelData
                    required property int index

                    width: iconView.cellWidth
                    height: iconView.cellHeight
                    radius: 4
                    color: pane.rowColor(index)

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
                                   : pane.main.palette.mid
                        }
                        Label {
                            width: parent.width
                            horizontalAlignment: Text.AlignHCenter
                            wrapMode: Text.Wrap
                            maximumLineCount: 2
                            elide: Text.ElideRight
                            text: modelData.name
                            color: pane.textColor(index)
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

    // Shows which pane has the focus when two are side by side.
    Rectangle {
        anchors.fill: parent
        color: "transparent"
        border.color: pane.main.palette.highlight
        border.width: pane.active && pane.main.splitView ? 2 : 0
        z: 5
    }

    // Click handling for file rows: select, extend the selection, open on
    // double-click, middle-click opens a folder in a new tab, right-click shows the menu.
    component ItemClick: MouseArea {
        required property var modelData
        required property int index

        acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
        onClicked: function (mouse) {
            if (mouse.button === Qt.MiddleButton) {
                if (modelData.kind === "directory")
                    pane.paneBackend.openInNewTab(modelData.path);
            } else if (mouse.button === Qt.RightButton) {
                if (pane.selected.indexOf(index) < 0)
                    pane.selectOnly(index);
                contextMenu.popup();
            } else if (mouse.modifiers & Qt.ControlModifier) {
                pane.toggleRow(index);
            } else if (mouse.modifiers & Qt.ShiftModifier) {
                pane.selectRange(index);
            } else {
                pane.selectOnly(index);
            }
        }
        onDoubleClicked: pane.openEntry(modelData)
    }

    // Context menu for files and empty space in this pane.
    Menu {
        id: contextMenu
        MenuItem {
            action: pane.main.acts.openInNewTab
            visible: pane.main.acts.openInNewTab.enabled
            height: visible ? implicitHeight : 0
        }
        MenuItem { action: pane.main.acts.newFolder }
        MenuSeparator {}
        MenuItem { action: pane.main.acts.cut }
        MenuItem { action: pane.main.acts.copy }
        MenuItem { action: pane.main.acts.paste }
        MenuSeparator {}
        MenuItem { action: pane.main.acts.rename }
        MenuItem { action: pane.main.acts.trash }
        MenuSeparator {}
        MenuItem { action: pane.main.acts.properties }
    }
}
