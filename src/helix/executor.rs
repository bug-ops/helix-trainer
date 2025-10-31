//! Command executor trait for decoupling game logic from simulator implementation

use crate::game::EditorState;
use crate::security::UserError;

/// Mode of the editor
///
/// Re-export from simulator for convenience
pub use super::simulator::Mode;

/// Trait for executing editor commands
///
/// This trait decouples the game logic from the concrete HelixSimulator
/// implementation, enabling easier testing and potential alternative backends.
///
/// # Examples
///
/// ```ignore
/// fn process_command<E: CommandExecutor>(executor: &mut E, cmd: &str) -> Result<(), UserError> {
///     executor.execute_command(cmd)?;
///     let state = executor.to_editor_state()?;
///     println!("Cursor at: {:?}", state.cursor_position());
///     Ok(())
/// }
/// ```
pub trait CommandExecutor {
    /// Execute a Helix editor command
    ///
    /// # Arguments
    ///
    /// * `cmd` - The command string to execute (e.g., "h", "dd", "i")
    ///
    /// # Errors
    ///
    /// Returns `UserError` if the command is invalid or execution fails
    fn execute_command(&mut self, cmd: &str) -> Result<(), UserError>;

    /// Get the current editor state
    ///
    /// # Errors
    ///
    /// Returns `UserError` if state extraction fails
    fn to_editor_state(&self) -> Result<EditorState, UserError>;

    /// Get the current mode (Normal or Insert)
    fn mode(&self) -> Mode;
}
