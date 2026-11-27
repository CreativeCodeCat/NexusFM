//! C ABI over the NexusFM core, for the Qt Widgets frontend.
//!
//! The Qt side (`qt/`) includes `nexusfm_ffi.h` and links the static library
//! built from this crate. Everything here is a thin wrapper: listing, sorting
//! and configuration rules stay in the Rust crates.
//!
//! Ownership: a [`NfmListing`] is created by [`nfm_listing_open`] and must be
//! released with [`nfm_listing_free`]. Strings and entries read from a listing
//! stay valid until that listing is freed.

use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;
use std::ptr;
use std::time::UNIX_EPOCH;

use nexusfm_config::{Config, config_file_path};
use nexusfm_core::{Entry, EntryKind, SortKey, SortOrder, sort_entries};
use nexusfm_filesystem::list_directory;

/// Entry kinds as seen by C. Must match `NfmKind` in the header.
pub const KIND_FILE: u8 = 0;
pub const KIND_DIRECTORY: u8 = 1;
pub const KIND_SYMLINK: u8 = 2;
pub const KIND_OTHER: u8 = 3;

/// Sort keys as seen by C. Must match `NfmSortKey` in the header.
pub const SORT_NAME: u8 = 0;
pub const SORT_SIZE: u8 = 1;
pub const SORT_MODIFIED: u8 = 2;

/// Settings passed between Rust and C. Must match `NfmConfig` in the header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NfmConfig {
    pub show_hidden: bool,
    pub sort_key: u8,
    pub descending: bool,
}

/// One directory entry as seen by C. Must match `NfmEntry` in the header.
///
/// `name` and `path` point into the listing and are valid until it is freed.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NfmEntry {
    pub name: *const c_char,
    pub path: *const c_char,
    pub kind: u8,
    pub size: u64,
    pub has_modified: bool,
    /// Seconds since the Unix epoch. Only meaningful when `has_modified` is set.
    pub modified_unix: i64,
}

struct OwnedEntry {
    name: CString,
    path: CString,
    kind: u8,
    size: u64,
    modified_unix: Option<i64>,
}

/// A directory listing. Opaque to C.
pub struct NfmListing {
    error: Option<CString>,
    entries: Vec<OwnedEntry>,
}

fn c_string(text: &str) -> CString {
    // Interior NUL bytes cannot be represented in a C string; drop them.
    CString::new(text.replace('\0', "")).expect("NUL bytes were removed")
}

fn sort_key_from_c(value: u8) -> SortKey {
    match value {
        SORT_SIZE => SortKey::Size,
        SORT_MODIFIED => SortKey::Modified,
        _ => SortKey::Name,
    }
}

fn kind_to_c(kind: EntryKind) -> u8 {
    match kind {
        EntryKind::File => KIND_FILE,
        EntryKind::Directory => KIND_DIRECTORY,
        EntryKind::Symlink => KIND_SYMLINK,
        EntryKind::Other => KIND_OTHER,
    }
}

fn modified_to_unix(entry: &Entry) -> Option<i64> {
    let time = entry.modified?;
    Some(match time.duration_since(UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => -i64::try_from(before.duration().as_secs()).unwrap_or(i64::MAX),
    })
}

/// Reads the user configuration into `out`.
///
/// Missing or broken configuration yields the defaults. Returns `false` only
/// when the configuration exists but could not be read or parsed.
///
/// # Safety
/// `out` must be a valid, writable pointer to an `NfmConfig`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_config_load(out: *mut NfmConfig) -> bool {
    if out.is_null() {
        return false;
    }

    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    let (config, ok) = match config_file_path(xdg.as_deref(), home.as_deref()) {
        None => (Config::default(), true),
        Some(path) => match Config::load(&path) {
            Ok(config) => (config, true),
            Err(_) => (Config::default(), false),
        },
    };

    let value = NfmConfig {
        show_hidden: config.show_hidden,
        sort_key: match config.sort.key {
            SortKey::Name => SORT_NAME,
            SortKey::Size => SORT_SIZE,
            SortKey::Modified => SORT_MODIFIED,
        },
        descending: config.sort.descending,
    };
    // SAFETY: the caller guarantees `out` is valid and writable.
    unsafe { out.write(value) };
    ok
}

/// Lists `path` using `config` for hidden-file visibility and sorting.
///
/// Always returns a listing. If the directory cannot be read, the listing has
/// no entries and [`nfm_listing_error`] describes the failure. Returns null only
/// when `path` is null.
///
/// # Safety
/// `path` must be null or a valid NUL-terminated UTF-8 (or lossy) string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_listing_open(
    path: *const c_char,
    config: NfmConfig,
) -> *mut NfmListing {
    if path.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: the caller guarantees `path` is a valid C string.
    let path = PathBuf::from(
        unsafe { CStr::from_ptr(path) }
            .to_string_lossy()
            .into_owned(),
    );

    let listing = match list_directory(&path) {
        Ok(mut entries) => {
            if !config.show_hidden {
                entries.retain(|entry| !entry.name.starts_with('.'));
            }
            let order = SortOrder {
                key: sort_key_from_c(config.sort_key),
                descending: config.descending,
            };
            sort_entries(&mut entries, order);

            NfmListing {
                error: None,
                entries: entries
                    .iter()
                    .map(|entry| OwnedEntry {
                        name: c_string(&entry.name),
                        path: c_string(&entry.path.to_string_lossy()),
                        kind: kind_to_c(entry.kind),
                        size: entry.size,
                        modified_unix: modified_to_unix(entry),
                    })
                    .collect(),
            }
        }
        Err(err) => NfmListing {
            error: Some(c_string(&format!("cannot read {}: {err}", path.display()))),
            entries: Vec::new(),
        },
    };

    Box::into_raw(Box::new(listing))
}

