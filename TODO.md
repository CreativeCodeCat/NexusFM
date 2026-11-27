# TODO

Baseline development roadmap for **NexusFM**, a Nemo-inspired Linux desktop file manager written in Rust with a Qt 6 Widgets frontend.

The goal is to achieve as much feature parity with Linux Mint's Nemo as possible, preserving its traditional desktop workflow, extensive functionality and customisation while maintaining a modular, efficient Rust architecture.

Items are grouped by subsystem and roughly ordered by implementation dependencies.

## 1. Foundation

- [x] Entry types and sorting rules (directories first, case-insensitive names) in `nexusfm-core`
- [x] Directory listing behind `list_directory` in `nexusfm-filesystem`
- [x] Terminal listing stand-in in the `nexusfm` binary
- [ ] Shared error types and user-facing error reporting across crates
- [x] Configuration loading from TOML using XDG config directories
- [x] Logging and diagnostics using `tracing`
- [ ] Replace `std::fs` backend with GIO/GVFS
- [ ] Preserve `list_directory` as the filesystem abstraction boundary
- [ ] Filesystem metadata abstraction
- [ ] MIME type detection
- [ ] Default application lookup
- [ ] File monitoring and automatic directory refresh
- [ ] Filesystem change notifications
- [ ] Symlink handling
- [ ] Hidden file handling
- [ ] Permissions and ownership metadata
- [ ] URI-based location abstraction
- [ ] Support local, remote and virtual filesystem locations

## 2. Qt 6 Widgets Frontend

### Main Window

- [ ] `qt-bridge` crate using `cxx-qt`
- [x] CMake project under `qt/` that builds and links the Rust core
- [x] Main application window
- [x] Startup directory from CLI argument or `$HOME`
- [ ] Traditional menu bar (File, Edit, View, Go, Bookmarks, Help)
- [x] Navigation toolbar (back, forward, up, reload)
- [ ] Breadcrumb location bar
- [x] Editable location entry
- [ ] Search button
- [ ] Sidebar toggle
- [ ] Split-view toggle
- [x] Main file view backed by a Qt model over the core listing
- [x] Status bar
- [x] Application icon from `assets/icon.png`
- [ ] Window size and position persistence
- [ ] Remember sidebar width and visibility
- [ ] Remember toolbar and status bar visibility
- [ ] Multiple application windows
- [ ] Window title reflecting the current location

### Appearance and Customisation

- [ ] Traditional desktop application appearance inspired by Nemo
- [ ] Support system Qt themes
- [ ] Light and dark themes
- [ ] Configurable icon sizes
- [ ] Configurable toolbar icon sizes
- [ ] Configurable interface density
- [ ] Custom stylesheet support
- [ ] Native Qt menus and dialogs
- [ ] Configurable fonts
- [ ] High-DPI support
- [ ] Keyboard navigation throughout the interface

## 3. File Views

### Icon View

- [ ] Grid-based icon layout
- [ ] Configurable icon sizes
- [ ] File and folder names underneath icons
- [ ] Thumbnail previews
- [ ] Selection highlighting
- [ ] Automatic icon positioning
- [ ] Configurable grid spacing
- [ ] Zoom in and zoom out
- [ ] Arrange icons by name, size, type and modification date
- [ ] Reverse sorting
- [ ] Directories first
- [ ] Optional manual icon arrangement

### Detailed List View

- [ ] Detailed list view backed by a Qt model
- [x] Name column
- [x] Size column
- [x] Type column
- [x] Modified column
- [ ] Permissions column
- [ ] Owner column
- [ ] Configurable visible columns
- [ ] Column resizing and reordering
- [x] Click-to-sort columns
- [ ] Directories first
- [ ] Expandable directory rows where appropriate
- [ ] Alternating row colours
- [ ] Selection highlighting

### Compact View

- [ ] Compact list layout
- [ ] Configurable icon sizes
- [ ] Efficient use of horizontal space
- [ ] Selection highlighting
- [ ] Sorting support

### View Behaviour

- [ ] Icon, detailed and compact view switching
- [ ] Per-directory view preferences
- [ ] Global default view preference
- [ ] Remember zoom level
- [ ] Remember sorting preferences
- [x] Show and hide hidden files
- [ ] Configurable default sorting
- [ ] Configurable directory-first sorting
- [ ] Automatic refresh when filesystem changes

