//! Ghostty attach/detach flow — spawn Ghostty for interactive terminal access.

use std::process::{Child, Command};

use anyhow::{Context, Result};

/// A running Ghostty session attached to an app.
#[derive(Debug)]
pub struct GhosttySession {
    child: Child,
    pub app_id: i64,
}

impl GhosttySession {
    /// Spawn a new Ghostty window running the given command in the given directory.
    /// Command: ghostty -e /bin/sh -c "cd <working_dir> && <command>"
    pub fn spawn(working_dir: &str, command: &str, app_id: i64) -> Result<Self> {
        // Check if ghostty is available first
        if !is_ghostty_available() {
            anyhow::bail!("ghostty not found in PATH");
        }

        let args = build_ghostty_command(working_dir, command);

        let child = Command::new("ghostty")
            .args(&args)
            .spawn()
            .context("Failed to spawn ghostty process")?;

        Ok(Self { child, app_id })
    }

    /// Check if the Ghostty process is still running (non-blocking).
    /// Returns true if still alive, false if exited.
    pub fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false, // Process has exited
            Ok(None) => true,     // Still running
            Err(_) => false,      // Error checking — treat as exited
        }
    }

    /// Wait for the Ghostty process to exit (blocking).
    pub fn wait(&mut self) -> Result<()> {
        self.child.wait().context("Failed to wait for ghostty")?;
        Ok(())
    }

    /// Kill the Ghostty process.
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("Failed to kill ghostty")?;
        Ok(())
    }
}

/// Check if ghostty binary is available in PATH.
pub fn is_ghostty_available() -> bool {
    which::which("ghostty").is_ok()
}

/// Build the command arguments for ghostty execution.
/// Returns the args to pass after the `ghostty` binary.
pub fn build_ghostty_command(working_dir: &str, command: &str) -> Vec<String> {
    // Escape single quotes in working_dir for safe shell quoting
    let escaped_dir = working_dir.replace("'", "'\\''");
    let shell_command = format!("cd '{}' && {}", escaped_dir, command);

    vec![
        "-e".to_string(),
        "/bin/sh".to_string(),
        "-c".to_string(),
        shell_command,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ghostty_available_does_not_panic() {
        // Result depends on system — just ensure it doesn't panic
        let _ = is_ghostty_available();
    }

    #[test]
    fn test_build_ghostty_command_basic() {
        let args = build_ghostty_command("/tmp/myapp", "npm start");
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "-e");
        assert_eq!(args[1], "/bin/sh");
        assert_eq!(args[2], "-c");
        assert_eq!(args[3], "cd '/tmp/myapp' && npm start");
    }

    #[test]
    fn test_build_ghostty_command_handles_spaces() {
        let args = build_ghostty_command("/home/user/my project", "cargo run");
        assert_eq!(args[3], "cd '/home/user/my project' && cargo run");
    }

    #[test]
    fn test_build_ghostty_command_handles_single_quotes() {
        let args = build_ghostty_command("/home/user/it's a dir", "echo hello");
        // Single quotes should be escaped: ' -> '\''
        assert_eq!(args[3], "cd '/home/user/it'\\''s a dir' && echo hello");
    }

    #[test]
    #[ignore] // Only runs if ghostty is available on the system
    fn test_spawn_with_simple_command() {
        if !is_ghostty_available() {
            return;
        }
        let mut session = GhosttySession::spawn("/tmp", "echo hello", 1).unwrap();
        // Give it a moment to start
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Kill to clean up
        let _ = session.kill();
    }

    #[test]
    fn test_spawn_fails_without_ghostty() {
        if is_ghostty_available() {
            // Can't test this if ghostty IS available
            return;
        }
        let result = GhosttySession::spawn("/tmp", "echo hello", 1);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ghostty not found"),
            "Expected 'ghostty not found' error, got: {}",
            err_msg
        );
    }
}
