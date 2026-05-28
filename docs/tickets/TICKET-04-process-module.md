# TICKET-04: Process Module (PTY + Restart)

## Priority: High
## Dependencies: TICKET-02, TICKET-03
## Blocks: TICKET-07, TICKET-10, TICKET-11

## Description
Implement process management: spawn child processes in PTY, capture output, and handle restart logic with a max retry policy.

## Acceptance Criteria
- [ ] Spawn a child process in a PTY using `portable-pty`
- [ ] Capture stdout/stderr output from the PTY into a shared buffer
- [ ] Track process state: Starting, Running, Stopped, Crashed
- [ ] On non-zero exit: immediately restart, increment crash counter
- [ ] At 5 consecutive failures: mark as Crashed, stop retrying
- [ ] Manual stop resets crash counter
- [ ] Manual start resets crash counter
- [ ] Inject environment variables from app config
- [ ] Set working directory for child process
- [ ] Track PID for metrics collection
- [ ] Track start time for runtime alerts

## API

```rust
pub enum ProcessState {
    Stopped,
    Starting,
    Running { pid: u32, started_at: Instant },
    Crashed { retries: u8, last_error: String },
}

pub struct ManagedProcess {
    pub app_id: i64,
    pub state: ProcessState,
    pub output_buffer: Arc<Mutex<VecDeque<u8>>>,
}

impl ManagedProcess {
    pub fn start(&mut self, working_dir: &str, command: &str, env_vars: &HashMap<String, String>) -> Result<()>;
    pub fn stop(&mut self) -> Result<()>;
    pub fn restart(&mut self) -> Result<()>;
}
```

## Files
- `src/process/mod.rs`
- `src/process/pty.rs`
- `src/process/restart.rs`

## Tests
- Test spawning a simple command (e.g., `echo hello`)
- Test output capture from PTY
- Test process stop (sends signal, process exits)
- Test restart logic (process exits non-zero, auto-restarts)
- Test max retry (after 5 crashes, state becomes Crashed)
- Test crash counter reset on manual stop/start
- Test environment variable injection
- Test working directory is set correctly