## 4. Navigation

- [x] Open directories and files
- [x] Open files using their default applications
- [x] Back and forward navigation
- [x] Parent directory navigation
- [ ] Breadcrumb navigation
- [x] Editable location entry
- [ ] Tabs
- [ ] Tab duplication
- [ ] Tab reordering
- [ ] Tab closing
- [ ] Restore previously open tabs
- [ ] Tab context menus
- [ ] Split-pane browsing
- [ ] Independent navigation history per pane
- [ ] Independent view modes per pane
- [ ] Synchronised scrolling where appropriate
- [ ] Drag and drop between panes
- [ ] Keyboard shortcuts for common actions
- [ ] Type-to-select / incremental search
- [ ] Recursive search
- [ ] Search results displayed as a file view
- [ ] Search filters (name, type, size and modification date)
- [ ] Search within the current directory or recursively
- [ ] Recent locations history
- [ ] Recently accessed files
- [ ] Configurable navigation behaviour

## 5. Sidebar

### Places

- [x] Home
- [ ] Desktop
- [x] Documents
- [x] Downloads
- [x] Music
- [x] Pictures
- [x] Videos
- [ ] Trash
- [x] File system root
- [ ] Recently used files

### Devices

- [ ] Mounted volumes
- [ ] Removable drives
- [ ] Optical drives
- [ ] USB devices
- [ ] Network locations
- [ ] Mount and unmount actions
- [ ] Eject and safely remove actions
- [ ] Device capacity indicators
- [ ] Device context menus

### Bookmarks

- [ ] User-defined bookmarks
- [ ] Add and remove bookmarks
- [ ] Reorder bookmarks
- [ ] Rename bookmarks
- [ ] Bookmark context menus
- [ ] Bookmark persistence
- [ ] Drag directories into the sidebar to create bookmarks
- [ ] Bookmark separators
- [ ] Bookmark icons

### Sidebar Behaviour

- [ ] Resizable sidebar
- [ ] Collapsible sidebar
- [ ] Drag-and-drop bookmark management
- [ ] Context menus for sidebar entries
- [ ] Automatic device updates
- [ ] Configurable sidebar sections
- [ ] Persist sidebar state

## 6. Selection and Context Menus

### Selection

- [ ] Single selection
- [ ] Multiple selection
- [ ] Range selection
- [ ] Toggle selection
- [ ] Select all
- [ ] Invert selection
- [ ] Select by pattern
- [ ] Selection persistence where appropriate
- [ ] Keyboard selection navigation

### Context Menus

- [ ] Context menu for files
- [ ] Context menu for directories
- [ ] Context menu for multiple selections
- [ ] Context menu for empty space
- [ ] Context menu for sidebar entries
- [ ] Context menu for tabs
- [ ] Open
- [ ] Open with
- [x] Cut, copy and paste
- [x] Rename
- [x] Move to trash
- [ ] Delete permanently
- [x] Create folder
- [ ] Create document
- [ ] Compress
- [ ] Extract
- [ ] Send to
- [ ] Create link
- [ ] Properties
- [ ] Custom script actions
- [ ] Extension-provided actions

### File Properties

- [ ] Properties dialog
- [ ] General information
- [ ] File name
- [ ] File type
- [ ] MIME type
- [ ] File size and allocated size
- [ ] Location
- [ ] Created, modified and accessed timestamps where available
- [ ] Permissions
- [ ] Owner and group
- [ ] Executable status
- [ ] Symlink target
- [ ] Parent directory information
- [ ] Emblems and custom icons
- [ ] Open-with application configuration

## 7. File Operations (`operations` crate)

### Basic Operations

- [x] Copy
- [x] Move
- [x] Rename
- [x] Delete to trash
- [ ] Permanent deletion
- [x] Create folder
- [ ] Create empty file
- [ ] Create symbolic link
- [ ] Create hard link where supported
- [ ] Copy and move between tabs
- [ ] Copy and move between split panes
- [ ] Drag-and-drop file operations

### Advanced Operations

- [ ] Bulk rename
- [ ] Rename using templates
- [ ] Rename using sequential numbering
- [ ] Rename using find and replace
- [ ] Rename using regular expressions
- [ ] Compress files and folders
- [ ] Extract archives
- [ ] Support common archive formats
- [ ] Archive creation options
- [ ] Archive extraction options
- [ ] Calculate directory sizes
- [ ] Recursive directory size calculation

