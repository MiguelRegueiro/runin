# Changelog

All notable changes to this project are documented in this file.

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
