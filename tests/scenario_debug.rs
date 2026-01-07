//! Debug test for tracing scenario execution step-by-step
//!
//! This test validates each scenario by executing its solution commands
//! and comparing the actual result with the expected target.

use helix_trainer::config::ScenarioLoader;
use helix_trainer::game::command_context::extract_count_and_command;
use helix_trainer::game::{EditorState, GameSession, SessionAfterAction};
use std::path::Path;

/// Detailed result of scenario execution
#[derive(Debug)]
struct ScenarioResult {
    id: String,
    name: String,
    passed: bool,
    // Setup
    setup_content: String,
    setup_cursor: (usize, usize),
    setup_selection: Option<[usize; 4]>,
    // Target
    target_content: String,
    target_cursor: (usize, usize),
    target_selection: Option<[usize; 4]>,
    // Actual result after execution
    actual_content: String,
    actual_cursor: (usize, usize),
    actual_selection: Option<[usize; 4]>,
    // Differences
    content_match: bool,
    cursor_match: bool,
    selection_match: bool,
    // Commands executed
    commands: Vec<String>,
    // Error if any
    error: Option<String>,
}

fn state_to_selection_array(state: &EditorState) -> Option<[usize; 4]> {
    state
        .selection()
        .map(|sel| [sel.start.row, sel.start.col, sel.end.row, sel.end.col])
}

fn execute_scenario_and_compare(scenario: &helix_trainer::config::Scenario) -> ScenarioResult {
    let mut result = ScenarioResult {
        id: scenario.id.clone(),
        name: scenario.name.clone(),
        passed: false,
        setup_content: scenario.setup.file_content.clone(),
        setup_cursor: scenario.setup.cursor_position,
        setup_selection: scenario.setup.selection,
        target_content: scenario.target.file_content.clone(),
        target_cursor: scenario.target.cursor_position,
        target_selection: scenario.target.selection,
        actual_content: String::new(),
        actual_cursor: (0, 0),
        actual_selection: None,
        content_match: false,
        cursor_match: false,
        selection_match: false,
        commands: scenario.solution.commands.clone(),
        error: None,
    };

    // Create session
    let session = match GameSession::new(scenario.clone()) {
        Ok(s) => s,
        Err(e) => {
            result.error = Some(format!("Failed to create session: {:?}", e));
            return result;
        }
    };

    // Execute all commands
    let mut current_session: Option<SessionAfterAction> =
        Some(SessionAfterAction::StillActive(session));

    for (i, cmd) in scenario.solution.commands.iter().enumerate() {
        if let Some(state) = current_session.take() {
            match state {
                SessionAfterAction::StillActive(s) => {
                    let (count, base_cmd) = extract_count_and_command(cmd);
                    let mut active = Some(s);

                    for _ in 0..count {
                        if let Some(sess) = active.take() {
                            match sess.record_action(base_cmd.to_string()) {
                                Ok(new_state) => match new_state {
                                    SessionAfterAction::Completed(c) => {
                                        current_session = Some(SessionAfterAction::Completed(c));
                                        break;
                                    }
                                    SessionAfterAction::StillActive(next) => {
                                        active = Some(next);
                                    }
                                },
                                Err(e) => {
                                    result.error =
                                        Some(format!("Command {} '{}' failed: {:?}", i, cmd, e));
                                    return result;
                                }
                            }
                        }
                    }

                    if current_session.is_none()
                        && let Some(remaining) = active
                    {
                        current_session = Some(SessionAfterAction::StillActive(remaining));
                    }
                }
                SessionAfterAction::Completed(c) => {
                    current_session = Some(SessionAfterAction::Completed(c));
                    break;
                }
            }
        }
    }

    // Get final state
    let final_state = match current_session {
        Some(SessionAfterAction::StillActive(s)) => s.current_state().clone(),
        Some(SessionAfterAction::Completed(c)) => c.current_state().clone(),
        None => {
            result.error = Some("No final state available".to_string());
            return result;
        }
    };

    // Record actual results
    result.actual_content = final_state.content().to_string();
    let cursor = final_state.cursor_position();
    result.actual_cursor = (cursor.row, cursor.col);
    result.actual_selection = state_to_selection_array(&final_state);

    // Compare content
    result.content_match = result.actual_content == result.target_content;

    // Compare cursor
    result.cursor_match = result.actual_cursor == result.target_cursor;

    // Compare selection
    result.selection_match = match (result.actual_selection, result.target_selection) {
        (Some(actual), Some(target)) => actual == target,
        (None, None) => true,
        (None, Some(_)) => false, // Target expects selection but we have none
        (Some(_), None) => true,  // Target doesn't require selection
    };

    // Overall pass: content must match, and if target has selection, it must match
    // If target has no selection, cursor position must match
    result.passed = result.content_match
        && (if result.target_selection.is_some() {
            result.selection_match
        } else {
            result.cursor_match
        });

    result
}