### Operation Management

- [ ] Asynchronous file operations
- [ ] Progress reporting
- [ ] Cancellation
- [ ] Multiple simultaneous operations
- [ ] Operation queue
- [ ] Pause and resume where supported
- [ ] Conflict handling (skip, overwrite, rename)
- [ ] Confirmation dialogs
- [ ] Error reporting
- [ ] Partial operation recovery
- [ ] Undo rename
- [ ] Undo move
- [ ] Undo trash
- [ ] Operation history
- [ ] Background operation notifications

### Permissions

- [ ] Change file permissions
- [ ] Change directory permissions
- [ ] Recursive permission changes
- [ ] Change ownership
- [ ] Change group
- [ ] Permission presets
- [ ] Execute permission controls
- [ ] Permission error handling

## 8. Bookmarks (`bookmarks` crate)

- [ ] Add bookmarks
- [ ] Remove bookmarks
- [ ] Rename bookmarks
- [ ] Reorder bookmarks
- [ ] Persist bookmarks in the XDG config directory
- [ ] Display bookmarks in the sidebar
- [ ] Bookmark icons
- [ ] Bookmark separators
- [ ] Import and export bookmarks
- [ ] Support freedesktop GTK bookmarks file where appropriate
- [ ] Detect unavailable bookmark locations

## 9. Thumbnails and Previews (`thumbnails` crate)

- [ ] Image previews
- [ ] Video thumbnails
- [ ] PDF thumbnails
- [ ] Audio thumbnails where supported
- [ ] Thumbnail generation in background
- [ ] On-disk thumbnail cache following the freedesktop thumbnail specification
- [ ] Thumbnail cache invalidation
- [ ] Thumbnail size limits
- [ ] Configurable thumbnail sizes
- [ ] Toggle previews on and off
- [ ] Respect thumbnail generation preferences
- [ ] Avoid blocking the UI during thumbnail generation
- [ ] Handle unsupported file formats gracefully

## 10. Emblems and Customisation

- [ ] File emblems
- [ ] Directory emblems
- [ ] Built-in emblem collection
- [ ] Custom emblem support
- [ ] Apply emblems from context menus
- [ ] Remove emblems
- [ ] Persist emblem assignments
- [ ] Custom folder icons
- [ ] Custom file icons where supported
- [ ] Configurable icon themes
- [ ] Freedesktop icon theme support
- [ ] File type-specific icons

## 11. Desktop and Application Integration

- [ ] Open files using default applications
- [ ] Open with application selection dialog
- [ ] Set default applications
- [ ] Remember application associations
- [ ] MIME type associations
- [ ] Desktop entry file
- [ ] Application icon installation
- [ ] Application launcher integration
- [ ] D-Bus application integration
- [ ] Single-instance support where appropriate
- [ ] Command-line interface
- [ ] Open specified locations from CLI
- [ ] Open new windows from CLI
- [ ] Support file manager D-Bus integration where applicable
- [ ] Desktop notifications for long-running operations
- [ ] Trash integration following freedesktop specifications
- [ ] Respect XDG user directories

## 12. Devices and Mounts

- [ ] Enumerate mounted volumes
- [ ] Enumerate removable volumes
- [ ] Mount volumes
- [ ] Unmount volumes
- [ ] Eject removable media
- [ ] Safely remove devices
- [ ] Monitor device addition and removal
- [ ] React to mount and unmount events
- [ ] D-Bus integration using `zbus`
- [ ] UDisks2 integration
- [ ] Display volume labels
- [ ] Display filesystem types
- [ ] Display free and total space
- [ ] Mount error reporting
- [ ] Automatic sidebar updates
- [ ] Support GVFS-mounted devices

## 13. Network and Remote Locations

- [ ] GVFS network integration
- [ ] Browse network locations
- [ ] SMB/CIFS shares
- [ ] SFTP locations
- [ ] FTP locations where supported by GVFS
- [ ] WebDAV locations where supported
- [ ] Connect to server dialog
- [ ] Enter server address manually
- [ ] Browse network shares
- [ ] Save remote locations
- [ ] Disconnect remote locations
- [ ] Display remote locations in sidebar
- [ ] Remote file operations
- [ ] Remote file monitoring where supported
- [ ] Network connection error reporting
- [ ] Handle authentication prompts
- [ ] Credential storage through the desktop secret service

