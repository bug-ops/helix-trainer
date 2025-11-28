# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Windows ARM64 support (aarch64-pc-windows-msvc)
- Linux ARM64 support (aarch64-unknown-linux-gnu, aarch64-unknown-linux-musl)
- Cross-compilation support for ARM targets using `cross`

### Changed
- Release workflow now builds for 9 platforms (was 5)

## [0.1.0] - 2025-01-XX

### Added

**Phase 1: Smart Learning & Gamification System**

- FSRS-based spaced repetition learning system for scientifically-optimized practice
  - Card state management (New, Learning, Review, Relearning)
  - Performance tracking per scenario (Again, Hard, Good, Easy ratings)
  - Intelligent review scheduling (20-30% fewer reviews than traditional methods)
  - Review analytics and statistics
  - Persistent storage in `~/.config/helix-trainer/learning.json`

- Daily quests gamification system with Duolingo-style mechanics
  - Three quest types: Practice (3 scenarios), Learning (2 new), Review (5 cards)
  - Daily quest rotation at midnight local time
  - Streak tracking with loss aversion mechanics
  - Quest completion rewards (XP bonuses)
  - Persistent storage in `~/.config/helix-trainer/quests.json`

- XP and leveling progression system
  - Experience points for scenario completion
  - Performance-based XP multipliers (1.0x to 2.0x)
  - Quest completion bonuses (50-150 XP)
  - Level progression with exponential scaling
  - Profile screen with detailed statistics

- User profile and statistics tracking
  - Total scenarios completed
  - Average performance score
  - Current streak and longest streak
  - Total XP and current level
  - Review history and analytics
  - Persistent storage in `~/.config/helix-trainer/profile.json`

- Enhanced UI screens
  - Profile screen with level, XP, and statistics
  - Statistics screen with review calendar and performance graphs
  - Quest tracker in main menu
  - XP breakdown in results screen
  - Success animations with XP rewards

**Phase A: Foundation (Complete)**

- Core TUI framework with ratatui 0.29
  - Main menu with scenario selection
  - Task screen with editor simulation
  - Results screen with performance rating
  - Keyboard-driven navigation (Vim-style)

- Helix simulator using official helix-core library (v25.07.1)
  - 30+ commands: h,j,k,l,w,b,e,0,$,x,dd,i,a,I,A,o,O,r,c,y,p,P,u,U,gg,G,J,>,<
  - Repeat command (.) for efficient editing workflows
  - Insert mode with text input, Backspace, arrow keys
  - Multi-key command buffer (dd, gg)
  - Yank/paste clipboard support
  - Automatic completion detection
  - Cursor and selection visualization

- Beautiful UI components
  - Large key history display (tui-big-text, 8-line tall characters)
  - Success popup with 1.5s delay
  - Diff highlighting (red/green) in results
  - Action count indicators
  - Performance rating with emoji (Perfect/Excellent/Good/Fair/Poor)
  - Hint system (F1 key)

- TOML scenario system with security validation
  - 20 training scenarios (basic to intermediate)
  - Configurable setup, target, solution, hints
  - Performance scoring (optimal count, max points, tolerance)
  - Input validation and sanitization
  - Path traversal prevention
  - Content size limits

- Testing and quality infrastructure
  - 164 passing tests (unit, integration, property-based)
  - Zero clippy warnings policy
  - cargo-nextest for fast test execution
  - Proptest for property-based testing
  - 100% test coverage for core modules

- Cross-platform CI/CD pipeline
  - GitHub Actions workflows for Linux, macOS, Windows
  - Automated testing with cargo-nextest
  - Rustfmt (nightly) formatting checks
  - Clippy lints with -D warnings
  - Security audit with cargo-deny
  - Rust 2024 edition support
  - MSRV: 1.85

- Internationalization (i18n) support
  - rust-i18n integration for multi-language support
  - English locale (default)
  - Infrastructure for future language additions

- Build optimization
  - sccache integration for fast incremental builds
  - Cargo cache optimization
  - 5x faster rebuilds (10s vs 54s)

### Changed

- Upgraded to Rust 2024 edition (latest stable)
- Migrated to helix-core 25.07.1 (from 24.07)
- Optimized dependencies (removed tokio, unnecessary features)
- Refactored codebase into 31 modular files
- Enhanced error messages with context and suggestions
- Improved scoring algorithm for better feedback

### Fixed

- Replace command cursor initialization bugs
- Multi-key command handling edge cases
- Insert mode arrow key navigation
- Yank/paste clipboard state management
- Undo/redo transaction handling

### Security

- All input validation with bounds checking
- Path traversal prevention in scenario loading
- Content size limits (1MB scenarios, 100KB files)
- Safe arithmetic with checked operations
- No unsafe blocks in codebase
- Dependency security audits with cargo-deny

## [0.0.1] - 2024-10-01

### Added

- Initial project structure
- Basic TUI skeleton with ratatui
- Prototype scenario loader
- Proof of concept demo

---

[Unreleased]: https://github.com/bug-ops/helix-trainer/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bug-ops/helix-trainer/releases/tag/v0.1.0
[0.0.1]: https://github.com/bug-ops/helix-trainer/releases/tag/v0.0.1
