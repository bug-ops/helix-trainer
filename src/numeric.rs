//! Float-to-integer conversions shared by scoring and rendering code.
//!
//! `as` casts from floats are lossy in three ways (truncation, sign loss, NaN). Keeping the
//! only such cast here, behind a clamp, lets callers stay free of lint exceptions.

/// Converts a ratio to a whole percentage, discarding the fractional part.
///
/// The result is clamped to `0..=100`; NaN yields `0`.
pub(crate) fn floor_percent(ratio: f64) -> u32 {
    clamped_percent((ratio * 100.0).floor())
}

/// Converts a ratio to a whole percentage, rounded to nearest.
///
/// The result is clamped to `0..=100`; NaN yields `0`.
pub(crate) fn round_percent(ratio: f64) -> u32 {
    clamped_percent((ratio * 100.0).round())
}

fn clamped_percent(percent: f64) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0.0..=100.0 first; a float-to-int `as` maps NaN to 0"
    )]
    let whole = percent.clamp(0.0, 100.0) as u32;
    whole
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_percent_discards_fraction() {
        assert_eq!(floor_percent(0.999), 99);
        assert_eq!(floor_percent(0.5), 50);
    }

    #[test]
    fn round_percent_rounds_to_nearest() {
        assert_eq!(round_percent(0.666), 67);
        assert_eq!(round_percent(0.664), 66);
    }

    #[test]
    fn percent_is_clamped_and_nan_is_zero() {
        assert_eq!(floor_percent(-0.5), 0);
        assert_eq!(round_percent(7.0), 100);
        assert_eq!(floor_percent(f64::NAN), 0);
    }
}
