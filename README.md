# Helix Keybindings Trainer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

Interactive terminal trainer for mastering [Helix](https://helix-editor.com/) editor keybindings through hands-on practice.

## ✨ Features

- 🎮 **Interactive Learning** - Practice Helix commands in real scenarios
- 📊 **Performance Scoring** - Get rated on efficiency (Perfect/Excellent/Good/Fair/Poor)
- 💡 **Smart Hints** - Progressive hints when you need help
- 🎯 **Optimal Solutions** - Learn the most efficient way to solve each task
- 🎨 **Beautiful UI** - Large key history display, success animations, diff highlighting
- 📴 **100% Offline** - No internet required, all data stored locally
- 🔒 **Privacy-First** - No telemetry, tracking, or data collection

## 🚀 Installation

### From Source

```bash
git clone https://github.com/bug-ops/helix-trainer.git
cd helix-trainer
cargo build --release
./target/release/helix-trainer
```

### Requirements

- **Rust**: 1.90 or higher (Rust 2024 edition)
- **Terminal**: Unicode support recommended for best experience

## 🎯 Quick Start

```bash
cargo run --release
```

### Controls

| Screen | Action | Keys |
|--------|--------|------|
| **Menu** | Navigate | ↑/↓ or j/k |
| | Select | Enter |
| **Training** | Execute commands | h, j, k, l, dd, x, i, etc. |
| | Show hint | F1 |
| | Abandon scenario | Esc |
| **Results** | Retry scenario | r |
| | Return to menu | m |
| | Quit | q |

## 📚 Supported Commands

### Movement (11 commands)

- `h, j, k, l` - Character/line navigation
- `w, b, e` - Word movement
- `0, $` - Line start/end
- `gg, G` - Document start/end

### Editing (17 commands)

- `i, a` - Insert/append
- `I, A` - Insert/append at line bounds
- `o, O` - Open line below/above
- `r` - Replace character
- `c` - Change selection
- `x` - Delete character
- `dd` - Delete line
- `J` - Join lines
- `>, <` - Indent/dedent

### Clipboard (3 commands)

- `y` - Yank (copy)
- `p` - Paste after
- `P` - Paste before

### Undo/Redo (2 commands)

- `u` - Undo
- `U` - Redo

### Insert Mode

- Text input
- `Backspace` - Delete previous character
- Arrow keys - Navigate while inserting
- `Esc` - Return to normal mode

**Total**: 30+ commands implemented

## 📖 Scenarios

Currently includes **20 training scenarios**:

- Line deletion
- Word selection
- Insert/append modes
- Line operations (open, join, indent)
- Text replacement
- Clipboard operations (yank/paste)

Training scenarios are defined in TOML format. See [scenarios/](scenarios/) directory for examples organized by category.

### Example Scenario

```toml
[[scenarios]]
id = "delete_line_001"
name = "Delete current line"
description = "Delete the line where cursor is located"

[scenarios.setup]
file_content = "line 1\nline 2\nline 3"
cursor_position = [1, 0]

[scenarios.target]
file_content = "line 1\nline 3"
cursor_position = [1, 0]

[scenarios.solution]
commands = ["dd"]
description = "Press 'dd' to delete line"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
```

## 🛠️ Development

### Running Tests

```bash
# Fast test runner (recommended)
cargo nextest run --lib

# Standard test runner
cargo test --lib
```

### Code Quality Checks

```bash
# Format code (requires nightly)
cargo +nightly fmt

# Lint code (zero warnings policy)
cargo clippy --all-targets --all-features -- -D warnings

# Security audit
cargo deny check

# Build release
cargo build --release
```

### Development Workflow

```bash
# Create feature branch
git checkout -b feature/your-feature

# Make changes, run full check pipeline
cargo +nightly fmt
cargo nextest run
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release

# Commit and push
git commit -m "feat: description"
git push origin feature/your-feature
gh pr create
```

## 🏗️ Architecture

### Tech Stack

| Component | Library | Version |
|-----------|---------|---------|
| **TUI Framework** | [ratatui](https://ratatui.rs/) | 0.29 |
| **Terminal I/O** | [crossterm](https://github.com/crossterm-rs/crossterm) | 0.29 |
| **Editor Core** | [helix-core](https://github.com/helix-editor/helix) | 25.07.1 |
| **Large Text** | [tui-big-text](https://crates.io/crates/tui-big-text) | 0.7 |
| **Config** | [serde](https://serde.rs/) + [toml](https://toml.io/) | 1.0 + 0.9 |
| **Errors** | [thiserror](https://github.com/dtolnay/thiserror) + [anyhow](https://github.com/dtolnay/anyhow) | 2.0 + 1.0 |

### Project Structure

```plain
src/
├── main.rs              # Entry point + key mapping
├── lib.rs               # Library root
├── ui/                  # Terminal UI (Elm Architecture)
│   ├── state.rs         # App state + message handling
│   └── render.rs        # Pure rendering functions
├── game/                # Game logic
│   ├── session.rs       # Scenario execution
│   ├── scorer.rs        # Performance rating
│   └── editor_state.rs  # Editor state wrapper
├── helix/               # Helix integration
│   └── simulator.rs     # HelixSimulator using helix-core
├── config/              # Configuration
│   └── scenarios.rs     # TOML scenario parser
└── security/            # Security & validation
    ├── mod.rs           # Security primitives
    └── arithmetic.rs    # Safe arithmetic operations

scenarios/
├── basic/               # Basic editing scenarios
├── movement/            # Movement command scenarios
├── editing/             # Advanced editing scenarios
└── clipboard/           # Clipboard & undo/redo scenarios
                         # Total: 20 training scenarios

.github/
├── workflows/           # CI/CD pipelines
└── ISSUE_TEMPLATE/      # Issue templates
```

### Implementation Status

- ✅ **Stage 1**: Foundation (TUI, modules, tests) - 100%
- ✅ **Stage 2**: Helix Integration (30+ commands, simulator) - 100%
- ✅ **Phase A**: Essential commands & scenarios - 100%
- 🔄 **Phase B**: Progress tracking & statistics - 0%
- 📋 **Phase C**: Advanced features - 0%

## 📊 Metrics

- **Lines of Code**: ~6,842 (Rust)
- **Test Count**: 153 (all passing ✅)
- **Test Coverage**: 100% for core modules
- **Commands**: 30+ implemented
- **Scenarios**: 20 training scenarios
- **CI Platforms**: Linux, macOS, Windows

## 🤝 Contributing

Contributions welcome! We follow a strict PR-based workflow.

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### Quick Guidelines

1. **Fork** the repository
2. Create a **feature branch** (`git checkout -b feature/amazing-feature`)
3. Make changes and add **tests**
4. Run **full check pipeline** (fmt, nextest, clippy, build)
5. **Commit** with conventional commits (`feat:`, `fix:`, `docs:`, etc.)
6. **Push** and create a **Pull Request**
7. Wait for **CI checks** to pass
8. Get **review** and merge

### CI Checks

All PRs must pass:

- ✅ Tests on Linux, macOS, Windows (cargo-nextest)
- ✅ Formatting (rustfmt nightly)
- ✅ Lints (clippy with -D warnings)
- ✅ Security audit (cargo-deny)
- ✅ Build verification (release mode)

## 📝 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [Helix Editor](https://helix-editor.com/) - For the amazing modal editor
- [Ratatui](https://ratatui.rs/) - For the excellent TUI framework
- [tui-big-text](https://crates.io/crates/tui-big-text) - For large text rendering
- Inspired by vim-tutor and other interactive learning tools

## 🗺️ Roadmap

### Phase A (Complete - 100%)

- [x] Core movement commands (11)
- [x] Essential editing commands (17)
- [x] Clipboard operations (3)
- [x] Undo/redo support (2)
- [x] Insert mode enhancements
- [x] Beautiful UI with large key display
- [x] 20 training scenarios across 4 categories
- [ ] Repeat (.) command (moved to Phase B)

### Phase B (Planned)

- [ ] Progress tracking
- [ ] Statistics and performance history
- [ ] User profiles
- [ ] More intermediate scenarios

### Phase C (Future)

- [ ] Difficulty levels
- [ ] Custom scenario editor
- [ ] Export/import progress
- [ ] Advanced commands (macros, registers)
- [ ] Tutorial mode for beginners
- [ ] Achievement system

## 📚 Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines
- [SECURITY.md](SECURITY.md) - Security policy

## 🐛 Issues & Feedback

Found a bug or have a suggestion? Please [open an issue](https://github.com/bug-ops/helix-trainer/issues)!

**Issue Templates:**

- Bug Report
- Feature Request
- Scenario Request
