# TICKET-21: API Mask Integration Testing & Polish

## Priority: Medium
## Dependencies: TICKET-19, TICKET-20
## Blocks: None

## Description
End-to-end integration testing of the full API mask feature. Verify the complete flow: create mask via TUI form, activate it, send requests through it, verify header injection, view request log, deactivate, edit, delete.

Also handles polish items:
- Help screen updated with mask keybindings
- README updated with mask feature documentation

## Acceptance Criteria
- [ ] Integration test: full lifecycle (create → activate → proxy request → deactivate → delete)
- [ ] Integration test: multiple masks active simultaneously on different ports
- [ ] Integration test: SSE/streaming response passes through correctly
- [ ] Integration test: auto-start masks activate on TUI launch
- [ ] Integration test: header values never appear in any output/log
- [ ] Help screen (`?`) shows mask-related keybindings when on Masks tab
- [ ] README updated with API Mask section (purpose, usage flow, keybindings)
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` — all tests pass

## Files
- `tests/mask_integration_test.rs`
- `src/tui/ui.rs` (help screen update)
- `README.md` (documentation)

## Tests
- All integration tests pass
- `cargo clippy` clean
- `cargo test` passes
- Manual test: curl through mask to a real API (e.g. httpbin.org)
