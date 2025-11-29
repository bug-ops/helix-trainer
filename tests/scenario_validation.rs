//! Integration tests for validating all scenario files
//!
//! This test suite loads and validates every scenario in the scenarios/ directory
//! to ensure they are correctly defined and can be executed.

use helix_trainer::config::ScenarioLoader;
use helix_trainer::game::GameSession;
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

                // Create game session
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
                                match s.record_action(cmd.clone()) {
                                    Ok(result) => {
                                        session_or_completed = Some(result);
                                        // If completed, stop
                                        if matches!(
                                            session_or_completed,
                                            Some(SessionAfterAction::Completed(_))
                                        ) {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        failed_scenarios.push((
                                            scenario.id.clone(),
                                            format!("Failed at command {} '{}': {:?}", i, cmd, e),
                                        ));
                                        break;
                                    }
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
