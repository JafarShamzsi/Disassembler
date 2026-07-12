# Contributing

Thanks for helping improve this terminal-native disassembler.

## Development Setup

Install a stable Rust toolchain, then run:

```bash
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

Use `tests/notepad.exe` for local smoke checks when you need a small PE fixture:

```bash
cargo run -- tests/notepad.exe --cfg --metrics
cargo run -- tests/notepad.exe --names
cargo run -- tests/notepad.exe --tui --save-project analysis.disproj.json
```

## Pull Request Expectations

- Keep changes scoped to one feature, bug fix, or refactor.
- Add or update tests for new analysis behavior, project persistence, exporters, or TUI state transitions.
- Update README or ROADMAP when user-visible commands, tabs, project files, or workflows change.
- Run `cargo fmt`, `cargo test`, and strict `cargo clippy` before opening a PR.
- Avoid committing generated analysis files unless they are intentional fixtures.

## TUI Work

The TUI is split by responsibility:

- `src/tui/app.rs`: state, navigation, search, project actions.
- `src/tui/input.rs`: keyboard handling.
- `src/tui/views.rs`: Ratatui rendering.
- `src/tui/session.rs`: terminal lifecycle.

Prefer testing state transitions in `src/tui/mod.rs` instead of trying to automate interactive terminal sessions.

## Analysis Quality

Reverse-engineering features should be explicit about confidence. If a heuristic can be noisy, label it as inferred or experimental in UI/export text and tests.