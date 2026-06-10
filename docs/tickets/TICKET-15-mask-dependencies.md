# TICKET-15: Add Proxy Dependencies

## Priority: High
## Dependencies: None
## Blocks: TICKET-16, TICKET-17

## Description
Add the HTTP proxy dependencies (hyper, hyper-util, http-body-util, reqwest) to Cargo.toml and ensure the project compiles.

## Acceptance Criteria
- [ ] `hyper = { version = "1.10.1", features = ["server", "http1"] }` added
- [ ] `hyper-util = { version = "0.1.20", features = ["tokio", "server", "http1"] }` added
- [ ] `http-body-util = "0.1.3"` added
- [ ] `reqwest = { version = "0.13.4", features = ["stream"] }` added
- [ ] `cargo check` passes
- [ ] `cargo build` passes

## Files
- `Cargo.toml`

## Tests
- `cargo check` passes
- `cargo build` passes
