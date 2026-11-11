//! Integration tests for validating all scenario files
//!
//! This test suite validates every embedded scenario to ensure they are
//! correctly defined and can be executed.
//!
//! IMPORTANT: The `test_all_scenarios_execute_solution` test validates commands
//! key-by-key, exactly as the UI does. This catches issues like multi-key commands
//! (gs, gg, ge) that would fail in the UI due to missing command buffer handling.

use helix_trainer::config::{QuestLoader, QuestSpec, ScenarioLoader};
use helix_trainer::game::{GameSession, PlayableScenario};
use helix_trainer::helix::commands::{
    CMD_REPLAY_MACRO, CMD_SELECT_REGEX, CMD_SPLIT_SELECTION, CMD_TOGGLE_MACRO_RECORDING,
};
use helix_trainer::helix::simulator::command_line::CommandLine;
use helix_trainer::input::typestate::{HandlerResult, InputStateMachine, command_to_key_event};

#[test]
fn test_all_scenarios_load_successfully() {
    let loader = ScenarioLoader::new();
    let mut total_scenarios = 0;
    let mut failed_scenarios = Vec::new();

    // Load embedded scenarios for "en" locale
    println!("\nValidating embedded scenarios for locale 'en'");

    match loader.load_from_embedded("en") {
        Ok(scenarios) => {
            println!(
                "  ✓ Loaded {} scenarios from embedded data",
                scenarios.len()
            );
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
                        println!("    ✗ {error_msg}");
                        failed_scenarios.push((scenario.id.clone(), error_msg));
                    }
                }
            }
        }
        Err(e) => {
            panic!("Failed to load embedded scenarios: {e:?}");
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Validation Summary:");
    println!("  Total scenarios validated: {total_scenarios}");
    println!("  Failed scenarios: {}", failed_scenarios.len());

    if !failed_scenarios.is_empty() {
        println!("\nFailed scenarios:");
        for (id, error) in &failed_scenarios {
            println!("  - {id}: {error}");
        }
        panic!(
            "Scenario validation failed! {} scenario(s) have errors",
            failed_scenarios.len()
        );
    }

    println!("\n✓ All embedded scenarios validated successfully!");
}

#[test]
fn test_all_scenario_ids_are_unique() {
    use std::collections::HashMap;

    let loader = ScenarioLoader::new();
    let scenarios = loader
        .load_from_embedded("en")
        .expect("Failed to load embedded scenarios");

    let mut seen: HashMap<helix_trainer::ids::ScenarioId, Vec<String>> = HashMap::new();
    for scenario in &scenarios {
        seen.entry(scenario.id.clone())
            .or_default()
            .push(scenario.name.clone());
    }

    let duplicates: Vec<String> = seen
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|(id, names)| format!("'{}' used by: {}", id, names.join(", ")))
        .collect();

    assert!(
        duplicates.is_empty(),
        "Duplicate scenario ids found (completion tracking is keyed by id, so \
         duplicates silently alias progress between different scenarios):\n  {}",
        duplicates.join("\n  ")
    );
}

/// Cross-checks every scenario id referenced by a quest (`SpeedRun`'s
/// `scenario_id` and `conditions.requires_scenarios`) against the loaded
/// scenario collection, so renaming or removing a scenario id can't silently
/// leave a quest pointing at nothing.
#[test]
fn test_quest_scenario_references_exist() {
    use std::collections::HashSet;

    let scenario_ids: HashSet<helix_trainer::ids::ScenarioId> = ScenarioLoader::new()
        .load_from_embedded("en")
        .expect("Failed to load embedded scenarios")
        .into_iter()
        .map(|s| s.id)
        .collect();

    let templates = QuestLoader::default()
        .load_for_locale("en")
        .expect("Failed to load quest templates for locale 'en'");

    let mut dangling = Vec::new();
    for template in &templates {
        if let QuestSpec::SpeedRun { scenario_id, .. } = &template.spec
            && !scenario_ids.contains(scenario_id)
        {
            dangling.push(format!(
                "quest '{}': spec.scenario_id '{}'",
                template.id, scenario_id
            ));
        }

        for scenario_id in &template.conditions.requires_scenarios {
            if !scenario_ids.contains(scenario_id) {
                dangling.push(format!(
                    "quest '{}': conditions.requires_scenarios '{}'",
                    template.id, scenario_id
                ));
            }
        }
    }

    assert!(
        dangling.is_empty(),
        "Quest(s) reference scenario id(s) that don't exist in the loaded scenario \
         collection (a renamed or removed scenario id silently orphans the quest):\n  {}",
        dangling.join("\n  ")
    );
}

