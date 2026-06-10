//! Process management — PTY spawning, output capture, and restart logic.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;

pub mod health;
pub mod pty;
pub mod restart;

pub use self::pty::PtyProcess;
pub use self::restart::RestartPolicy;

/// The lifecycle state of a managed process.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Stopped,
    Starting,
    Running { pid: u32, started_at: Instant },
    Crashed { retries: u8, last_error: String },
    Attached,
}

/// A process managed by apprunner, including its PTY, output buffer, and restart policy.
pub struct ManagedProcess {
    pub app_id: i64,
    pub state: ProcessState,
    pub output_buffer: Arc<Mutex<Vec<u8>>>,
    pub restart_policy: RestartPolicy,
    pty_process: Option<PtyProcess>,
}

impl ManagedProcess {
    pub fn new(app_id: i64) -> Self {
        Self {
            app_id,
            state: ProcessState::Stopped,
            output_buffer: Arc::new(Mutex::new(Vec::new())),
            restart_policy: RestartPolicy::new(),
            pty_process: None,
        }
    }

    /// Start the process with the given configuration.
    pub fn start(
        &mut self,
        working_dir: &str,
        command: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<()> {
        self.state = ProcessState::Starting;

        let proc = PtyProcess::spawn(command, working_dir, env_vars, self.output_buffer.clone())?;

        let pid = proc.pid().unwrap_or(0);
        self.pty_process = Some(proc);
        self.state = ProcessState::Running {
            pid,
            started_at: Instant::now(),
        };

        Ok(())
    }

    /// Stop the process gracefully (kills it and resets restart policy).
    pub fn stop(&mut self) -> Result<()> {
        if let Some(ref mut proc) = self.pty_process {
            let _ = proc.kill();
        }
        self.pty_process = None;
        self.state = ProcessState::Stopped;
        self.restart_policy.reset();
        Ok(())
    }

    /// Restart the process (stop then start).
    pub fn restart(
        &mut self,
        working_dir: &str,
        command: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<()> {
        // Kill existing process if any
        if let Some(ref mut proc) = self.pty_process {
            let _ = proc.kill();
        }
        self.pty_process = None;

        self.start(working_dir, command, env_vars)
    }

    /// Check process health — called on tick. Returns `true` if state changed.
    ///
    /// If the process exited with non-zero, the restart policy decides whether
    /// to retry or mark the process as crashed.
    pub fn tick(
        &mut self,
        working_dir: &str,
        command: &str,
        env_vars: &HashMap<String, String>,
    ) -> bool {
        // Only check if we think we're running
        if !matches!(self.state, ProcessState::Running { .. }) {
            return false;
        }

        let exited = if let Some(ref mut proc) = self.pty_process {
            match proc.try_wait() {
                Ok(Some(status)) => Some(status.success()),
                Ok(None) => None,      // Still running
                Err(_) => Some(false), // Treat errors as crashes
            }
        } else {
            return false;
        };

        match exited {
            Some(true) => {
                // Normal exit (code 0)
                self.pty_process = None;
                self.state = ProcessState::Stopped;
                true
            }
            Some(false) => {
                // Non-zero exit — attempt restart
                self.pty_process = None;
                let should_retry = self.restart_policy.record_crash();

                if should_retry {
                    // Attempt restart
                    if self.start(working_dir, command, env_vars).is_err() {
                        self.state = ProcessState::Crashed {
                            retries: self.restart_policy.crash_count(),
                            last_error: "Failed to restart process".to_string(),
                        };
                    }
                } else {
                    self.state = ProcessState::Crashed {
                        retries: self.restart_policy.crash_count(),
                        last_error: "Process exited with non-zero status".to_string(),
                    };
                }
                true
            }
            None => {
                // Still running, no state change
                false
            }
        }
    }

    /// Cron-specific tick: check if process exited (any exit code) and mark stopped.
    /// Does NOT attempt restart. Returns `true` if the process exited.
    pub fn tick_cron(&mut self) -> bool {
        if !matches!(self.state, ProcessState::Running { .. }) {
            return false;
        }

        let exited = if let Some(ref mut proc) = self.pty_process {
            match proc.try_wait() {
                Ok(Some(_)) => true,
                Ok(None) => false,
                Err(_) => true,
            }
        } else {
            return false;
        };

        if exited {
            self.pty_process = None;
            self.state = ProcessState::Stopped;
        }
        exited
    }

    /// Get current output buffer contents.
    pub fn get_output(&self) -> Vec<u8> {
        self.output_buffer.lock().unwrap().clone()
    }

    /// Clear the output buffer.
    pub fn clear_output(&mut self) {
        self.output_buffer.lock().unwrap().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_creates_stopped_state() {
        let proc = ManagedProcess::new(1);
        assert_eq!(proc.state, ProcessState::Stopped);
        assert_eq!(proc.app_id, 1);
        assert!(proc.pty_process.is_none());
    }

    #[test]
    fn test_start_transitions_to_running() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        proc.start("/tmp", "sleep 10", &env_vars).unwrap();

        assert!(matches!(proc.state, ProcessState::Running { .. }));
        assert!(proc.pty_process.is_some());

        // Clean up
        proc.stop().unwrap();
    }

    #[test]
    fn test_stop_transitions_to_stopped_and_resets_counter() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        proc.start("/tmp", "sleep 10", &env_vars).unwrap();
        // Simulate some crashes in the policy
        proc.restart_policy.record_crash();
        proc.restart_policy.record_crash();
        assert_eq!(proc.restart_policy.crash_count(), 2);

        proc.stop().unwrap();

        assert_eq!(proc.state, ProcessState::Stopped);
        assert_eq!(proc.restart_policy.crash_count(), 0);
        assert!(proc.pty_process.is_none());
    }

    #[test]
    fn test_tick_with_dead_process_triggers_restart() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        // Start a process that exits immediately with non-zero
        proc.start("/tmp", "exit 1", &env_vars).unwrap();
        std::thread::sleep(Duration::from_millis(500));

        // Tick should detect exit and restart
        let changed = proc.tick("/tmp", "sleep 10", &env_vars);
        assert!(changed);

        // Should have restarted (Running state) with crash_count = 1
        assert!(
            matches!(proc.state, ProcessState::Running { .. }),
            "Expected Running state after restart, got: {:?}",
            proc.state
        );
        assert_eq!(proc.restart_policy.crash_count(), 1);

        // Clean up
        proc.stop().unwrap();
    }

