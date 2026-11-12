//! Insert-mode `Tab` after text on the line (#583). The cases recorded from
//! `hx` for `rs` live in the golden table; `py` and `md` have no bracket rule,
//! so `Tab` after text must leave the document and the selection as they are.

use super::{SCENARIO_14, probed_in};
use crate::grammar::commands::*;
use crate::helix::Language;

#[test]
fn tab_after_text_leaves_python_and_markdown_untouched() {
    for language in [Language::Py, Language::Md] {
        let without_tab = probed_in(SCENARIO_14, language, &[CMD_APPEND_LINE_END, CMD_ESCAPE]);
        let with_tab = probed_in(
            SCENARIO_14,
            language,
            &[CMD_APPEND_LINE_END, CMD_INSERT_TAB, CMD_ESCAPE],
        );
        assert_eq!(with_tab, without_tab, "{language:?}");
    }
}