## 14. Scripts and Extensions

### Scripts

- [ ] User-defined scripts directory
- [ ] Discover available scripts automatically
- [ ] Scripts submenu in context menu
- [ ] Execute scripts on selected files
- [ ] Execute scripts on selected directories
- [ ] Pass selected file paths as arguments
- [ ] Pass current directory as working directory
- [ ] Environment variables for scripts
- [ ] Script execution status
- [ ] Script output handling
- [ ] Script error reporting
- [ ] Script permissions and validation

### Extensions

- [ ] Extension architecture
- [ ] Extension discovery
- [ ] Enable and disable extensions
- [ ] Extension configuration
- [ ] Context menu extension hooks
- [ ] Toolbar extension hooks
- [ ] Menu extension hooks
- [ ] Sidebar extension hooks
- [ ] File properties extension hooks
- [ ] Custom actions
- [ ] Extension lifecycle management
- [ ] Extension API documentation
- [ ] Extension examples
- [ ] Extension compatibility checks
- [ ] Extension security considerations

## 15. Archive Management

- [ ] Archive creation
- [ ] Archive extraction
- [ ] ZIP support
- [ ] TAR support
- [ ] GZIP support
- [ ] BZIP2 support
- [ ] XZ support
- [ ] 7-Zip support where available
- [ ] Create compressed archives from selections
- [ ] Extract archives into current directory
- [ ] Extract archives into a new directory
- [ ] Archive progress reporting
- [ ] Archive conflict handling
- [ ] Archive context menu integration
- [ ] Archive MIME type detection

## 16. Search

- [ ] Search within current directory
- [ ] Recursive search
- [ ] Search by filename
- [ ] Search by file extension
- [ ] Search by MIME type
- [ ] Search by size
- [ ] Search by modification date
- [ ] Search by file content where supported
- [ ] Case-sensitive search
- [ ] Regular expression search
- [ ] Search result filtering
- [ ] Search result sorting
- [ ] Search result navigation
- [ ] Open containing directory
- [ ] Search cancellation
- [ ] Background search execution
- [ ] Search progress reporting

## 17. Recent Files and History

- [ ] Recently accessed files
- [ ] Recently visited directories
- [ ] Recent files sidebar entry
- [ ] Recent locations history
- [ ] Clear recent history
- [ ] Persist navigation history where appropriate
- [ ] Restore previous session
- [ ] Restore open tabs
- [ ] Restore split-pane state
- [ ] Configurable history limits
- [ ] Respect desktop recent-file privacy settings

## 18. User Experience and Accessibility

- [ ] Keyboard shortcuts
- [ ] Customisable keyboard shortcuts
- [ ] Full keyboard navigation
- [ ] Type-to-select
- [ ] Incremental search
- [ ] Configurable double-click or single-click opening
- [ ] Configurable confirmation dialogs
- [ ] Configurable deletion behaviour
- [ ] Configurable hidden file visibility
- [ ] Configurable default view
- [ ] Configurable default sorting
- [ ] Tooltips
- [ ] Accessible widget names and descriptions
- [ ] Screen reader support through Qt accessibility
- [ ] High-contrast theme compatibility
- [ ] Proper focus management
- [ ] Responsive interface during background operations

## 19. Performance and Reliability

- [ ] Asynchronous directory loading
- [ ] Lazy thumbnail generation
- [ ] Lazy metadata loading
- [ ] Efficient Qt item models
- [ ] Avoid unnecessary filesystem queries
- [ ] Cache directory metadata
- [ ] Cache MIME type information
- [ ] Efficient large-directory handling
- [ ] Virtualised file views where appropriate
- [ ] Background filesystem monitoring
- [ ] Memory usage optimisation
- [ ] Startup time optimisation
- [ ] Graceful handling of disconnected devices
- [ ] Graceful handling of inaccessible directories
- [ ] Robust handling of filename encoding
- [ ] Support filenames containing unusual characters
- [ ] Handle concurrent filesystem changes
- [ ] Avoid blocking the GUI thread

## 20. Testing and Quality