    #[test]
    fn test_tick_after_5_crashes_sets_crashed_state() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        // Exhaust the restart policy by recording crashes manually
        // (simulating 4 previous crashes so the next record_crash is the 5th)
        for _ in 0..4 {
            proc.restart_policy.record_crash();
        }

        // Start a process that exits with non-zero
        proc.start("/tmp", "exit 1", &env_vars).unwrap();
        std::thread::sleep(Duration::from_millis(500));

        // Tick — this will be the 5th crash recorded, which still returns true (retry)
        let changed = proc.tick("/tmp", "exit 1", &env_vars);
        assert!(changed);

        // Wait for the restarted process to die too
        std::thread::sleep(Duration::from_millis(500));

        // Tick again — this will be the 6th crash, which returns false (exhausted)
        let changed = proc.tick("/tmp", "exit 1", &env_vars);
        assert!(changed);

        assert!(
            matches!(proc.state, ProcessState::Crashed { .. }),
            "Expected Crashed state, got: {:?}",
            proc.state
        );
    }

    #[test]
    fn test_tick_normal_exit_sets_stopped() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        // Start a process that exits successfully
        proc.start("/tmp", "true", &env_vars).unwrap();
        std::thread::sleep(Duration::from_millis(500));

        let changed = proc.tick("/tmp", "true", &env_vars);
        assert!(changed);
        assert_eq!(proc.state, ProcessState::Stopped);
    }

    #[test]
    fn test_get_output_and_clear() {
        let mut proc = ManagedProcess::new(1);
        let env_vars = HashMap::new();

        proc.start("/tmp", "echo test_output_xyz", &env_vars)
            .unwrap();
        std::thread::sleep(Duration::from_millis(500));

        let output = proc.get_output();
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("test_output_xyz"),
            "Expected output to contain 'test_output_xyz', got: {:?}",
            text
        );

        proc.clear_output();
        let output = proc.get_output();
        assert!(output.is_empty());

        proc.stop().unwrap();
    }
}
