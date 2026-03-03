# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

## [0.2.0] - 2026-03-03

- Added hidden directory support controls:
  - Persistent config toggles via `runin config --include-hidden` / `--no-include-hidden`.
  - Added matching root toggles: `runin config --include-root` / `--no-include-root`.
  - One-run override via `-H, --hidden`.
  - Interactive config prompt includes `Include hidden paths (y/n)`.
- Improved config and runtime robustness:
  - Added `search_root` validation (must exist and be a directory) before running `fd`.
  - Improved `fd` failure errors with contextual details.
  - Tightened dependency checks to require successful `--version` execution.
  - Made home expansion safer by supporting start-only `$HOME`, `${HOME}`, and `~`.
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
