//! The changes of one Insert session, composed as they arrive.
//!
//! Composing each new change into one accumulated [`ChangeSet`] copies the
//! whole accumulation per key, so a long session grows quadratic. Composition
//! is associative, so the changes are kept as a stack of segments whose ranks
//! behave like the digits of a binary counter: a new change is a rank-0
//! segment, and two neighbours of equal rank merge into one of the next rank.
//! Each change is then copied O(log n) times in total, and the whole
//! composition is built once, when it is asked for.

use helix_core::ChangeSet;

#[derive(Debug)]
struct Segment {
    rank: u32,
    changes: ChangeSet,
}

/// A sequence of consecutive [`ChangeSet`]s that composes in amortized
/// logarithmic time per change.
///
/// The result is the same as composing the changes one after the other: each
/// change applies to the document the previous one produced.
#[derive(Debug)]
pub(crate) struct ComposedChanges {
    segments: Vec<Segment>,
}

impl ComposedChanges {
    /// Start from `initial`, usually the identity of the document.
    pub(super) fn new(initial: ChangeSet) -> Self {
        Self {
            segments: vec![Segment {
                rank: 0,
                changes: initial,
            }],
        }
    }

    /// Append `changes`, which must apply to the document the changes so far
    /// produce.
    pub(super) fn push(&mut self, changes: ChangeSet) {
        self.segments.push(Segment { rank: 0, changes });
        while let [.., older, newer] = self.segments.as_slice()
            && older.rank == newer.rank
        {
            let (Some(newer), Some(older)) = (self.segments.pop(), self.segments.pop()) else {
                break;
            };
            self.segments.push(Segment {
                rank: older.rank + 1,
                changes: older.changes.compose(newer.changes),
            });
        }
    }

    /// All changes so far as one [`ChangeSet`].
    pub(super) fn composed(&self) -> ChangeSet {
        self.segments
            .iter()
            .map(|segment| segment.changes.clone())
            .reduce(ChangeSet::compose)
            .unwrap_or_else(|| ChangeSet::with_capacity(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::{Rope, Transaction};
    use proptest::prelude::*;

    /// A change set of `edits` on `doc`, each edit `(start, length, text)` clamped to the document.
    fn change_of(doc: &Rope, edits: &[(usize, usize, String)]) -> ChangeSet {
        let len = doc.len_chars();
        let mut spans: Vec<(usize, usize, String)> = edits
            .iter()
            .map(|(start, length, text)| {
                let start = start % (len + 1);
                (start, (start + length % 3).min(len), text.clone())
            })
            .collect();
        spans.sort_by_key(|&(start, _, _)| start);
        let mut last_end = 0;
        spans.retain_mut(|(start, end, _)| {
            let fits = *start >= last_end;
            if fits {
                last_end = *end;
            }
            fits
        });
        Transaction::change(
            doc,
            spans
                .into_iter()
                .map(|(start, end, text)| (start, end, (!text.is_empty()).then(|| text.into()))),
        )
        .changes()
        .clone()
    }

    fn edits() -> impl Strategy<Value = Vec<(usize, usize, String)>> {
        prop::collection::vec((0usize..40, 0usize..3, "[a-c\n]{0,3}"), 0..4)
    }

    proptest! {
        #[test]
        fn batched_composition_equals_sequential_composition(
            base in "[a-c\n]{0,12}",
            steps in prop::collection::vec(edits(), 1..40),
        ) {
            let mut doc = Rope::from_str(&base);
            let mut sequential = ChangeSet::new(doc.slice(..));
            let mut batched = ComposedChanges::new(sequential.clone());
            for step in &steps {
                let change = change_of(&doc, step);
                prop_assert!(change.apply(&mut doc));
                sequential = sequential.compose(change.clone());
                batched.push(change);
            }
            let batched = batched.composed();
            prop_assert_eq!(batched.changes(), sequential.changes());
        }
    }

    #[test]
    fn a_single_segment_composes_to_itself() {
        let doc = Rope::from_str("abc");
        let initial = ChangeSet::new(doc.slice(..));
        assert_eq!(
            ComposedChanges::new(initial.clone()).composed().changes(),
            initial.changes()
        );
    }
}
