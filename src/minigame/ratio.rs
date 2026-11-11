//! Unit-carrying fraction type for time and progress values.

/// A fraction in `0.0..=1.0` (not a percentage).
///
/// Distinguishes "fraction of the time limit used" from the 0-100 percentages
/// shown in the UI; convert explicitly with [`Ratio::as_percent`].
///
/// # Examples
///
/// ```
/// use helix_trainer::minigame::Ratio;
///
/// assert_eq!(Ratio::new(1.7), Ratio::ONE);
/// assert_eq!(Ratio::new(-0.2), Ratio::ZERO);
/// assert_eq!(Ratio::new(0.456).as_percent(), 46);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Ratio(f64);

impl Ratio {
    /// No part of the whole.
    pub const ZERO: Self = Self(0.0);
    /// The whole.
    pub const ONE: Self = Self(1.0);

    /// Clamps `value` into `0.0..=1.0`; NaN (e.g. `0.0 / 0.0` for an empty limit) becomes
    /// [`Ratio::ONE`], since a zero-length limit is already exhausted.
    #[must_use]
    pub fn new(value: f64) -> Self {
        if value.is_nan() {
            Self::ONE
        } else {
            Self(value.clamp(0.0, 1.0))
        }
    }

    /// The fraction as a float in `0.0..=1.0`.
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }

    /// The fraction as a whole percentage in `0..=100`, rounded to nearest.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value is within 0.0..=1.0, so the rounded percentage always fits in u8"
    )]
    #[expect(
        clippy::cast_sign_loss,
        reason = "a float-to-int `as` saturates, so a negative value maps to 0"
    )]
    pub fn as_percent(self) -> u8 {
        (self.0 * 100.0).round() as u8
    }
}
