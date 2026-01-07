//! Integration tests for validating all scenario files
//!
//! This test suite loads and validates every scenario in the scenarios/ directory
//! to ensure they are correctly defined and can be executed.
//!
//! IMPORTANT: The `test_all_scenarios_execute_solution` test validates commands
//! key-by-key, exactly as the UI does. This catches issues like multi-key commands
//! (gs, gg, ge) that would fail in the UI due to missing command buffer handling.

use helix_trainer::config::ScenarioLoader;
use helix_trainer::game::GameSession;
use helix_trainer::game::command_context::{
    ParsedCommand, extract_count_and_command, parse_command_buffer,
};
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn test_all_scenarios_load_successfully() {
    let scenarios_dir = Path::new("scenarios");

    if !scenarios_dir.exists() {
        panic!("Scenarios directory not found: {:?}", scenarios_dir);
    }

    let loader = ScenarioLoader::new();
    let mut total_scenarios = 0;
    let mut failed_files = Vec::new();

    // Walk through all .toml files in scenarios directory
    for entry in WalkDir::new(scenarios_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();
        println!("\nValidating: {}", path.display());

        match loader.load(path) {
            Ok(scenarios) => {
                println!("  ✓ Loaded {} scenarios", scenarios.len());
                total_scenarios += scenarios.len();

                // Validate each scenario can create a GameSession
                for scenario in scenarios {
                    match GameSession::new(scenario.clone()) {
                        Ok(_) => {
                            println!("    ✓ Scenario '{}' ({})", scenario.name, scenario.id);
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "Failed to create GameSession for '{}' ({}): {:?}",
                                scenario.name, scenario.id, e
                            );
                            println!("    ✗ {}", error_msg);
                            failed_files.push((path.to_path_buf(), error_msg));
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to load file: {:?}", e);
                println!("  ✗ {}", error_msg);
                failed_files.push((path.to_path_buf(), error_msg));
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Validation Summary:");
    println!("  Total scenarios validated: {}", total_scenarios);
    println!("  Failed files: {}", failed_files.len());

    if !failed_files.is_empty() {
        println!("\nFailed files:");
        for (path, error) in &failed_files {
            println!("  - {}: {}", path.display(), error);
        }
        panic!(
            "Scenario validation failed! {} file(s) have errors",
            failed_files.len()
        );
    }

    println!("\n✓ All scenarios validated successfully!");
}

/// Commands that are special keys handled outside the normal command buffer
/// These are processed directly by the UI without going through parse_command_buffer
const SPECIAL_KEY_COMMANDS: &[&str] = &[
    "Escape",     // Exit insert mode
    "Backspace",  // Delete character in insert mode
    "ArrowLeft",  // Cursor movement in insert mode
    "ArrowRight", // Cursor movement in insert mode
    "ArrowUp",    // Cursor movement in insert mode
    "ArrowDown",  // Cursor movement in insert mode
];

/// Helper to check if command is a modifier key command (Alt-*, Ctrl-*)
/// These are received as single key events, not character by character
fn is_modifier_key_command(cmd: &str) -> bool {
    cmd.starts_with("Alt-") || cmd.starts_with("Ctrl-")
}

/// Commands that enter insert mode
const INSERT_MODE_COMMANDS: &[&str] = &["i", "a", "I", "A", "o", "O", "c"];

/// Validate commands by simulating key-by-key input like the UI does
///
/// This function validates that each command in a scenario's solution can be
/// properly parsed character by character, just as the UI would process it.
/// It tracks insert mode state to properly handle text input vs commands.
///
/// Returns an error for the first invalid command found.
fn validate_commands_ui_style(commands: &[String]) -> Result<(), String> {
    let mut in_insert_mode = false;

    for (i, cmd) in commands.iter().enumerate() {
        // Handle insert mode content (old <insert:...> format)
        if cmd.starts_with("<insert:") {
            continue;
        }

        // Handle special key commands that bypass normal command buffer
        if SPECIAL_KEY_COMMANDS.contains(&cmd.as_str()) {
            if cmd == "Escape" {
                in_insert_mode = false;
            }
            continue;
        }

        // Handle modifier key commands (Alt-*, Ctrl-*) - these are single key events
        // They bypass character-by-character parsing since they're received atomically
        if is_modifier_key_command(cmd) {
            continue;
        }

        // In insert mode, single characters are text input, not commands
        if in_insert_mode {
            // Only single chars are valid in insert mode (text input)
            if cmd.len() == 1 {
                continue;
            }
            // Multi-char strings in insert mode should use <insert:...> format
            return Err(format!(
                "Command {} '{}': Multi-char command in insert mode should use <insert:...> format",
                i, cmd
            ));
        }

        // Check if this command enters insert mode
        if INSERT_MODE_COMMANDS.contains(&cmd.as_str()) {
            in_insert_mode = true;
        }

        // Validate command through command buffer parsing (normal mode)
        if let Err(e) = validate_single_command(cmd) {
            return Err(format!("Command {} '{}': {}", i, cmd, e));
        }
    }

    Ok(())
}

/// Validate a single normal mode command through command buffer parsing
fn validate_single_command(cmd: &str) -> Result<(), String> {
    let mut buffer = String::new();

    for (i, ch) in cmd.chars().enumerate() {
        buffer.push(ch);

        match parse_command_buffer(&buffer) {
            ParsedCommand::Complete(resolved) => {
                // Command resolved - check it matches expected
                if resolved != *cmd {
                    return Err(format!(
                        "Buffer resolved to '{}' but expected '{}' at char {} ('{}')",
                        resolved, cmd, i, ch
                    ));
                }
                return Ok(());
            }
            ParsedCommand::Partial => {
                // Still waiting for more input - continue
                if i == cmd.len() - 1 {
                    return Err("Left buffer in partial state - missing key handler".to_string());
                }
            }
            ParsedCommand::Invalid => {
                return Err(format!(
                    "Became invalid at char {} ('{}') - buffer: '{}'",
                    i, ch, buffer
                ));
            }
        }
    }

    // Buffer should have resolved by now
    Err("Never completed".to_string())
}

#[test]
fn test_all_scenarios_execute_solution() {
    let scenarios_dir = Path::new("scenarios");

    if !scenarios_dir.exists() {
        panic!("Scenarios directory not found: {:?}", scenarios_dir);
    }

    let loader = ScenarioLoader::new();
    let mut total_scenarios = 0;
    let mut failed_scenarios = Vec::new();

    // Walk through all .toml files
    for entry in WalkDir::new(scenarios_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("toml"))
    {
        let path = entry.path();

        if let Ok(scenarios) = loader.load(path) {
            for scenario in scenarios {
                total_scenarios += 1;

                // PHASE 1: Validate all commands are parseable key-by-key
                // This catches issues like 'gs' not being recognized when typed as 'g' then 's'
                if let Err(e) = validate_commands_ui_style(&scenario.solution.commands) {
                    failed_scenarios.push((
                        scenario.id.clone(),
                        format!("UI-style validation failed: {}", e),
                    ));
                }

                // PHASE 2: Execute solution and verify completion
                let session = match GameSession::new(scenario.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        failed_scenarios.push((
                            scenario.id.clone(),
                            format!("Failed to create session: {:?}", e),
                        ));
                        continue;
                    }
                };

                // Execute solution commands
                use helix_trainer::game::SessionAfterAction;
                let mut session_or_completed: Option<SessionAfterAction> =
                    Some(SessionAfterAction::StillActive(session));

                for (i, cmd) in scenario.solution.commands.iter().enumerate() {
                    if let Some(state) = session_or_completed.take() {
                        match state {
                            SessionAfterAction::StillActive(s) => {
                                // Extract count and base command (e.g., "3h" -> count=3, base_cmd="h")
                                let (count, base_cmd) = extract_count_and_command(cmd);

                                // Execute base command `count` times
                                let mut current_session = Some(s);
                                let mut execution_error = None;

                                for _ in 0..count {
                                    if let Some(active) = current_session.take() {
                                        match active.record_action(base_cmd.to_string()) {
                                            Ok(result) => match result {
                                                SessionAfterAction::Completed(c) => {
                                                    session_or_completed =
                                                        Some(SessionAfterAction::Completed(c));
                                                    break;
                                                }
                                                SessionAfterAction::StillActive(next) => {
                                                    current_session = Some(next);
                                                }
                                            },
                                            Err(e) => {
                                                execution_error = Some(e);
                                                break;
                                            }
                                        }
                                    }
                                }

                                // Handle execution error
                                if let Some(e) = execution_error {
                                    failed_scenarios.push((
                                        scenario.id.clone(),
                                        format!("Failed at command {} '{}': {:?}", i, cmd, e),
                                    ));
                                    break;
                                }

                                // If not completed, store remaining session
                                if session_or_completed.is_none()
                                    && let Some(remaining) = current_session
                                {
                                    session_or_completed =
                                        Some(SessionAfterAction::StillActive(remaining));
                                }

                                // If completed, stop processing commands
                                if matches!(
                                    session_or_completed,
                                    Some(SessionAfterAction::Completed(_))
                                ) {
                                    break;
                                }
                            }
                            SessionAfterAction::Completed(c) => {
                                // Already completed
                                session_or_completed = Some(SessionAfterAction::Completed(c));
                                break;
                            }
                        }
                    }
                }

                // Check if scenario is completed
                if let Some(final_state) = session_or_completed {
                    match final_state {
                        SessionAfterAction::Completed(_) => {
                            // Success!
                        }
                        SessionAfterAction::StillActive(s) => {
                            if !s.check_completion() {
                                failed_scenarios.push((
                                    scenario.id.clone(),
                                    format!(
                                        "Solution did not complete scenario. Current state:\n  Content: '{}'\n  Cursor: {:?}\n  Target content: '{}'\n  Target cursor: {:?}",
                                        s.current_state().content(),
                                        s.current_state().cursor_position(),
                                        s.target_state().content(),
                                        s.target_state().cursor_position()
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Execution Summary:");
    println!("  Total scenarios tested: {}", total_scenarios);
    println!("  Failed scenarios: {}", failed_scenarios.len());

    if !failed_scenarios.is_empty() {
        println!("\nFailed scenarios:");
        for (id, error) in &failed_scenarios {
            println!("  - {}: {}", id, error);
        }
        panic!(
            "Scenario execution failed! {} scenario(s) have errors",
            failed_scenarios.len()
        );
    }

    println!("\n✓ All scenario solutions execute successfully!");
}
