# TICKET-17: Proxy Engine Core

## Priority: High
## Dependencies: TICKET-15, TICKET-16
## Blocks: TICKET-19, TICKET-20

## Description
Implement the reverse proxy engine that listens on a local port and forwards requests to the upstream target URL, injecting configured headers. This is the core networking logic, independent of the TUI.

Key behaviors:
- Binds to `127.0.0.1:{listen_port}`
- Receives any HTTP request (all methods, all paths)
- Forwards the request to `{target_url}{original_path}?{original_query}`
- Overwrites request headers with configured mask headers
- Streams the response back chunk-by-chunk (no buffering)
- Returns 502 on upstream connection failure
- Never exposes header values in error messages
- No timeout (waits indefinitely for upstream)

## Acceptance Criteria
- [ ] `src/mask/mod.rs` — module declarations
- [ ] `src/mask/proxy.rs` — proxy server implementation:
  - `MaskProxy` struct holding config (target_url, listen_port, headers)
  - `start()` method that binds and starts serving (returns a shutdown handle)
  - `stop()` method via shutdown handle (graceful shutdown)
- [ ] `src/mask/log.rs` — request log ring buffer:
  - `RequestLog` struct (50-entry VecDeque)
  - `LogEntry` struct: timestamp, method, path, status_code (Option), latency_ms, error (Option<String>)
  - Entries logged at response-header time (TTFB)
  - Error entries for connection failures (no status code, error message present)
- [ ] Full path pass-through (caller path forwarded verbatim)
- [ ] Query parameters forwarded as-is
- [ ] Request body forwarded as-is (streaming)
- [ ] Response streamed back chunk-by-chunk
- [ ] Configured headers always overwrite caller's headers
- [ ] Port conflict detected on bind — returns clear error
- [ ] Never logs/displays header values, only header names

## Files
- `src/mask/mod.rs`
- `src/mask/proxy.rs`
- `src/mask/log.rs`
- `src/lib.rs` (add `pub mod mask;`)

## Tests
- Unit test: ring buffer evicts old entries past 50
- Integration test: start proxy, send request, verify forwarded correctly
- Integration test: verify headers are overwritten
- Integration test: verify streaming response (SSE simulation)
- Integration test: verify 502 on unreachable upstream
- Integration test: verify port conflict error