/// Commands that are special keys handled outside the normal command buffer
/// These are processed directly by the UI without going through the input state machine
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

        // Handle ':'-prefixed command-line invocations - these are assembled
        // atomically by `CommandLinePending` (not resolved char-by-char through
        // the KeyTrie, which has no notion of a command-line buffer), so
        // validate them by parsing directly instead.
        if cmd.starts_with(':') {
            if let Err(e) = CommandLine::parse(cmd) {
                return Err(format!(
                    "Command {i} '{cmd}': :-command failed to parse: {e:?}"
                ));
            }
            continue;
        }

        // Handle 'q'/'Q' (macro record toggle/replay) - dispatched directly
        // by `execute_command_any_mode`'s early return, not the
        // registry-backed KeyTrie that the input state machine walks, so
        // char-by-char validation has no entry for them.
        if cmd == CMD_TOGGLE_MACRO_RECORDING || cmd == CMD_REPLAY_MACRO {
            continue;
        }

        // Handle "s <pattern>" / "S <pattern>" regex-selection prompt
        // invocations - assembled atomically by `RegexPromptPending` (also
        // not resolved char-by-char through the KeyTrie), so validate by
        // checking the pattern compiles as a regex directly instead.
        let regex_pattern = cmd
            .strip_prefix(CMD_SELECT_REGEX)
            .and_then(|rest| rest.strip_prefix(' '))
            .or_else(|| {
                cmd.strip_prefix(CMD_SPLIT_SELECTION)
                    .and_then(|rest| rest.strip_prefix(' '))
            });
        if let Some(pattern) = regex_pattern {
            if let Err(e) = helix_stdx::rope::Regex::new(pattern) {
                return Err(format!(
                    "Command {i} '{cmd}': regex-selection pattern failed to compile: {e:?}"
                ));
            }
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
                "Command {i} '{cmd}': Multi-char command in insert mode should use <insert:...> format"
            ));
        }

        // Check if this command enters insert mode
        if INSERT_MODE_COMMANDS.contains(&cmd.as_str()) {
            in_insert_mode = true;
        }

        // Validate command through command buffer parsing (normal mode)
        if let Err(e) = validate_single_command(cmd) {
            return Err(format!("Command {i} '{cmd}': {e}"));
        }
    }

    Ok(())
}

/// Validate a single normal mode command by feeding it key by key through the same
/// `InputStateMachine` the gameplay screens use
fn validate_single_command(cmd: &str) -> Result<(), String> {
    let mut machine = InputStateMachine::new();
    let last = cmd.chars().count().saturating_sub(1);

    for (i, ch) in cmd.chars().enumerate() {
        match machine.process_key(command_to_key_event(&ch.to_string())) {
            HandlerResult::Execute(resolved) => {
                if resolved.as_ref() != cmd {
                    return Err(format!(
                        "Input resolved to '{resolved}' but expected '{cmd}' at char {i} ('{ch}')"
                    ));
                }
                return Ok(());
            }
            HandlerResult::Transition(_) => {
                if i == last {
                    return Err("Left input in a pending state - missing key handler".to_string());
                }
            }
            HandlerResult::Cancel | HandlerResult::Stay => {
                return Err(format!("Became invalid at char {i} ('{ch}')"));
            }
        }
    }

    Err("Never completed".to_string())
}

#[test]
fn test_all_scenario_command_line_entries_parse() {
    let loader = ScenarioLoader::new();
    let scenarios = loader
        .load_from_embedded("en")
        .expect("Failed to load embedded scenarios");

    let mut checked = 0;
    for scenario in &scenarios {
        for cmd in &scenario.solution.commands {
            if let Some(colon_cmd) = cmd.strip_prefix(':').map(|_| cmd) {
                CommandLine::parse(colon_cmd).unwrap_or_else(|e| {
                    panic!(
                        "scenario '{}': ':' entry '{}' failed to parse: {:?}",
                        scenario.id, colon_cmd, e
                    )
                });
                checked += 1;
            }
        }
    }

    assert!(
        checked > 0,
        "expected at least one ':'-prefixed solution entry across all scenarios"
    );
}

#[test]
fn test_after_taught_commands_scenarios_declare_taught_commands() {
    use helix_trainer::config::CompletionRule;

    let scenarios = ScenarioLoader::new()
        .load_from_embedded("en")
        .expect("Failed to load embedded scenarios");
    for scenario in scenarios {
        let Some(metadata) = scenario.metadata.as_ref() else {
            continue;
        };
        if metadata.completion == CompletionRule::AfterTaughtCommands {
            assert!(
                !metadata.commands_taught.is_empty(),
                "{}: after_taught_commands needs non-empty commands_taught",
                scenario.id
            );
        }
    }
}

