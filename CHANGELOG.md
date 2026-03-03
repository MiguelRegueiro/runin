# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

- Added hidden directory support controls:
  - Persistent config toggle via `runin config --include-hidden <true|false>`.
  - One-run override via `-H, --hidden`.
  - Interactive config prompt includes `Include hidden paths (y/n)`.
- Improved CLI help output:
  - Added explicit clap color styling (cyan headers, green usage, yellow literals).
  - Refined help content and examples, including `runin -H`.
  - Kept custom usage/examples guidance in help footer.
- Refactored project layout for maintainability:
  - Moved runtime code into `src/lib.rs` with a thin `src/main.rs` entrypoint.
  - Split config persistence into `src/config.rs`.
  - Moved non-UI tests into `src/tests.rs`.
  - Removed temporary/editor artifacts and aligned code with rustfmt.

## [0.1.0] - 2026-03-03

- Initial public release.
- Fast project selection using `fd` piped into `fzf`.
- Command execution in selected directory via direct `exec`.
- Interactive `runin config` flow with three prompts:
  - `Search root [current]:`
  - `Default command [current]:`
- `Include root [y|n]:` toggle (default `false`) for showing `search_root` in the picker.
- Config persistence at `~/.config/runin/config.toml`.
- Runtime dependency checks for `fd` and `fzf` with clear install guidance.
- Basic tests for config roundtrip/parse errors and config UI input sanitization.
