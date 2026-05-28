# TICKET-10: Ghostty Attach/Detach

## Priority: Medium
## Dependencies: TICKET-04, TICKET-07
## Blocks: None

## Description
Implement the attach flow: stop the managed PTY process, launch Ghostty with the app's command, and resume managed mode when Ghostty exits.

## Acceptance Criteria
- [ ] On attach: stop the managed PTY process gracefully
- [ ] Spawn Ghostty: `ghostty -e <shell> -c "cd <working_dir> && <command>"`
- [ ] TUI shows "attached" state for the app while Ghostty is open
- [ ] Block the app from being started/stopped while attached
- [ ] On Ghostty exit (window closed): resume managed mode
- [ ] Reset crash counter on detach (resume)
- [ ] Handle case where `ghostty` binary is not in PATH (show error)
- [ ] Handle case where Ghostty crashes immediately

## Flow

```
1. User presses 'a' on running/stopped app
2. If running: stop managed process
3. Spawn: ghostty -e /bin/sh -c "cd /path && command"
4. Set app state to Attached
5. Wait for ghostty process to exit (non-blocking, polled in event loop)
6. On exit: set state to Stopped, reset crash counter
7. If auto_start: restart in managed mode
```

## Files
- `src/ghostty/mod.rs`

## Tests
- Test ghostty binary detection (is in PATH or not)
- Test command construction (proper escaping of paths with spaces)
- Test state transitions: Running -> Attached -> Stopped
- Test state transitions: Stopped -> Attached -> Stopped
- Test crash counter reset after detach
