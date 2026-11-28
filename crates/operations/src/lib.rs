// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! File operations for NexusFM.
//!
//! Operations never overwrite existing files: a destination that already exists
//! is an error, so the caller decides what to do. Symbolic links are copied as
//! links, not followed.

use std::path::{Path, PathBuf};

use gio::prelude::*;
use nexusfm_core::NexusError as OperationError;

pub type Result<T> = nexusfm_core::Result<T>;

/// Attributes read when deciding what kind of object a source is.
const TYPE_ATTRIBUTES: &str = "standard::type";

/// Checks that `name` can be used as a single file name in a directory.
pub fn validate_name(name: &str) -> Result<()> {
    let invalid =
        name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0');
    if invalid {
        Err(OperationError::InvalidName(name.to_string()))
    } else {
        Ok(())
    }
}

/// Creates a folder called `name` inside `parent`. `parent` may be a local
/// path or a URI.
pub fn create_directory(parent: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let target = file(parent).child(name);
    target
        .make_directory(None::<&gio::Cancellable>)
        .map_err(|err| glib_error(&location_of(&target), err))?;
    Ok(location_of(&target))
}

/// Creates an empty file called `name` inside `parent`.
pub fn create_empty_file(parent: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let target = file(parent).child(name);
    let stream = target
        .create(gio::FileCreateFlags::NONE, None::<&gio::Cancellable>)
        .map_err(|err| glib_error(&location_of(&target), err))?;
    let _ = stream.close(None::<&gio::Cancellable>);
    Ok(location_of(&target))
}

/// Renames `path` to `new_name` in the same folder.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf> {
    validate_name(new_name)?;
    let source = file(path);
    if !source.query_exists(None::<&gio::Cancellable>) {
        return Err(OperationError::NotFound(path.to_path_buf()));
    }
    let Some(parent) = source.parent() else {
        return Err(OperationError::InvalidName(new_name.to_string()));
    };
    let target = parent.child(new_name);
    if location_of(&target) == path {
        return Ok(location_of(&target));
    }
    if target.query_exists(None::<&gio::Cancellable>) {
        return Err(OperationError::AlreadyExists(location_of(&target)));
    }
    source
        .move_(
            &target,
            gio::FileCopyFlags::NOFOLLOW_SYMLINKS,
            None::<&gio::Cancellable>,
            None::<&mut dyn FnMut(i64, i64)>,
        )
        .map_err(|err| glib_error(path, err))?;
    Ok(location_of(&target))
}

/// Copies `src` into the folder `dest_dir`, keeping its name. Folders are
/// copied recursively. Symbolic links are copied as links.
pub fn copy(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let (source, target) = destination_for(src, dest_dir)?;
    copy_tree(&source, &target)?;
    Ok(location_of(&target))
}

/// Moves `src` into the folder `dest_dir`, keeping its name. GIO falls back to
/// copy-then-delete when the move crosses filesystems.
pub fn move_into(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let (source, target) = destination_for(src, dest_dir)?;
    source
        .move_(
            &target,
            gio::FileCopyFlags::NOFOLLOW_SYMLINKS,
            None::<&gio::Cancellable>,
            None::<&mut dyn FnMut(i64, i64)>,
        )
        .map_err(|err| glib_error(src, err))?;
    Ok(location_of(&target))
}

/// Moves `path` to the desktop trash.
pub fn move_to_trash(path: &Path) -> Result<()> {
    let target = file(path);
    if !target.query_exists(None::<&gio::Cancellable>) {
        return Err(OperationError::NotFound(path.to_path_buf()));
    }
    target
        .trash(None::<&gio::Cancellable>)
        .map_err(|err| OperationError::Trash {
            path: path.to_path_buf(),
            reason: err.message().to_string(),
        })
}

/// A GIO handle for a local path or a URI.
fn file(path: &Path) -> gio::File {
    if nexusfm_core::location::is_uri(&path.to_string_lossy()) {
        gio::File::for_uri(&path.to_string_lossy())
    } else {
        gio::File::for_path(path)
    }
}

/// The path or URI to show for a GIO file.
fn location_of(file: &gio::File) -> PathBuf {
    file.path()
        .unwrap_or_else(|| PathBuf::from(file.uri().as_str()))
}

/// Turns a GIO error into the matching NexusFM error for `path`.
fn glib_error(path: &Path, err: gio::glib::Error) -> OperationError {
    if err.matches(gio::IOErrorEnum::Exists) {
        OperationError::AlreadyExists(path.to_path_buf())
    } else if err.matches(gio::IOErrorEnum::NotFound) {
        OperationError::NotFound(path.to_path_buf())
    } else {
        OperationError::Message(format!("{}: {}", path.display(), err.message()))
    }
}

/// Checks that `src` can go into `dest_dir` and returns the source and target files.
fn destination_for(src: &Path, dest_dir: &Path) -> Result<(gio::File, gio::File)> {
    let source = file(src);
    if !source.query_exists(None::<&gio::Cancellable>) {
        return Err(OperationError::NotFound(src.to_path_buf()));
    }
    let folder = file(dest_dir);
    let is_folder = folder.query_file_type(
        gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
        None::<&gio::Cancellable>,
    ) == gio::FileType::Directory;
    if !is_folder {
        return Err(OperationError::NotFound(dest_dir.to_path_buf()));
    }
    let name = source
        .basename()
        .ok_or_else(|| OperationError::InvalidName(src.display().to_string()))?;

    let is_dir = source.query_file_type(
        gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
        None::<&gio::Cancellable>,
    ) == gio::FileType::Directory;
    if is_dir && is_inside(dest_dir, src) {
        return Err(OperationError::IntoItself(src.to_path_buf()));
    }

    let target = folder.child(name);
    if target.query_exists(None::<&gio::Cancellable>) {
        return Err(OperationError::AlreadyExists(location_of(&target)));
    }
    Ok((source, target))
}