fn print_scenario_result(result: &ScenarioResult) {
    println!("\n{}", "=".repeat(70));
    println!("Scenario: {} ({})", result.name, result.id);
    println!("{}", "=".repeat(70));
    println!("Commands: {:?}", result.commands);
    println!();

    println!("SETUP:");
    println!("  Content: {:?}", result.setup_content);
    println!("  Cursor: {:?}", result.setup_cursor);
    if let Some(sel) = result.setup_selection {
        println!("  Selection: {:?}", sel);
    }
    println!();

    println!("TARGET:");
    println!("  Content: {:?}", result.target_content);
    println!("  Cursor: {:?}", result.target_cursor);
    if let Some(sel) = result.target_selection {
        println!("  Selection: {:?}", sel);
    }
    println!();

    println!("ACTUAL:");
    println!("  Content: {:?}", result.actual_content);
    println!("  Cursor: {:?}", result.actual_cursor);
    if let Some(sel) = result.actual_selection {
        println!("  Selection: {:?}", sel);
    }
    println!();

    println!("COMPARISON:");
    println!(
        "  Content match: {} {}",
        if result.content_match {
            "[OK]"
        } else {
            "[FAIL]"
        },
        if !result.content_match {
            format!(
                "(expected: {:?}, got: {:?})",
                result.target_content, result.actual_content
            )
        } else {
            String::new()
        }
    );
    println!(
        "  Cursor match: {} {}",
        if result.cursor_match {
            "[OK]"
        } else {
            "[FAIL]"
        },
        if !result.cursor_match {
            format!(
                "(expected: {:?}, got: {:?})",
                result.target_cursor, result.actual_cursor
            )
        } else {
            String::new()
        }
    );
    if result.target_selection.is_some() {
        println!(
            "  Selection match: {} {}",
            if result.selection_match {
                "[OK]"
            } else {
                "[FAIL]"
            },
            if !result.selection_match {
                format!(
                    "(expected: {:?}, got: {:?})",
                    result.target_selection, result.actual_selection
                )
            } else {
                String::new()
            }
        );
    }

    if let Some(ref error) = result.error {
        println!("  ERROR: {}", error);
    }

    println!();
    println!("RESULT: {}", if result.passed { "PASS" } else { "FAIL" });
}

fn test_scenario_file(path: &Path) -> Vec<ScenarioResult> {
    let loader = ScenarioLoader::new();
    let scenarios = match loader.load(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load {}: {:?}", path.display(), e);
            return vec![];
        }
    };

    scenarios.iter().map(execute_scenario_and_compare).collect()
}

#[test]
fn debug_text_objects_scenarios() {
    let path = Path::new("scenarios/en/selection/text-objects.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "TEXT-OBJECTS SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

#[test]
fn debug_surround_scenarios() {
    let path = Path::new("scenarios/en/editing/surround.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "SURROUND SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

#[test]
fn debug_paragraph_scenarios() {
    let path = Path::new("scenarios/en/movement/paragraph.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "PARAGRAPH SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

#[test]
fn debug_advanced_selection_scenarios() {
    let path = Path::new("scenarios/en/selection/advanced-selection.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "ADVANCED-SELECTION SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

#[test]
fn debug_advanced_editing_scenarios() {
    let path = Path::new("scenarios/en/editing/advanced-editing.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "ADVANCED-EDITING SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

#[test]
fn debug_basic_search_scenarios() {
    let path = Path::new("scenarios/en/search/basic-search.toml");
    let results = test_scenario_file(path);

    let mut failed = 0;
    for result in &results {
        if !result.passed {
            print_scenario_result(result);
            failed += 1;
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "BASIC-SEARCH SUMMARY: {} passed, {} failed out of {}",
        results.len() - failed,
        failed,
        results.len()
    );

    if failed > 0 {
        panic!("{} scenarios failed", failed);
    }
}

/// Run all scenario files and generate a comprehensive report
#[test]
fn debug_all_new_scenarios() {
    let files = [
        (
            "text-objects",
            Path::new("scenarios/en/selection/text-objects.toml"),
        ),
        ("surround", Path::new("scenarios/en/editing/surround.toml")),
        (
            "paragraph",
            Path::new("scenarios/en/movement/paragraph.toml"),
        ),
        (
            "advanced-selection",
            Path::new("scenarios/en/selection/advanced-selection.toml"),
        ),
        (
            "advanced-editing",
            Path::new("scenarios/en/editing/advanced-editing.toml"),
        ),
        (
            "basic-search",
            Path::new("scenarios/en/search/basic-search.toml"),
        ),
    ];

    let mut all_failed: Vec<(String, ScenarioResult)> = Vec::new();
    let mut total_passed = 0;
    let mut total_failed = 0;

    for (category, path) in &files {
        let results = test_scenario_file(path);

        for result in results {
            if result.passed {
                total_passed += 1;
            } else {
                total_failed += 1;
                all_failed.push((category.to_string(), result));
            }
        }
    }

    // Print all failures
    println!("\n{}", "#".repeat(70));
    println!("# FALSE POSITIVE REPORT");
    println!("{}", "#".repeat(70));

    if all_failed.is_empty() {
        println!("\nNo false positives found! All scenarios pass validation.");
    } else {
        println!("\nFound {} false positives:\n", all_failed.len());

        for (category, result) in &all_failed {
            println!("Category: {}", category);
            print_scenario_result(result);
        }
    }

    println!("\n{}", "=".repeat(70));
    println!(
        "OVERALL SUMMARY: {} passed, {} failed",
        total_passed, total_failed
    );
    println!("{}", "=".repeat(70));

    if total_failed > 0 {
        panic!("{} scenarios are false positives", total_failed);
    }
}
