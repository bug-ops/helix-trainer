//! Async state management for progressive data loading
//!
//! This module provides type-safe async state tracking for resources that
//! are loaded asynchronously in the background. It uses a three-state model
//! (Loading, Ready, Failed) to ensure compile-time safety for resource access.

use crate::config::Scenario;
use crate::gamification::{QuestTemplateRegistry, UserProfile};

/// Represents the loading state of an async resource
///
/// Designed for compile-time safety: operations on data can only be
/// performed in the Ready state.
///
/// # Examples
///
/// ```
/// use helix_trainer::async_state::AsyncState;
///
/// let mut state: AsyncState<i32> = AsyncState::Loading;
/// assert!(state.is_loading());
///
/// state = AsyncState::Ready(42);
/// assert!(state.is_ready());
/// assert_eq!(state.as_ref(), Some(&42));
/// ```
#[derive(Debug, Clone, Default)]
pub enum AsyncState<T> {
    /// Resource is being loaded
    #[default]
    Loading,

    /// Resource loaded successfully
    Ready(T),

    /// Loading failed with error message
    Failed(String),
}

impl<T> AsyncState<T> {
    /// Check if data is ready
    pub fn is_ready(&self) -> bool {
        matches!(self, AsyncState::Ready(_))
    }

    /// Check if still loading
    pub fn is_loading(&self) -> bool {
        matches!(self, AsyncState::Loading)
    }

    /// Check if loading failed
    pub fn is_failed(&self) -> bool {
        matches!(self, AsyncState::Failed(_))
    }

    /// Get reference to data if ready
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            AsyncState::Ready(data) => Some(data),
            _ => None,
        }
    }

    /// Get mutable reference to data if ready
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            AsyncState::Ready(data) => Some(data),
            _ => None,
        }
    }

    /// Map the inner value if ready
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> AsyncState<U> {
        match self {
            AsyncState::Loading => AsyncState::Loading,
            AsyncState::Ready(data) => AsyncState::Ready(f(data)),
            AsyncState::Failed(err) => AsyncState::Failed(err),
        }
    }
}

/// Messages sent from background loaders to main event loop
///
/// These messages communicate the results of async operations to the
/// main application state.
#[derive(Debug)]
pub enum DataLoadMessage {
    /// Scenarios loaded successfully
    ScenariosReady(Vec<Scenario>),

    /// Scenarios failed to load
    ScenariosError(String),

    /// Profile loaded successfully
    ProfileReady(UserProfile),

    /// Profile failed to load (with fallback new profile)
    ProfileError {
        error: String,
        fallback: UserProfile,
    },

    /// Quest registry loaded (lazy, for Profile screen)
    QuestRegistryReady(QuestTemplateRegistry),

    /// Quest registry failed
    QuestRegistryError(String),

    /// Profile saved successfully
    ProfileSaved,

    /// Profile save failed
    ProfileSaveError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_state_default_is_loading() {
        let state: AsyncState<i32> = AsyncState::default();
        assert!(state.is_loading());
        assert!(!state.is_ready());
        assert!(!state.is_failed());
    }

    #[test]
    fn test_async_state_loading() {
        let state: AsyncState<i32> = AsyncState::Loading;
        assert!(state.is_loading());
        assert!(!state.is_ready());
        assert!(!state.is_failed());
        assert_eq!(state.as_ref(), None);
    }

    #[test]
    fn test_async_state_ready() {
        let state = AsyncState::Ready(42);
        assert!(!state.is_loading());
        assert!(state.is_ready());
        assert!(!state.is_failed());
        assert_eq!(state.as_ref(), Some(&42));
    }

    #[test]
    fn test_async_state_failed() {
        let state: AsyncState<i32> = AsyncState::Failed("error".to_string());
        assert!(!state.is_loading());
        assert!(!state.is_ready());
        assert!(state.is_failed());
        assert_eq!(state.as_ref(), None);
    }

    #[test]
    fn test_async_state_as_mut() {
        let mut state = AsyncState::Ready(42);
        if let Some(value) = state.as_mut() {
            *value = 100;
        }
        assert_eq!(state.as_ref(), Some(&100));
    }

    #[test]
    fn test_async_state_map() {
        let state = AsyncState::Ready(42);
        let mapped = state.map(|x| x * 2);
        assert_eq!(mapped.as_ref(), Some(&84));

        let loading: AsyncState<i32> = AsyncState::Loading;
        let mapped_loading = loading.map(|x| x * 2);
        assert!(mapped_loading.is_loading());

        let failed: AsyncState<i32> = AsyncState::Failed("error".to_string());
        let mapped_failed = failed.map(|x| x * 2);
        assert!(mapped_failed.is_failed());
    }
}