#[test]
fn test_all_scenarios_execute_solution() {
    use helix_trainer::game::SessionAfterAction;
    let loader = ScenarioLoader::new();
    let mut total_scenarios = 0;
    let mut failed_scenarios = Vec::new();

    // Load embedded scenarios for "en" locale
    let scenarios = loader
        .load_from_embedded("en")
        .expect("Failed to load embedded scenarios");

    for scenario in scenarios {
        total_scenarios += 1;

        // PHASE 1: Validate all commands are parseable key-by-key
        // This catches issues like 'gs' not being recognized when typed as 'g' then 's'
        if let Err(e) = validate_commands_ui_style(&scenario.solution.commands) {
            failed_scenarios.push((
                scenario.id.clone(),
                format!("UI-style validation failed: {e}"),
            ));
        }

        // PHASE 2: Execute solution and verify completion
        let session = match GameSession::new(scenario.clone()) {
            Ok(s) => s,
            Err(e) => {
                failed_scenarios.push((
                    scenario.id.clone(),
                    format!("Failed to create session: {e:?}"),
                ));
                continue;
            }
        };

        // Execute solution commands
        let mut session_or_completed: Option<SessionAfterAction> =
            Some(SessionAfterAction::StillActive(session));

        for (i, cmd) in scenario.solution.commands.iter().enumerate() {
            if let Some(state) = session_or_completed.take() {
                match state {
                    SessionAfterAction::StillActive(s) => {
                        // The full command string, count prefix included, is one action
                        let mut current_session = None;
                        let mut execution_error = None;

                        match s.record_action(cmd.clone()) {
                            Ok(SessionAfterAction::Completed(c)) => {
                                session_or_completed = Some(SessionAfterAction::Completed(c));
                            }
                            Ok(SessionAfterAction::StillActive(next)) => {
                                current_session = Some(next);
                            }
                            Err(e) => execution_error = Some(e),
                        }

                        // Handle execution error
                        if let Some(e) = execution_error {
                            failed_scenarios.push((
                                scenario.id.clone(),
                                format!("Failed at command {i} '{cmd}': {e:?}"),
                            ));
                            break;
                        }

                        // If not completed, store remaining session
                        if session_or_completed.is_none()
                            && let Some(remaining) = current_session
                        {
                            session_or_completed = Some(SessionAfterAction::StillActive(remaining));
                        }

                        // If completed, stop processing commands
                        if matches!(session_or_completed, Some(SessionAfterAction::Completed(_))) {
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
                SessionAfterAction::Completed(done) => {
                    if done.completion_progress() != 100 {
                        failed_scenarios.push((
                            scenario.id.clone(),
                            format!("final progress {} != 100", done.completion_progress()),
                        ));
                    }
                    if done.action_count() != scenario.scoring.optimal_count.get() {
                        failed_scenarios.push((
                            scenario.id.clone(),
                            format!(
                                "actions {} != optimal_count {}",
                                done.action_count(),
                                scenario.scoring.optimal_count
                            ),
                        ));
                    }
                }
                SessionAfterAction::StillActive(s) => {
                    if !s.check_completion() {
                        failed_scenarios.push((
                            scenario.id.clone(),
                            format!(
                                "Solution did not complete scenario.\n  Current: content='{}', cursor={:?}, selection={:?}\n  Target:  content='{}', cursor={:?}, selection={:?}",
                                s.current_content(),
                                s.current_cursor(),
                                s.current_selection(),
                                s.target_content(),
                                s.target_cursor(),
                                s.target_selection()
                            ),
                        ));
                    }
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Execution Summary:");
    println!("  Total scenarios tested: {total_scenarios}");
    println!("  Failed scenarios: {}", failed_scenarios.len());

    if !failed_scenarios.is_empty() {
        println!("\nFailed scenarios:");
        for (id, error) in &failed_scenarios {
            println!("  - {id}: {error}");
        }
        panic!(
            "Scenario execution failed! {} scenario(s) have errors",
            failed_scenarios.len()
        );
    }

    println!("\n✓ All scenario solutions execute successfully!");
}

/// SC-014 / FR-153: deserialization canonicalizes `commands_taught`, so a bundled value that is
/// not already in normalized form would silently change meaning. Reads the raw TOML.
#[test]
fn test_bundled_commands_taught_are_normalized() {
    let mut checked = 0;
    for entry in walkdir::WalkDir::new("scenarios")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
    {
        let raw = std::fs::read_to_string(entry.path()).unwrap();
        let file: toml::Value = toml::from_str(&raw).unwrap();
        for scenario in file["scenarios"].as_array().into_iter().flatten() {
            let taught = scenario["metadata"]["commands_taught"].as_array();
            for command in taught.into_iter().flatten().filter_map(toml::Value::as_str) {
                assert_eq!(
                    helix_trainer::helix::commands::normalize_command_id(command),
                    command,
                    "{}: commands_taught value is not normalized",
                    entry.path().display()
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "no commands_taught values found");
}
