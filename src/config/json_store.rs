//! Shared persistence for the JSON documents under the data directory.
//!
//! [`JsonStore`] is the single implementation behind `profile.json` and
//! `config.json`: size-capped reads of regular files only, and atomic,
//! durable, owner-private writes. Document-specific concerns (the profile's
//! lock file and quarantine) stay with their owners.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::security::{SecurityError, path_validator, private_fs};

/// Per-process counter used to make temporary save files unique.
///
/// Combined with the process id, this guarantees two concurrent saves (e.g.
/// two stores pointed at the same path, as happens in tests) never share a
/// temp file name and race each other's rename.
static TMP_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Builds a pid+counter-tagged sibling temp path for `base_path`, e.g.
/// `profile.json` becomes `profile.json.1234.5.tmp`.
pub(crate) fn unique_tmp_path(base_path: &Path) -> PathBuf {
    let suffix = TMP_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    base_path.with_added_extension(format!("{}.{suffix}.tmp", std::process::id()))
}

/// Fsync a file's parent directory so a preceding `fs::rename` into it is
/// durable across a power loss, not just visible after a normal process
/// restart. POSIX does not guarantee a directory-entry update is on disk
/// until the directory itself is synced; Windows/NTFS has no equivalent
/// "open a directory as a file" operation, so this is Unix-only.
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) {
    // `Path::parent()` returns `Some("")` for a bare relative filename;
    // treat that as the current directory.
    let parent = match path.parent() {
        Some(p) if !p.is_empty() => p,
        _ => Path::new("."),
    };
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

/// Atomically replaces `path` with `bytes`, creating the parent directory
/// (owner-private) if needed.
///
/// Writes a unique sibling temp file, `fsync`s it, renames it over `path`
/// (atomic on one filesystem on POSIX and Windows, so a reader sees the old
/// or the complete new file, never a torn one) and, on Unix, `fsync`s the
/// parent directory so the rename survives a power loss.
///
/// # Errors
///
/// Returns the underlying I/O error; the temp file is removed on failure.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        private_fs::create_dir_all(parent)?;
    }

    let tmp_path = unique_tmp_path(path);
    let result = private_fs::create_file(&tmp_path)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| fs::rename(&tmp_path, path));
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    #[cfg(unix)]
    fsync_parent_dir(path);
    Ok(())
}

/// Failure of a [`JsonStore`] operation. `label` names the document
/// ("profile", "config") in the message.
#[derive(Debug, Error)]
pub enum JsonStoreError {
    /// The file could not be inspected or was rejected before parsing
    /// (not a regular file, unreadable, invalid UTF-8).
    #[error("Failed to read {label}: {reason}")]
    Read {
        /// Document name.
        label: &'static str,
        /// Underlying cause.
        reason: String,
    },

    /// The file exceeds the document's size cap.
    #[error("{label} file too large: {source}")]
    TooLarge {
        /// Document name.
        label: &'static str,
        /// The size-guard rejection.
        source: SecurityError,
    },

    /// The file is not valid JSON for the document type.
    #[error("Failed to parse {label} JSON: {source}")]
    Parse {
        /// Document name.
        label: &'static str,
        /// The parser error.
        source: serde_json::Error,
    },

    /// The document could not be serialized.
    #[error("Failed to serialize {label}: {source}")]
    Serialize {
        /// Document name.
        label: &'static str,
        /// The serializer error.
        source: serde_json::Error,
    },

    /// Writing, syncing or renaming the file failed.
    #[error("Failed to write {label}: {source}")]
    Write {
        /// Document name.
        label: &'static str,
        /// The I/O error.
        source: io::Error,
    },
}

/// Atomic, size-guarded JSON document `T` stored at one path.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::json_store::JsonStore;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct Doc {
///     value: u32,
/// }
///
/// let dir = tempfile::tempdir().unwrap();
/// let store = JsonStore::<Doc>::new(dir.path().join("doc.json"), "doc", 1024);
/// assert_eq!(store.load().unwrap(), None);
/// store.save(&Doc { value: 7 }).unwrap();
/// assert_eq!(store.load().unwrap(), Some(Doc { value: 7 }));
/// ```
pub struct JsonStore<T> {
    path: PathBuf,
    label: &'static str,
    max_size: u64,
    document: PhantomData<fn() -> T>,
}

impl<T> JsonStore<T> {
    /// Creates a store for the document at `path`; reads larger than
    /// `max_size` bytes are rejected.
    pub fn new(path: impl Into<PathBuf>, label: &'static str, max_size: u64) -> Self {
        Self {
            path: path.into(),
            label,
            max_size,
            document: PhantomData,
        }
    }

