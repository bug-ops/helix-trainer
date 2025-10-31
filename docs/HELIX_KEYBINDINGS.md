# Helix Editor - Complete Keybindings Reference

> Source: <https://docs.helix-editor.com/keymap.html>
> Generated: 2025-10-30

This document contains a comprehensive reference of all Helix editor keybindings for use in developing training scenarios.

## Table of Contents

- [Normal Mode](#normal-mode)
- [Insert Mode](#insert-mode)
- [Select Mode](#select-mode)
- [View Mode (z)](#view-mode-z)
- [Goto Mode (g)](#goto-mode-g)
- [Match Mode (m)](#match-mode-m)
- [Window Mode (Ctrl-w)](#window-mode-ctrl-w)
- [Space Mode (Space)](#space-mode-space)
- [Picker Mode](#picker-mode)
- [Prompt Mode](#prompt-mode)
- [Unimpaired Mappings](#unimpaired-mappings)

---

## Normal Mode

### Movement Commands

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `h` | `Left` | `move_char_left` | Move left one character |
| `j` | `Down` | `move_visual_line_down` | Move down one visual line |
| `k` | `Up` | `move_visual_line_up` | Move up one visual line |
| `l` | `Right` | `move_char_right` | Move right one character |
| `w` | | `move_next_word_start` | Move to next word start |
| `b` | | `move_prev_word_start` | Move to previous word start |
| `e` | | `move_next_word_end` | Move to next word end |
| `W` | | `move_next_long_word_start` | Move to next WORD start (whitespace-separated) |
| `B` | | `move_prev_long_word_start` | Move to previous WORD start |
| `E` | | `move_next_long_word_end` | Move to next WORD end |
| `t` | | `find_till_char` | Find till next occurrence of character |
| `f` | | `find_next_char` | Find next occurrence of character |
| `T` | | `till_prev_char` | Find till previous occurrence of character |
| `F` | | `find_prev_char` | Find previous occurrence of character |
| `G` | | `goto_line` | Go to line number (or end if no number) |
| `Alt-.` | | `repeat_last_motion` | Repeat last motion (f, t, m, etc.) |
| `Home` | | `goto_line_start` | Go to start of line |
| `End` | | `goto_line_end` | Go to end of line |
| `Ctrl-b` | `PageUp` | `page_up` | Move page up |
| `Ctrl-f` | `PageDown` | `page_down` | Move page down |
| `Ctrl-u` | | `page_cursor_half_up` | Move cursor and page half page up |
| `Ctrl-d` | | `page_cursor_half_down` | Move cursor and page half page down |
| `Ctrl-i` | | `jump_forward` | Jump forward on jumplist |
| `Ctrl-o` | | `jump_backward` | Jump backward on jumplist |
| `Ctrl-s` | | `save_selection` | Save current selection to jumplist |

### Change Commands

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `r` | | `replace` | Replace each character in selection with another character |
| `R` | | `replace_with_yanked` | Replace selection with yanked text |
| `~` | | `switch_case` | Switch case of selected text (toggle) |
| `` ` `` | | `switch_to_lowercase` | Switch selected text to lowercase |
| ``Alt-` `` | | `switch_to_uppercase` | Switch selected text to uppercase |
| `i` | | `insert_mode` | Enter insert mode before selection |
| `a` | | `append_mode` | Enter insert mode after selection |
| `I` | | `insert_at_line_start` | Enter insert mode at start of line |
| `A` | | `insert_at_line_end` | Enter insert mode at end of line |
| `o` | | `open_below` | Open new line below and enter insert mode |
| `O` | | `open_above` | Open new line above and enter insert mode |
| `.` | | | Repeat last insert operation |
| `u` | | `undo` | Undo last change |
| `U` | | `redo` | Redo last undone change |
| `Alt-u` | | `earlier` | Move backward in history |
| `Alt-U` | | `later` | Move forward in history |
| `y` | | `yank` | Yank (copy) selection |
| `p` | | `paste_after` | Paste after selection |
| `P` | | `paste_before` | Paste before selection |
| `"<reg>` | | `select_register` | Select a register for next yank or paste |
| `>` | | `indent` | Indent selection |
| `<` | | `unindent` | Unindent selection |
| `=` | | `format_selections` | Format selection using LSP |
| `d` | | `delete_selection` | Delete selection (and yank) |
| `Alt-d` | | `delete_selection_noyank` | Delete selection without yanking |
| `c` | | `change_selection` | Change selection (delete and enter insert mode) |
| `Alt-c` | | `change_selection_noyank` | Change selection without yanking |
| `Ctrl-a` | | `increment` | Increment number under cursor |
| `Ctrl-x` | | `decrement` | Decrement number under cursor |
| `Q` | | `record_macro` | Start/stop recording macro to register |
| `q` | | `replay_macro` | Play back recorded macro from register |

### Shell Commands

| Key | Command | Description |
|-----|---------|-------------|
| `\|` | `shell_pipe` | Pipe each selection through shell command, replacing with output |
| `Alt-\|` | `shell_pipe_to` | Pipe each selection to shell command, ignore output |
| `!` | `shell_insert_output` | Insert shell command output before each selection |
| `Alt-!` | `shell_append_output` | Append shell command output after each selection |
| `$` | `shell_keep_pipe` | Pipe selections to shell command, keep only selections where command succeeded |

### Selection Manipulation

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `s` | | `select_regex` | Select all regex matches inside selections |
| `S` | | `split_selection` | Split selections on regex matches |
| `Alt-s` | | `split_selection_on_newline` | Split selections on newlines |
| `Alt-minus` | `Alt--` | `merge_selections` | Merge selections |
| `Alt-_` | | `merge_consecutive_selections` | Merge consecutive selections |
| `&` | | `align_selections` | Align selections in columns |
| `_` | | `trim_selections` | Trim whitespace from selections |
| `;` | | `collapse_selection` | Collapse selections to their cursor |
| `Alt-;` | | `flip_selections` | Flip the direction of selections (swap cursor and anchor) |
| `Alt-:` | | `ensure_selections_forward` | Ensure all selections face forward |
| `,` | | `keep_primary_selection` | Keep only the primary selection |
| `Alt-,` | | `remove_primary_selection` | Remove the primary selection |
| `C` | | `copy_selection_on_next_line` | Copy selection to next line (add cursor below) |
| `Alt-C` | | `copy_selection_on_prev_line` | Copy selection to previous line (add cursor above) |
| `(` | | `rotate_selections_backward` | Rotate primary selection backward |
| `)` | | `rotate_selections_forward` | Rotate primary selection forward |
| `Alt-(` | | `rotate_selection_contents_backward` | Rotate selection contents backward |
| `Alt-)` | | `rotate_selection_contents_forward` | Rotate selection contents forward |
| `%` | | `select_all` | Select entire file |
| `x` | | `extend_line_below` | Select current line, extend selection to include next line |
| `X` | | `extend_to_line_bounds` | Extend selection to line bounds (line-wise selection) |
| `Alt-x` | | `shrink_to_line_bounds` | Shrink selection to line bounds |
| `J` | | `join_selections` | Join lines inside selection (remove newlines) |
| `Alt-J` | | `join_selections_space` | Join lines with space |
| `K` | | `keep_selections` | Keep selections matching regex |
| `Alt-K` | | `remove_selections` | Remove selections matching regex |
| `Ctrl-c` | | `toggle_comments` | Toggle line comments for selections |

### Tree-sitter Selection Commands

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Alt-o` | `Alt-Up` | `expand_selection` | Expand selection to parent syntax node |
| `Alt-i` | `Alt-Down` | `shrink_selection` | Shrink syntax tree object selection |
| `Alt-p` | `Alt-Left` | `select_prev_sibling` | Select previous sibling node in syntax tree |
| `Alt-n` | `Alt-Right` | `select_next_sibling` | Select next sibling node in syntax tree |
| `Alt-a` | | `select_all_siblings` | Select all sibling nodes in syntax tree |
| `Alt-I` | `Alt-Shift-Down` | `select_all_children` | Select all children of current syntax node |
| `Alt-e` | | `move_parent_node_end` | Move to end of parent syntax node |
| `Alt-b` | | `move_parent_node_start` | Move to start of parent syntax node |

### Search Commands

| Key | Command | Description |
|-----|---------|-------------|
| `/` | `search` | Search for regex pattern forward |
| `?` | `rsearch` | Search for regex pattern backward |
| `n` | `search_next` | Select next search match |
| `N` | `search_prev` | Select previous search match |
| `*` | `search_selection_detect_word_boundaries` | Use current selection as search pattern (word boundaries) |
| `Alt-*` | `search_selection` | Use current selection as search pattern (exact) |

### Mode Switching

| Key | Command | Description |
|-----|---------|-------------|
| `v` | `select_mode` | Enter select (extend) mode |
| `g` | | Enter goto mode |
| `m` | | Enter match mode |
| `:` | `command_mode` | Enter command mode |
| `z` | | Enter view mode |
| `Z` | | Enter sticky view mode |
| `Ctrl-w` | | Enter window mode |
| `Space` | | Enter space mode |

---

## Insert Mode

In insert mode, most keys insert their character. Special keys:

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Escape` | | `normal_mode` | Switch to normal mode |
| `Ctrl-s` | | `commit_undo_checkpoint` | Commit a new undo checkpoint |
| `Ctrl-x` | | `completion` | Autocomplete |
| `Ctrl-r` | | `insert_register` | Insert contents of register |
| `Ctrl-w` | `Alt-Backspace` | `delete_word_backward` | Delete previous word |
| `Alt-d` | | `delete_word_forward` | Delete next word |
| `Ctrl-u` | | `kill_to_line_start` | Delete from cursor to start of line |
| `Ctrl-k` | | `kill_to_line_end` | Delete from cursor to end of line |
| `Ctrl-h` | `Backspace` | `delete_char_backward` | Delete previous character |
| `Ctrl-d` | `Delete` | `delete_char_forward` | Delete next character |
| `Ctrl-j` | `Enter` | `insert_newline` | Insert newline |
| `Up` | | `move_line_up` | Move to line above |
| `Down` | | `move_line_down` | Move to line below |
| `Left` | | `move_char_left` | Move left |
| `Right` | | `move_char_right` | Move right |
| `PageUp` | | `page_up` | Move page up |
| `PageDown` | | `page_down` | Move page down |
| `Home` | | `goto_line_start` | Move to line start |
| `End` | | `goto_line_end` | Move to line end |

Note: Arrow keys and navigation in insert mode are discouraged - use Escape to return to normal mode for navigation.

---

## Select Mode

Select mode (entered with `v` in normal mode) echoes all normal mode commands, but selections are extended instead of replaced. Press `v` again to return to normal mode.

---

## View Mode (z)

Entered by pressing `z` in normal mode. Used for scrolling and viewing without changing selections.

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `z` | `c` | `align_view_center` | Vertically center the line |
| `t` | | `align_view_top` | Align the line to the top of the screen |
| `b` | | `align_view_bottom` | Align the line to the bottom of the screen |
| `m` | | `align_view_middle` | Align the line to the middle of the screen (horizontally) |
| `j` | `Down` | `scroll_down` | Scroll the view downwards |
| `k` | `Up` | `scroll_up` | Scroll the view upwards |
| `Ctrl-f` | `PageDown` | `page_down` | Move page down |
| `Ctrl-b` | `PageUp` | `page_up` | Move page up |
| `Ctrl-u` | | `page_cursor_half_up` | Move cursor and page half page up |
| `Ctrl-d` | | `page_cursor_half_down` | Move cursor and page half page down |

---

## Goto Mode (g)

Entered by pressing `g` in normal mode. Jumps to various locations in file or workspace.

| Key | Command | Description |
|-----|---------|-------------|
| `g` | `goto_file_start` | Go to line number `<n>` (if preceded by count) else start of file |
| `\|` | `goto_column` | Go to column number `<n>` (if preceded by count) else start of line |
| `e` | `goto_last_line` | Go to the end of the file |
| `f` | `goto_file` | Go to files in the selections |
| `h` | `goto_line_start` | Go to the start of the line |
| `l` | `goto_line_end` | Go to the end of the line |
| `s` | `goto_first_nonwhitespace` | Go to first non-whitespace character of the line |
| `t` | `goto_window_top` | Go to the top of the screen |
| `c` | `goto_window_center` | Go to the middle of the screen |
| `b` | `goto_window_bottom` | Go to the bottom of the screen |
| `d` | `goto_definition` | Go to definition (requires LSP) |
| `y` | `goto_type_definition` | Go to type definition (requires LSP) |
| `r` | `goto_reference` | Go to references (requires LSP) |
| `i` | `goto_implementation` | Go to implementation (requires LSP) |
| `a` | `goto_last_accessed_file` | Go to the last accessed/alternate file |
| `m` | `goto_last_modified_file` | Go to the last modified/alternate file |
| `n` | `goto_next_buffer` | Go to next buffer |
| `p` | `goto_previous_buffer` | Go to previous buffer |
| `.` | `goto_last_modification` | Go to last modification in current file |
| `j` | `move_line_down` | Move down textual (instead of visual) line |
| `k` | `move_line_up` | Move up textual (instead of visual) line |
| `w` | `goto_word` | Show labels at each word and select the word that belongs to entered labels |

---

## Match Mode (m)

Entered by pressing `m` in normal mode. Handles bracket matching, surrounding characters, and text objects.

| Key | Command | Description |
|-----|---------|-------------|
| `m` | `match_brackets` | Go to matching bracket (uses tree-sitter) |
| `s <char>` | `surround_add` | Surround current selection with `<char>` |
| `r <from><to>` | `surround_replace` | Replace surround character `<from>` with `<to>` |
| `d <char>` | `surround_delete` | Delete surround character `<char>` |
| `a <object>` | `select_textobject_around` | Select around text object |
| `i <object>` | `select_textobject_inner` | Select inside text object |

### Text Objects

After pressing `ma` or `mi`, press one of these:

- `w` - word
- `W` - WORD
- `p` - paragraph
- `(`, `[`, `{`, `<` - surrounded by brackets
- `'`, `"`, `` ` `` - surrounded by quotes
- `m` - closest surrounding pair (smart)
- `f` - function
- `t` - type (class, struct, enum)
- `a` - argument/parameter
- `c` - comment
- `T` - test
- `g` - change (diff hunk)

---

## Window Mode (Ctrl-w)

Entered by pressing `Ctrl-w` in normal mode. Manages split windows and navigation.

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `w` | `Ctrl-w` | `rotate_view` | Switch to next window |
| `v` | `Ctrl-v` | `vsplit` | Vertical right split |
| `s` | `Ctrl-s` | `hsplit` | Horizontal bottom split |
| `f` | | `goto_file` | Go to files in selections in horizontal splits |
| `F` | | `goto_file` | Go to files in selections in vertical splits |
| `h` | `Ctrl-h`, `Left` | `jump_view_left` | Move to left split |
| `j` | `Ctrl-j`, `Down` | `jump_view_down` | Move to split below |
| `k` | `Ctrl-k`, `Up` | `jump_view_up` | Move to split above |
| `l` | `Ctrl-l`, `Right` | `jump_view_right` | Move to right split |
| `q` | `Ctrl-q` | `wclose` | Close current window |
| `o` | `Ctrl-o` | `wonly` | Only keep the current window, closing all the others |
| `H` | | `swap_view_left` | Swap window to the left |
| `J` | | `swap_view_down` | Swap window downwards |
| `K` | | `swap_view_up` | Swap window upwards |
| `L` | | `swap_view_right` | Swap window to the right |

---

## Space Mode (Space)

Entered by pressing `Space` in normal mode. Contains pickers and various utility commands.

### File and Buffer Operations

| Key | Command | Description |
|-----|---------|-------------|
| `f` | `file_picker` | Open file picker (search files in LSP workspace root) |
| `F` | `file_picker_in_current_directory` | Open file picker at current working directory |
| `b` | `buffer_picker` | Open buffer picker |
| `j` | `jumplist_picker` | Open jumplist picker |
| `g` | `changed_file_picker` | Open changed file picker (git) |
| `G` | | Debug (experimental) |
| `'` | `last_picker` | Open last fuzzy picker |

### LSP Operations

| Key | Command | Description |
|-----|---------|-------------|
| `k` | `hover` | Show documentation for item under cursor in popup (LSP) |
| `s` | `symbol_picker` | Open document symbol picker (LSP) |
| `S` | `workspace_symbol_picker` | Open workspace symbol picker (LSP) |
| `d` | `diagnostics_picker` | Open document diagnostics picker (LSP) |
| `D` | `workspace_diagnostics_picker` | Open workspace diagnostics picker (LSP) |
| `r` | `rename_symbol` | Rename symbol (LSP) |
| `a` | `code_action` | Apply code action (LSP) |
| `h` | `select_references_to_symbol_under_cursor` | Select all references to symbol under cursor (LSP) |

### Comments and Editing

| Key | Command | Description |
|-----|---------|-------------|
| `c` | `toggle_comments` | Comment/uncomment selections |
| `C` | `toggle_block_comments` | Block comment/uncomment selections |
| `Alt-c` | `toggle_line_comments` | Line comment/uncomment selections |

### Clipboard Operations

| Key | Command | Description |
|-----|---------|-------------|
| `p` | `paste_clipboard_after` | Paste system clipboard after selections |
| `P` | `paste_clipboard_before` | Paste system clipboard before selections |
| `y` | `yank_to_clipboard` | Yank selections to system clipboard |
| `Y` | `yank_main_selection_to_clipboard` | Yank main selection to system clipboard |
| `R` | `replace_selections_with_clipboard` | Replace selections by system clipboard contents |

### Utility

| Key | Command | Description |
|-----|---------|-------------|
| `w` | | Enter window mode (same as `Ctrl-w`) |
| `/` | `global_search` | Global search in workspace folder (ripgrep) |
| `?` | `command_palette` | Open command palette |

---

## Picker Mode

When a picker (fuzzy finder) is open:

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Shift-Tab` | `Up`, `Ctrl-p` | | Previous entry |
| `Tab` | `Down`, `Ctrl-n` | | Next entry |
| `PageUp` | | | Previous page |
| `PageDown` | | | Next page |
| `Home` | | | Go to first entry |
| `End` | | | Go to last entry |
| `Enter` | | | Open selected item, close picker |
| `Alt-Enter` | | | Open selected item without closing picker |
| `Ctrl-s` | | | Open in horizontal split |
| `Ctrl-v` | | | Open in vertical split |
| `Ctrl-t` | | | Toggle preview panel |
| `Escape` | `Ctrl-c` | | Close picker |

---

## Prompt Mode

When a prompt (command line) is open:

### Navigation

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Escape` | `Ctrl-c` | | Close prompt |
| `Alt-b` | `Ctrl-Left` | | Move cursor to previous word |
| `Ctrl-b` | `Left` | | Move cursor left |
| `Alt-f` | `Ctrl-Right` | | Move cursor to next word |
| `Ctrl-f` | `Right` | | Move cursor right |
| `Ctrl-e` | `End` | | Move cursor to end of line |
| `Ctrl-a` | `Home` | | Move cursor to start of line |

### Editing

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Ctrl-w` | `Alt-Backspace`, `Ctrl-Backspace` | | Delete previous word |
| `Alt-d` | `Alt-Delete`, `Ctrl-Delete` | | Delete next word |
| `Ctrl-u` | | | Delete to start of line |
| `Ctrl-k` | | | Delete to end of line |
| `Backspace` | `Ctrl-h` | | Delete previous character |
| `Delete` | `Ctrl-d` | | Delete next character |

### History and Completion

| Key | Alt Keys | Command | Description |
|-----|----------|---------|-------------|
| `Ctrl-p` | `Up` | | Previous history entry |
| `Ctrl-n` | `Down` | | Next history entry |
| `Tab` | | | Select next completion |
| `BackTab` | | | Select previous completion |
| `Enter` | | | Confirm and close prompt |

---

## Unimpaired Mappings

These are bracket-based mappings for quick navigation:

### Diagnostics

| Key | Description |
|-----|-------------|
| `[d` | Go to previous diagnostic |
| `]d` | Go to next diagnostic |
| `[D` | Go to first diagnostic |
| `]D` | Go to last diagnostic |

### Syntax Tree Navigation

| Key | Description |
|-----|-------------|
| `[f` | Go to previous function |
| `]f` | Go to next function |
| `[t` | Go to previous type definition |
| `]t` | Go to next type definition |
| `[a` | Go to previous argument/parameter |
| `]a` | Go to next argument/parameter |
| `[c` | Go to previous comment |
| `]c` | Go to next comment |
| `[T` | Go to previous test |
| `]T` | Go to next test |
| `[p` | Go to previous paragraph |
| `]p` | Go to next paragraph |

### Git Changes

| Key | Description |
|-----|-------------|
| `[g` | Go to previous change (diff hunk) |
| `]g` | Go to next change (diff hunk) |
| `[G` | Go to first change |
| `]G` | Go to last change |

### Whitespace

| Key | Description |
|-----|-------------|
| `[Space` | Add newline above |
| `]Space` | Add newline below |

---

## Command Mode Commands

Access command mode with `:` in normal mode. Some useful commands:

- `:quit` / `:q` - Close current view
- `:quit!` / `:q!` - Close without saving
- `:open` / `:o` - Open file
- `:buffer-close` / `:bc` - Close current buffer
- `:write` / `:w` - Write buffer to file
- `:write-quit` / `:wq` - Write and close
- `:new` - Create new scratch buffer
- `:format` / `:fmt` - Format buffer with LSP
- `:indent-style` - Set indent style (tabs/spaces)
- `:line-ending` - Set line ending style
- `:earlier` / `:later` - Time travel through undo history
- `:write-all` / `:wa` - Write all modified buffers
- `:quit-all` / `:qa` - Close all views
- `:cquit` / `:cq` - Force quit with non-zero exit code
- `:theme` - Change theme
- `:clipboard-yank` - Yank main selection to clipboard
- `:clipboard-paste-after` - Paste clipboard after selections
- `:clipboard-paste-before` - Paste clipboard before selections
- `:show-clipboard-provider` - Show clipboard provider info
- `:change-current-directory` / `:cd` - Change current working directory
- `:show-directory` / `:pwd` - Show current working directory
- `:encoding` - Set encoding
- `:reload` - Reload file from disk
- `:update` - Write if buffer modified
- `:lsp-workspace-command` - Execute LSP workspace command
- `:log-open` - Open log file
- `:vsplit` / `:vs` - Open file in vertical split
- `:hsplit` / `:hs` / `:sp` - Open file in horizontal split
- `:tutor` - Open tutorial
- `:goto` / `:g` - Go to line number
- `:set-language` / `:lang` - Set language for syntax highlighting
- `:set-option` / `:set` - Set config option
- `:toggle` - Toggle config option
- `:get-option` / `:get` - Get config option value
- `:sort` - Sort selections
- `:rsort` - Sort selections in reverse
- `:reflow` - Hard-wrap text at text-width
- `:tree-sitter-scopes` - Show tree-sitter scopes
- `:debug-start` / `:dbg` - Start debug session
- `:debug-remote` / `:dbg-tcp` - Connect to debug adapter
- `:debug-eval` - Evaluate expression in debug context
- `:vsplit-new` / `:vnew` - Create scratch buffer in vertical split
- `:hsplit-new` / `:hnew` - Create scratch buffer in horizontal split
- `:insert-output` - Insert shell command output
- `:append-output` - Append shell command output
- `:pipe` - Pipe selections through shell command
- `:pipe-to` - Pipe selections to shell command (ignore output)
- `:run-shell-command` / `:sh` - Run shell command and show output

---

## Implementation Checklist

Track which commands have been implemented in the simulator:

### Movement (Normal Mode)

- [x] h, j, k, l - Basic movement (left, down, up, right)
- [x] w - Move to next word start
- [x] b - Move to previous word start
- [x] e - Move to next word end
- [ ] W, B, E - WORD movement (whitespace-separated)
- [ ] f, t, F, T - Character finding
- [x] G - Go to line end (or line number with count)
- [x] gg - Go to document start
- [ ] Alt-. - Repeat motion
- [x] 0 - Go to line start
- [x] $ - Go to line end
- [ ] Ctrl-b, Ctrl-f - Page up/down
- [ ] Ctrl-u, Ctrl-d - Half page up/down
- [ ] Ctrl-i, Ctrl-o - Jump forward/backward
- [ ] Ctrl-s - Save to jumplist

### Changes (Normal Mode)

- [x] r + char - Replace character with another char
- [ ] R - Replace selection with yanked text
- [ ] ~, `, Alt-` - Case switching
- [x] i - Enter insert mode before selection
- [x] a - Enter insert mode after selection (append)
- [x] I - Insert at line start
- [x] A - Append at line end
- [x] o - Open line below and enter insert mode
- [x] O - Open line above and enter insert mode
- [ ] . - Repeat last insert operation
- [x] u - Undo last change
- [x] U - Redo last undone change (Note: different from Helix's Alt-U)
- [ ] Alt-u, Alt-U - History navigation (earlier/later)
- [x] y - Yank (copy) selection
- [x] p - Paste after selection
- [x] P - Paste before selection
- [ ] " + reg - Select register for yank/paste
- [x] > - Indent selection
- [x] < - Unindent selection
- [ ] = - Format selection (LSP)
- [x] d - Delete selection (only 'dd' for line deletion implemented)
- [ ] Alt-d - Delete without yanking
- [x] c - Change selection (delete and enter insert mode)
- [ ] Alt-c - Change without yanking
- [ ] Ctrl-a, Ctrl-x - Increment/decrement number
- [ ] Q, q - Record/replay macro

### Selection & Line Operations

- [ ] s, S - Select/split by regex
- [ ] Alt-s - Split on newlines
- [ ] &, _ - Align/trim selections
- [ ] ;, Alt-; - Collapse/flip selections
- [ ] ,, Alt-, - Primary selection operations
- [ ] C, Alt-C - Copy selection to line above/below
- [ ] % - Select all (entire file)
- [x] x - Extend line below (limited implementation)
- [ ] X, Alt-x - Line bounds operations
- [x] J - Join lines (remove newlines)
- [ ] Alt-J - Join lines with space
- [ ] K, Alt-K - Keep/remove selections by regex
- [ ] Ctrl-c - Toggle comments

### Tree-sitter & Advanced Selection

- [ ] Alt-o, Alt-i - Expand/shrink selection
- [ ] Alt-p, Alt-n - Select prev/next sibling
- [ ] Alt-a - Select all siblings
- [ ] Alt-I - Select all children
- [ ] Alt-e, Alt-b - Move to parent node end/start

### Search (Normal Mode)

- [ ] / - Search forward
- [ ] ? - Search backward
- [ ] n, N - Next/previous match
- [ ] * - Search selection (word boundaries)
- [ ] Alt-* - Search selection (exact)

### Special Modes

- [ ] g - Goto mode (none implemented)
- [ ] m - Match mode (none implemented)
- [ ] z - View mode (none implemented)
- [ ] Ctrl-w - Window mode (none implemented)
- [ ] Space - Space mode (none implemented)
- [ ] v - Select mode (none implemented)

### Insert Mode Commands

- [x] Escape - Return to normal mode
- [x] Backspace - Delete previous character
- [x] Arrow keys - Navigation in insert mode
- [x] Text input - Insert characters
- [ ] Ctrl-x - Autocomplete
- [ ] Ctrl-w, Alt-Backspace - Delete word backward
- [ ] Alt-d - Delete word forward
- [ ] Ctrl-u - Kill to line start
- [ ] Ctrl-k - Kill to line end

---

## Implementation Summary

**Phase A Complete: Essential Commands (100%)**

**Implemented:** 30 commands covering all essential Helix operations

### By Category

**Movement** - 11 commands:

- Basic: h, j, k, l
- Word: w, b, e
- Line: 0, $
- Document: gg, G

**Editing** - 13 commands:

- Insert modes: i, a, I, A, o, O
- Delete/Change: dd, c, x
- Character: r + char
- History: u, U

**Indentation** - 2 commands:

- Indent/unindent: >, <

**Line operations** - 1 command:

- Join lines: J

**Clipboard** - 3 commands:

- Yank/paste: y, p, P

### Training Scenarios Coverage

- ✅ 20 scenarios covering all 30 implemented commands
- ✅ Multiple difficulty levels per command
- ✅ Hints and alternative solutions provided
- ✅ Organized in thematic directory structure

### Not Yet Implemented (Future Phases)

- Selection manipulation (s, S, %, etc.)
- Search (/, ?, n, N, *, etc.)
- Special modes (g, m, z, Ctrl-w, Space, v)
- Tree-sitter selections
- LSP integration commands
- Macros and registers
- Advanced clipboard operations

---

*End of Keybindings Reference*
