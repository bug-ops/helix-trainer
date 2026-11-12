//! `ConfigStorage::load` must never panic, whatever `config.json` holds.

#![no_main]

use helix_trainer::config::ConfigStorage;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("config.json");
    std::fs::write(&path, data).expect("write the fuzz input");
    // An error is a normal outcome for malformed input; only a panic is a finding.
    let _ = ConfigStorage::with_path(&path).load();
});
