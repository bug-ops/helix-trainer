//! Document positions shared by the simulator, scenarios and rendering
//!
//! [`Pos`] is the one representation of a `(row, col)` location in a
//! document, and [`CursorRange`] the one representation of a selection with
//! its fixed and moving ends. Keeping row and column behind accessors means a
//! swapped pair cannot compile, unlike two bare `usize` values.

/// A zero-based `(row, col)` position inside a document.
///
/// Rows order before columns, so positions sort in document order.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Pos;
///
/// let pos = Pos::new(2, 5);
/// assert_eq!((pos.row(), pos.col()), (2, 5));
/// assert!(Pos::new(1, 9) < Pos::new(2, 0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
    row: usize,
    col: usize,
}

impl Pos {
    /// A position at `row` and `col`, both zero-based.
    ///
    /// Scenario files go through the validating `TryFrom<[usize; 2]>` impl
    /// instead, which bounds both coordinates.
    #[must_use]
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// The zero-based row.
    #[must_use]
    pub const fn row(self) -> usize {
        self.row
    }

    /// The zero-based column.
    #[must_use]
    pub const fn col(self) -> usize {
        self.col
    }
}

/// A selection from `anchor` (fixed end) to `head` (the cursor end).
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::{CursorRange, Pos};
///
/// let range = CursorRange::new(Pos::new(0, 1), Pos::new(2, 3));
/// assert_eq!(range.anchor().col(), 1);
/// assert_eq!(range.head().row(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorRange {
    anchor: Pos,
    head: Pos,
}

impl CursorRange {
    /// A selection fixed at `anchor` whose cursor sits at `head`.
    #[must_use]
    pub const fn new(anchor: Pos, head: Pos) -> Self {
        Self { anchor, head }
    }

    /// The fixed end of the selection.
    #[must_use]
    pub const fn anchor(self) -> Pos {
        self.anchor
    }

    /// The moving end of the selection, where the cursor sits.
    #[must_use]
    pub const fn head(self) -> Pos {
        self.head
    }
}