- [ ] Unit tests for core rules
- [ ] Unit tests for sorting
- [ ] Unit tests for filesystem metadata
- [x] Unit tests for configuration
- [ ] Integration tests for directory listing
- [ ] Integration tests for file operations using temporary directories
- [ ] Tests for copy, move, rename and delete
- [ ] Tests for symlinks
- [ ] Tests for hidden files
- [ ] Tests for permissions
- [ ] Tests for error handling
- [ ] Tests for concurrent filesystem changes
- [ ] Qt frontend integration tests
- [ ] GUI interaction tests
- [ ] Large-directory performance tests
- [ ] Memory usage tests
- [ ] CI running `cargo test`
- [ ] CI building the Qt frontend
- [ ] Formatting checks using `cargo fmt`
- [ ] Linting using `cargo clippy`

## 21. Packaging and Distribution

- [ ] Desktop entry file
- [ ] Application icon installation
- [ ] Installable application bundle
- [ ] Installation instructions
- [ ] Build instructions
- [ ] Runtime dependency documentation
- [ ] Packaging notes for Debian/Ubuntu
- [ ] Packaging notes for Fedora
- [ ] Packaging notes for Arch Linux
- [ ] AUR package
- [ ] Flatpak packaging
- [ ] AppImage packaging
- [ ] Version information
- [ ] Release notes
- [ ] Upgrade and configuration migration handling

## 22. Documentation

- [ ] README
- [ ] Installation guide
- [ ] Build instructions
- [ ] Architecture documentation
- [ ] Crate documentation
- [ ] Configuration reference
- [ ] Keyboard shortcut reference
- [ ] Extension development guide
- [ ] Script integration guide
- [ ] Contribution guidelines
- [ ] Troubleshooting guide
- [ ] Packaging documentation

---

## Development Priorities

### Phase 1 — Core Application

- [ ] Rust core and filesystem abstraction
- [ ] GIO/GVFS integration
- [x] Qt 6 Widgets frontend
- [x] Main window and navigation toolbar
- [ ] Sidebar and Places
- [ ] Icon, detailed and compact views
- [x] Basic file opening
- [x] Basic file operations
- [x] Status bar

### Phase 2 — Everyday File Management

- [ ] Tabs
- [ ] Split-pane browsing
- [ ] Back/forward navigation history
- [ ] Context menus
- [ ] File properties
- [ ] Bookmarks
- [ ] Drag and drop
- [ ] Thumbnails
- [ ] Hidden files
- [ ] Keyboard shortcuts
- [ ] Configuration and preferences

### Phase 3 — Advanced File Management

- [ ] Bulk renaming
- [ ] Archive management
- [ ] Search
- [ ] Emblems
- [ ] Custom icons
- [ ] File operation queue
- [ ] Undo support
- [ ] Recent files
- [ ] Advanced sorting and filtering

### Phase 4 — Desktop Integration

- [ ] Device management
- [ ] Mount and unmount support
- [ ] Network browsing
- [ ] Connect to server
- [ ] GVFS integration
- [ ] D-Bus integration
- [ ] Trash management
- [ ] Desktop notifications
- [ ] Application associations

### Phase 5 — Nemo-Style Extensibility

- [ ] User scripts
- [ ] Extension architecture
- [ ] Custom context menu actions
- [ ] Extension configuration
- [ ] Additional file properties
- [ ] Customisable toolbar actions
- [ ] Plugin documentation

### Phase 6 — Polish and Release

- [ ] Performance optimisation
- [ ] Accessibility
- [ ] Comprehensive testing
- [ ] Packaging
- [ ] Installation documentation
- [ ] Release candidate
- [ ] Stable 1.0 release

---

## Architectural Principles

- Keep the application core independent of Qt.
- Keep filesystem operations independent of the UI.
- Use GIO/GVFS rather than reimplementing Linux filesystem integration.
- Use asynchronous operations for potentially slow filesystem tasks.
- Keep the GUI responsive at all times.
- Prefer native Qt Widgets over custom-rendered interfaces.
- Follow freedesktop.org specifications wherever applicable.
- Keep configuration and user data within XDG directories.
- Design extensions to be isolated from the core application.
- Prioritise reliability, usability and low resource consumption.

**Primary goal:** Deliver a fully featured, Nemo-inspired Linux file manager with a familiar traditional desktop interface, implemented in Rust and Qt 6 Widgets.