//! Stable fingerprint of a resolved keymap
//!
//! The fingerprint is persisted in the user profile, so its algorithm must not
//! depend on the toolchain (`DefaultHasher`) or on `Debug` formatting. It is
//! FNV-1a 64 over length-prefixed UTF-8 fields; the algorithm is part of the
//! stored value ([`KeymapFingerprint::Fnv1a64V1`]) so a future change cannot be
//! mistaken for a keymap change.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const CURRENT_TAG: &str = "Fnv1a64V1";

/// Incremental FNV-1a 64-bit hasher.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fnv1a64(u64);

impl Fnv1a64 {
    pub(crate) const fn new() -> Self {
        Self(FNV_OFFSET_BASIS)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
    }

    /// Hash a UTF-8 field as its byte length (`u64` little-endian) followed by its bytes,
    /// so adjacent fields can never merge into an ambiguous byte stream.
    pub(crate) fn write_field(&mut self, field: &str) {
        self.write(&(field.len() as u64).to_le_bytes());
        self.write(field.as_bytes());
    }

    pub(crate) const fn finish(self) -> u64 {
        self.0
    }
}

/// Fingerprint of a resolved keymap, tagged with the algorithm that produced it.
///
/// Serialized externally tagged (`{"Fnv1a64V1": 123}`), which is distinguishable from
/// a legacy bare number (see [`StoredKeymapFingerprint`]).
///
/// # Examples
///
/// ```
/// use helix_trainer::input::keymap::KeymapOverlay;
///
/// let stock = KeymapOverlay::identity().fingerprint();
/// assert_eq!(stock, KeymapOverlay::identity().fingerprint());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeymapFingerprint {
    /// FNV-1a 64 over the bindings sorted by (context, key); each binding hashes its
    /// context name, Helix key string and canonical target as length-prefixed fields.
    Fnv1a64V1(u64),
}

/// A keymap fingerprint as read from a profile file.
///
/// Profiles written before the algorithm tag existed hold a bare `u64` produced by
/// the unspecified std hasher; it cannot be compared with a current fingerprint, so
/// it never raises the mismatch notification and is replaced on the next launch.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::keymap::fingerprint::StoredKeymapFingerprint;
/// use helix_trainer::input::keymap::KeymapOverlay;
///
/// let legacy: StoredKeymapFingerprint = serde_json::from_str("12345").unwrap();
/// let current = KeymapOverlay::identity().fingerprint();
/// assert!(!legacy.contradicts(current));
///
/// let stored = StoredKeymapFingerprint::Current(current);
/// let json = serde_json::to_string(&stored).unwrap();
/// assert_eq!(serde_json::from_str::<StoredKeymapFingerprint>(&json).unwrap(), stored);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StoredKeymapFingerprint {
    /// A fingerprint of the current, tagged algorithm.
    Current(KeymapFingerprint),
    /// An untagged value from the pre-tag algorithm.
    Legacy(u64),
}

impl StoredKeymapFingerprint {
    /// Whether review history recorded under this value is known to span a different
    /// keymap than `current`. A legacy value is never known to.
    pub fn contradicts(self, current: KeymapFingerprint) -> bool {
        match self {
            Self::Current(stored) => stored != current,
            Self::Legacy(_) => false,
        }
    }
}

/// Field-level deserializer for `Option<StoredKeymapFingerprint>` that reads a
/// fingerprint written by a newer version as absent.
///
/// A JSON object with a single unknown tag key maps to `None`, the same no-notice
/// path as a first activation; the next save replaces it with the current tag. Any
/// other malformed shape is still an error so the profile is quarantined as before.
///
/// # Errors
///
/// Returns the underlying error for a value that is neither `null`, a legacy number,
/// a well-formed current tag nor an object with exactly one unknown key.
pub fn lenient_stored_fingerprint<'de, D>(
    deserializer: D,
) -> Result<Option<StoredKeymapFingerprint>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    if let Some(object) = value.as_object()
        && object.len() == 1
        && !object.contains_key(CURRENT_TAG)
    {
        return Ok(None);
    }
    StoredKeymapFingerprint::deserialize(value)
        .map(Some)
        .map_err(D::Error::custom)
}

