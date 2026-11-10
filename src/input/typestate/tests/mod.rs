//! Tests for typestate input handling

use std::num::NonZeroUsize;

/// A non-zero count for building input states in tests.
fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("test counts are non-zero")
}

mod base_handler_tests;
mod count_handler_tests;
mod find_replace_tests;
mod goto_handler_tests;
mod grammar_tests;
mod handler_result_tests;
mod input_state_tests;
mod key_mapping_tests;
mod match_handler_tests;
mod state_machine_tests;
mod state_types_tests;
mod text_object_tests;
mod typestate_wrapper_tests;
mod unmatched_handler_tests;
mod view_handler_tests;
