# TICKET-03: Health Check Module

## Priority: High
## Dependencies: TICKET-01
## Blocks: TICKET-04, TICKET-08

## Description
Implement health checks that validate app configuration before saving and before starting a process.

## Acceptance Criteria
- [ ] Validate `working_dir` exists and is a directory
- [ ] Validate command binary is resolvable (first token checked against PATH using `which` crate)
- [ ] Validate env_vars JSON is well-formed
- [ ] Return structured error messages for each validation failure
- [ ] Can be called for both "pre-save" (form validation) and "pre-start" checks

## API

```rust
pub struct HealthCheckResult {
    pub is_healthy: bool,
    pub errors: Vec<HealthError>,
}

pub enum HealthError {
    DirNotFound(String),
    DirNotDirectory(String),
    CommandNotFound(String),
    InvalidEnvVars(String),
}

pub fn check_app_health(working_dir: &str, command: &str, env_vars: &str) -> HealthCheckResult;
```

## Files
- `src/process/health.rs`

## Tests
- Test with valid directory and command
- Test with non-existent directory
- Test with path that exists but is a file (not dir)
- Test with invalid command (not in PATH)
- Test with malformed JSON env vars
- Test with valid JSON env vars
- Test with empty command string
