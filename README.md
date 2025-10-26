# Helix Keybindings Trainer

An interactive terminal user interface (TUI) application for learning Helix editor keybindings through practice and immediate feedback.

## Vision

- **Goal**: Help users master Helix through hands-on practice and instant feedback
- **Success Metric**: Minimize the number of commands needed to complete tasks
- **Key Feature**: Shows optimal solutions if the user completes a task suboptimally

## Quick Start

### Prerequisites

- Rust 1.70 or later
- Helix editor installed and available in PATH
- Terminal with 256 color support

### Installation

```bash
# Clone the repository
git clone https://github.com/example/helix-trainer.git
cd helix-trainer

# Build the project
cargo build --release

# Run the trainer
cargo run --release
```

## Architecture

The application is organized into four main modules:

### 1. UI Module (`src/ui/`)
Terminal user interface components built with ratatui:
- Main menu navigation
- Task presentation screen
- Results and feedback display
- Statistics dashboard

### 2. Game Module (`src/game/`)
Core game engine and session management:
- Scenario loading and management
- User action tracking
- Score calculation
- Editor state simulation

### 3. Helix Module (`src/helix/`)
Integration with Helix editor:
- PTY (pseudo-terminal) control
- Command interception
- Buffer state synchronization

### 4. Config Module (`src/config/`)
Configuration and scenario management:
- TOML scenario file parsing
- Application settings

## Development

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run with logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check code quality
cargo clippy

# Format code
cargo fmt
```

### Project Structure

```
helix-trainer/
├── Cargo.toml              # Project manifest
├── src/
│   ├── main.rs            # Entry point
│   ├── lib.rs             # Library root
│   ├── ui/                # TUI components
│   ├── game/              # Game engine
│   ├── helix/             # Helix integration
│   └── config/            # Configuration
├── scenarios/             # TOML scenario files
├── examples/              # Example programs
└── README.md
```

## Scenarios

Scenarios are defined in TOML format. Each scenario contains:
- Task description and difficulty level
- Initial editor state and target state
- Optimal solution(s)
- Alternative solutions
- Hint system
- Scoring configuration

See `scenarios/basic.toml` for example scenarios.

## Testing

The project follows comprehensive testing practices:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test '*'
```

## Dependencies

- **ratatui**: TUI framework
- **crossterm**: Terminal I/O and event handling
- **tokio**: Async runtime
- **serde**: Serialization/deserialization
- **toml**: TOML file parsing
- **tracing**: Logging and diagnostics
- **anyhow**: Error handling
- **thiserror**: Custom error types

## Contributing

Contributions are welcome! Please ensure:
- Code compiles without warnings
- All tests pass
- Code follows Rust best practices
- Documentation is up to date

## License

This project is licensed under the MIT License.

## Development Status

**Current Phase**: Foundation (MVP)

- [x] Project initialization
- [x] Basic module structure
- [ ] Scenario loading system
- [ ] Editor state simulation
- [ ] Scoring system
- [ ] Main game loop
- [ ] TUI components
- [ ] Helix integration
