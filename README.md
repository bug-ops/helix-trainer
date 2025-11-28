# Helix Trainer

[![CI Status](https://img.shields.io/github/actions/workflow/status/bug-ops/helix-trainer/ci.yml?branch=main)](https://github.com/bug-ops/helix-trainer/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Release](https://img.shields.io/github/v/release/bug-ops/helix-trainer)](https://github.com/bug-ops/helix-trainer/releases/latest)

**Master Helix editor keybindings through scientifically-optimized spaced repetition and gamified training.**

Stop learning commands in isolation. Train real development workflows with FSRS-powered spaced repetition (20-30% faster mastery), daily quests, XP progression, and anti-farming mechanics that ensure genuine skill development.

<!-- Demo GIF/screenshot will go here -->
> 🎬 **Demo GIF coming soon** — Watch training in action with mastery tracking, daily quests, and real-time feedback

---

## ✨ Features

- 🧠 **FSRS Spaced Repetition** — 20-30% fewer reviews than traditional methods (research-proven)
- 🎯 **Daily Quest System** — Duolingo-style challenges with streak tracking
- 📊 **Scenario Mastery** — Three-tier progression (Learning → Proficient → Mastered) with graduated XP scaling
- 🛡️ **Anti-Farming Protection** — Session penalties prevent XP exploitation
- ⚡ **Real Helix Accuracy** — Uses official `helix-core` library (v25.07.1)
- 🎮 **31 Commands** — Movement, editing, clipboard, undo/redo, repeat
- 🔒 **100% Offline** — No cloud, no tracking, all data stays local (`~/.config/helix-trainer/`)
- 📚 **20 Training Scenarios** — From basics to intermediate workflows

---

## 📦 Installation

### Pre-built Binaries (Recommended)

Download for your platform from [**Releases**](https://github.com/bug-ops/helix-trainer/releases/latest):

<details>
<summary><b>Linux (x86_64)</b></summary>

```bash
# GNU libc (most distributions)
wget https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-x86_64-unknown-linux-gnu.tar.gz
tar -xzf helix-trainer-v0.1.1-x86_64-unknown-linux-gnu.tar.gz
cd helix-trainer-v0.1.1-x86_64-unknown-linux-gnu
./helix-trainer

# musl (Alpine Linux, static binary)
wget https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-x86_64-unknown-linux-musl.tar.gz
tar -xzf helix-trainer-v0.1.1-x86_64-unknown-linux-musl.tar.gz
cd helix-trainer-v0.1.1-x86_64-unknown-linux-musl
./helix-trainer
```
</details>

<details>
<summary><b>Linux (ARM64/aarch64)</b></summary>

```bash
# GNU libc
wget https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-aarch64-unknown-linux-gnu.tar.gz
tar -xzf helix-trainer-v0.1.1-aarch64-unknown-linux-gnu.tar.gz
cd helix-trainer-v0.1.1-aarch64-unknown-linux-gnu
./helix-trainer

# musl
wget https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-aarch64-unknown-linux-musl.tar.gz
tar -xzf helix-trainer-v0.1.1-aarch64-unknown-linux-musl.tar.gz
cd helix-trainer-v0.1.1-aarch64-unknown-linux-musl
./helix-trainer
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
# Apple Silicon (M1/M2/M3/M4)
curl -LO https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-aarch64-apple-darwin.tar.gz
tar -xzf helix-trainer-v0.1.1-aarch64-apple-darwin.tar.gz
cd helix-trainer-v0.1.1-aarch64-apple-darwin
./helix-trainer

# Intel
curl -LO https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-x86_64-apple-darwin.tar.gz
tar -xzf helix-trainer-v0.1.1-x86_64-apple-darwin.tar.gz
cd helix-trainer-v0.1.1-x86_64-apple-darwin
./helix-trainer
```
</details>

<details>
<summary><b>Windows</b></summary>

Download from [Releases page](https://github.com/bug-ops/helix-trainer/releases/latest):
- **x86_64**: `helix-trainer-v0.1.1-x86_64-pc-windows-msvc.zip`
- **ARM64**: `helix-trainer-v0.1.1-aarch64-pc-windows-msvc.zip`

Extract and run `helix-trainer.exe`
</details>

**Verify checksums** (optional but recommended):
```bash
sha256sum -c helix-trainer-*.sha256
```

### Build from Source

```bash
git clone https://github.com/bug-ops/helix-trainer.git
cd helix-trainer
cargo build --release
./target/release/helix-trainer
```

**Requirements**: Rust 1.85+ (2024 edition), terminal with Unicode support

---

## 🚀 Quick Start

```bash
helix-trainer
```

The interactive TUI will guide you through:

1. **Daily Quests** — Fresh challenges every day (Practice, Learning, Review)
2. **Training Scenarios** — 20 scenarios with instant feedback
3. **Progress Tracking** — XP, levels, streaks, mastery progression
4. **Performance Analytics** — Review calendar, mastery stats

### Example Training Session

```
Main Menu
├─ Daily Quests (3 active)
│  ├─ ✅ Practice: Complete 3 scenarios
│  ├─ ⏳ Learning: Try 2 new scenarios
│  └─ 🔄 Review: 5 cards due
├─ Training Scenarios (20 available)
│  ├─ Basic Movement (Mastered - 20% XP)
│  ├─ Word Navigation (Proficient - 50% XP)
│  └─ Delete Line (Learning - 100% XP)
├─ Profile (Level 5, 842 XP)
└─ Statistics
```

---

## 📚 Commands Supported

| Category | Commands |
|----------|----------|
| **Movement** | `h, j, k, l, w, b, e, 0, $, gg, G` |
| **Editing** | `i, a, I, A, o, O, r, c, x, dd, J, >, <` |
| **Clipboard** | `y, p, P` |
| **Undo/Redo** | `u, U` |
| **Repeat** | `.` (repeat last action) |
| **Insert Mode** | Text input, Backspace, arrow keys, Esc |

All commands powered by `helix-core` v25.07.1 for 100% accuracy.

---

## 🎓 Why This Project Exists

**Traditional editor tutorials teach commands. Real development requires workflows.**

Most Helix/Vim tutorials:
- Teach `dd` (delete line) in isolation ❌
- Show `w` (next word) on synthetic text ❌
- Stop at "congratulations, you know the basics!" ❌

**Real development requires:**
- Navigate to failing test → jump to implementation → fix bug → stage changes → commit ✅
- Refactor function across 3 files using LSP ✅
- Debug by jumping between error logs and source code ✅

**Helix Trainer bridges this gap** through:

### 1. Scientifically-Optimized Learning (FSRS)

- **20-30% fewer reviews** than traditional spaced repetition
- **99.6% better accuracy** than older algorithms (tested on 350M+ reviews)
- **Identifies YOUR weaknesses** and schedules smart practice
- Same algorithm as Anki 23.10+ (research-proven)

### 2. Scenario Mastery System

Prevents XP farming while ensuring genuine skill development:

- **Three-tier progression**: Learning (100% XP) → Proficient (50% XP) → Mastered (20% XP)
- **Session spam protection**: Same-day penalties (100% → 70% → 30%)
- **Bounded tracking**: 10,000 scenario limit with validation
- **Performance benchmarks**: <1ms XP calculations, ~288 bytes per scenario

### 3. Gamification That Works

Duolingo-proven mechanics:
- **Daily quests** with fresh challenges
- **Streak tracking** with loss aversion
- **XP & levels** (exponential scaling)
- **Achievements** for milestones

### 4. 100% Offline & Privacy-First

- No cloud services, no internet required
- All data stored locally (`~/.config/helix-trainer/`)
- No telemetry, tracking, or data collection
- Your learning stays on your machine

---

## 📊 Current Status

### ✅ Phase 1: Smart Learning & Gamification (COMPLETE - v0.1.1)

- FSRS spaced repetition system
- Daily quest system
- XP/leveling with scenario mastery
- Profile & statistics tracking
- Anti-farming protection
- 164 passing tests, zero clippy warnings

### 🔄 Phase 2: Workflow Simulator (Planned - 6+ months)

The flagship feature that makes Helix Trainer unique:

- Mock LSP server for realistic scenarios
- Git repository state simulation
- Multi-file navigation training
- Real development workflow scenarios (CI debugging, refactoring, etc.)

**Why this matters**: Bridges the tutorial → productivity gap that nobody else solves.

---

## ⚙️ Technology Stack

| Component | Library | Version |
|-----------|---------|---------|
| **TUI Framework** | [ratatui](https://ratatui.rs/) | 0.29 |
| **Terminal I/O** | [crossterm](https://github.com/crossterm-rs/crossterm) | 0.29 |
| **Editor Core** | [helix-core](https://github.com/helix-editor/helix) | 25.07.1 |
| **Spaced Repetition** | [fsrs](https://crates.io/crates/fsrs) | 5.2 |
| **Large Text** | [tui-big-text](https://crates.io/crates/tui-big-text) | 0.7 |

**Project Metrics**:
- **Language**: Rust 2024 Edition
- **MSRV**: 1.85
- **Lines of Code**: ~5,759
- **Tests**: 164 (all passing)
- **Binary Size**: ~3MB (release mode)
- **Build Time**: ~1.5-2s (incremental, with sccache)

---

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup and workflow
- Code standards and quality checks
- Pull request process
- Testing requirements

**Quick contributor setup**:

```bash
# Clone repository
git clone https://github.com/bug-ops/helix-trainer.git
cd helix-trainer

# Run quality checks before committing
cargo +nightly fmt
cargo nextest run
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

With `sccache` configured, rebuilds are 5x faster (~10s incremental).

---

## 📄 Documentation

- [CHANGELOG.md](CHANGELOG.md) — Release history and version notes
- [CONTRIBUTING.md](CONTRIBUTING.md) — Contribution guidelines
- [SECURITY.md](SECURITY.md) — Security policy

---

## ❓ FAQ

<details>
<summary><b>Why not just use <code>:tutor</code> in Helix?</b></summary>

`:tutor` is excellent for one-time learning. Helix Trainer adds:
- Spaced repetition for long-term retention
- Gamification for daily habit formation
- Progress tracking and analytics
- (Phase 2) Full workflow training

They're complementary tools, not competitors.
</details>

<details>
<summary><b>Why FSRS instead of traditional spaced repetition?</b></summary>

FSRS is 20-30% more efficient (research-proven on 350M+ reviews). It's the same algorithm Anki switched to in v23.10+. We use the best available learning science.
</details>

<details>
<summary><b>Can I use this offline?</b></summary>

Yes, 100% offline. No internet required, all data local, zero telemetry.
</details>

<details>
<summary><b>When is Phase 2 (Workflow Simulator)?</b></summary>

After Phase 1 stabilizes (~3 months). We're building the foundation (habits, engagement) before the flagship feature.
</details>

<details>
<summary><b>Is this only for Helix, or can I learn Vim/Neovim?</b></summary>

Helix-specific. Many commands overlap with Vim, but Helix uses a different selection-first model. For Vim, try OpenVim or VimGenius.
</details>

---

## 🌟 Roadmap

| Phase | Status | Timeline | Focus |
|-------|--------|----------|-------|
| **Phase A** | ✅ Complete | — | Foundation (30+ commands, 20 scenarios, TUI) |
| **Phase 1** | ✅ Complete | — | Smart learning (FSRS, quests, mastery) |
| **Phase 2** | 📋 Planned | 6 months | Workflow simulator (LSP, git, multi-file) |
| **Phase 3** | 💡 Future | TBD | Network effects (multiplayer, scenarios marketplace) |

---

## 📈 Success Metrics

We're tracking:
- **80%+ users** practice recommended scenarios (spaced repetition)
- **70%+ users** complete 1+ quest per day
- **Average streak**: >7 days
- **D7 retention**: >40%
- **Review efficiency**: 20-30% fewer reviews vs random practice

Research-backed targets, not arbitrary KPIs.

---

## 🙏 Acknowledgments

- [Helix Editor](https://helix-editor.com/) — For the amazing modal editor
- [Ratatui](https://ratatui.rs/) — For the excellent TUI framework
- [FSRS Research Team](https://github.com/open-spaced-repetition) — For the algorithm
- [Anki](https://apps.ankiweb.net/) — Inspiration for spaced repetition

Inspired by vim-tutor, OpenVim, and decades of learning science research.

---

## 📝 License

Licensed under MIT — see [LICENSE](LICENSE) for details.

---

## 🚀 Get Started Now

```bash
# Download and run
wget https://github.com/bug-ops/helix-trainer/releases/latest/download/helix-trainer-v0.1.1-x86_64-unknown-linux-gnu.tar.gz
tar -xzf helix-trainer-v0.1.1-x86_64-unknown-linux-gnu.tar.gz
cd helix-trainer-v0.1.1-x86_64-unknown-linux-gnu
./helix-trainer
```

The journey from beginner to proficient starts with one practice session. ⌨️
