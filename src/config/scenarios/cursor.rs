//! Cursor description of a scenario setup or target
//!
//! A scenario states its cursor with one of four TOML keys. They are parsed
//! into the closed enum [`CursorSpec`], so an ambiguous or empty description
//! is a load error and every reader sees one value.

use std::fmt;

use serde::Deserialize;

use super::ScenarioValidationError;
use crate::security::limits::{MAX_CURSORS_PER_SCENARIO, MAX_SELECTIONS_PER_SCENARIO};

/// Largest accepted row or column of a scenario position.
const MAX_COORDINATE: usize = 10_000;

/// A zero-based `(row, col)` position inside a scenario document.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::Pos;
///
/// let pos = Pos::try_from([2, 5])?;
/// assert_eq!((pos.row(), pos.col()), (2, 5));
/// assert!(Pos::try_from([10_001, 0]).is_err());
/// # Ok::<(), helix_trainer::config::scenarios::ScenarioValidationError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pos {
    row: usize,
    col: usize,
}

impl Pos {
    /// The zero-based row.
    #[must_use]
    pub fn row(self) -> usize {
        self.row
    }

    /// The zero-based column.
    #[must_use]
    pub fn col(self) -> usize {
        self.col
    }
}

impl TryFrom<[usize; 2]> for Pos {
    type Error = ScenarioValidationError;

    fn try_from([row, col]: [usize; 2]) -> Result<Self, Self::Error> {
        let actual = row.max(col);
        if actual > MAX_COORDINATE {
            return Err(ScenarioValidationError::PositionOutOfRange {
                max: MAX_COORDINATE,
                actual,
            });
        }
        Ok(Self { row, col })
    }
}

/// A selection from `anchor` (fixed end) to `head` (the cursor end).
///
/// # Examples
///
/// ```
/// use helix_trainer::config::CursorRange;
///
/// let range = CursorRange::try_from([0, 1, 2, 3])?;
/// assert_eq!(range.anchor().col(), 1);
/// assert_eq!(range.head().row(), 2);
/// # Ok::<(), helix_trainer::config::scenarios::ScenarioValidationError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorRange {
    anchor: Pos,
    head: Pos,
}

impl CursorRange {
    /// The fixed end of the selection.
    #[must_use]
    pub fn anchor(self) -> Pos {
        self.anchor
    }

    /// The moving end of the selection, where the cursor sits.
    #[must_use]
    pub fn head(self) -> Pos {
        self.head
    }
}

impl TryFrom<[usize; 4]> for CursorRange {
    type Error = ScenarioValidationError;

    fn try_from(
        [anchor_row, anchor_col, head_row, head_col]: [usize; 4],
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            anchor: Pos::try_from([anchor_row, anchor_col])?,
            head: Pos::try_from([head_row, head_col])?,
        })
    }
}

/// Why a list does not fit [`Bounded`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundError {
    /// The list has no items.
    Empty,
    /// The list has more than `max` items.
    TooMany {
        /// The most items allowed.
        max: usize,
        /// The number of items given.
        actual: usize,
    },
}

/// A non-empty list of at most `MAX` items.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::{BoundError, Bounded};
///
/// assert!(Bounded::<u8, 2>::try_from(vec![1, 2]).is_ok());
/// assert_eq!(Bounded::<u8, 2>::try_from(vec![]), Err(BoundError::Empty));
/// assert_eq!(
///     Bounded::<u8, 2>::try_from(vec![1, 2, 3]),
///     Err(BoundError::TooMany { max: 2, actual: 3 })
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounded<T, const MAX: usize>(Vec<T>);

impl<T, const MAX: usize> Bounded<T, MAX> {
    /// The items, in order; never empty.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

impl<T, const MAX: usize> TryFrom<Vec<T>> for Bounded<T, MAX> {
    type Error = BoundError;

    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        if items.is_empty() {
            Err(BoundError::Empty)
        } else if items.len() > MAX {
            Err(BoundError::TooMany {
                max: MAX,
                actual: items.len(),
            })
        } else {
            Ok(Self(items))
        }
    }
}

/// One of the four TOML keys that describe a cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorField {
    /// `cursor_position = [row, col]`
    CursorPosition,
    /// `selection = [r0, c0, r1, c1]`
    Selection,
    /// `cursors = [[row, col], ...]`
    Cursors,
    /// `selections = [[r0, c0, r1, c1], ...]`
    Selections,
}

impl fmt::Display for CursorField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CursorPosition => "cursor_position",
            Self::Selection => "selection",
            Self::Cursors => "cursors",
            Self::Selections => "selections",
        })
    }
}

