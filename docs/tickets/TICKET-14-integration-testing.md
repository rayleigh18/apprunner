# TICKET-14: Integration Testing + Polish

## Priority: Medium
## Dependencies: All previous tickets
## Blocks: None

## Description
End-to-end integration tests and final polish pass. Ensure all modules work together correctly.

## Acceptance Criteria
- [ ] Integration test: add app via DB, start it, verify output captured
- [ ] Integration test: app crash triggers restart, verify retry count
- [ ] Integration test: 5 crashes triggers Crashed state
- [ ] All unit tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Binary runs without panic on fresh system (no existing DB)
- [ ] README.md with:
  - Project description
  - Installation instructions
  - Usage guide
  - Keybinding reference
  - Screenshots/demo (placeholder)

## Files
- `tests/integration_test.rs`
- `README.md`

## Tests
- Full cargo test suite passes
- Clippy clean
- Fmt check passes
- Integration tests cover: DB + Process + Metrics lifecycle
