//! View command definitions
//!
//! Registers view/viewport commands (z, zz, zt, zb, zm, zj, zk)

use crate::helix::commands::{
    CMD_SCROLL_DOWN, CMD_SCROLL_UP, CMD_VIEW_BOTTOM, CMD_VIEW_CENTER, CMD_VIEW_CENTER_HORIZONTAL,
    CMD_VIEW_TOP,
};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Binding, Category, CommandMetadata, HelixName};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::view;

/// Register all view commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // z or zz - center view on cursor
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("align_view_center")),
            CMD_VIEW_CENTER,
            Category::View,
        )
        .with_summary("Center view")
        .with_help("Center the viewport vertically on the cursor line."),
        view::view_center,
    ));

    // zt - align cursor to top
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("align_view_top")),
            CMD_VIEW_TOP,
            Category::View,
        )
        .with_summary("Align top")
        .with_help("Align the cursor line to the top of the viewport."),
        view::view_align_top,
    ));

    // zb - align cursor to bottom
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("align_view_bottom")),
            CMD_VIEW_BOTTOM,
            Category::View,
        )
        .with_summary("Align bottom")
        .with_help("Align the cursor line to the bottom of the viewport."),
        view::view_align_bottom,
    ));

    // zm - center horizontally
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("align_view_middle")),
            CMD_VIEW_CENTER_HORIZONTAL,
            Category::View,
        )
        .with_summary("Center horizontal")
        .with_help("Center the viewport horizontally on the cursor column."),
        view::view_center_horizontal,
    ));

    // zj - scroll down
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("scroll_down")),
            CMD_SCROLL_DOWN,
            Category::View,
        )
        .with_summary("Scroll down")
        .with_help("Scroll the viewport down by one line."),
        view::scroll_down,
    ));

    // zk - scroll up
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("scroll_up")),
            CMD_SCROLL_UP,
            Category::View,
        )
        .with_summary("Scroll up")
        .with_help("Scroll the viewport up by one line."),
        view::scroll_up,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register(&mut registry);

        assert!(registry.contains(CMD_VIEW_CENTER), "Missing view_center");
        assert!(registry.contains(CMD_VIEW_TOP), "Missing view_align_top");
        assert!(
            registry.contains(CMD_VIEW_BOTTOM),
            "Missing view_align_bottom"
        );
        assert!(
            registry.contains(CMD_VIEW_CENTER_HORIZONTAL),
            "Missing view_center_horizontal"
        );
        assert!(registry.contains(CMD_SCROLL_DOWN), "Missing scroll_down");
        assert!(registry.contains(CMD_SCROLL_UP), "Missing scroll_up");
    }

    #[test]
    fn test_view_commands_category() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register(&mut registry);

        let view_cmds = registry.commands_in_category(Category::View);
        assert!(view_cmds.len() >= 6, "Expected at least 6 view commands");
    }
}