/// Whether `dest` is `src` or lies inside it. Local paths are compared after
/// resolving links; URIs are compared as written.
fn is_inside(dest: &Path, src: &Path) -> bool {
    if nexusfm_core::location::is_uri(&src.to_string_lossy()) {
        return dest.starts_with(src);
    }
    match (src.canonicalize(), dest.canonicalize()) {
        (Ok(src), Ok(dest)) => dest.starts_with(src),
        _ => false,
    }
}

/// Copies a file, symbolic link or folder tree from `src` to `dest`, which must not exist.
fn copy_tree(src: &gio::File, dest: &gio::File) -> Result<()> {
    let kind = src
        .query_info(
            TYPE_ATTRIBUTES,
            gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
            None::<&gio::Cancellable>,
        )
        .map_err(|err| glib_error(&location_of(src), err))?
        .file_type();

    if kind == gio::FileType::Directory {
        dest.make_directory(None::<&gio::Cancellable>)
            .map_err(|err| glib_error(&location_of(dest), err))?;
        let children = src
            .enumerate_children(
                "standard::name",
                gio::FileQueryInfoFlags::NOFOLLOW_SYMLINKS,
                None::<&gio::Cancellable>,
            )
            .map_err(|err| glib_error(&location_of(src), err))?;
        for child in children {
            let child = child.map_err(|err| glib_error(&location_of(src), err))?;
            let name = child.name();
            copy_tree(&src.child(&name), &dest.child(&name))?;
        }
        return Ok(());
    }

    src.copy(
        dest,
        gio::FileCopyFlags::NOFOLLOW_SYMLINKS,
        None::<&gio::Cancellable>,
        None::<&mut dyn FnMut(i64, i64)>,
    )
    .map_err(|err| glib_error(&location_of(src), err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_bad_names() {
        for name in ["", ".", "..", "a/b", "x\0y"] {
            assert!(validate_name(name).is_err(), "{name:?} should be rejected");
        }
        assert!(validate_name("notes.txt").is_ok());
    }

    #[test]
    fn creates_directory_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let folder = create_directory(dir.path(), "new").unwrap();
        assert!(folder.is_dir());
        let file = create_empty_file(dir.path(), "empty.txt").unwrap();
        assert_eq!(fs::metadata(&file).unwrap().len(), 0);
    }

    #[test]
    fn create_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        create_directory(dir.path(), "x").unwrap();
        assert!(matches!(
            create_directory(dir.path(), "x"),
            Err(OperationError::AlreadyExists(_))
        ));
        fs::write(dir.path().join("f"), b"keep").unwrap();
        assert!(matches!(
            create_empty_file(dir.path(), "f"),
            Err(OperationError::AlreadyExists(_))
        ));
        assert_eq!(fs::read(dir.path().join("f")).unwrap(), b"keep");
    }

    #[test]
    fn renames_within_directory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, b"hi").unwrap();
        let renamed = rename(&file, "b.txt").unwrap();
        assert_eq!(renamed, dir.path().join("b.txt"));
        assert!(!file.exists());
        assert_eq!(fs::read(&renamed).unwrap(), b"hi");
    }

    #[test]
    fn rename_refuses_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        fs::write(&a, b"1").unwrap();
        fs::write(dir.path().join("b"), b"2").unwrap();
        assert!(matches!(
            rename(&a, "b"),
            Err(OperationError::AlreadyExists(_))
        ));
        assert_eq!(fs::read(dir.path().join("b")).unwrap(), b"2");
    }

    #[test]
    fn copies_folders_recursively() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(src.join("inner")).unwrap();
        fs::write(src.join("inner/deep.txt"), b"deep").unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let copied = copy(&src, &dest).unwrap();
        assert_eq!(fs::read(copied.join("inner/deep.txt")).unwrap(), b"deep");
        assert!(src.join("inner/deep.txt").exists());
    }

    #[test]
    fn copy_refuses_existing_target() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f");
        fs::write(&src, b"new").unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        fs::write(dest.join("f"), b"old").unwrap();

        assert!(matches!(
            copy(&src, &dest),
            Err(OperationError::AlreadyExists(_))
        ));
        assert_eq!(fs::read(dest.join("f")).unwrap(), b"old");
    }

    #[test]
    fn moves_file_into_directory() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("f");
        fs::write(&src, b"data").unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let moved = move_into(&src, &dest).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read(moved).unwrap(), b"data");
    }

    #[test]
    fn refuses_to_move_folder_into_itself() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let inner = src.join("inner");
        fs::create_dir_all(&inner).unwrap();
        assert!(matches!(
            move_into(&src, &inner),
            Err(OperationError::IntoItself(_))
        ));
        assert!(inner.is_dir());
    }

    #[test]
    fn copies_symlinks_as_links() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        fs::write(&target, b"x").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let dest = dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let copied = copy(&link, &dest).unwrap();
        assert!(
            fs::symlink_metadata(&copied)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn missing_source_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            copy(&dir.path().join("nope"), dir.path()),
            Err(OperationError::NotFound(_))
        ));
    }
}
