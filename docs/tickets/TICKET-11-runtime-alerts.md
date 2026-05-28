# TICKET-11: Runtime Alert Logic

## Priority: Medium
## Dependencies: TICKET-04, TICKET-06, TICKET-07
## Blocks: None

## Description
Implement tick-based runtime checking that alerts the user when an app has been running longer than its configured maximum (or the global default of 5 hours).

## Acceptance Criteria
- [ ] On each tick (2s), check all running apps against their max_runtime_secs
- [ ] If app has per-app `max_runtime_secs` set, use that
- [ ] Otherwise, use global default from config table (18000 seconds = 5 hours)
- [ ] If exceeded: show amber `⏱` indicator in sidebar next to app name
- [ ] If exceeded: show warning message in status bar
- [ ] Does NOT kill or stop the process — notification only
- [ ] Alert clears if app is manually restarted (timer resets)
- [ ] Alert clears if app is stopped

## API

```rust
pub struct RuntimeAlert {
    pub app_id: i64,
    pub app_name: String,
    pub running_since: Instant,
    pub max_allowed: Duration,
    pub exceeded_by: Duration,
}

pub fn check_runtime_alerts(
    processes: &HashMap<i64, ManagedProcess>,
    apps: &[AppConfig],
    global_max: u64,
) -> Vec<RuntimeAlert>;
```

## Files
- `src/process/mod.rs` (or a new `src/alerts.rs`)

## Tests
- Test no alert when under threshold
- Test alert when global default exceeded
- Test per-app override takes precedence over global
- Test alert clears on restart
- Test alert clears on stop
- Test exceeded_by calculation is correct