/// The cursor of a scenario setup or target: exactly one of four forms.
///
/// Deserialized from the TOML keys `cursor_position`, `selection`, `cursors`
/// and `selections`; exactly one must be present.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::{CursorSpec, Pos};
///
/// #[derive(serde::Deserialize)]
/// struct Doc {
///     #[serde(flatten)]
///     cursor: CursorSpec,
/// }
///
/// let doc: Doc = toml::from_str("cursor_position = [1, 2]")?;
/// assert_eq!(doc.cursor, CursorSpec::Point(Pos::try_from([1, 2])?));
/// assert!(toml::from_str::<Doc>("cursor_position = [0, 0]\nselection = [0, 0, 0, 1]").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(try_from = "RawCursorSpec")]
pub enum CursorSpec {
    /// One cursor.
    Point(Pos),
    /// One selection.
    Range(CursorRange),
    /// Several cursors.
    Points(Bounded<Pos, MAX_CURSORS_PER_SCENARIO>),
    /// Several selections.
    Ranges(Bounded<CursorRange, MAX_SELECTIONS_PER_SCENARIO>),
}

/// The four optional TOML keys, before the exactly-one rule is applied.
#[derive(Deserialize)]
struct RawCursorSpec {
    #[serde(default)]
    cursor_position: Option<[usize; 2]>,
    #[serde(default)]
    selection: Option<[usize; 4]>,
    #[serde(default)]
    cursors: Option<Vec<[usize; 2]>>,
    #[serde(default)]
    selections: Option<Vec<[usize; 4]>>,
}

impl RawCursorSpec {
    fn present_fields(&self) -> Vec<CursorField> {
        [
            (self.cursor_position.is_some(), CursorField::CursorPosition),
            (self.selection.is_some(), CursorField::Selection),
            (self.cursors.is_some(), CursorField::Cursors),
            (self.selections.is_some(), CursorField::Selections),
        ]
        .into_iter()
        .filter_map(|(present, field)| present.then_some(field))
        .collect()
    }
}

fn bounded<T, const MAX: usize>(
    items: Vec<T>,
    field: CursorField,
    too_many: fn(usize, usize) -> ScenarioValidationError,
) -> Result<Bounded<T, MAX>, ScenarioValidationError> {
    Bounded::try_from(items).map_err(|e| match e {
        BoundError::Empty => ScenarioValidationError::EmptyCursorList { field },
        BoundError::TooMany { max, actual } => too_many(max, actual),
    })
}

fn positions(raw: Vec<[usize; 2]>) -> Result<Vec<Pos>, ScenarioValidationError> {
    raw.into_iter().map(Pos::try_from).collect()
}

fn ranges(raw: Vec<[usize; 4]>) -> Result<Vec<CursorRange>, ScenarioValidationError> {
    raw.into_iter().map(CursorRange::try_from).collect()
}

impl TryFrom<RawCursorSpec> for CursorSpec {
    type Error = ScenarioValidationError;

