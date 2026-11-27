# NexusFM

A lightweight, Nemo-inspired file manager written in Rust, with a Qt 6 Widgets interface.

## Status

Base project. Working today:

- `nexusfm-core`: entry types and sorting rules (directories first, case-insensitive names).
- `nexusfm-filesystem`: directory listing with `std::fs`. Sits behind `list_directory` so GIO/GVFS can replace it later.
- `nexusfm`: terminal listing of a directory, as a stand-in for the window.

Planned, not started: the Qt 6 Widgets front end (via `cxx-qt`), tabs and split panes, bookmarks, file operations, thumbnails, and device management.

## Layout

```
crates/
  core/         UI-independent types and rules
  filesystem/   Filesystem backend
  application/  Binary entry point
```

## Build and run

```
cargo test
cargo run -p nexusfm -- /path/to/directory
```

Without a path, it lists `$HOME`.
