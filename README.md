# NexusFM

A lightweight file manager written in Rust, with a Qt 6 Quick (QML) interface.

## Features

- Browse directories as a detailed list, a compact list or a grid of icons (View menu). Click a column header to sort; click again to reverse.
- Navigate with Back, Forward, Up, Home and Reload, or type a location in the location bar: an absolute path, `~/...`, or a `file://` URI. Other URIs such as `smb://` and `trash:///` open through GVFS. File operations and the Properties dialog work on these locations too, through GIO.
- Tabs: `Ctrl+T` opens a new tab in the current folder, `Ctrl+W` closes it, and `Ctrl+Tab` / `Ctrl+Shift+Tab` switch tabs. Middle-click a folder, or choose Open in New Tab from its context menu, to open it in a new tab. Each tab has its own back and forward history; the clipboard is shared. Open tabs are remembered in `~/.local/state/nexusfm/tabs.json` and come back the next time NexusFM starts without a folder argument. Folders that no longer exist, and network locations, are not reopened.
- Split view (`F3`): a second file pane beside the first, with its own tabs, history and view. Copy or cut in one pane and paste in the other; shortcuts and menus act on the pane with the focus, shown by a highlighted border.
- Drag and drop: drag selected items to another pane, to a folder row, or to another program. Dropping onto a folder moves the items into it (Ctrl copies). Drop a folder on the sidebar to bookmark it. Items dragged from another program are copied.
- Places sidebar with Home, Desktop, Documents, Downloads, Music, Pictures, Videos, Trash, Recent and the file system root.
- Devices: mounted drives, USB sticks and network shares appear in the sidebar with their free space. Click an unmounted device to mount and open it; right-click to mount or unmount.
- Bookmarks: press `Ctrl+D` to bookmark the current folder. Drag a bookmark onto another to reorder the list, or drop a folder on the sidebar to bookmark it. Right-click a bookmark to rename, move, add a separator below it, or remove it. Bookmarks that are no longer there are shown dimmed as unavailable. Bookmarks are saved to `~/.config/nexusfm/bookmarks`, in the GTK bookmarks format; the Bookmarks menu imports GTK's own bookmarks or any file in that format, and exports the list to one.
- Automatic refresh: changes made to the open folder by other programs appear within about a second. Selected items stay selected.
- Show or hide hidden files (View menu or `Ctrl+H`).
- Open folders in the window; open files with their default application.
- File operations from the Edit menu and right-click menu: New Folder, Cut, Copy, Paste, Rename (`F2`) and Move to Trash (`Delete`). Existing files are never overwritten.
- File properties (Alt+Enter, or Properties in the context menu): type, size, dates, permissions, owner, group and symlink target.
- The window title shows the current folder. View menu toggles the sidebar (`F9`), toolbar and status bar. The window size, sidebar width and these toggles are remembered in `~/.local/state/nexusfm/window.json`.
- Custom title bar with minimize, maximize and close buttons, and edge resizing.
- Settings from a TOML config file (see below).

Not built yet: icon and compact views, tabs, split panes, bookmarks, thumbnails, device management, and an empty-file menu item (the backend exists). The project roadmap is in [TODO.md](TODO.md).

## Layout

```
crates/
  core/         UI-independent types and rules (entries, sorting)
  config/       User configuration (TOML, XDG config directory)
  filesystem/   Filesystem backend (directory listing)
  operations/   File operations (create, rename, copy, move, trash)
  session/      Window state without a GUI: history, clipboard, sorting, places
  application/  Terminal listing (`nexusfm` binary)
  qt/           Qt Quick window (`nexusfm-qt` binary): Rust backend in
                src/backend.rs, interface in qml/
assets/         Application icon
```

The core crates have no Qt dependency. The Qt window talks to `nexusfm-session` through a `cxx-qt` bridge, and the interface itself is written in QML.

## Requirements

- Rust 1.85 or newer (the workspace uses edition 2024)
- GLib and GIO 2.x development files (`pkg-config gio-2.0`); folders, file operations and properties all go through GIO
- GVFS, for remote and virtual locations such as `smb://`, `sftp://` and `trash:///`
- Qt 6 development files: Qt Core, Gui, Qml, Quick and QuickControls2 (Qt 6.5 or newer), with `qmake6` on the PATH or `QMAKE` set to its path

## Build and run

Run the terminal listing:

```
cargo run -p nexusfm -- /path/to/directory
```

Without a path, it lists `$HOME`.

Run the Qt window:

```
cargo run -p nexusfm-qt -- /path/to/directory
```

Without a path, the window opens in `$HOME`.

Run the tests and lints:

```
cargo test --workspace
cargo clippy --workspace --all-targets
```

## Desktop entry and icon

The window icon in the taskbar and task switcher comes from a desktop entry. To install it for your user:

```
cargo install --path crates/qt
mkdir -p ~/.local/share/applications ~/.local/share/icons/hicolor/512x512/apps
cp packaging/nexusfm.desktop ~/.local/share/applications/
cp assets/icon.png ~/.local/share/icons/hicolor/512x512/apps/nexusfm.png
```

The entry runs `nexusfm-qt`, so the binary must be on your `PATH` (`cargo install` puts it in `~/.cargo/bin`).

## Configuration

Settings are a Lua script, `config.lua`, in the NexusFM config folder: `$XDG_CONFIG_HOME/nexusfm/`, or `~/.config/nexusfm/` if `XDG_CONFIG_HOME` is not set. The script must return a table:

```lua
return {
  show_hidden = false,
  view = "detailed",        -- "detailed", "compact" or "icon"
  sort = {
    key = "name",           -- "name", "size" or "modified"
    descending = false,
  },
}
```

Settings you leave out keep their defaults, and unknown keys are ignored. Without a `config.lua`, the defaults are used. The script can use the math, string, table and UTF-8 libraries. It cannot read or write files, run programs, or load other code, and it is stopped after two seconds.

The Qt window saves its settings to `config.lua` when you change the sort order, the view, or hidden files. Comments and custom code in the file are replaced at that point, so keep anything you want to keep in a separate file. The first time the app rewrites your file, the original is copied to `config.lua.bak`, and that copy is not overwritten by later saves.

If a `config.lua` has an error, a warning is logged and the defaults are used. Earlier versions read `config.toml`; that file is no longer read. If it is present without a `config.lua`, a warning asks you to move its settings.

## Logging

Set `RUST_LOG` to control log output from the terminal listing, for example `RUST_LOG=debug`. Log output goes to stderr.

## License

NexusFM is free software, licensed under the GNU General Public License, version 3. See [LICENSE](LICENSE) for the license text and [NOTICE](NOTICE) for the copyright notice.
