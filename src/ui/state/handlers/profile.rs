//! Profile and statistics message handlers
//!
//! Handles profile screen navigation and XP awards

use crate::security::UserError;
use crate::ui::state::{AppState, Screen};

/// Handle ShowProfile message
///
/// Navigates to the profile screen
pub fn handle_show_profile(state: &mut AppState) -> Result<(), UserError> {
    state.screen = Screen::Profile;
    Ok(())
}

/// Handle ShowStatistics message
///
/// Navigates to the statistics screen
pub fn handle_show_statistics(state: &mut AppState) -> Result<(), UserError> {
    state.screen = Screen::Statistics;
    Ok(())
}

/// Handle AwardXP message
///
/// Awards XP to the user profile and saves if level up occurs
pub fn handle_award_xp(state: &mut AppState, amount: u64) -> Result<(), UserError> {
    let mut profile = state.profile.borrow_mut();
    let leveled_up = profile.add_xp(amount);

    if leveled_up {
        drop(profile); // Release borrow before save
        state
            .save_profile_immediate()
            .map_err(|_| UserError::OperationFailed)?;
    }
    Ok(())
}
