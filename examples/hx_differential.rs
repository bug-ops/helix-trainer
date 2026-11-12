//! Replay the golden table in a live `hx` and report drift.
//!
//! Every case of `src/helix/simulator/tests/hx_goldens.toml` is typed into a
//! local `hx` running in tmux (an 80x26 terminal, an empty configuration), and
//! the resulting document is compared with the recorded one. The simulator is
//! not involved: this checks that the table still says what `hx` does, so it
//! is the drift check to run when the pinned `helix-core` changes.
//!
//! Needs `hx` and `tmux` on `PATH`; it is never part of CI.
//!
//! Run with: `cargo run --example hx_differential [-- [--write] [--filter TEXT]]`
//!
//! `--write` replaces the recorded text of every replayed case with what `hx`
//! produced and rewrites the table.

use helix_trainer::helix::Language;
use helix_trainer::helix::goldens::{GoldenCase, GoldenTable};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread::sleep;
use std::time::Duration;

const TABLE: &str = "src/helix/simulator/tests/hx_goldens.toml";
const HEADER: &str = "# Cases recorded from hx 25.07.1 (see examples/hx_differential.rs).\n\
# Regenerate the expected text with `cargo run --example hx_differential -- --write`.\n\n";
const SETTLE: Duration = Duration::from_millis(60);

/// One thing to send to tmux.
enum Send {
    /// Text typed as it is.
    Literal(String),
    /// A named tmux key such as `Escape` or `C-w`.
    Named(String),
}

/// Prompt commands: the key opens a prompt that the rest of the string fills.
fn prompt_split(key: &str) -> Option<(&str, &str)> {
    ["/", "?", "s", "S", "K", "Alt-K", ":"]
        .iter()
        .find_map(|prompt| {
            let rest = key.strip_prefix(prompt)?;
            let rest = if *prompt == ":" {
                rest
            } else {
                rest.strip_prefix(' ')?
            };
            (!rest.is_empty()).then_some((*prompt, rest))
        })
}

fn named(key: &str) -> Option<String> {
    let simple = match key {
        "Escape" => "Escape",
        "Backspace" => "BSpace",
        "Delete" => "DC",
        "Left" => "Left",
        "Right" => "Right",
        "Up" => "Up",
        "Down" => "Down",
        "\t" => "Tab",
        "\n" => "Enter",
        _ => "",
    };
    if !simple.is_empty() {
        return Some(simple.to_string());
    }
    let (prefix, rest) = key.split_once('-')?;
    let mut chars = rest.chars();
    let (Some(ch), None) = (chars.next(), chars.next()) else {
        return None;
    };
    match prefix {
        "Ctrl" => Some(format!("C-{ch}")),
        "Alt" => Some(format!("M-{ch}")),
        _ => None,
    }
}

fn sends(key: &str) -> Vec<Send> {
    if let Some(name) = named(key) {
        return vec![Send::Named(name)];
    }
    match prompt_split(key) {
        Some((prompt, text)) => {
            let mut out = sends(prompt);
            out.push(Send::Literal(text.to_string()));
            out.push(Send::Named("Enter".to_string()));
            out
        }
        None => key
            .chars()
            .map(|ch| Send::Literal(ch.to_string()))
            .collect(),
    }
}

fn tmux(args: &[&str]) -> Result<(), String> {
    let status = Command::new("tmux")
        .args(args)
        .status()
        .map_err(|error| format!("tmux: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("tmux {args:?} failed"))
}

fn send(session: &str, what: &Send) -> Result<(), String> {
    match what {
        Send::Literal(text) => tmux(&["send-keys", "-t", session, "-l", text]),
        Send::Named(name) => tmux(&["send-keys", "-t", session, name]),
    }?;
    sleep(SETTLE);
    Ok(())
}

/// `path` as one single-quoted shell word.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn extension(language: Language) -> &'static str {
    match language {
        Language::Rs => "rs",
        Language::Md => "md",
        Language::Py => "py",
    }
}

/// Type the case into `hx` and return the file it saved.
fn run_in_hx(case: &GoldenCase, scratch: &Path) -> Result<String, String> {
    let file = scratch.join(format!("case.{}", extension(case.language)));
    let config = scratch.join("empty.toml");
    std::fs::write(&file, &case.text).map_err(|error| error.to_string())?;
    std::fs::write(&config, "").map_err(|error| error.to_string())?;
    let session = format!("hxdiff{}", std::process::id());
    let command = format!(
        "hx -c {} {} 2>/dev/null",
        shell_quote(&config),
        shell_quote(&file)
    );
    tmux(&[
        "new-session",
        "-d",
        "-s",
        &session,
        "-x",
        "80",
        "-y",
        "26",
        &command,
    ])?;
    let result = (|| {
        sleep(Duration::from_millis(800));
        for setup in [
            ":set-option auto-format false",
            ":set-option insert-final-newline false",
        ] {
            send(&session, &Send::Literal(setup.to_string()))?;
            send(&session, &Send::Named("Enter".to_string()))?;
        }
        for key in &case.keys {
            for what in sends(key) {
                send(&session, &what)?;
            }
        }
        sleep(Duration::from_millis(400));
        for key in case.probe.keys() {
            for what in sends(key) {
                send(&session, &what)?;
            }
        }
        sleep(Duration::from_millis(300));
        for command in [":w!", ":q!"] {
            send(&session, &Send::Literal(command.to_string()))?;
            send(&session, &Send::Named("Enter".to_string()))?;
            sleep(Duration::from_millis(300));
        }
        std::fs::read_to_string(&file).map_err(|error| error.to_string())
    })();
    let _ = tmux(&["kill-session", "-t", &session]);
    result
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let write = args.iter().any(|arg| arg == "--write");
    let filter = args
        .iter()
        .position(|arg| arg == "--filter")
        .and_then(|at| args.get(at + 1));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(TABLE);
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut table = match GoldenTable::parse(&source) {
        Ok(table) => table,
        Err(error) => {
            eprintln!("cannot parse {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let scratch_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("cannot create a temporary directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    let scratch = scratch_dir.path();

    let mut drift = 0_usize;
    for case in &mut table.cases {
        if filter.is_some_and(|text| !case.id.contains(text.as_str())) {
            continue;
        }
        match run_in_hx(case, scratch) {
            Ok(actual) if actual == case.expected => println!("ok     {}", case.id),
            Ok(actual) => {
                drift += 1;
                println!(
                    "DRIFT  {}\n  recorded: {:?}\n  hx:       {:?}",
                    case.id, case.expected, actual
                );
                if write {
                    case.expected = actual;
                }
            }
            Err(error) => {
                drift += 1;
                println!("ERROR  {}: {error}", case.id);
            }
        }
    }

    if write {
        match table.render() {
            Ok(text) => {
                if let Err(error) = std::fs::write(&path, format!("{HEADER}{text}")) {
                    eprintln!("cannot write {}: {error}", path.display());
                    return ExitCode::FAILURE;
                }
                println!("table written");
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("cannot render the table: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if drift == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{drift} case(s) drifted");
        ExitCode::FAILURE
    }
}
