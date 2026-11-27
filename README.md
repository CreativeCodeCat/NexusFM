# NexusFM

A lightweight file manager written in Rust, with a Qt 6 Quick (QML) interface.

## Features

- Browse directories in a detailed file list. Click a column header to sort; click again to reverse.
- Navigate with Back, Forward, Up, Home and Reload, or type a path in the location bar.
- Places sidebar with Home, Desktop, Documents, Downloads, Music, Pictures, Videos and the file system root.
- Show or hide hidden files (View menu or `Ctrl+H`).
- Open folders in the window; open files with their default application.
- File operations from the Edit menu and right-click menu: New Folder, Cut, Copy, Paste, Rename (`F2`) and Move to Trash (`Delete`). Existing files are never overwritten.
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

## Configuration

Settings are read from `$XDG_CONFIG_HOME/nexusfm/config.toml`, or `~/.config/nexusfm/config.toml` if `XDG_CONFIG_HOME` is not set.

```toml
show_hidden = false

[sort]
key = "name"        # "name", "size" or "modified"
descending = false
```

A missing file uses the defaults. An invalid file is reported and the defaults are used. Unknown keys are ignored.

## Logging

Set `RUST_LOG` to control log output from the terminal listing, for example `RUST_LOG=debug`. Log output goes to stderr.

## License

NexusFM is free software, licensed under the GNU General Public License, version 3. See [LICENSE](LICENSE) for the license text and [NOTICE](NOTICE) for the copyright notice.
