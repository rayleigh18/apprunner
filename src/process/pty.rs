//! PTY spawning and output capture.

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

/// A process running inside a pseudo-terminal.
pub struct PtyProcess {
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    #[allow(dead_code)]
    reader_handle: Option<std::thread::JoinHandle<()>>,
}

impl PtyProcess {
    /// Spawn a new process in a PTY.
    ///
    /// The command is executed via `/bin/sh -c <command>`.
    /// A background thread reads PTY output into `output_buffer`.
    pub fn spawn(
        command: &str,
        working_dir: &str,
        env_vars: &HashMap<String, String>,
        output_buffer: Arc<Mutex<Vec<u8>>>,
    ) -> Result<Self> {
        let pty_system = native_pty_system();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.arg("-c");
        cmd.arg(command);
        cmd.cwd(working_dir);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)?;

        // Get a reader from the master side
        let mut reader = pair.master.try_clone_reader()?;

        // Spawn a thread to continuously read output
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Ok(mut output) = output_buffer.lock() {
                            output.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: pair.master,
            child,
            reader_handle: Some(handle),
        })
    }

    /// Get the child's PID, if available.
    pub fn pid(&self) -> Option<u32> {
        self.child.process_id()
    }

    /// Check if the process has exited (non-blocking).
    pub fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }

    /// Kill the process.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_spawn_echo_captures_output() {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let env_vars = HashMap::new();

        let mut proc =
            PtyProcess::spawn("echo hello", "/tmp", &env_vars, output_buffer.clone()).unwrap();

        // Wait for process to finish and output to arrive
        std::thread::sleep(Duration::from_millis(500));

        let output = output_buffer.lock().unwrap();
        let text = String::from_utf8_lossy(&output);
        assert!(
            text.contains("hello"),
            "Expected output to contain 'hello', got: {:?}",
            text
        );

        // Process should have exited
        let status = proc.try_wait().unwrap();
        assert!(status.is_some());
    }

    #[test]
    fn test_spawn_with_working_dir() {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let env_vars = HashMap::new();

        let mut proc = PtyProcess::spawn("pwd", "/tmp", &env_vars, output_buffer.clone()).unwrap();

        std::thread::sleep(Duration::from_millis(500));

        let output = output_buffer.lock().unwrap();
        let text = String::from_utf8_lossy(&output);
        // On macOS /tmp is a symlink to /private/tmp
        assert!(
            text.contains("/tmp") || text.contains("/private/tmp"),
            "Expected output to contain '/tmp', got: {:?}",
            text
        );

        let status = proc.try_wait().unwrap();
        assert!(status.is_some());
    }

    #[test]
    fn test_kill_terminates_process() {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let env_vars = HashMap::new();

        let mut proc =
            PtyProcess::spawn("sleep 60", "/tmp", &env_vars, output_buffer.clone()).unwrap();

        std::thread::sleep(Duration::from_millis(100));

        // Process should be running
        let status = proc.try_wait().unwrap();
        assert!(status.is_none(), "Process should still be running");

        // Kill it
        proc.kill().unwrap();
        std::thread::sleep(Duration::from_millis(100));

        // Now it should have exited
        let status = proc.try_wait().unwrap();
        assert!(status.is_some(), "Process should have exited after kill");
    }

    #[test]
    fn test_try_wait_on_exited_process() {
        let output_buffer = Arc::new(Mutex::new(Vec::new()));
        let env_vars = HashMap::new();

        let mut proc = PtyProcess::spawn("true", "/tmp", &env_vars, output_buffer.clone()).unwrap();

        std::thread::sleep(Duration::from_millis(500));

        let status = proc.try_wait().unwrap();
        assert!(status.is_some(), "Process should have exited");
    }
}
