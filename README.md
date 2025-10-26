# Helix Keybindings Trainer

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

Interactive terminal trainer for mastering [Helix](https://helix-editor.com/) editor keybindings through hands-on practice.

## Features

- 🎮 **Interactive Learning** - Practice Helix commands in real scenarios
- 📊 **Performance Scoring** - Get rated on efficiency (Perfect/Excellent/Good/Fair/Poor)
- 💡 **Smart Hints** - Progressive hints when you need help
- 🎯 **Optimal Solutions** - Learn the most efficient way to solve each task
- 📴 **100% Offline** - No internet required, all data stored locally
- 🔒 **Privacy-First** - No telemetry, tracking, or data collection

## Installation

### From Source

```bash
git clone https://github.com/bug-ops/helix-trainer.git
cd helix-trainer
cargo build --release
./target/release/helix-trainer
```

### Requirements

- Rust 1.70 or higher
- Terminal with unicode support

## Quick Start

```bash
cargo run --release
```

**Controls:**

- **Menu**: ↑/↓ or j/k to navigate, Enter to select scenario
- **Training**: Use Helix commands (h,j,k,l,dd,x,etc.), F1 for hints, Esc to quit
- **Results**: r to retry, m for menu, q to quit

## Supported Commands

Currently supports 14 core Helix commands:

**Movement**: h, j, k, l, w, b, e, 0, $, gg, G
**Editing**: x (delete char), dd (delete line), i (insert mode)
**Undo**: u (undo), Ctrl-r (redo)

## Scenarios

Training scenarios are defined in TOML format:

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

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
```

See `scenarios/basic.toml` for examples.

## Development

```bash
# Run tests
cargo test --lib

# Check code quality
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Run with debug logging
RUST_LOG=debug cargo run
```

## Architecture

Built with:

- **TUI**: [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- **Editor**: [helix-core](https://github.com/helix-editor/helix) - Official Helix primitives
- **Config**: [serde](https://serde.rs/) + [toml](https://github.com/toml-lang/toml) - Scenario format

**Implementation Status:**

- ✅ Stage 1: Foundation (TUI, game engine, scoring)
- ✅ Stage 2: Helix Integration (simulator, commands, visualization)
- 🔄 Stage 3: Polish (statistics, export, custom scenarios)

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Ensure `cargo test` and `cargo clippy` pass
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Helix Editor](https://helix-editor.com/) - For the amazing editor
- [Ratatui](https://ratatui.rs/) - For the TUI framework
- Inspired by vim-tutor and other interactive learning tools

## Roadmap

- [ ] More scenarios (intermediate, advanced)
- [ ] Statistics and progress tracking
- [ ] Custom scenario editor
- [ ] Export/import progress
- [ ] Additional Helix commands
- [ ] Tutorial mode for beginners
