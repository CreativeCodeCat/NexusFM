# NexusFM

A lightweight file manager written in Rust, with a Qt 6 Widgets interface.

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
  ffi/          C interface over the Rust crates, used by the Qt window
  application/  Terminal listing (`nexusfm` binary)
  qt/           Launches the Qt window (`nexusfm-qt` binary)
qt/             Qt 6 Widgets frontend (C++, built with CMake)
assets/         Application icon
```

The core crates have no Qt dependency. The Qt window calls the Rust crates through `crates/ffi`.

## Requirements

- Rust 1.85 or newer (the workspace uses edition 2024)
- CMake 3.21 or newer, a C++17 compiler, and Qt 6 Widgets development files (Qt 6.5 or newer)

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

The first build also runs CMake to compile the Qt frontend. Without a path, the window opens in `$HOME`.

To build the Qt frontend directly with CMake:

```
cmake -S qt -B qt/build
cmake --build qt/build
./qt/build/nexusfm-qt /path/to/directory
```

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
