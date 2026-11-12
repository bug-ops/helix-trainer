//! `resolve_str` must never panic, whatever keymap text it is given.

#![no_main]

use helix_trainer::input::keymap::resolve_str;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    // An error is a normal outcome for malformed input; only a panic is a finding.
    let _ = resolve_str(text);
});