impl From<KeymapFingerprint> for StoredKeymapFingerprint {
    fn from(fingerprint: KeymapFingerprint) -> Self {
        Self::Current(fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv(bytes: &[u8]) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.write(bytes);
        hasher.finish()
    }

    #[test]
    fn fnv1a64_matches_the_published_test_vectors() {
        assert_eq!(fnv(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn length_prefix_separates_adjacent_fields() {
        let mut ab_c = Fnv1a64::new();
        ab_c.write_field("ab");
        ab_c.write_field("c");
        let mut a_bc = Fnv1a64::new();
        a_bc.write_field("a");
        a_bc.write_field("bc");
        assert_ne!(ab_c.finish(), a_bc.finish());
    }

    #[test]
    fn untagged_number_reads_as_legacy_and_tagged_as_current() {
        let legacy: StoredKeymapFingerprint = serde_json::from_str("18446744073709551615").unwrap();
        assert_eq!(legacy, StoredKeymapFingerprint::Legacy(u64::MAX));

        let tagged: StoredKeymapFingerprint = serde_json::from_str(r#"{"Fnv1a64V1":7}"#).unwrap();
        assert_eq!(
            tagged,
            StoredKeymapFingerprint::Current(KeymapFingerprint::Fnv1a64V1(7))
        );
    }

    #[derive(Deserialize)]
    struct Holder {
        #[serde(default, deserialize_with = "lenient_stored_fingerprint")]
        fingerprint: Option<StoredKeymapFingerprint>,
    }

    fn load(json: &str) -> Result<Option<StoredKeymapFingerprint>, serde_json::Error> {
        serde_json::from_str::<Holder>(json).map(|holder| holder.fingerprint)
    }

    #[test]
    fn current_tag_constant_matches_the_serialized_tag() {
        let json = serde_json::to_value(KeymapFingerprint::Fnv1a64V1(1)).unwrap();
        assert!(json.as_object().unwrap().contains_key(CURRENT_TAG));
    }

    #[test]
    fn unknown_single_tag_reads_as_absent() {
        assert_eq!(load(r#"{"fingerprint":{"Sha256V2":"abc"}}"#).unwrap(), None);
        assert_eq!(load(r#"{"fingerprint":{"Sha256V2":[1,2]}}"#).unwrap(), None);
    }

    #[test]
    fn known_shapes_still_load() {
        assert_eq!(load("{}").unwrap(), None);
        assert_eq!(load(r#"{"fingerprint":null}"#).unwrap(), None);
        assert_eq!(
            load(r#"{"fingerprint":5}"#).unwrap(),
            Some(StoredKeymapFingerprint::Legacy(5))
        );
        assert_eq!(
            load(r#"{"fingerprint":{"Fnv1a64V1":7}}"#).unwrap(),
            Some(KeymapFingerprint::Fnv1a64V1(7).into())
        );
    }

    #[test]
    fn other_malformed_shapes_remain_errors() {
        for json in [
            r#"{"fingerprint":"text"}"#,
            r#"{"fingerprint":{"A":1,"B":2}}"#,
            r#"{"fingerprint":{"Fnv1a64V1":"x"}}"#,
            r#"{"fingerprint":{}}"#,
            r#"{"fingerprint":[1]}"#,
        ] {
            assert!(load(json).is_err(), "{json}");
        }
    }

    #[test]
    fn only_a_different_current_value_contradicts() {
        let current = KeymapFingerprint::Fnv1a64V1(1);
        assert!(!StoredKeymapFingerprint::Legacy(1).contradicts(current));
        assert!(!StoredKeymapFingerprint::Current(current).contradicts(current));
        assert!(
            StoredKeymapFingerprint::Current(KeymapFingerprint::Fnv1a64V1(2)).contradicts(current)
        );
    }
}
