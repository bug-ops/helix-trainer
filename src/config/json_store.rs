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

/// Number of fresh temp names tried before giving up when each one is taken.
const TMP_CREATE_ATTEMPTS: u32 = 16;

/// Creates a new owner-private sibling temp file for `base_path`.
///
/// The file is created with `O_EXCL` semantics, so a pre-planted file or
/// symlink at a predicted name is never followed or truncated: that name is
/// skipped and the next one tried.
///
/// # Errors
///
/// Returns the underlying I/O error, or [`io::ErrorKind::AlreadyExists`] when
/// every attempted name was taken.
pub(crate) fn create_unique_tmp(base_path: &Path) -> io::Result<(PathBuf, fs::File)> {
    for _ in 0..TMP_CREATE_ATTEMPTS {
        let tmp_path = unique_tmp_path(base_path);
        match private_fs::create_new_file(&tmp_path) {
            Ok(file) => return Ok((tmp_path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free temporary file name",
    ))
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

    let (tmp_path, mut file) = create_unique_tmp(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| fs::rename(&tmp_path, path));
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

    /// Copying the document from its previous location failed; the
    /// destination was not created.
    #[error("Failed to migrate {label} from {}: {reason}", from.display())]
    Migrate {
        /// Document name.
        label: &'static str,
        /// The previous location.
        from: PathBuf,
        /// Underlying cause.
        reason: String,
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
    legacy: Option<PathBuf>,
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
            legacy: None,
            document: PhantomData,
        }
    }

    /// Declares the document's previous location (`None` keeps the store
    /// as it is).
    ///
    /// On [`JsonStore::load`], when the destination file is absent and the
    /// previous one exists, it is copied (never moved) into place through a
    /// temporary file, `fsync` and rename. Once the destination exists the
    /// previous location is never read again, and it is left untouched.
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
    /// let old = JsonStore::<Doc>::new(dir.path().join("old.json"), "doc", 1024);
    /// old.save(&Doc { value: 5 }).unwrap();
    ///
    /// let new = JsonStore::<Doc>::new(dir.path().join("new.json"), "doc", 1024)
    ///     .migrating_from(Some(old.path().to_path_buf()));
    /// assert_eq!(new.load().unwrap(), Some(Doc { value: 5 }));
    /// assert!(old.exists());
    /// ```
    #[must_use]
    pub fn migrating_from(mut self, legacy: Option<PathBuf>) -> Self {
        self.legacy = legacy;
        self
    }

    /// Path of the document file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The previous location declared with [`JsonStore::migrating_from`].
    pub fn legacy_path(&self) -> Option<&Path> {
        self.legacy.as_deref()
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
    /// file, is unreadable, exceeds the size cap, or does not parse, or when
    /// copying from the declared previous location fails.
    pub fn load(&self) -> Result<Option<T>, JsonStoreError> {
        self.migrate_legacy()?;
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

impl<T> JsonStore<T> {
    fn migrate_legacy(&self) -> Result<(), JsonStoreError> {
        let Some(legacy) = &self.legacy else {
            return Ok(());
        };
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            _ => return Ok(()),
        }
        if let Err(e) = fs::metadata(legacy)
            && e.kind() == io::ErrorKind::NotFound
        {
            return Ok(());
        }

        let migrate_err = |reason: String| JsonStoreError::Migrate {
            label: self.label,
            from: legacy.clone(),
            reason,
        };
        let contents = path_validator::read_capped(legacy, self.max_size)
            .map_err(|e| migrate_err(e.to_string()))?;
        write_atomic(&self.path, contents.as_bytes()).map_err(|e| migrate_err(e.to_string()))?;
        tracing::info!(
            label = self.label,
            from = %legacy.display(),
            to = %self.path.display(),
            "Migrated document to the new data directory"
        );
        Ok(())
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
            legacy: self.legacy.clone(),
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
            .field("legacy", &self.legacy)
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

    /// Failure is injected by making the destination a directory, so the final
    /// rename fails deterministically even as root.
    #[test]
    fn failed_save_leaves_no_temp_file_and_reports_write_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        fs::create_dir_all(store.path()).unwrap();

        assert!(matches!(
            store.save(&Doc { value: 2 }),
            Err(JsonStoreError::Write { .. })
        ));
        assert!(store.path().is_dir());
        let names: Vec<_> = fs::read_dir(store.path().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["doc.json"]);
    }

    /// Relies on nextest's process-per-test isolation for the counter.
    #[test]
    fn save_skips_a_pre_planted_temp_name_and_leaves_it_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();

        let next = TMP_SUFFIX_COUNTER.load(Ordering::Relaxed);
        let planted = store
            .path()
            .with_added_extension(format!("{}.{next}.tmp", std::process::id()));
        fs::write(&planted, "planted").unwrap();

        store.save(&Doc { value: 2 }).unwrap();

        assert_eq!(store.load().unwrap(), Some(Doc { value: 2 }));
        assert_eq!(fs::read_to_string(&planted).unwrap(), "planted");
    }

    #[cfg(unix)]
    #[test]
    fn save_does_not_follow_a_symlink_planted_at_the_temp_name() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), 1024);
        store.save(&Doc { value: 1 }).unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, "precious").unwrap();

        let next = TMP_SUFFIX_COUNTER.load(Ordering::Relaxed);
        let planted = store
            .path()
            .with_added_extension(format!("{}.{next}.tmp", std::process::id()));
        std::os::unix::fs::symlink(&victim, &planted).unwrap();

        store.save(&Doc { value: 2 }).unwrap();

        assert_eq!(fs::read_to_string(&victim).unwrap(), "precious");
        assert_eq!(store.load().unwrap(), Some(Doc { value: 2 }));
    }

    #[test]
    fn create_unique_tmp_gives_up_when_every_name_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("doc.json");
        let next = TMP_SUFFIX_COUNTER.load(Ordering::Relaxed);
        for i in 0..u64::from(TMP_CREATE_ATTEMPTS) {
            let taken =
                base.with_added_extension(format!("{}.{}.tmp", std::process::id(), next + i));
            fs::write(taken, "x").unwrap();
        }

        let err = create_unique_tmp(&base).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
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

    fn migrating_store(dir: &Path, max_size: u64) -> (JsonStore<Doc>, JsonStore<Doc>) {
        let old = JsonStore::new(dir.join("old").join("doc.json"), "doc", max_size);
        let new = JsonStore::new(dir.join("new").join("doc.json"), "doc", max_size)
            .migrating_from(Some(old.path().to_path_buf()));
        (old, new)
    }

    #[test]
    fn migration_copies_when_destination_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = migrating_store(dir.path(), 1024);
        old.save(&Doc { value: 9 }).unwrap();

        assert_eq!(new.load().unwrap(), Some(Doc { value: 9 }));
        assert_eq!(
            fs::read(old.path()).unwrap(),
            fs::read(new.path()).unwrap(),
            "copy, not move: the source stays byte-identical"
        );
    }

    #[test]
    fn existing_destination_wins_and_source_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = migrating_store(dir.path(), 1024);
        old.save(&Doc { value: 1 }).unwrap();
        new.save(&Doc { value: 2 }).unwrap();
        fs::write(old.path(), "{garbage").unwrap();

        assert_eq!(new.load().unwrap(), Some(Doc { value: 2 }));
    }

    #[test]
    fn migration_is_a_noop_without_a_source() {
        let dir = tempfile::tempdir().unwrap();
        let (_, new) = migrating_store(dir.path(), 1024);
        assert_eq!(new.load().unwrap(), None);
        assert!(!new.exists());
    }

    #[test]
    fn migration_resumes_per_document() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = migrating_store(dir.path(), 1024);
        old.save(&Doc { value: 4 }).unwrap();
        assert_eq!(new.load().unwrap(), Some(Doc { value: 4 }));
        fs::remove_file(new.path()).unwrap();

        assert_eq!(new.load().unwrap(), Some(Doc { value: 4 }));
    }

    #[test]
    fn oversized_source_fails_migration_and_creates_no_destination() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = migrating_store(dir.path(), 8);
        old.save(&Doc { value: 1234567 }).unwrap();

        assert!(matches!(new.load(), Err(JsonStoreError::Migrate { .. })));
        assert!(!new.exists());
    }

    #[test]
    fn non_regular_source_fails_migration_and_creates_no_destination() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = migrating_store(dir.path(), 1024);
        fs::create_dir_all(old.path()).unwrap();

        assert!(matches!(new.load(), Err(JsonStoreError::Migrate { .. })));
        assert!(!new.exists());
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
