# Scenarios and quests authoring guide

This guide explains how to create training scenarios and daily quests for Helix Trainer. Scenarios teach specific Helix commands through interactive exercises, while quests provide daily goals to motivate practice.

## Table of contents

- [Scenario format](#scenario-format)
  - [Required fields](#required-fields)
  - [Optional fields](#optional-fields)
  - [Metadata fields](#metadata-fields)
- [Scenario examples](#scenario-examples)
  - [Movement scenario](#movement-scenario)
  - [Editing scenario](#editing-scenario)
  - [Selection scenario](#selection-scenario)
  - [Text object scenario](#text-object-scenario)
  - [Surround scenario](#surround-scenario)
  - [Search scenario](#search-scenario)
- [Quest format](#quest-format)
  - [Quest types](#quest-types)
  - [Quest conditions](#quest-conditions)
- [Quest examples](#quest-examples)
  - [Command practice quest](#command-practice-quest)
  - [Scenario completion quest](#scenario-completion-quest)
  - [Speed run quest](#speed-run-quest)
  - [Time invested quest](#time-invested-quest)
  - [Exploration quest](#exploration-quest)
- [Validation rules and limits](#validation-rules-and-limits)
- [Best practices](#best-practices)
- [Testing scenarios](#testing-scenarios)

---

## Scenario format

Scenarios are defined in TOML files under `scenarios/<locale>/`. Each file contains an array of `[[scenarios]]` entries.

### Required fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier (alphanumeric + underscore, max 64 chars) |
| `name` | string | Short display name |
| `description` | string | What the user needs to accomplish |
| `setup` | table | Initial editor state |
| `target` | table | Expected final state |
| `solution` | table | Optimal command sequence |
| `scoring` | table | Scoring configuration |

#### Setup table

```toml
[scenarios.setup]
file_content = "hello world"      # Initial buffer content
cursor_position = [0, 0]          # [row, col] - 0-indexed
selection = [0, 0, 0, 5]          # Optional: [start_row, start_col, end_row, end_col]
language = "rs"                   # Optional: file-extension-style token for syntax highlighting
                                   # (e.g. "rs", "md", "py"); defaults to "rs" when omitted;
                                   # unrecognized values fall back to plain text, never crash
```

#### Target table

```toml
[scenarios.target]
file_content = "hello world"      # Expected final content
cursor_position = [0, 5]          # Expected cursor position
selection = [0, 0, 0, 5]          # Optional: expected selection
```

#### Solution table

```toml
[scenarios.solution]
commands = ["w"]                  # Optimal command sequence
description = "Press 'w' to move to next word"
```

#### Scoring table

```toml
[scenarios.scoring]
optimal_count = 1                 # Number of commands in optimal solution
max_points = 100                  # Maximum score achievable
tolerance = 0                     # Extra commands allowed before penalty
```

### Optional fields

| Field | Type | Description |
|-------|------|-------------|
| `hints` | array | Help text shown to user (max 10) |
| `alternatives` | array | Alternative valid solutions (max 20) |
| `metadata` | table | Category, difficulty, tags |

#### Alternative solutions

```toml
[[scenarios.alternatives]]
commands = ["l", "l", "l", "l", "l"]
points_multiplier = 0.5
description = "Use 'l' five times (less efficient)"
```

### Metadata fields

```toml
[scenarios.metadata]
category = "movement"              # movement, editing, clipboard, search, selection, textobjects, advanced, registers
difficulty = "beginner"            # beginner, intermediate, advanced
tags = ["word", "motion"]          # Flexible filtering tags
commands_taught = ["w"]            # Commands this scenario teaches
prerequisites = ["move_right_001"] # Scenarios to complete first
estimated_time_seconds = 10        # Expected completion time
locale = "en"                      # Language code
```

---

## Scenario examples

### Movement scenario

Basic cursor movement using `w` (word forward):

```toml
[[scenarios]]
id = "word_forward_001"
name = "Move to next word"
description = "Use 'w' to jump to the start of the next word"

hints = [
    "'w' moves forward to the start of the next word",
    "This is faster than pressing 'l' multiple times",
]

[scenarios.setup]
file_content = "hello world"
cursor_position = [0, 0]

[scenarios.target]
file_content = "hello world"
cursor_position = [0, 6]

[scenarios.solution]
commands = ["w"]
description = "Press 'w' to move to 'world'"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "movement"
difficulty = "beginner"
tags = ["word", "motion", "forward"]
commands_taught = ["w"]
estimated_time_seconds = 5
```

### Editing scenario

Join lines using `J`:

```toml
[[scenarios]]
id = "join_lines_001"
name = "Join two lines"
description = "Use 'J' to join the current line with the next line"

hints = [
    "'J' joins lines by removing the newline",
    "A space is added between the joined content",
]

[scenarios.setup]
file_content = "hello\nworld"
cursor_position = [0, 0]

[scenarios.target]
file_content = "hello world"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["J"]
description = "Press 'J' to join lines (adds space)"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "editing"
difficulty = "intermediate"
tags = ["join", "lines", "editing"]
commands_taught = ["J"]
estimated_time_seconds = 10
```

### Selection scenario

Select current line using `x`:

```toml
[[scenarios]]
id = "select_line_001"
name = "Select current line"
description = "Use 'x' to select the entire current line"

hints = [
    "'x' selects the current line including the newline",
    "Use 'xd' to delete a line (select + delete)",
]

[scenarios.setup]
file_content = "line 1\nline 2\nline 3"
cursor_position = [1, 0]

[scenarios.target]
file_content = "line 1\nline 2\nline 3"
cursor_position = [1, 0]
selection = [1, 0, 2, 0]

[scenarios.solution]
commands = ["x"]
description = "Press 'x' to select line 2"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "selection"
difficulty = "beginner"
tags = ["select", "line"]
commands_taught = ["x"]
estimated_time_seconds = 5
```

### Text object scenario

Select inside word using `miw`:

```toml
[[scenarios]]
id = "text_object_miw_001"
name = "Select inside word"
description = "Use 'miw' to select the word under cursor"

hints = [
    "'mi' starts match-inside, followed by the object type",
    "'w' is the word object",
    "'miw' selects just the word characters, not surrounding spaces",
]

[scenarios.setup]
file_content = "hello beautiful world"
cursor_position = [0, 6]

[scenarios.target]
file_content = "hello beautiful world"
cursor_position = [0, 6]
selection = [0, 6, 0, 15]

[scenarios.solution]
commands = ["miw"]
description = "Press 'miw' to select 'beautiful'"

[scenarios.scoring]
optimal_count = 3
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "selection"
difficulty = "intermediate"
tags = ["text-object", "word", "selection"]
commands_taught = ["miw"]
estimated_time_seconds = 10
```

### Surround scenario

Delete surrounding parentheses using `md(`:

```toml
[[scenarios]]
id = "surround_delete_parens_001"
name = "Delete surrounding parentheses"
description = "Use 'md(' to remove the parentheses around text"

hints = [
    "'md' enters surround delete mode, then type the character to delete",
    "Cursor must be inside the pair to delete",
]

[scenarios.setup]
file_content = "(hello) world"
cursor_position = [0, 3]

[scenarios.target]
file_content = "hello world"
cursor_position = [0, 2]

[scenarios.solution]
commands = ["md("]
description = "Press 'md(' to delete the parentheses"

[scenarios.scoring]
optimal_count = 3
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "editing"
difficulty = "intermediate"
tags = ["surround", "delete", "parentheses"]
commands_taught = ["md"]
estimated_time_seconds = 10
```

### Search scenario

Search for word under cursor using `*` and `n`:

```toml
[[scenarios]]
id = "search_word_001"
name = "Search word forward"
description = "Use '*' to search for the word under cursor and 'n' to jump to next match"

hints = [
    "'*' selects word under cursor and sets search pattern",
    "'n' jumps to next match",
]

[scenarios.setup]
file_content = "hello world hello there"
cursor_position = [0, 0]

[scenarios.target]
file_content = "hello world hello there"
cursor_position = [0, 12]
selection = [0, 12, 0, 17]

[scenarios.solution]
commands = ["*", "n"]
description = "Press '*' to select 'hello' and set search pattern, then 'n' to jump to next match"

[scenarios.scoring]
optimal_count = 2
max_points = 100
tolerance = 0

[scenarios.metadata]
category = "search"
difficulty = "beginner"
tags = ["search", "word", "forward"]
commands_taught = ["*", "n"]
estimated_time_seconds = 10
```

---

## Quest format

Quests are defined in `quests/<locale>/daily.toml`. Each file requires a `[metadata]` section and an array of `[[quests]]` entries.

```toml
[metadata]
version = "1.0"
locale = "en"

[[quests]]
id = "quest_id"
name = "Quest Name"
description = "What the user needs to do"
type = "command_practice"
difficulty = "easy"

[quests.params]
# Type-specific parameters
```

### Quest types

| Type | Description | Parameters |
|------|-------------|------------|
| `command_practice` | Use a command N times | `command`, `target` |
| `scenario_completion` | Complete N scenarios | `target` |
| `speed_run` | Complete scenario within time limit | `scenario_id`, `time_limit_seconds` |
| `time_invested` | Practice for N minutes | `target_minutes` |
| `exploration` | Use N different commands | `target_commands` |

### Quest conditions

Optional unlock conditions:

```toml
[quests.conditions]
min_level = 5                     # Minimum player level
max_level = 10                    # Maximum player level (for easy quests)
requires_commands = ["w", "b"]    # Commands player must have used
requires_scenarios = ["word_001"] # Scenarios player must have completed
```

Optional XP override:

```toml
[quests.xp]
base_reward = 50                  # Custom XP reward (max 1000)
```

---

## Quest examples

### Command practice quest

```toml
[[quests]]
id = "cmd_w_easy"
name = "Word Navigation"
description = "Move forward 5 words using 'w'"
type = "command_practice"
difficulty = "easy"

[quests.params]
command = "w"
target = 5
```

### Scenario completion quest

```toml
[[quests]]
id = "scenario_3_medium"
name = "Scenario Trio"
description = "Complete 3 scenarios"
type = "scenario_completion"
difficulty = "medium"

[quests.params]
target = 3
```

### Speed run quest

```toml
[[quests]]
id = "speed_delete_hard"
name = "Speed Run: Delete Line"
description = "Complete 'delete_line_001' in under 5 seconds"
type = "speed_run"
difficulty = "hard"

[quests.params]
scenario_id = "delete_line_001"
time_limit_seconds = 5

[quests.conditions]
requires_scenarios = ["delete_line_001"]
```

### Time invested quest

```toml
[[quests]]
id = "time_10_medium"
name = "Extended Practice"
description = "Practice for 10 minutes"
type = "time_invested"
difficulty = "medium"

[quests.params]
target_minutes = 10
```

### Exploration quest

```toml
[[quests]]
id = "explore_10_hard"
name = "Command Explorer"
description = "Use 10 different commands today"
type = "exploration"
difficulty = "hard"

[quests.params]
target_commands = 10
```

---

## Validation rules and limits

### Scenario limits

| Limit | Value | Description |
|-------|-------|-------------|
| `MAX_SCENARIOS_PER_FILE` | 100 | Maximum scenarios in one TOML file |
| `MAX_SCENARIO_FILE_SIZE` | 10 MB | Maximum file size |
| `MAX_FILE_CONTENT_LENGTH` | 100,000 | Maximum characters in `file_content` |
| `MAX_COMMAND_SEQUENCE_LENGTH` | 100 | Maximum commands in solution |
| `MAX_HINTS` | 10 | Maximum hints per scenario |
| `MAX_ALTERNATIVES` | 20 | Maximum alternative solutions |
| ID max length | 64 | Characters in `id` field |
| ID format | alphanumeric + `_` | Valid characters for `id` |

### Quest limits

| Limit | Value | Description |
|-------|-------|-------------|
| `MAX_QUEST_TEMPLATES_PER_FILE` | 100 | Maximum quests in one file |
| `MAX_QUEST_TARGET` | 100 | Maximum target value for practice quests |
| `MAX_SPEED_RUN_TIME_SECONDS` | 3600 | Maximum speed run time (1 hour) |
| `MAX_CUSTOM_XP_REWARD` | 1000 | Maximum custom XP reward |
| `MAX_QUEST_NAME_LENGTH` | 100 | Maximum name length |
| `MAX_QUEST_DESCRIPTION_LENGTH` | 500 | Maximum description length |
| `MAX_REQUIRED_CONDITIONS` | 20 | Maximum conditions per quest |

### Position limits

| Limit | Value | Description |
|-------|-------|-------------|
| Maximum row/column | 10,000 | For cursor and selection positions |

---

## Best practices

### Scenario design

1. **Start simple** - Begin with single-command scenarios, then combine commands
2. **Use realistic content** - Use code snippets or prose that reflects real editing
3. **Validate cursor positions** - Ensure positions are within content bounds
4. **Test your solutions** - Verify the command sequence achieves the target
5. **Write clear hints** - Explain the command without giving away the answer

### Naming conventions

- **IDs**: Use pattern `<category>_<command>_<number>` (e.g., `word_forward_001`)
- **Names**: Short, action-oriented (e.g., "Move to next word")
- **Descriptions**: Full sentence explaining the goal

### Content guidelines

- Keep `file_content` small (under 500 characters for basic scenarios)
- Use `\n` for newlines in content strings
- For multi-key commands like `miw`, set `optimal_count` to the keystroke count (3)
- Set appropriate difficulty based on command complexity

### Quest balance

| Difficulty | Target range | Typical commands |
|------------|--------------|------------------|
| Easy | 1-10 | Basic movement (h, j, k, l, w, b) |
| Medium | 3-15 | Editing, clipboard (d, y, p, c) |
| Hard | 5-30 | Advanced (text objects, surround) |

---

## Testing scenarios

### Run validation tests

```bash
# Validate all scenarios load and execute correctly
cargo nextest run test_all_scenarios_execute

# Run all scenario validation tests
cargo nextest run scenario_validation
```

### Validate quest loading

```bash
# Validate all quests load correctly
cargo nextest run quest

# Specific quest validation
cargo nextest run test_validate_all_quest_templates
```

### Manual testing

1. Add your scenario to the appropriate TOML file
2. Run the trainer: `cargo run --release`
3. Navigate to your scenario and test it
4. Verify:
   - Initial state matches `setup`
   - Solution commands achieve `target`
   - Scoring works correctly
   - Hints are helpful

> [!TIP]
> Use `cargo nextest run test_all_scenarios_execute --no-capture` to see detailed output for failed scenarios.

### Common issues

| Issue | Cause | Solution |
|-------|-------|----------|
| "Cursor position out of bounds" | Position exceeds content length | Check row/col against content |
| "Selection validation failed" | Selection end before start | Ensure `[start_row, start_col, end_row, end_col]` order |
| "Unknown command" | Command not registered | Check command exists in Helix keymap |
| "Solution did not complete" | Commands don't reach target | Test commands manually in Helix |

---

## File structure

```
scenarios/
└── en/
    ├── basic/
    │   ├── delete.toml
    │   ├── insert.toml
    │   └── replace.toml
    ├── movement/
    │   ├── basic-movement.toml
    │   ├── word.toml
    │   └── find-till.toml
    ├── editing/
    │   ├── surround.toml
    │   └── advanced-editing.toml
    ├── selection/
    │   ├── text-objects.toml
    │   └── advanced-selection.toml
    └── search/
        └── basic-search.toml

quests/
└── en/
    └── daily.toml
```

---

## Additional resources

- [Helix keybindings reference](HELIX_KEYBINDINGS.md)
- [Official Helix keymap](https://docs.helix-editor.com/keymap.html)