    fn try_from(raw: RawCursorSpec) -> Result<Self, Self::Error> {
        let fields = raw.present_fields();
        match (
            raw.cursor_position,
            raw.selection,
            raw.cursors,
            raw.selections,
        ) {
            (Some(pos), None, None, None) => Ok(Self::Point(Pos::try_from(pos)?)),
            (None, Some(range), None, None) => Ok(Self::Range(CursorRange::try_from(range)?)),
            (None, None, Some(points), None) => Ok(Self::Points(bounded(
                positions(points)?,
                CursorField::Cursors,
                |max, actual| ScenarioValidationError::TooManyCursors { max, actual },
            )?)),
            (None, None, None, Some(list)) => Ok(Self::Ranges(bounded(
                ranges(list)?,
                CursorField::Selections,
                |max, actual| ScenarioValidationError::TooManySelections { max, actual },
            )?)),
            (None, None, None, None) => Err(ScenarioValidationError::NoCursorSpecified),
            _ => Err(ScenarioValidationError::AmbiguousCursorSpec { fields }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    fn raw() -> RawCursorSpec {
        RawCursorSpec {
            cursor_position: None,
            selection: None,
            cursors: None,
            selections: None,
        }
    }

    fn pos(row: usize, col: usize) -> Pos {
        Pos::try_from([row, col]).unwrap()
    }

    #[test]
    fn position_limit_is_inclusive() {
        assert!(Pos::try_from([MAX_COORDINATE, MAX_COORDINATE]).is_ok());
        for bad in [[MAX_COORDINATE + 1, 0], [0, MAX_COORDINATE + 1]] {
            assert_eq!(
                Pos::try_from(bad),
                Err(ScenarioValidationError::PositionOutOfRange {
                    max: MAX_COORDINATE,
                    actual: MAX_COORDINATE + 1
                })
            );
        }
    }

    #[test]
    fn a_range_checks_both_ends() {
        assert!(CursorRange::try_from([0, 0, 0, 0]).is_ok());
        assert!(CursorRange::try_from([MAX_COORDINATE + 1, 0, 0, 0]).is_err());
        assert!(CursorRange::try_from([0, 0, 0, MAX_COORDINATE + 1]).is_err());
    }

    #[test]
    fn exactly_one_field_builds_the_matching_variant() {
        let point = CursorSpec::try_from(RawCursorSpec {
            cursor_position: Some([1, 2]),
            ..raw()
        });
        assert_eq!(point, Ok(CursorSpec::Point(pos(1, 2))));

        let range = CursorSpec::try_from(RawCursorSpec {
            selection: Some([0, 0, 1, 1]),
            ..raw()
        });
        assert_matches!(range, Ok(CursorSpec::Range(_)));

        let points = CursorSpec::try_from(RawCursorSpec {
            cursors: Some(vec![[0, 0], [1, 1]]),
            ..raw()
        });
        assert_matches!(points, Ok(CursorSpec::Points(p)) if p.as_slice().len() == 2);

        let ranges = CursorSpec::try_from(RawCursorSpec {
            selections: Some(vec![[0, 0, 0, 3], [1, 0, 1, 3]]),
            ..raw()
        });
        assert_matches!(ranges, Ok(CursorSpec::Ranges(r)) if r.as_slice().len() == 2);
    }

    #[test]
    fn no_field_is_rejected() {
        assert_eq!(
            CursorSpec::try_from(raw()),
            Err(ScenarioValidationError::NoCursorSpecified)
        );
    }

    #[test]
    fn every_pair_and_the_full_set_is_ambiguous() {
        let all = [
            (CursorField::CursorPosition, 0),
            (CursorField::Selection, 1),
            (CursorField::Cursors, 2),
            (CursorField::Selections, 3),
        ];
        for mask in 0u8..16 {
            if mask.count_ones() < 2 {
                continue;
            }
            let has = |bit: u8| mask & (1 << bit) != 0;
            let spec = RawCursorSpec {
                cursor_position: has(0).then_some([0, 0]),
                selection: has(1).then_some([0, 0, 0, 0]),
                cursors: has(2).then(|| vec![[0, 0]]),
                selections: has(3).then(|| vec![[0, 0, 0, 0]]),
            };
            let expected: Vec<CursorField> = all
                .iter()
                .filter(|(_, bit)| has(*bit))
                .map(|(field, _)| *field)
                .collect();
            assert_eq!(
                CursorSpec::try_from(spec),
                Err(ScenarioValidationError::AmbiguousCursorSpec { fields: expected })
            );
        }
    }

    #[test]
    fn empty_lists_are_rejected() {
        let cursors = CursorSpec::try_from(RawCursorSpec {
            cursors: Some(vec![]),
            ..raw()
        });
        assert_eq!(
            cursors,
            Err(ScenarioValidationError::EmptyCursorList {
                field: CursorField::Cursors
            })
        );
        let selections = CursorSpec::try_from(RawCursorSpec {
            selections: Some(vec![]),
            ..raw()
        });
        assert_eq!(
            selections,
            Err(ScenarioValidationError::EmptyCursorList {
                field: CursorField::Selections
            })
        );
    }

    #[test]
    fn list_length_limits_are_inclusive() {
        let at_limit = RawCursorSpec {
            cursors: Some(vec![[0, 0]; MAX_CURSORS_PER_SCENARIO]),
            ..raw()
        };
        assert!(CursorSpec::try_from(at_limit).is_ok());
        let over = RawCursorSpec {
            cursors: Some(vec![[0, 0]; MAX_CURSORS_PER_SCENARIO + 1]),
            ..raw()
        };
        assert_matches!(
            CursorSpec::try_from(over),
            Err(ScenarioValidationError::TooManyCursors { max, actual })
                if max == MAX_CURSORS_PER_SCENARIO && actual == max + 1
        );

        let at_limit = RawCursorSpec {
            selections: Some(vec![[0, 0, 0, 0]; MAX_SELECTIONS_PER_SCENARIO]),
            ..raw()
        };
        assert!(CursorSpec::try_from(at_limit).is_ok());
        let over = RawCursorSpec {
            selections: Some(vec![[0, 0, 0, 0]; MAX_SELECTIONS_PER_SCENARIO + 1]),
            ..raw()
        };
        assert_matches!(
            CursorSpec::try_from(over),
            Err(ScenarioValidationError::TooManySelections { .. })
        );
    }

    #[test]
    fn a_point_range_is_valid() {
        let spec = CursorSpec::try_from(RawCursorSpec {
            selection: Some([0, 5, 0, 5]),
            ..raw()
        });
        assert_matches!(spec, Ok(CursorSpec::Range(r)) if r.anchor() == r.head());
    }
}