    /// Path of the document file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the document file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

impl<T: DeserializeOwned> JsonStore<T> {
    /// Loads the document; `Ok(None)` means the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`JsonStoreError`] when the path exists but is not a regular
    /// file, is unreadable, exceeds the size cap, or does not parse.
    pub fn load(&self) -> Result<Option<T>, JsonStoreError> {
        match fs::metadata(&self.path) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(JsonStoreError::Read {
                    label: self.label,
                    reason: e.to_string(),
                });
            }
        }

        let contents = path_validator::read_capped(&self.path, self.max_size).map_err(|e| {
            if matches!(e, SecurityError::FileTooLarge { .. }) {
                JsonStoreError::TooLarge {
                    label: self.label,
                    source: e,
                }
            } else {
                JsonStoreError::Read {
                    label: self.label,
                    reason: e.to_string(),
                }
            }
        })?;

        serde_json::from_str(&contents)
            .map(Some)
            .map_err(|source| JsonStoreError::Parse {
                label: self.label,
                source,
            })
    }
}

impl<T: Serialize> JsonStore<T> {
    /// Atomically persists `document`, creating the parent directory if needed.
    ///
    /// # Errors
    ///
    /// Returns [`JsonStoreError`] if serialization or any write step fails;
    /// the previous file is left intact.
    pub fn save(&self, document: &T) -> Result<(), JsonStoreError> {
        let json =
            serde_json::to_string_pretty(document).map_err(|source| JsonStoreError::Serialize {
                label: self.label,
                source,
            })?;
        write_atomic(&self.path, json.as_bytes()).map_err(|source| JsonStoreError::Write {
            label: self.label,
            source,
        })
    }
}

impl<T> Clone for JsonStore<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            label: self.label,
            max_size: self.max_size,
            document: PhantomData,
        }
    }
}

impl<T> fmt::Debug for JsonStore<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonStore")
            .field("path", &self.path)
            .field("label", &self.label)
            .field("max_size", &self.max_size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Doc {
        value: u32,
    }

    fn store(dir: &Path, max_size: u64) -> JsonStore<Doc> {
        JsonStore::new(dir.join("nested").join("doc.json"), "doc", max_size)
    }

    #[test]
    fn missing_file_loads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(store(dir.path(), 1024).load().unwrap(), None);
    }

    #[test]
    fn save_creates_parent_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 3 }).unwrap();
        assert!(store.exists());
        assert_eq!(store.load().unwrap(), Some(Doc { value: 3 }));
    }

    #[test]
    fn save_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();
        store.save(&Doc { value: 2 }).unwrap();
        let names: Vec<_> = fs::read_dir(store.path().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["doc.json"]);
    }

    /// Failure is injected by pre-occupying the exact temp path the next save
    /// will use with a directory, which fails deterministically even as root.
    /// Relies on nextest's process-per-test isolation for the counter.
    #[test]
    fn failed_save_does_not_corrupt_existing_document() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();
        let before = fs::read_to_string(store.path()).unwrap();

        let next = TMP_SUFFIX_COUNTER.load(Ordering::Relaxed);
        let predicted = store
            .path()
            .with_added_extension(format!("{}.{next}.tmp", std::process::id()));
        fs::create_dir(&predicted).unwrap();

        assert!(matches!(
            store.save(&Doc { value: 2 }),
            Err(JsonStoreError::Write { .. })
        ));
        assert_eq!(fs::read_to_string(store.path()).unwrap(), before);
        assert_eq!(store.load().unwrap(), Some(Doc { value: 1 }));
    }

    #[test]
    fn oversized_file_is_rejected_as_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 8);
        store.save(&Doc { value: 1234567 }).unwrap();
        let err = store.load().unwrap_err();
        assert!(matches!(err, JsonStoreError::TooLarge { .. }), "{err}");
        assert!(err.to_string().contains("too large"));
    }

    #[test]
    fn malformed_json_is_a_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();
        fs::write(store.path(), "{broken").unwrap();
        assert!(matches!(store.load(), Err(JsonStoreError::Parse { .. })));
    }

    #[test]
    fn directory_in_place_of_file_is_a_read_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        fs::create_dir_all(store.path()).unwrap();
        assert!(matches!(store.load(), Err(JsonStoreError::Read { .. })));
    }

    #[cfg(unix)]
    #[test]
    fn written_file_and_created_directory_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(store.path()), 0o600);
        assert_eq!(mode(store.path().parent().unwrap()), 0o700);
    }
}
