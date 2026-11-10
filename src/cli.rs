//! Command-line argument parsing.
//!
//! The grammar is deliberately tiny (`--version`/`-V`, `--help`/`-h`), so the
//! parser is a pure function of the argument list rather than a dependency.
//! It runs before any terminal, config or data-loader work.

use std::{ffi::OsString, fmt};

/// What the process should do, decided from its command-line arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    /// No arguments: start the TUI.
    Run,
    /// Print the version and exit successfully.
    Version,
    /// Print usage and exit successfully.
    Help,
    /// Reject the invocation.
    Error(CliError),
}

/// Why an argument list was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// The argument is not a recognized flag, or follows `--version`/`--help`.
    UnknownArgument(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(arg) => {
                write!(
                    f,
                    "unknown argument '{arg}'\nTry 'helix-trainer --help' for usage."
                )
            }
        }
    }
}

/// Exit code used for rejected invocations.
pub const USAGE_EXIT_CODE: i32 = 2;

/// Text printed by `--version`.
pub fn version_text() -> String {
    format!("helix-trainer {}", env!("CARGO_PKG_VERSION"))
}

/// Text printed by `--help`.
pub fn help_text() -> String {
    format!(
        "{}\n\nUSAGE:\n    helix-trainer [OPTIONS]\n\nOPTIONS:\n    -h, --help       Print \
         help and exit\n    -V, --version    Print version and exit",
        version_text()
    )
}

/// Parse the arguments that follow the program name.
///
/// The first argument decides the action; any further argument after
/// `--version` or `--help` is an error, as is any unrecognized argument.
pub fn parse_args<I>(args: I) -> CliAction
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return CliAction::Run;
    };
    let action = match first.to_str() {
        Some("--version" | "-V") => CliAction::Version,
        Some("--help" | "-h") => CliAction::Help,
        _ => return unknown(&first),
    };
    match args.next() {
        None => action,
        Some(extra) => unknown(&extra),
    }
}

fn unknown(arg: &OsString) -> CliAction {
    CliAction::Error(CliError::UnknownArgument(
        arg.to_string_lossy().into_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> CliAction {
        parse_args(args.iter().map(OsString::from))
    }

    fn unknown_arg(arg: &str) -> CliAction {
        CliAction::Error(CliError::UnknownArgument(arg.to_owned()))
    }

    #[test]
    fn no_arguments_runs() {
        assert_eq!(parse(&[]), CliAction::Run);
    }

    #[test]
    fn version_flags() {
        assert_eq!(parse(&["--version"]), CliAction::Version);
        assert_eq!(parse(&["-V"]), CliAction::Version);
    }

    #[test]
    fn help_flags() {
        assert_eq!(parse(&["--help"]), CliAction::Help);
        assert_eq!(parse(&["-h"]), CliAction::Help);
    }

    #[test]
    fn unknown_and_empty_arguments_are_errors() {
        assert_eq!(parse(&["--bogus"]), unknown_arg("--bogus"));
        assert_eq!(parse(&[""]), unknown_arg(""));
    }

    #[test]
    fn extra_arguments_after_known_flag_are_errors() {
        assert_eq!(parse(&["--version", "extra"]), unknown_arg("extra"));
        assert_eq!(parse(&["--version", "--help"]), unknown_arg("--help"));
        assert_eq!(parse(&["--help", "x"]), unknown_arg("x"));
    }

    #[test]
    fn error_message_names_argument_and_hints_help() {
        let msg = CliError::UnknownArgument("--bogus".into()).to_string();
        assert!(msg.contains("--bogus") && msg.contains("--help"));
    }

    #[test]
    fn version_text_uses_package_version() {
        assert_eq!(
            version_text(),
            format!("helix-trainer {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
