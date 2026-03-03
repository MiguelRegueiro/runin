# Contributing

Thanks for contributing to `runin`.

This document explains how to set up your environment, how changes are expected to be developed, and how the codebase is organized.

## Project goals

`runin` is a focused CLI tool:

- quickly select a project directory with `fd` + `fzf`
- run a command inside the selected directory
- stay fast, predictable, and easy to maintain

Contributions should preserve that minimal scope.

## Prerequisites

- Rust stable toolchain
- `fd` and `fzf` available in `PATH` (runtime dependencies)

Install Rust (if needed):

```bash
rustup toolchain install stable
rustup default stable
```

## Local development

Build and run:

```bash
cargo build
cargo run -- --help
```

Common flows:

```bash
cargo run --
cargo run -- nvim .
cargo run -- config
```

## Quality gates (must pass)

Before opening a PR, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo package --locked
cargo publish --dry-run --locked
```

These mirror CI expectations.

## Project structure

### `src/main.rs`

Binary entrypoint only.

- delegates to library code (`runin::main_entry()`)
- keeps startup wiring minimal

### `src/lib.rs`

Core CLI orchestration.

- defines CLI arguments/subcommands with `clap`
- coordinates config loading/saving and command execution flow
- handles dependency checks and directory selection pipeline execution

### `src/config.rs`

Configuration model and persistence.

- defines `Config` and defaults
- reads/writes `~/.config/runin/config.toml`
- expands `$HOME` in configured paths

### `src/config_ui.rs`

Interactive configuration prompt UI.

- `runin config` interactive mode
- input normalization and toggle parsing
- prompt styling and validation messages

## Architecture decisions

These module boundaries are intentional:

- `main.rs` stays very small by design.
  - Reason: keep process/bootstrap concerns separate from application logic.
  - Benefit: easier testing and cleaner maintenance.
- `lib.rs` owns runtime orchestration.
  - Reason: this is where command flow and cross-module coordination belong.
- `config.rs` and `config_ui.rs` are separate on purpose.
  - `config.rs` handles data model + file persistence.
  - `config_ui.rs` handles terminal prompting and input normalization.
  - Benefit: storage logic can evolve independently from interactive UX.

### `src/tests.rs`

Library unit tests (non-UI behavior).

- config roundtrip/defaulting/parse errors
- path parsing and helper behavior
- hidden-path resolution behavior

## Change guidelines

- Keep behavior changes explicit and small.
- Prefer readable, boring code over clever abstractions.
- Keep CLI output stable unless intentionally changed.
- If you change user-visible behavior, update docs/changelog in the same PR.
- Add or update tests for behavior changes.

## Commit and PR guidance

Use clear commit messages (Conventional Commits style is preferred):

- `feat: ...`
- `fix: ...`
- `refactor: ...`
- `style: ...`
- `docs: ...`

PR descriptions should include:

- what changed
- why it changed
- how it was validated (commands + results)

## Release and changelog notes

- Add notable user-facing changes under `## [Unreleased]` in `CHANGELOG.md`.
- Keep entries concise and grouped by theme.

## Scope boundaries

Please avoid broadening the project beyond its core use-case (project selection + command execution) unless discussed first in an issue.
