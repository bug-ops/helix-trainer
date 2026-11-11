//! Float-to-integer conversions shared by scoring and rendering code.
//!
//! `as` casts from floats are lossy in three ways (truncation, sign loss, NaN). Keeping the
//! only such cast here, behind a clamp, lets callers stay free of lint exceptions.

/// How a float is brought to a whole number before conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rounding {
    /// Round half away from zero.
    Nearest,
    /// Discard the fractional part toward negative infinity.
    Floor,
}

impl Rounding {
    fn apply(self, value: f64) -> f64 {
        match self {
            Self::Nearest => value.round(),
            Self::Floor => value.floor(),
        }
    }
}

/// Converts a float to `u64` with the given rounding, clamped to the `u64` range; NaN yields `0`.
pub(crate) fn to_u64(value: f64, rounding: Rounding) -> u64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a float-to-int `as` saturates at the integer range and maps NaN to 0"
    )]
    let whole = rounding.apply(value) as u64;
    whole
}

/// Converts a float to `u32` with the given rounding, clamped to the `u32` range; NaN yields `0`.
pub(crate) fn to_u32(value: f64, rounding: Rounding) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a float-to-int `as` saturates at the integer range and maps NaN to 0"
    )]
    let whole = rounding.apply(value) as u32;
    whole
}

/// Scales an integer by a float factor with the given rounding, clamped to the `u64` range; NaN
/// yields `0`.
///
/// Values above 2^53 lose precision in the `f64` product.
pub(crate) fn scale_u64(value: u64, factor: f64, rounding: Rounding) -> u64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "scaled values (XP, scores) are far below 2^53"
    )]
    let scaled = value as f64 * factor;
    to_u64(scaled, rounding)
}

/// Converts a ratio to a whole percentage, discarding the fractional part.
///
/// The result is clamped to `0..=100`; NaN yields `0`.
pub(crate) fn floor_percent(ratio: f64) -> u32 {
    clamped_percent(ratio * 100.0, Rounding::Floor)
}

/// Converts a ratio to a whole percentage, rounded to nearest.
///
/// The result is clamped to `0..=100`; NaN yields `0`.
pub(crate) fn round_percent(ratio: f64) -> u32 {
    clamped_percent(ratio * 100.0, Rounding::Nearest)
}

fn clamped_percent(percent: f64, rounding: Rounding) -> u32 {
    to_u32(percent, rounding).min(100)
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
    fn conversions_apply_rounding() {
        assert_eq!(to_u32(2.5, Rounding::Nearest), 3);
        assert_eq!(to_u32(2.5, Rounding::Floor), 2);
        assert_eq!(to_u64(2.4, Rounding::Nearest), 2);
        assert_eq!(scale_u64(100, 0.7, Rounding::Nearest), 70);
        assert_eq!(scale_u64(15, 0.5, Rounding::Floor), 7);
    }

    #[test]
    fn conversions_clamp_and_map_nan_to_zero() {
        for rounding in [Rounding::Nearest, Rounding::Floor] {
            assert_eq!(to_u32(f64::NAN, rounding), 0);
            assert_eq!(to_u64(f64::NAN, rounding), 0);
            assert_eq!(scale_u64(10, f64::NAN, rounding), 0);
            assert_eq!(to_u32(f64::INFINITY, rounding), u32::MAX);
            assert_eq!(to_u64(f64::INFINITY, rounding), u64::MAX);
            assert_eq!(to_u32(f64::NEG_INFINITY, rounding), 0);
            assert_eq!(to_u64(-5.0, rounding), 0);
            assert_eq!(scale_u64(10, f64::INFINITY, rounding), u64::MAX);
        }
    }

    #[test]
    fn scheduled_days_keeps_nan_at_one() {
        assert_eq!(to_u32(f64::NAN, Rounding::Nearest).max(1), 1);
        assert_eq!(to_u32(0.2, Rounding::Nearest).max(1), 1);
        assert_eq!(to_u32(36.5, Rounding::Nearest).max(1), 37);
    }

    proptest::proptest! {
        #[test]
        fn to_u32_never_panics_and_floor_not_above_nearest(v in proptest::num::f64::ANY) {
            proptest::prop_assert!(to_u32(v, Rounding::Floor) <= to_u32(v, Rounding::Nearest));
        }

        #[test]
        fn scheduled_days_matches_previous_f32_formula(interval in proptest::num::f32::ANY) {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "reference implementation of the replaced inline cast"
            )]
            let previous = interval.round().max(1.0) as u32;
            let current = to_u32(f64::from(interval), Rounding::Nearest).max(1);
            proptest::prop_assert_eq!(current, previous);
        }

        #[test]
        fn scale_by_one_is_identity_below_2_pow_53(v in 0u64..(1 << 53)) {
            proptest::prop_assert_eq!(scale_u64(v, 1.0, Rounding::Nearest), v);
            proptest::prop_assert_eq!(scale_u64(v, 1.0, Rounding::Floor), v);
        }
    }

    #[test]
    fn percent_is_clamped_and_nan_is_zero() {
        assert_eq!(floor_percent(-0.5), 0);
        assert_eq!(round_percent(7.0), 100);
        assert_eq!(floor_percent(f64::NAN), 0);
    }
}
