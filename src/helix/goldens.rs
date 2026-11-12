//! The golden table: cases recorded from a real `hx` and replayed here.
//!
//! One TOML table holds every recorded case. The library differential test
//! replays them in the simulator, and `examples/hx_differential.rs` replays
//! them in a live `hx` through tmux to report drift and to regenerate the
//! expected text. This module is the shared model of both.

use super::{AnyModeSimulator, Language, error::SimulatorError};
use crate::grammar::commands::{CMD_CHANGE, CMD_DELETE_SELECTION, CMD_ESCAPE};
use serde::{Deserialize, Serialize};

/// What is typed after a case's keys to make their effect visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Probe {
    /// `c @ Escape`: the selection the keys left behind is replaced by `@`.
    Selection,
    /// `d`: the selection the keys left behind is deleted.
    Delete,
    /// Nothing: the text as the keys left it.
    None,
}

impl Probe {
    /// The keys of this probe, in typing order.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Selection => &[CMD_CHANGE, "@", CMD_ESCAPE],
            Self::Delete => &[CMD_DELETE_SELECTION],
            Self::None => &[],
        }
    }
}

/// One recorded case: a document, the keys typed into it, and the text `hx`
/// ended up with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenCase {
    /// Unique name of the case.
    pub id: String,
    /// The document the keys start from.
    pub text: String,
    /// Language of the document; selects the file extension in `hx`.
    pub language: Language,
    /// Keys in the simulator's command spelling (`"Escape"`, `"Ctrl-w"`,
    /// `"/ pattern"` for a prompt, `"3o"` for a counted command).
    pub keys: Vec<String>,
    /// The probe typed after `keys`.
    pub probe: Probe,
    /// The document as `hx` left it.
    pub expected: String,
}

impl GoldenCase {
    /// Run the case in the simulator and return the resulting document.
    ///
    /// # Errors
    ///
    /// Returns the first [`SimulatorError`] a key produces.
    pub fn simulate(&self) -> Result<String, SimulatorError> {
        let mut sim = AnyModeSimulator::new(self.text.clone()).with_language(self.language);
        for key in self
            .keys
            .iter()
            .map(String::as_str)
            .chain(self.probe.keys().iter().copied())
        {
            sim.execute_command(key)?;
        }
        Ok(sim.display().content())
    }
}

/// Every golden case, as stored in `hx_goldens.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenTable {
    /// The cases, in file order.
    #[serde(rename = "case")]
    pub cases: Vec<GoldenCase>,
}

impl GoldenTable {
    /// Parse the table from its TOML text.
    ///
    /// # Errors
    ///
    /// Returns the TOML error when the text is not a valid table.
    pub fn parse(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }

    /// The table as TOML text, the inverse of [`Self::parse`].
    ///
    /// # Errors
    ///
    /// Returns the TOML error when a value cannot be written.
    pub fn render(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_round_trips_through_toml() {
        let table = GoldenTable {
            cases: vec![GoldenCase {
                id: "x".to_string(),
                text: "ab\n".to_string(),
                language: Language::Rs,
                keys: vec!["l".to_string(), "Escape".to_string()],
                probe: Probe::Selection,
                expected: "a@\n".to_string(),
            }],
        };
        assert_eq!(GoldenTable::parse(&table.render().unwrap()).unwrap(), table);
    }

    #[test]
    fn simulate_appends_the_probe() {
        let case = GoldenCase {
            id: "x".to_string(),
            text: "ab".to_string(),
            language: Language::Rs,
            keys: vec!["l".to_string()],
            probe: Probe::Delete,
            expected: "a".to_string(),
        };
        assert_eq!(case.simulate().unwrap(), "a");
    }
}