/// Returns the error message for a listing, or null if it loaded successfully.
///
/// # Safety
/// `listing` must come from [`nfm_listing_open`] and not yet be freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_listing_error(listing: *const NfmListing) -> *const c_char {
    // SAFETY: the caller guarantees `listing` is valid.
    match unsafe { listing.as_ref() }.and_then(|l| l.error.as_ref()) {
        Some(message) => message.as_ptr(),
        None => ptr::null(),
    }
}

/// Returns the number of entries in a listing.
///
/// # Safety
/// `listing` must come from [`nfm_listing_open`] and not yet be freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_listing_count(listing: *const NfmListing) -> usize {
    // SAFETY: the caller guarantees `listing` is valid.
    unsafe { listing.as_ref() }.map_or(0, |l| l.entries.len())
}

/// Copies entry `index` into `out`. Returns `false` if the index is out of range.
///
/// # Safety
/// `listing` must come from [`nfm_listing_open`] and not yet be freed, and `out`
/// must be a valid, writable pointer to an `NfmEntry`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_listing_get(
    listing: *const NfmListing,
    index: usize,
    out: *mut NfmEntry,
) -> bool {
    // SAFETY: the caller guarantees `listing` is valid, or null.
    let Some(listing) = (unsafe { listing.as_ref() }) else {
        return false;
    };
    let Some(entry) = listing.entries.get(index) else {
        return false;
    };
    if out.is_null() {
        return false;
    }

    let value = NfmEntry {
        name: entry.name.as_ptr(),
        path: entry.path.as_ptr(),
        kind: entry.kind,
        size: entry.size,
        has_modified: entry.modified_unix.is_some(),
        modified_unix: entry.modified_unix.unwrap_or(0),
    };
    // SAFETY: the caller guarantees `out` is valid and writable.
    unsafe { out.write(value) };
    true
}

/// Frees a listing and every string taken from it. Null is ignored.
///
/// # Safety
/// `listing` must be null or come from [`nfm_listing_open`], and must not be
/// used again after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nfm_listing_free(listing: *mut NfmListing) {
    if !listing.is_null() {
        // SAFETY: the caller guarantees the pointer came from `Box::into_raw`.
        drop(unsafe { Box::from_raw(listing) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn default_config() -> NfmConfig {
        NfmConfig {
            show_hidden: false,
            sort_key: SORT_NAME,
            descending: false,
        }
    }

    unsafe fn names(listing: *const NfmListing) -> Vec<String> {
        let count = unsafe { nfm_listing_count(listing) };
        (0..count)
            .map(|i| {
                let mut entry = std::mem::MaybeUninit::<NfmEntry>::uninit();
                assert!(unsafe { nfm_listing_get(listing, i, entry.as_mut_ptr()) });
                let entry = unsafe { entry.assume_init() };
                unsafe { CStr::from_ptr(entry.name) }
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn lists_sorted_entries_and_hides_dotfiles() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), b"x").unwrap();
        std::fs::write(dir.path().join(".secret"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("zdir")).unwrap();

        let path = CString::new(dir.path().to_str().unwrap()).unwrap();
        unsafe {
            let listing = nfm_listing_open(path.as_ptr(), default_config());
            assert!(nfm_listing_error(listing).is_null());
            assert_eq!(names(listing), ["zdir", "b.txt"]);
            nfm_listing_free(listing);
        }
    }

    #[test]
    fn shows_dotfiles_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".secret"), b"x").unwrap();

        let path = CString::new(dir.path().to_str().unwrap()).unwrap();
        let config = NfmConfig {
            show_hidden: true,
            ..default_config()
        };
        unsafe {
            let listing = nfm_listing_open(path.as_ptr(), config);
            assert_eq!(names(listing), [".secret"]);
            nfm_listing_free(listing);
        }
    }

    #[test]
    fn missing_directory_reports_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = CString::new(dir.path().join("absent").to_str().unwrap()).unwrap();
        unsafe {
            let listing = nfm_listing_open(path.as_ptr(), default_config());
            assert!(!nfm_listing_error(listing).is_null());
            assert_eq!(nfm_listing_count(listing), 0);
            nfm_listing_free(listing);
        }
    }

    #[test]
    fn null_path_returns_null() {
        unsafe {
            assert!(nfm_listing_open(ptr::null(), default_config()).is_null());
        }
    }

    #[test]
    fn out_of_range_index_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = CString::new(dir.path().to_str().unwrap()).unwrap();
        unsafe {
            let listing = nfm_listing_open(path.as_ptr(), default_config());
            let mut entry = std::mem::MaybeUninit::<NfmEntry>::uninit();
            assert!(!nfm_listing_get(listing, 0, entry.as_mut_ptr()));
            nfm_listing_free(listing);
        }
    }
}
