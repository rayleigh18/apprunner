# TICKET-06: Metrics Module

## Priority: Medium
## Dependencies: TICKET-01
## Blocks: TICKET-07, TICKET-11

## Description
Implement CPU and memory monitoring for managed processes using the `sysinfo` crate. Polls at a configurable interval (default 2 seconds).

## Acceptance Criteria
- [ ] Poll CPU% and RSS memory for a given PID
- [ ] Main process only (not process tree)
- [ ] Return `None` if process is not running
- [ ] Handle process disappearing between polls gracefully
- [ ] Format memory as human-readable (e.g., "84M", "1.2G")
- [ ] CPU% as single decimal (e.g., "3.2%")
- [ ] Refresh interval configurable (default 2s)

## API

```rust
pub struct ProcessMetrics {
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_display: String,  // "84M", "1.2G"
    pub cpu_display: String,     // "3.2%"
}

pub struct MetricsCollector {
    system: sysinfo::System,
    refresh_interval: Duration,
}

impl MetricsCollector {
    pub fn new(refresh_interval: Duration) -> Self;
    pub fn refresh(&mut self);
    pub fn get_metrics(&self, pid: u32) -> Option<ProcessMetrics>;
    pub fn format_memory(bytes: u64) -> String;
}
```

## Files
- `src/metrics/mod.rs`

## Tests
- Test format_memory with various sizes (bytes, KB, MB, GB)
- Test get_metrics returns None for non-existent PID
- Test get_metrics returns Some for current process (self-test)
- Test refresh doesn't panic on dead PIDs
