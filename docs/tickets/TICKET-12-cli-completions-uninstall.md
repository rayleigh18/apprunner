# TICKET-12: CLI Parsing + Completions + Uninstall

## Priority: Medium
## Dependencies: TICKET-02
## Blocks: TICKET-13

## Description
Implement the CLI interface using clap with subcommands for launching the TUI, generating shell completions, and uninstalling.

## Acceptance Criteria
- [ ] `apprunner` (no args) — launches TUI
- [ ] `apprunner --uninstall` — removes binary, completions, and DB
- [ ] `apprunner completions <shell>` — prints completions script to stdout
- [ ] Supported shells for completions: zsh, bash, fish
- [ ] Uninstall flow:
  1. Confirm with user: "Remove apprunner, completions, and all app configs? [y/N]"
  2. Detect binary location (check /usr/local/bin and ~/.local/bin)
  3. Remove binary
  4. Remove `~/.zfunc/_apprunner`
  5. Remove `~/.local/share/apprunner/` directory
  6. Print success message

## CLI Definition

```rust
#[derive(Parser)]
#[command(name = "apprunner", about = "Local app runner TUI")]
struct Cli {
    /// Uninstall apprunner (removes binary, completions, and data)
    #[arg(long)]
    uninstall: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
```

## Files
- `src/main.rs`
- `src/install.rs`

## Tests
- Test CLI parsing: no args -> TUI mode
- Test CLI parsing: --uninstall flag detected
- Test CLI parsing: completions subcommand with shell arg
- Test completions output is valid (non-empty, contains "apprunner")
- Test uninstall detects binary location correctly
