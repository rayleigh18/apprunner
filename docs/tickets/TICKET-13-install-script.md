# TICKET-13: Install Script

## Priority: Medium
## Dependencies: TICKET-12
## Blocks: None

## Description
Create `install.sh` — a shell script that builds apprunner from source and installs the binary + zsh completions.

## Acceptance Criteria
- [ ] Check for `cargo` in PATH
  - If missing: print message, ask "Install Rust via mise? [Y/n]"
  - If Y: `curl https://mise.jdx.dev/install.sh | sh && mise use -g rust`
  - If n: print "Install Rust manually: https://rustup.rs" and exit 1
- [ ] Run `cargo build --release`
- [ ] Ask install location:
  - "[1] /usr/local/bin (may need sudo)"
  - "[2] ~/.local/bin"
- [ ] Copy `target/release/apprunner` to chosen location (with sudo if needed)
- [ ] Create `~/.zfunc/` if it doesn't exist
- [ ] Generate completions: `<binary> completions zsh > ~/.zfunc/_apprunner`
- [ ] Check `.zshrc` for `fpath+=~/.zfunc` — append if missing
- [ ] Check `.zshrc` for `compinit` — append `autoload -Uz compinit && compinit` if missing
- [ ] Print success message with "restart your shell or run: source ~/.zshrc"
- [ ] Script is idempotent (safe to run multiple times)
- [ ] Script handles errors gracefully (set -e, trap)

## Files
- `install.sh` (root of project)

## Tests
- Script is valid bash (shellcheck passes)
- Script detects missing cargo correctly
- Script doesn't duplicate .zshrc entries on re-run
- Manual testing for install/uninstall cycle
