// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! File operations for NexusFM.
//!
//! Operations never overwrite existing files: a destination that already exists
//! is an error, so the caller decides what to do. Symbolic links are copied as
//! links, not followed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors from file operations. Each carries the path it concerns.
#[derive(Debug, Error)]
pub enum OperationError {
    #[error("{} already exists", .0.display())]
    AlreadyExists(PathBuf),
    #[error("{} does not exist", .0.display())]
    NotFound(PathBuf),
    #[error("\"{0}\" is not a valid name")]
    InvalidName(String),
    #[error("cannot move or copy {} into itself", .0.display())]
    IntoItself(PathBuf),
    #[error("cannot move {} to the trash: {reason}", path.display())]
    Trash { path: PathBuf, reason: String },
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub type Result<T> = std::result::Result<T, OperationError>;

fn io_error(path: &Path) -> impl FnOnce(io::Error) -> OperationError + '_ {
    move |source| OperationError::Io {
        path: path.to_path_buf(),
        source,
    }
}

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

/// Creates a folder called `name` inside `parent`.
pub fn create_directory(parent: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let path = parent.join(name);
    fs::create_dir(&path).map_err(|err| match err.kind() {
        io::ErrorKind::AlreadyExists => OperationError::AlreadyExists(path.clone()),
        _ => io_error(&path)(err),
    })?;
    Ok(path)
}

/// Creates an empty file called `name` inside `parent`.
pub fn create_empty_file(parent: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name)?;
    let path = parent.join(name);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|err| match err.kind() {
            io::ErrorKind::AlreadyExists => OperationError::AlreadyExists(path.clone()),
            _ => io_error(&path)(err),
        })?;
    Ok(path)
}

/// Renames `path` to `new_name` in the same directory.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf> {
    validate_name(new_name)?;
    if !exists(path) {
        return Err(OperationError::NotFound(path.to_path_buf()));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("/"));
    let target = parent.join(new_name);
    if target == path {
        return Ok(target);
    }
    if exists(&target) {
        return Err(OperationError::AlreadyExists(target));
    }
    fs::rename(path, &target).map_err(io_error(path))?;
    Ok(target)
}

/// Copies `src` into the directory `dest_dir`, keeping its name. Folders are
/// copied recursively.
pub fn copy(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let target = destination_for(src, dest_dir)?;
    copy_tree(src, &target)?;
    Ok(target)
}

/// Moves `src` into the directory `dest_dir`, keeping its name. Falls back to
/// copy-then-delete when the move crosses filesystems.
pub fn move_into(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    let target = destination_for(src, dest_dir)?;
    match fs::rename(src, &target) {
        Ok(()) => Ok(target),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            copy_tree(src, &target)?;
            remove_tree(src)?;
            Ok(target)
        }
        Err(err) => Err(io_error(src)(err)),
    }
}

/// Moves `path` to the desktop trash.
pub fn move_to_trash(path: &Path) -> Result<()> {
    if !exists(path) {
        return Err(OperationError::NotFound(path.to_path_buf()));
    }
    trash::delete(path).map_err(|err| OperationError::Trash {
        path: path.to_path_buf(),
        reason: err.to_string(),
    })
}

/// Checks that `src` can go into `dest_dir` and returns the target path.
fn destination_for(src: &Path, dest_dir: &Path) -> Result<PathBuf> {
    if !exists(src) {
        return Err(OperationError::NotFound(src.to_path_buf()));
    }
    if !dest_dir.is_dir() {
        return Err(OperationError::NotFound(dest_dir.to_path_buf()));
    }
    let name = src
        .file_name()
        .ok_or_else(|| OperationError::InvalidName(src.display().to_string()))?;

    let src_meta = fs::symlink_metadata(src).map_err(io_error(src))?;
    if src_meta.is_dir() {
        let canonical_src = src.canonicalize().map_err(io_error(src))?;
        let canonical_dest = dest_dir.canonicalize().map_err(io_error(dest_dir))?;
        if canonical_dest.starts_with(&canonical_src) {
            return Err(OperationError::IntoItself(src.to_path_buf()));
        }
    }

    let target = dest_dir.join(name);
    if exists(&target) {
        return Err(OperationError::AlreadyExists(target));
    }
    Ok(target)
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn copy_tree(src: &Path, dest: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(src).map_err(io_error(src))?;
    let file_type = meta.file_type();

    if file_type.is_symlink() {
        let link = fs::read_link(src).map_err(io_error(src))?;
        return create_symlink(&link, dest).map_err(io_error(dest));
    }

    if file_type.is_dir() {
        fs::create_dir(dest).map_err(io_error(dest))?;
        for item in fs::read_dir(src).map_err(io_error(src))? {
            let item = item.map_err(io_error(src))?;
            copy_tree(&item.path(), &dest.join(item.file_name()))?;
        }
        fs::set_permissions(dest, meta.permissions()).map_err(io_error(dest))?;
        return Ok(());
    }

    fs::copy(src, dest).map_err(io_error(src))?;
    Ok(())
}

fn remove_tree(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(io_error(path))?;
    if meta.is_dir() {
        fs::remove_dir_all(path).map_err(io_error(path))
    } else {
        fs::remove_file(path).map_err(io_error(path))
    }
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn create_symlink(target: &Path, link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "cannot copy link {} -> {}",
            link.display(),
            target.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
