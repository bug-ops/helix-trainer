//! Runs the built binary to check `--version`/`--help` never touch the
//! terminal (#452). stdin is `/dev/null`-like (null), so reaching the TUI
//! would fail.

use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_helix-trainer"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("binary should launch")
}

#[test]
fn version_prints_and_exits_zero() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag]);
        assert_eq!(out.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim(),
            format!("helix-trainer {}", env!("CARGO_PKG_VERSION"))
        );
        assert!(!out.stdout.contains(&0x1b), "no escape sequences on stdout");
    }
}

#[test]
fn help_prints_usage_and_exits_zero() {
    for flag in ["--help", "-h"] {
        let out = run(&[flag]);
        assert_eq!(out.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&out.stdout).contains("USAGE"));
    }
}

#[test]
fn unknown_argument_exits_two_naming_it() {
    let out = run(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--bogus"));
    assert_eq!(out.stdout, [] as [u8; 0]);
}

#[test]
fn trailing_argument_after_version_is_an_error() {
    let out = run(&["--version", "extra"]);
    assert_eq!(out.status.code(), Some(2));
}
