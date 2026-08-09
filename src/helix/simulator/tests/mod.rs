//! Tests for HelixSimulator organized by functionality
//!
//! This module contains comprehensive tests for the Helix editor simulator:
//! - `basic_tests`: Core simulator creation and initialization
//! - `movement_tests`: Cursor movement commands (h, l, j, k, w, b, e, etc.)
//! - `editing_tests`: Text modification commands (delete, undo, indent, etc.)
//! - `selection_tests`: Selection operations (x, %, etc.)
//! - `mode_tests`: Mode transitions and insert mode operations
//! - `clipboard_tests`: Yank and paste operations
//! - `repeat_recording_tests`: Repeat buffer recording verification
//! - `repeat_execution_tests`: Dot command execution tests
//! - `multi_cursor_tests`: Multi-cursor state conversion (Issue #141)

mod basic_tests;
mod clipboard_tests;
mod editing_tests;
mod macro_tests;
mod mode_tests;
mod movement_tests;
mod multi_cursor_tests;
mod repeat_execution_tests;
mod repeat_recording_tests;
mod selection_tests;
