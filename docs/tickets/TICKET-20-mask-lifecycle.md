# TICKET-20: Mask Lifecycle Management in TUI

## Priority: High
## Dependencies: TICKET-17, TICKET-18
## Blocks: TICKET-21

## Description
Wire the proxy engine into the TUI event loop. Masks can be activated/deactivated from the mask list. Auto-start masks launch on TUI startup. Multiple masks run concurrently as tokio tasks. Proxy status and errors are reflected in the mask list and request log.

## Acceptance Criteria
- [ ] `src/mask/manager.rs` — manages running mask proxies:
  - `MaskManager` struct holds active proxy handles + request logs
  - `activate(mask_config) -> Result<()>` — starts proxy, stores handle
  - `deactivate(mask_id) -> Result<()>` — sends shutdown signal
  - `get_log(mask_id) -> &RequestLog` — access request log for display
  - `status(mask_id) -> MaskStatus` (Inactive, Active, Error(String))
- [ ] Auto-start: on TUI init, query `get_auto_start_masks()` and activate each
- [ ] Port conflict on activate → set status to `Error("port XXXX already in use")`, don't crash
- [ ] Deactivate gracefully shuts down the hyper server
- [ ] When TUI exits, all active proxies are shut down
- [ ] Mask list shows live status: `Active`, `Inactive`, `Error: <reason>`
- [ ] `s` key activates selected mask
- [ ] `x` key deactivates selected mask
- [ ] `Enter` on active mask shows its request log in detail view
- [ ] Request log detail view: scrollable with j/k/G/g, shows entries formatted as:
  `TIMESTAMP  METHOD PATH  STATUS  LATENCY`
- [ ] Error entries show `ERR` instead of status code

## Files
- `src/mask/manager.rs` (new file)
- `src/mask/mod.rs` (export manager)
- `src/app.rs` (integrate MaskManager, auto-start, lifecycle actions)
- `src/tui/ui.rs` (mask status display, request log rendering)
- `src/tui/input.rs` (wire activate/deactivate/focus-log actions)

## Tests
- Activate mask → status becomes Active
- Activate on used port → status becomes Error
- Deactivate → status becomes Inactive
- Auto-start masks activate on init
- All proxies shut down on exit
- Request log updates as requests flow through
