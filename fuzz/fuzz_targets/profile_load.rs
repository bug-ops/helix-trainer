//! `ProfileStorage::load` must never panic, whatever `profile.json` holds.
//!
//! Loading deserializes `ChallengeDay` and the lenient keymap fingerprint.

#![no_main]

use helix_trainer::gamification::ProfileStorage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("profile.json");
    std::fs::write(&path, data).expect("write the fuzz input");
    // An error is a normal outcome for malformed input; only a panic is a finding.
    let _ = ProfileStorage::with_path(&path).load();
});
