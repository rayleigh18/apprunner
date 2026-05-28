# TICKET-01: Project Scaffold

## Priority: High
## Dependencies: None
## Blocks: All other tickets

## Description
Initialize the Rust project with proper directory structure, Cargo.toml with all dependencies, and stub files for all modules.

## Acceptance Criteria
- [x] `Cargo.toml` with all dependencies defined
- [ ] `src/main.rs` with basic clap CLI skeleton
- [ ] `src/app.rs` with `App` struct stub
- [ ] Module stubs for: `tui/`, `db/`, `process/`, `vt/`, `metrics/`, `ghostty/`, `install.rs`
- [ ] Each module has `mod.rs` with basic module documentation
- [ ] Project compiles with `cargo check`

## Files
- `Cargo.toml`
- `src/main.rs`
- `src/app.rs`
- `src/tui/mod.rs`
- `src/tui/ui.rs`
- `src/tui/input.rs`
- `src/tui/form.rs`
- `src/tui/file_browser.rs`
- `src/db/mod.rs`
- `src/process/mod.rs`
- `src/process/pty.rs`
- `src/process/restart.rs`
- `src/process/health.rs`
- `src/vt/mod.rs`
- `src/metrics/mod.rs`
- `src/ghostty/mod.rs`
- `src/install.rs`

## Tests
- `cargo check` passes
- `cargo build` passes
