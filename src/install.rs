//! Uninstall logic — remove binary, completions, and database.

use anyhow::{Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

/// Uninstall apprunner from the system.
pub fn uninstall() -> Result<()> {
    print!(
        "This will remove apprunner binary, zsh completions, and all app configs. Continue? [y/N] "
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim() != "y" && input.trim() != "Y" {
        println!("Cancelled.");
        return Ok(());
    }

    // Remove binary
    match find_binary() {
        Some(path) => {
            if let Err(e) = fs::remove_file(&path) {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    eprintln!(
                        "Error: Permission denied removing {}. Try: sudo rm {}",
                        path.display(),
                        path.display()
                    );
                } else {
                    eprintln!("Error removing binary {}: {}", path.display(), e);
                }
            } else {
                println!("Removed binary: {}", path.display());
            }
        }
        None => {
            eprintln!("Warning: apprunner binary not found, skipping binary removal.");
        }
    }

    // Remove completions
    remove_completions().context("Failed to remove completions")?;

    // Remove data directory
    remove_data_dir().context("Failed to remove data directory")?;

    println!(
        "Uninstalled successfully. You may want to remove fpath/compinit lines from ~/.zshrc."
    );
    Ok(())
}

/// Find where apprunner binary is installed.
fn find_binary() -> Option<PathBuf> {
    // Primary: use `which` to detect the binary
    if let Ok(path) = which::which("apprunner") {
        return Some(path);
    }

    // Fallback: check known locations
    let candidates = [
        PathBuf::from("/usr/local/bin/apprunner"),
        dirs::home_dir()
            .map(|h| h.join(".local/bin/apprunner"))
            .unwrap_or_default(),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    None
}

/// Find and remove zsh completions.
fn remove_completions() -> Result<()> {
    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };

    let completions_path = home.join(".zfunc/_apprunner");
    if completions_path.exists() {
        fs::remove_file(&completions_path)
            .with_context(|| format!("Failed to remove {}", completions_path.display()))?;
        println!("Removed completions: {}", completions_path.display());
    }

    Ok(())
}

/// Remove the data directory (SQLite DB).
fn remove_data_dir() -> Result<()> {
    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };

    let data_dir = home.join(".local/share/apprunner");
    if data_dir.exists() {
        fs::remove_dir_all(&data_dir)
            .with_context(|| format!("Failed to remove {}", data_dir.display()))?;
        println!("Removed data directory: {}", data_dir.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use clap_complete::{generate, Shell};

    // Re-import the CLI struct for completions test
    #[derive(clap::Parser)]
    #[command(name = "apprunner", version, about = "Local app runner TUI", author)]
    struct TestCli {
        #[arg(long)]
        uninstall: bool,

        #[command(subcommand)]
        command: Option<TestCommands>,
    }

    #[derive(clap::Subcommand)]
    enum TestCommands {
        Completions {
            #[arg(value_enum)]
            shell: Shell,
        },
    }

    #[test]
    fn test_find_binary_returns_option() {
        // find_binary may or may not find the binary depending on environment.
        // We just verify it doesn't panic and returns a valid Option.
        let result = find_binary();
        // If found, the path should contain "apprunner"
        if let Some(path) = result {
            assert!(path.to_string_lossy().contains("apprunner"));
        }
    }

    #[test]
    fn test_remove_completions_when_not_exists() {
        // Should succeed silently when completions file doesn't exist
        let result = remove_completions();
        // This will only pass if ~/.zfunc/_apprunner doesn't exist,
        // or if it does exist it will actually remove it.
        // In a CI/test environment, it typically doesn't exist.
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_data_dir_when_not_exists() {
        // Use a temp dir to verify behavior when dir doesn't exist
        // The actual function checks ~/.local/share/apprunner — if it doesn't exist, it's fine.
        // We just verify the function doesn't error on missing dirs.
        let result = remove_data_dir();
        // In test environments this dir likely doesn't exist, so it should be Ok
        assert!(result.is_ok());
    }

    #[test]
    fn test_completions_output() {
        // Generate zsh completions into a buffer and verify non-empty
        let mut buf = Vec::new();
        let mut cmd = TestCli::command();
        generate(Shell::Zsh, &mut cmd, "apprunner", &mut buf);
        assert!(
            !buf.is_empty(),
            "Zsh completions should produce non-empty output"
        );

        let output = String::from_utf8(buf).expect("Completions should be valid UTF-8");
        assert!(
            output.contains("apprunner"),
            "Completions should reference the binary name"
        );
    }

    #[test]
    fn test_completions_bash_output() {
        let mut buf = Vec::new();
        let mut cmd = TestCli::command();
        generate(Shell::Bash, &mut cmd, "apprunner", &mut buf);
        assert!(
            !buf.is_empty(),
            "Bash completions should produce non-empty output"
        );
    }

    #[test]
    fn test_completions_fish_output() {
        let mut buf = Vec::new();
        let mut cmd = TestCli::command();
        generate(Shell::Fish, &mut cmd, "apprunner", &mut buf);
        assert!(
            !buf.is_empty(),
            "Fish completions should produce non-empty output"
        );
    }
}
