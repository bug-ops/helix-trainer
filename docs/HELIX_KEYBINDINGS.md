# Helix Editor — Complete Keybindings Reference

> **Source:** [docs.helix-editor.com/keymap.html](https://docs.helix-editor.com/keymap.html)
> **Updated:** 2025-10-30

Comprehensive reference of all Helix editor keybindings for developing training scenarios.

## Table of Contents

- [Normal Mode](#normal-mode)
- [Insert Mode](#insert-mode)
- [Select Mode](#select-mode)
- [View Mode](#view-mode-z)
- [Goto Mode](#goto-mode-g)
- [Match Mode](#match-mode-m)
- [Window Mode](#window-mode-ctrl-w)
- [Space Mode](#space-mode-space)
- [Picker Mode](#picker-mode)
- [Prompt Mode](#prompt-mode)
- [Unimpaired Mappings](#unimpaired-mappings)
- [Command Mode](#command-mode-commands)
- [Implementation Status](#implementation-checklist)

---

## Normal Mode

### Movement Commands

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>h</kbd> | <kbd>←</kbd> | `move_char_left` | Move left one character |
| <kbd>j</kbd> | <kbd>↓</kbd> | `move_visual_line_down` | Move down one visual line |
| <kbd>k</kbd> | <kbd>↑</kbd> | `move_visual_line_up` | Move up one visual line |
| <kbd>l</kbd> | <kbd>→</kbd> | `move_char_right` | Move right one character |
| <kbd>w</kbd> | | `move_next_word_start` | Move to next word start |
| <kbd>b</kbd> | | `move_prev_word_start` | Move to previous word start |
| <kbd>e</kbd> | | `move_next_word_end` | Move to next word end |
| <kbd>W</kbd> | | `move_next_long_word_start` | Move to next WORD start (whitespace-separated) |
| <kbd>B</kbd> | | `move_prev_long_word_start` | Move to previous WORD start |
| <kbd>E</kbd> | | `move_next_long_word_end` | Move to next WORD end |
| <kbd>t</kbd> | | `find_till_char` | Find till next occurrence of character |
| <kbd>f</kbd> | | `find_next_char` | Find next occurrence of character |
| <kbd>T</kbd> | | `till_prev_char` | Find till previous occurrence of character |
| <kbd>F</kbd> | | `find_prev_char` | Find previous occurrence of character |
| <kbd>G</kbd> | | `goto_line` | Go to line number (or end if no number) |
| <kbd>Alt</kbd>+<kbd>.</kbd> | | `repeat_last_motion` | Repeat last motion (f, t, m, etc.) |
| <kbd>Home</kbd> | | `goto_line_start` | Go to start of line |
| <kbd>End</kbd> | | `goto_line_end` | Go to end of line |
| <kbd>Ctrl</kbd>+<kbd>b</kbd> | <kbd>PgUp</kbd> | `page_up` | Move page up |
| <kbd>Ctrl</kbd>+<kbd>f</kbd> | <kbd>PgDn</kbd> | `page_down` | Move page down |
| <kbd>Ctrl</kbd>+<kbd>u</kbd> | | `page_cursor_half_up` | Move cursor and page half page up |
| <kbd>Ctrl</kbd>+<kbd>d</kbd> | | `page_cursor_half_down` | Move cursor and page half page down |
| <kbd>Ctrl</kbd>+<kbd>i</kbd> | | `jump_forward` | Jump forward on jumplist |
| <kbd>Ctrl</kbd>+<kbd>o</kbd> | | `jump_backward` | Jump backward on jumplist |
| <kbd>Ctrl</kbd>+<kbd>s</kbd> | | `save_selection` | Save current selection to jumplist |

### Change Commands

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>r</kbd> | | `replace` | Replace each character in selection with another character |
| <kbd>R</kbd> | | `replace_with_yanked` | Replace selection with yanked text |
| <kbd>~</kbd> | | `switch_case` | Switch case of selected text (toggle) |
| <kbd>`</kbd> | | `switch_to_lowercase` | Switch selected text to lowercase |
| <kbd>Alt</kbd>+<kbd>`</kbd> | | `switch_to_uppercase` | Switch selected text to uppercase |
| <kbd>i</kbd> | | `insert_mode` | Enter insert mode before selection |
| <kbd>a</kbd> | | `append_mode` | Enter insert mode after selection |
| <kbd>I</kbd> | | `insert_at_line_start` | Enter insert mode at start of line |
| <kbd>A</kbd> | | `insert_at_line_end` | Enter insert mode at end of line |
| <kbd>o</kbd> | | `open_below` | Open new line below and enter insert mode |
| <kbd>O</kbd> | | `open_above` | Open new line above and enter insert mode |
| <kbd>.</kbd> | | | Repeat last insert operation |
| <kbd>u</kbd> | | `undo` | Undo last change |
| <kbd>U</kbd> | | `redo` | Redo last undone change |
| <kbd>Alt</kbd>+<kbd>u</kbd> | | `earlier` | Move backward in history |
| <kbd>Alt</kbd>+<kbd>U</kbd> | | `later` | Move forward in history |
| <kbd>y</kbd> | | `yank` | Yank (copy) selection |
| <kbd>p</kbd> | | `paste_after` | Paste after selection |
| <kbd>P</kbd> | | `paste_before` | Paste before selection |
| <kbd>"</kbd>+reg | | `select_register` | Select a register for next yank or paste |
| <kbd>></kbd> | | `indent` | Indent selection |
| <kbd><</kbd> | | `unindent` | Unindent selection |
| <kbd>=</kbd> | | `format_selections` | Format selection using LSP |
| <kbd>d</kbd> | | `delete_selection` | Delete selection (and yank) |
| <kbd>Alt</kbd>+<kbd>d</kbd> | | `delete_selection_noyank` | Delete selection without yanking |
| <kbd>c</kbd> | | `change_selection` | Change selection (delete and enter insert mode) |
| <kbd>Alt</kbd>+<kbd>c</kbd> | | `change_selection_noyank` | Change selection without yanking |
| <kbd>Ctrl</kbd>+<kbd>a</kbd> | | `increment` | Increment number under cursor |
| <kbd>Ctrl</kbd>+<kbd>x</kbd> | | `decrement` | Decrement number under cursor |
| <kbd>Q</kbd> | | `record_macro` | Start/stop recording macro to register |
| <kbd>q</kbd> | | `replay_macro` | Play back recorded macro from register |

### Shell Commands

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>\|</kbd> | `shell_pipe` | Pipe each selection through shell command, replacing with output |
| <kbd>Alt</kbd>+<kbd>\|</kbd> | `shell_pipe_to` | Pipe each selection to shell command, ignore output |
| <kbd>!</kbd> | `shell_insert_output` | Insert shell command output before each selection |
| <kbd>Alt</kbd>+<kbd>!</kbd> | `shell_append_output` | Append shell command output after each selection |
| <kbd>$</kbd> | `shell_keep_pipe` | Pipe selections to shell command, keep only selections where command succeeded |

### Selection Manipulation

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>s</kbd> | | `select_regex` | Select all regex matches inside selections |
| <kbd>S</kbd> | | `split_selection` | Split selections on regex matches |
| <kbd>Alt</kbd>+<kbd>s</kbd> | | `split_selection_on_newline` | Split selections on newlines |
| <kbd>Alt</kbd>+<kbd>-</kbd> | | `merge_selections` | Merge selections |
| <kbd>Alt</kbd>+<kbd>_</kbd> | | `merge_consecutive_selections` | Merge consecutive selections |
| <kbd>&</kbd> | | `align_selections` | Align selections in columns |
| <kbd>_</kbd> | | `trim_selections` | Trim whitespace from selections |
| <kbd>;</kbd> | | `collapse_selection` | Collapse selections to their cursor |
| <kbd>Alt</kbd>+<kbd>;</kbd> | | `flip_selections` | Flip the direction of selections (swap cursor and anchor) |
| <kbd>Alt</kbd>+<kbd>:</kbd> | | `ensure_selections_forward` | Ensure all selections face forward |
| <kbd>,</kbd> | | `keep_primary_selection` | Keep only the primary selection |
| <kbd>Alt</kbd>+<kbd>,</kbd> | | `remove_primary_selection` | Remove the primary selection |
| <kbd>C</kbd> | | `copy_selection_on_next_line` | Copy selection to next line (add cursor below) |
| <kbd>Alt</kbd>+<kbd>C</kbd> | | `copy_selection_on_prev_line` | Copy selection to previous line (add cursor above) |
| <kbd>(</kbd> | | `rotate_selections_backward` | Rotate primary selection backward |
| <kbd>)</kbd> | | `rotate_selections_forward` | Rotate primary selection forward |
| <kbd>Alt</kbd>+<kbd>(</kbd> | | `rotate_selection_contents_backward` | Rotate selection contents backward |
| <kbd>Alt</kbd>+<kbd>)</kbd> | | `rotate_selection_contents_forward` | Rotate selection contents forward |
| <kbd>%</kbd> | | `select_all` | Select entire file |
| <kbd>x</kbd> | | `extend_line_below` | Select current line, extend selection to include next line |
| <kbd>X</kbd> | | `extend_to_line_bounds` | Extend selection to line bounds (line-wise selection) |
| <kbd>Alt</kbd>+<kbd>x</kbd> | | `shrink_to_line_bounds` | Shrink selection to line bounds |
| <kbd>J</kbd> | | `join_selections` | Join lines inside selection (remove newlines) |
| <kbd>Alt</kbd>+<kbd>J</kbd> | | `join_selections_space` | Join lines with space |
| <kbd>K</kbd> | | `keep_selections` | Keep selections matching regex |
| <kbd>Alt</kbd>+<kbd>K</kbd> | | `remove_selections` | Remove selections matching regex |
| <kbd>Ctrl</kbd>+<kbd>c</kbd> | | `toggle_comments` | Toggle line comments for selections |

### Tree-sitter Selection Commands

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>Alt</kbd>+<kbd>o</kbd> | <kbd>Alt</kbd>+<kbd>↑</kbd> | `expand_selection` | Expand selection to parent syntax node |
| <kbd>Alt</kbd>+<kbd>i</kbd> | <kbd>Alt</kbd>+<kbd>↓</kbd> | `shrink_selection` | Shrink syntax tree object selection |
| <kbd>Alt</kbd>+<kbd>p</kbd> | <kbd>Alt</kbd>+<kbd>←</kbd> | `select_prev_sibling` | Select previous sibling node in syntax tree |
| <kbd>Alt</kbd>+<kbd>n</kbd> | <kbd>Alt</kbd>+<kbd>→</kbd> | `select_next_sibling` | Select next sibling node in syntax tree |
| <kbd>Alt</kbd>+<kbd>a</kbd> | | `select_all_siblings` | Select all sibling nodes in syntax tree |
| <kbd>Alt</kbd>+<kbd>I</kbd> | <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>↓</kbd> | `select_all_children` | Select all children of current syntax node |
| <kbd>Alt</kbd>+<kbd>e</kbd> | | `move_parent_node_end` | Move to end of parent syntax node |
| <kbd>Alt</kbd>+<kbd>b</kbd> | | `move_parent_node_start` | Move to start of parent syntax node |

### Search Commands

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>/</kbd> | `search` | Search for regex pattern forward |
| <kbd>?</kbd> | `rsearch` | Search for regex pattern backward |
| <kbd>n</kbd> | `search_next` | Select next search match |
| <kbd>N</kbd> | `search_prev` | Select previous search match |
| <kbd>*</kbd> | `search_selection_detect_word_boundaries` | Use current selection as search pattern (word boundaries) |
| <kbd>Alt</kbd>+<kbd>*</kbd> | `search_selection` | Use current selection as search pattern (exact) |

### Mode Switching

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>v</kbd> | `select_mode` | Enter select (extend) mode |
| <kbd>g</kbd> | | Enter goto mode |
| <kbd>m</kbd> | | Enter match mode |
| <kbd>:</kbd> | `command_mode` | Enter command mode |
| <kbd>z</kbd> | | Enter view mode |
| <kbd>Z</kbd> | | Enter sticky view mode |
| <kbd>Ctrl</kbd>+<kbd>w</kbd> | | Enter window mode |
| <kbd>Space</kbd> | | Enter space mode |

---

## Insert Mode

In insert mode, most keys insert their character. Special keys:

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>Esc</kbd> | | `normal_mode` | Switch to normal mode |
| <kbd>Ctrl</kbd>+<kbd>s</kbd> | | `commit_undo_checkpoint` | Commit a new undo checkpoint |
| <kbd>Ctrl</kbd>+<kbd>x</kbd> | | `completion` | Autocomplete |
| <kbd>Ctrl</kbd>+<kbd>r</kbd> | | `insert_register` | Insert contents of register |
| <kbd>Ctrl</kbd>+<kbd>w</kbd> | <kbd>Alt</kbd>+<kbd>Backspace</kbd> | `delete_word_backward` | Delete previous word |
| <kbd>Alt</kbd>+<kbd>d</kbd> | | `delete_word_forward` | Delete next word |
| <kbd>Ctrl</kbd>+<kbd>u</kbd> | | `kill_to_line_start` | Delete from cursor to start of line |
| <kbd>Ctrl</kbd>+<kbd>k</kbd> | | `kill_to_line_end` | Delete from cursor to end of line |
| <kbd>Ctrl</kbd>+<kbd>h</kbd> | <kbd>Backspace</kbd> | `delete_char_backward` | Delete previous character |
| <kbd>Ctrl</kbd>+<kbd>d</kbd> | <kbd>Delete</kbd> | `delete_char_forward` | Delete next character |
| <kbd>Ctrl</kbd>+<kbd>j</kbd> | <kbd>Enter</kbd> | `insert_newline` | Insert newline |
| <kbd>↑</kbd> | | `move_line_up` | Move to line above |
| <kbd>↓</kbd> | | `move_line_down` | Move to line below |
| <kbd>←</kbd> | | `move_char_left` | Move left |
| <kbd>→</kbd> | | `move_char_right` | Move right |
| <kbd>PgUp</kbd> | | `page_up` | Move page up |
| <kbd>PgDn</kbd> | | `page_down` | Move page down |
| <kbd>Home</kbd> | | `goto_line_start` | Move to line start |
| <kbd>End</kbd> | | `goto_line_end` | Move to line end |

> [!TIP]
> Arrow keys and navigation in insert mode are discouraged. Use <kbd>Esc</kbd> to return to normal mode for navigation.

---

## Select Mode

Select mode (entered with <kbd>v</kbd> in normal mode) echoes all normal mode commands, but selections are extended instead of replaced. Press <kbd>v</kbd> again to return to normal mode.

---

## View Mode (<kbd>z</kbd>)

Entered by pressing <kbd>z</kbd> in normal mode. Used for scrolling and viewing without changing selections.

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>z</kbd> | <kbd>c</kbd> | `align_view_center` | Vertically center the line |
| <kbd>t</kbd> | | `align_view_top` | Align the line to the top of the screen |
| <kbd>b</kbd> | | `align_view_bottom` | Align the line to the bottom of the screen |
| <kbd>m</kbd> | | `align_view_middle` | Align the line to the middle of the screen (horizontally) |
| <kbd>j</kbd> | <kbd>↓</kbd> | `scroll_down` | Scroll the view downwards |
| <kbd>k</kbd> | <kbd>↑</kbd> | `scroll_up` | Scroll the view upwards |
| <kbd>Ctrl</kbd>+<kbd>f</kbd> | <kbd>PgDn</kbd> | `page_down` | Move page down |
| <kbd>Ctrl</kbd>+<kbd>b</kbd> | <kbd>PgUp</kbd> | `page_up` | Move page up |
| <kbd>Ctrl</kbd>+<kbd>u</kbd> | | `page_cursor_half_up` | Move cursor and page half page up |
| <kbd>Ctrl</kbd>+<kbd>d</kbd> | | `page_cursor_half_down` | Move cursor and page half page down |

---

## Goto Mode (<kbd>g</kbd>)

Entered by pressing <kbd>g</kbd> in normal mode. Jumps to various locations in file or workspace.

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>g</kbd> | `goto_file_start` | Go to line number N (if preceded by count) else start of file |
| <kbd>\|</kbd> | `goto_column` | Go to column number N (if preceded by count) else start of line |
| <kbd>e</kbd> | `goto_last_line` | Go to the end of the file |
| <kbd>f</kbd> | `goto_file` | Go to files in the selections |
| <kbd>h</kbd> | `goto_line_start` | Go to the start of the line |
| <kbd>l</kbd> | `goto_line_end` | Go to the end of the line |
| <kbd>s</kbd> | `goto_first_nonwhitespace` | Go to first non-whitespace character of the line |
| <kbd>t</kbd> | `goto_window_top` | Go to the top of the screen |
| <kbd>c</kbd> | `goto_window_center` | Go to the middle of the screen |
| <kbd>b</kbd> | `goto_window_bottom` | Go to the bottom of the screen |
| <kbd>d</kbd> | `goto_definition` | Go to definition (requires LSP) |
| <kbd>y</kbd> | `goto_type_definition` | Go to type definition (requires LSP) |
| <kbd>r</kbd> | `goto_reference` | Go to references (requires LSP) |
| <kbd>i</kbd> | `goto_implementation` | Go to implementation (requires LSP) |
| <kbd>a</kbd> | `goto_last_accessed_file` | Go to the last accessed/alternate file |
| <kbd>m</kbd> | `goto_last_modified_file` | Go to the last modified/alternate file |
| <kbd>n</kbd> | `goto_next_buffer` | Go to next buffer |
| <kbd>p</kbd> | `goto_previous_buffer` | Go to previous buffer |
| <kbd>.</kbd> | `goto_last_modification` | Go to last modification in current file |
| <kbd>j</kbd> | `move_line_down` | Move down textual (instead of visual) line |
| <kbd>k</kbd> | `move_line_up` | Move up textual (instead of visual) line |
| <kbd>w</kbd> | `goto_word` | Show labels at each word and select the word that belongs to entered labels |

---

## Match Mode (<kbd>m</kbd>)

Entered by pressing <kbd>m</kbd> in normal mode. Handles bracket matching, surrounding characters, and text objects.

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>m</kbd> | `match_brackets` | Go to matching bracket (uses tree-sitter) |
| <kbd>s</kbd> + char | `surround_add` | Surround current selection with char |
| <kbd>r</kbd> + from + to | `surround_replace` | Replace surround character from with to |
| <kbd>d</kbd> + char | `surround_delete` | Delete surround character char |
| <kbd>a</kbd> + object | `select_textobject_around` | Select around text object |
| <kbd>i</kbd> + object | `select_textobject_inner` | Select inside text object |

### Text Objects

After pressing <kbd>m</kbd><kbd>a</kbd> or <kbd>m</kbd><kbd>i</kbd>, press one of these:

| Key | Object |
|:---:|--------|
| <kbd>w</kbd> | word |
| <kbd>W</kbd> | WORD |
| <kbd>p</kbd> | paragraph |
| <kbd>(</kbd> <kbd>[</kbd> <kbd>{</kbd> <kbd><</kbd> | surrounded by brackets |
| <kbd>'</kbd> <kbd>"</kbd> <kbd>`</kbd> | surrounded by quotes |
| <kbd>m</kbd> | closest surrounding pair (smart) |
| <kbd>f</kbd> | function |
| <kbd>t</kbd> | type (class, struct, enum) |
| <kbd>a</kbd> | argument/parameter |
| <kbd>c</kbd> | comment |
| <kbd>T</kbd> | test |
| <kbd>g</kbd> | change (diff hunk) |

---

## Window Mode (<kbd>Ctrl</kbd>+<kbd>w</kbd>)

Entered by pressing <kbd>Ctrl</kbd>+<kbd>w</kbd> in normal mode. Manages split windows and navigation.

| Key | Alt | Command | Description |
|:---:|:---:|---------|-------------|
| <kbd>w</kbd> | <kbd>Ctrl</kbd>+<kbd>w</kbd> | `rotate_view` | Switch to next window |
| <kbd>v</kbd> | <kbd>Ctrl</kbd>+<kbd>v</kbd> | `vsplit` | Vertical right split |
| <kbd>s</kbd> | <kbd>Ctrl</kbd>+<kbd>s</kbd> | `hsplit` | Horizontal bottom split |
| <kbd>f</kbd> | | `goto_file` | Go to files in selections in horizontal splits |
| <kbd>F</kbd> | | `goto_file` | Go to files in selections in vertical splits |
| <kbd>h</kbd> | <kbd>Ctrl</kbd>+<kbd>h</kbd>, <kbd>←</kbd> | `jump_view_left` | Move to left split |
| <kbd>j</kbd> | <kbd>Ctrl</kbd>+<kbd>j</kbd>, <kbd>↓</kbd> | `jump_view_down` | Move to split below |
| <kbd>k</kbd> | <kbd>Ctrl</kbd>+<kbd>k</kbd>, <kbd>↑</kbd> | `jump_view_up` | Move to split above |
| <kbd>l</kbd> | <kbd>Ctrl</kbd>+<kbd>l</kbd>, <kbd>→</kbd> | `jump_view_right` | Move to right split |
| <kbd>q</kbd> | <kbd>Ctrl</kbd>+<kbd>q</kbd> | `wclose` | Close current window |
| <kbd>o</kbd> | <kbd>Ctrl</kbd>+<kbd>o</kbd> | `wonly` | Only keep the current window, closing all the others |
| <kbd>H</kbd> | | `swap_view_left` | Swap window to the left |
| <kbd>J</kbd> | | `swap_view_down` | Swap window downwards |
| <kbd>K</kbd> | | `swap_view_up` | Swap window upwards |
| <kbd>L</kbd> | | `swap_view_right` | Swap window to the right |

---

## Space Mode (<kbd>Space</kbd>)

Entered by pressing <kbd>Space</kbd> in normal mode. Contains pickers and various utility commands.

### File and Buffer Operations

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>f</kbd> | `file_picker` | Open file picker (search files in LSP workspace root) |
| <kbd>F</kbd> | `file_picker_in_current_directory` | Open file picker at current working directory |
| <kbd>b</kbd> | `buffer_picker` | Open buffer picker |
| <kbd>j</kbd> | `jumplist_picker` | Open jumplist picker |
| <kbd>g</kbd> | `changed_file_picker` | Open changed file picker (git) |
| <kbd>G</kbd> | | Debug (experimental) |
| <kbd>'</kbd> | `last_picker` | Open last fuzzy picker |

### LSP Operations

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>k</kbd> | `hover` | Show documentation for item under cursor in popup (LSP) |
| <kbd>s</kbd> | `symbol_picker` | Open document symbol picker (LSP) |
| <kbd>S</kbd> | `workspace_symbol_picker` | Open workspace symbol picker (LSP) |
| <kbd>d</kbd> | `diagnostics_picker` | Open document diagnostics picker (LSP) |
| <kbd>D</kbd> | `workspace_diagnostics_picker` | Open workspace diagnostics picker (LSP) |
| <kbd>r</kbd> | `rename_symbol` | Rename symbol (LSP) |
| <kbd>a</kbd> | `code_action` | Apply code action (LSP) |
| <kbd>h</kbd> | `select_references_to_symbol_under_cursor` | Select all references to symbol under cursor (LSP) |

### Comments and Editing

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>c</kbd> | `toggle_comments` | Comment/uncomment selections |
| <kbd>C</kbd> | `toggle_block_comments` | Block comment/uncomment selections |
| <kbd>Alt</kbd>+<kbd>c</kbd> | `toggle_line_comments` | Line comment/uncomment selections |

### Clipboard Operations

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>p</kbd> | `paste_clipboard_after` | Paste system clipboard after selections |
| <kbd>P</kbd> | `paste_clipboard_before` | Paste system clipboard before selections |
| <kbd>y</kbd> | `yank_to_clipboard` | Yank selections to system clipboard |
| <kbd>Y</kbd> | `yank_main_selection_to_clipboard` | Yank main selection to system clipboard |
| <kbd>R</kbd> | `replace_selections_with_clipboard` | Replace selections by system clipboard contents |

### Utility

| Key | Command | Description |
|:---:|---------|-------------|
| <kbd>w</kbd> | | Enter window mode (same as <kbd>Ctrl</kbd>+<kbd>w</kbd>) |
| <kbd>/</kbd> | `global_search` | Global search in workspace folder (ripgrep) |
| <kbd>?</kbd> | `command_palette` | Open command palette |

---

## Picker Mode

When a picker (fuzzy finder) is open:

| Key | Alt | Description |
|:---:|:---:|-------------|
| <kbd>Shift</kbd>+<kbd>Tab</kbd> | <kbd>↑</kbd>, <kbd>Ctrl</kbd>+<kbd>p</kbd> | Previous entry |
| <kbd>Tab</kbd> | <kbd>↓</kbd>, <kbd>Ctrl</kbd>+<kbd>n</kbd> | Next entry |
| <kbd>PgUp</kbd> | | Previous page |
| <kbd>PgDn</kbd> | | Next page |
| <kbd>Home</kbd> | | Go to first entry |
| <kbd>End</kbd> | | Go to last entry |
| <kbd>Enter</kbd> | | Open selected item, close picker |
| <kbd>Alt</kbd>+<kbd>Enter</kbd> | | Open selected item without closing picker |
| <kbd>Ctrl</kbd>+<kbd>s</kbd> | | Open in horizontal split |
| <kbd>Ctrl</kbd>+<kbd>v</kbd> | | Open in vertical split |
| <kbd>Ctrl</kbd>+<kbd>t</kbd> | | Toggle preview panel |
| <kbd>Esc</kbd> | <kbd>Ctrl</kbd>+<kbd>c</kbd> | Close picker |

---

## Prompt Mode

When a prompt (command line) is open:

### Navigation

| Key | Alt | Description |
|:---:|:---:|-------------|
| <kbd>Esc</kbd> | <kbd>Ctrl</kbd>+<kbd>c</kbd> | Close prompt |
| <kbd>Alt</kbd>+<kbd>b</kbd> | <kbd>Ctrl</kbd>+<kbd>←</kbd> | Move cursor to previous word |
| <kbd>Ctrl</kbd>+<kbd>b</kbd> | <kbd>←</kbd> | Move cursor left |
| <kbd>Alt</kbd>+<kbd>f</kbd> | <kbd>Ctrl</kbd>+<kbd>→</kbd> | Move cursor to next word |
| <kbd>Ctrl</kbd>+<kbd>f</kbd> | <kbd>→</kbd> | Move cursor right |
| <kbd>Ctrl</kbd>+<kbd>e</kbd> | <kbd>End</kbd> | Move cursor to end of line |
| <kbd>Ctrl</kbd>+<kbd>a</kbd> | <kbd>Home</kbd> | Move cursor to start of line |

### Editing

| Key | Alt | Description |
|:---:|:---:|-------------|
| <kbd>Ctrl</kbd>+<kbd>w</kbd> | <kbd>Alt</kbd>+<kbd>Backspace</kbd>, <kbd>Ctrl</kbd>+<kbd>Backspace</kbd> | Delete previous word |
| <kbd>Alt</kbd>+<kbd>d</kbd> | <kbd>Alt</kbd>+<kbd>Delete</kbd>, <kbd>Ctrl</kbd>+<kbd>Delete</kbd> | Delete next word |
| <kbd>Ctrl</kbd>+<kbd>u</kbd> | | Delete to start of line |
| <kbd>Ctrl</kbd>+<kbd>k</kbd> | | Delete to end of line |
| <kbd>Backspace</kbd> | <kbd>Ctrl</kbd>+<kbd>h</kbd> | Delete previous character |
| <kbd>Delete</kbd> | <kbd>Ctrl</kbd>+<kbd>d</kbd> | Delete next character |

### History and Completion

| Key | Alt | Description |
|:---:|:---:|-------------|
| <kbd>Ctrl</kbd>+<kbd>p</kbd> | <kbd>↑</kbd> | Previous history entry |
| <kbd>Ctrl</kbd>+<kbd>n</kbd> | <kbd>↓</kbd> | Next history entry |
| <kbd>Tab</kbd> | | Select next completion |
| <kbd>Shift</kbd>+<kbd>Tab</kbd> | | Select previous completion |
| <kbd>Enter</kbd> | | Confirm and close prompt |

---

## Unimpaired Mappings

Bracket-based mappings for quick navigation:

### Diagnostics

| Key | Description |
|:---:|-------------|
| <kbd>[</kbd><kbd>d</kbd> | Go to previous diagnostic |
| <kbd>]</kbd><kbd>d</kbd> | Go to next diagnostic |
| <kbd>[</kbd><kbd>D</kbd> | Go to first diagnostic |
| <kbd>]</kbd><kbd>D</kbd> | Go to last diagnostic |

### Syntax Tree Navigation

| Key | Description |
|:---:|-------------|
| <kbd>[</kbd><kbd>f</kbd> | Go to previous function |
| <kbd>]</kbd><kbd>f</kbd> | Go to next function |
| <kbd>[</kbd><kbd>t</kbd> | Go to previous type definition |
| <kbd>]</kbd><kbd>t</kbd> | Go to next type definition |
| <kbd>[</kbd><kbd>a</kbd> | Go to previous argument/parameter |
| <kbd>]</kbd><kbd>a</kbd> | Go to next argument/parameter |
| <kbd>[</kbd><kbd>c</kbd> | Go to previous comment |
| <kbd>]</kbd><kbd>c</kbd> | Go to next comment |
| <kbd>[</kbd><kbd>T</kbd> | Go to previous test |
| <kbd>]</kbd><kbd>T</kbd> | Go to next test |
| <kbd>[</kbd><kbd>p</kbd> | Go to previous paragraph |
| <kbd>]</kbd><kbd>p</kbd> | Go to next paragraph |

### Git Changes

| Key | Description |
|:---:|-------------|
| <kbd>[</kbd><kbd>g</kbd> | Go to previous change (diff hunk) |
| <kbd>]</kbd><kbd>g</kbd> | Go to next change (diff hunk) |
| <kbd>[</kbd><kbd>G</kbd> | Go to first change |
| <kbd>]</kbd><kbd>G</kbd> | Go to last change |

### Whitespace

| Key | Description |
|:---:|-------------|
| <kbd>[</kbd><kbd>Space</kbd> | Add newline above |
| <kbd>]</kbd><kbd>Space</kbd> | Add newline below |

---

## Command Mode Commands

Access command mode with <kbd>:</kbd> in normal mode. Common commands:

| Command | Alias | Description |
|---------|-------|-------------|
| `:quit` | `:q` | Close current view |
| `:quit!` | `:q!` | Close without saving |
| `:write` | `:w` | Write buffer to file |
| `:write-quit` | `:wq` | Write and close |
| `:write-all` | `:wa` | Write all modified buffers |
| `:quit-all` | `:qa` | Close all views |
| `:open` | `:o` | Open file |
| `:buffer-close` | `:bc` | Close current buffer |
| `:new` | | Create new scratch buffer |
| `:format` | `:fmt` | Format buffer with LSP |
| `:reload` | | Reload file from disk |
| `:update` | | Write if buffer modified |
| `:vsplit` | `:vs` | Open file in vertical split |
| `:hsplit` | `:hs`, `:sp` | Open file in horizontal split |
| `:goto` | `:g` | Go to line number |
| `:theme` | | Change theme |
| `:tutor` | | Open tutorial |
| `:earlier` | | Move backward in undo history |
| `:later` | | Move forward in undo history |
| `:set-language` | `:lang` | Set language for syntax highlighting |
| `:set-option` | `:set` | Set config option |
| `:toggle` | | Toggle config option |
| `:clipboard-yank` | | Yank main selection to clipboard |
| `:clipboard-paste-after` | | Paste clipboard after selections |
| `:sort` | | Sort selections |
| `:rsort` | | Sort selections in reverse |
| `:run-shell-command` | `:sh` | Run shell command and show output |

---

## Implementation Checklist

Track which commands have been implemented in the simulator:

### Movement (Normal Mode)

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>h</kbd> <kbd>j</kbd> <kbd>k</kbd> <kbd>l</kbd> | Basic movement (left, down, up, right) |
| ✅ | <kbd>w</kbd> | Move to next word start |
| ✅ | <kbd>b</kbd> | Move to previous word start |
| ✅ | <kbd>e</kbd> | Move to next word end |
| ✅ | <kbd>W</kbd> <kbd>B</kbd> <kbd>E</kbd> | WORD movement (whitespace-separated) |
| ✅ | <kbd>f</kbd> <kbd>t</kbd> <kbd>F</kbd> <kbd>T</kbd> | Character finding |
| ✅ | <kbd>G</kbd> | Go to line end (or line number with count) |
| ✅ | <kbd>g</kbd><kbd>g</kbd> | Go to document start |
| ✅ | <kbd>g</kbd><kbd>e</kbd> | Go to document end |
| ✅ | <kbd>g</kbd><kbd>h</kbd> | Go to line start |
| ✅ | <kbd>g</kbd><kbd>l</kbd> | Go to line end |
| ✅ | <kbd>g</kbd><kbd>s</kbd> | Go to first non-whitespace |
| ✅ | <kbd>Alt</kbd>+<kbd>.</kbd> | Repeat last motion (f/F/t/T) |
| ✅ | <kbd>0</kbd> | Go to line start |
| ✅ | <kbd>$</kbd> | Go to line end |
| ✅ | <kbd>^</kbd> | Go to first non-whitespace (alias for gs) |
| ✅ | <kbd>Ctrl</kbd>+<kbd>b</kbd>, <kbd>Ctrl</kbd>+<kbd>f</kbd> | Page up/down |
| ✅ | <kbd>Ctrl</kbd>+<kbd>u</kbd>, <kbd>Ctrl</kbd>+<kbd>d</kbd> | Half page up/down |
| ❌ | <kbd>Ctrl</kbd>+<kbd>i</kbd>, <kbd>Ctrl</kbd>+<kbd>o</kbd> | Jump forward/backward |
| ❌ | <kbd>Ctrl</kbd>+<kbd>s</kbd> | Save to jumplist |
| ✅ | <kbd>[</kbd><kbd>p</kbd>, <kbd>]</kbd><kbd>p</kbd> | Go to previous/next paragraph |

### Changes (Normal Mode)

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>r</kbd> + char | Replace character with another char |
| ✅ | <kbd>R</kbd> | Replace selection with yanked text |
| ✅ | <kbd>~</kbd> | Switch case (toggle) |
| ✅ | <kbd>`</kbd> | Switch to lowercase |
| ✅ | <kbd>Alt</kbd>+<kbd>`</kbd> | Switch to uppercase |
| ✅ | <kbd>i</kbd> | Enter insert mode before selection |
| ✅ | <kbd>a</kbd> | Enter insert mode after selection (append) |
| ✅ | <kbd>I</kbd> | Insert at line start |
| ✅ | <kbd>A</kbd> | Append at line end |
| ✅ | <kbd>o</kbd> | Open line below and enter insert mode |
| ✅ | <kbd>O</kbd> | Open line above and enter insert mode |
| ✅ | <kbd>.</kbd> | Repeat last insert operation |
| ✅ | <kbd>u</kbd> | Undo last change |
| ✅ | <kbd>U</kbd> | Redo last undone change |
| ❌ | <kbd>Alt</kbd>+<kbd>u</kbd>, <kbd>Alt</kbd>+<kbd>U</kbd> | History navigation (earlier/later) |
| ✅ | <kbd>y</kbd> | Yank (copy) selection |
| ✅ | <kbd>p</kbd> | Paste after selection |
| ✅ | <kbd>P</kbd> | Paste before selection |
| ❌ | <kbd>"</kbd> + reg | Select register for yank/paste |
| ✅ | <kbd>></kbd> | Indent selection |
| ✅ | <kbd><</kbd> | Unindent selection |
| ❌ | <kbd>=</kbd> | Format selection (LSP) |
| ✅ | <kbd>d</kbd> | Delete selection |
| ❌ | <kbd>Alt</kbd>+<kbd>d</kbd> | Delete without yanking |
| ✅ | <kbd>c</kbd> | Change selection (delete and enter insert mode) |
| ❌ | <kbd>Alt</kbd>+<kbd>c</kbd> | Change without yanking |
| ❌ | <kbd>Ctrl</kbd>+<kbd>a</kbd>, <kbd>Ctrl</kbd>+<kbd>x</kbd> | Increment/decrement number |
| ❌ | <kbd>Q</kbd>, <kbd>q</kbd> | Record/replay macro |

### Selection & Line Operations

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>s</kbd> | Select regex matches in selection |
| ✅ | <kbd>S</kbd> | Split selection on regex |
| ✅ | <kbd>Alt</kbd>+<kbd>s</kbd> | Split selection on newlines |
| ✅ | <kbd>&</kbd> | Align selections in columns |
| ✅ | <kbd>_</kbd> | Trim whitespace from selections |
| ✅ | <kbd>;</kbd> | Collapse selection to cursor |
| ❌ | <kbd>Alt</kbd>+<kbd>;</kbd> | Flip selection direction |
| ✅ | <kbd>,</kbd> | Keep only primary selection |
| ✅ | <kbd>Alt</kbd>+<kbd>,</kbd> | Remove primary selection |
| ✅ | <kbd>C</kbd> | Copy selection to next line |
| ✅ | <kbd>Alt</kbd>+<kbd>C</kbd> | Copy selection to previous line |
| ✅ | <kbd>%</kbd> | Select entire file |
| ✅ | <kbd>x</kbd> | Extend line below |
| ✅ | <kbd>X</kbd> | Extend selection to line bounds |
| ✅ | <kbd>Alt</kbd>+<kbd>x</kbd> | Shrink selection to line bounds |
| ✅ | <kbd>J</kbd> | Join lines (remove newlines) |
| ✅ | <kbd>Alt</kbd>+<kbd>J</kbd> | Join lines with space |
| ✅ | <kbd>K</kbd> | Keep selections matching regex |
| ✅ | <kbd>Alt</kbd>+<kbd>K</kbd> | Remove selections matching regex |
| ✅ | <kbd>Ctrl</kbd>+<kbd>c</kbd> | Toggle line comments |
| ✅ | <kbd>Alt</kbd>+<kbd>-</kbd> | Merge all selections |
| ✅ | <kbd>Alt</kbd>+<kbd>_</kbd> | Merge consecutive selections |
| ✅ | <kbd>v</kbd> | Enter select (extend) mode |

### Insert Mode Commands

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>Esc</kbd> | Return to normal mode |
| ✅ | <kbd>Backspace</kbd> | Delete previous character |
| ✅ | Arrow keys | Navigation in insert mode |
| ✅ | Text input | Insert characters |
| ❌ | <kbd>Ctrl</kbd>+<kbd>x</kbd> | Autocomplete |
| ❌ | <kbd>Ctrl</kbd>+<kbd>w</kbd>, <kbd>Alt</kbd>+<kbd>Backspace</kbd> | Delete word backward |
| ❌ | <kbd>Alt</kbd>+<kbd>d</kbd> | Delete word forward |
| ❌ | <kbd>Ctrl</kbd>+<kbd>u</kbd> | Kill to line start |
| ❌ | <kbd>Ctrl</kbd>+<kbd>k</kbd> | Kill to line end |

### Search Commands

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>/</kbd> | Search forward with regex |
| ✅ | <kbd>?</kbd> | Search backward with regex |
| ✅ | <kbd>n</kbd> | Jump to next match |
| ✅ | <kbd>N</kbd> | Jump to previous match |
| ✅ | <kbd>*</kbd> | Search word under cursor forward |
| ✅ | <kbd>#</kbd> | Search word under cursor backward |
| ✅ | <kbd>Alt</kbd>+<kbd>*</kbd> | Search selection text |

### View Commands

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>z</kbd> / <kbd>zz</kbd> | Center view on cursor |
| ✅ | <kbd>zt</kbd> | Scroll cursor to top |
| ✅ | <kbd>zb</kbd> | Scroll cursor to bottom |
| ✅ | <kbd>zm</kbd> | Center horizontally |
| ✅ | <kbd>zj</kbd> | Scroll view down |
| ✅ | <kbd>zk</kbd> | Scroll view up |

### Match Mode Commands

| Status | Keys | Description |
|:------:|------|-------------|
| ✅ | <kbd>mm</kbd> | Jump to matching bracket |
| ✅ | <kbd>ms</kbd> + char | Add surround (wrap selection) |
| ✅ | <kbd>md</kbd> + char | Delete surround |
| ✅ | <kbd>mr</kbd> + char + char | Replace surround |
| ✅ | <kbd>ma</kbd> + object | Select around text object |
| ✅ | <kbd>mi</kbd> + object | Select inside text object |

---

## Implementation Summary

**Phase 2.2 Complete: 90+ Commands Implemented**

**Implemented:** 90+ commands covering most essential Helix operations

### By Category

| Category | Count | Commands |
|----------|:-----:|----------|
| **Movement** | 23 | <kbd>h</kbd> <kbd>j</kbd> <kbd>k</kbd> <kbd>l</kbd> <kbd>w</kbd> <kbd>b</kbd> <kbd>e</kbd> <kbd>W</kbd> <kbd>B</kbd> <kbd>E</kbd> <kbd>0</kbd> <kbd>$</kbd> <kbd>^</kbd> <kbd>gg</kbd> <kbd>ge</kbd> <kbd>gh</kbd> <kbd>gl</kbd> <kbd>gs</kbd> <kbd>G</kbd> <kbd>f</kbd> <kbd>F</kbd> <kbd>t</kbd> <kbd>T</kbd> |
| **Paragraph** | 2 | <kbd>[p</kbd> <kbd>]p</kbd> |
| **Page** | 4 | <kbd>Ctrl+b</kbd> <kbd>Ctrl+f</kbd> <kbd>Ctrl+u</kbd> <kbd>Ctrl+d</kbd> |
| **Editing** | 17 | <kbd>i</kbd> <kbd>a</kbd> <kbd>I</kbd> <kbd>A</kbd> <kbd>o</kbd> <kbd>O</kbd> <kbd>d</kbd> <kbd>c</kbd> <kbd>r</kbd> <kbd>R</kbd> <kbd>~</kbd> <kbd>`</kbd> <kbd>Alt+`</kbd> <kbd>.</kbd> <kbd>u</kbd> <kbd>U</kbd> |
| **Selection** | 20 | <kbd>x</kbd> <kbd>X</kbd> <kbd>v</kbd> <kbd>;</kbd> <kbd>,</kbd> <kbd>%</kbd> <kbd>s</kbd> <kbd>S</kbd> <kbd>C</kbd> <kbd>K</kbd> <kbd>Alt+C</kbd> <kbd>Alt+K</kbd> <kbd>Alt+s</kbd> <kbd>Alt+x</kbd> <kbd>_</kbd> <kbd>&</kbd> <kbd>Alt+-</kbd> <kbd>Alt+_</kbd> <kbd>Alt+,</kbd> |
| **Search** | 7 | <kbd>/</kbd> <kbd>?</kbd> <kbd>n</kbd> <kbd>N</kbd> <kbd>*</kbd> <kbd>#</kbd> <kbd>Alt+*</kbd> |
| **View** | 6 | <kbd>z</kbd> <kbd>zz</kbd> <kbd>zt</kbd> <kbd>zb</kbd> <kbd>zm</kbd> <kbd>zj</kbd> <kbd>zk</kbd> |
| **Match Mode** | 6 | <kbd>mm</kbd> <kbd>ms</kbd> <kbd>md</kbd> <kbd>mr</kbd> <kbd>ma</kbd> <kbd>mi</kbd> |
| **Indentation** | 2 | <kbd>></kbd> <kbd><</kbd> |
| **Line ops** | 3 | <kbd>J</kbd> <kbd>Alt+J</kbd> <kbd>Ctrl+c</kbd> |
| **Clipboard** | 3 | <kbd>y</kbd> <kbd>p</kbd> <kbd>P</kbd> |
| **Repeat** | 2 | <kbd>.</kbd> <kbd>Alt+.</kbd> |

### Training Scenarios Coverage

- ✅ 136 scenarios covering 90+ implemented commands
- ✅ All scenarios use realistic Rust code
- ✅ Syntax highlighting for code display
- ✅ Multiple difficulty levels per command
- ✅ Hints and alternative solutions provided
- ✅ Organized in thematic directory structure

### Not Yet Implemented (Future Phases)

- Tree-sitter selections (<kbd>Alt+o</kbd>, <kbd>Alt+i</kbd>, etc.)
- LSP integration commands
- Macros and registers (<kbd>Q</kbd>, <kbd>q</kbd>, <kbd>"</kbd>)
- Window mode (<kbd>Ctrl+w</kbd>)
- Space mode (<kbd>Space</kbd>)
- Jumplist (<kbd>Ctrl+i</kbd>, <kbd>Ctrl+o</kbd>)

---

*End of Keybindings Reference*
