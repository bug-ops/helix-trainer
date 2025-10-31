//! Movement commands

use crate::helix::simulator::HelixSimulator;
use crate::security::UserError;
use helix_core::{
    Selection,
    doc_formatter::TextFormat,
    movement::{self, Movement},
    text_annotations::TextAnnotations,
};

/// Move left by count characters
pub(super) fn move_left(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_horizontally(
            slice,
            range,
            Direction::Backward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move right by count characters
pub(super) fn move_right(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_horizontally(
            slice,
            range,
            Direction::Forward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move down by count lines
pub(super) fn move_down(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_vertically(
            slice,
            range,
            Direction::Forward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move up by count lines
pub(super) fn move_up(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    use helix_core::movement::Direction;

    let slice = sim.doc.slice(..);
    let text_fmt = TextFormat::default();
    let mut annotations = TextAnnotations::default();

    let new_selection = sim.selection.clone().transform(|range| {
        movement::move_vertically(
            slice,
            range,
            Direction::Backward,
            count,
            Movement::Move,
            &text_fmt,
            &mut annotations,
        )
    });

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of next word
pub(super) fn move_next_word_start(
    sim: &mut HelixSimulator,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of previous word
pub(super) fn move_prev_word_start(
    sim: &mut HelixSimulator,
    count: usize,
) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_prev_word_start(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to end of next word
pub(super) fn move_next_word_end(sim: &mut HelixSimulator, count: usize) -> Result<(), UserError> {
    let slice = sim.doc.slice(..);
    let new_selection = sim
        .selection
        .clone()
        .transform(|range| movement::move_next_word_end(slice, range, count));

    sim.selection = new_selection;
    Ok(())
}

/// Move to start of current line
pub(super) fn move_line_start(sim: &mut HelixSimulator) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(line);

    sim.selection = Selection::point(line_start);
    Ok(())
}

/// Move to end of current line
pub(super) fn move_line_end(sim: &mut HelixSimulator) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let line = sim.doc.char_to_line(head);

    // Get position of next line, or end of document
    let line_end = if line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(line + 1) - 1
    } else {
        sim.doc.len_chars()
    };

    sim.selection = Selection::point(line_end);
    Ok(())
}

/// Move to start of document
pub(super) fn move_document_start(sim: &mut HelixSimulator) -> Result<(), UserError> {
    sim.selection = Selection::point(0);
    Ok(())
}

/// Move to end of document
pub(super) fn move_document_end(sim: &mut HelixSimulator) -> Result<(), UserError> {
    let end = sim.doc.len_chars();
    sim.selection = Selection::point(end);
    Ok(())
}
