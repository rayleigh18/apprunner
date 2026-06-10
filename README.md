# apprunner

A TUI app runner for managing local development processes. Run, monitor, and attach to multiple apps from a single terminal interface with Ghostty integration.

## Features

- **Process management** — Start, stop, and restart apps with auto-restart on crash (max 5 retries)
- **Template variables** — Parameterize commands with `{{placeholders}}` for ports, paths, secrets, etc.
- **Terminal preview** — Scrollback output with full ANSI color support rendered in the TUI
- **Ghostty attach** — Open any app in a full Ghostty window for interactive use, then resume managed mode
- **Resource monitoring** — Live CPU% and memory usage per process
- **Runtime alerts** — Get notified when apps run longer than expected (configurable per-app or global 5h default)
- **Health checks** — Validates working directory and command before starting
- **File browser** — Browse and select directories without leaving the TUI
- **Environment variables** — Inject per-app env vars
- **SQLite config** — All app configurations stored locally in `~/.local/share/apprunner/apprunner.db`

## Installation

```bash
git clone <repo-url>
cd apprunner
./install.sh
```

The install script will:
1. Check for Rust (offers to install via [mise](https://mise.jdx.dev) if missing)
2. Build from source (`cargo build --release`)
3. Install the binary (choice of `/usr/local/bin` or `~/.local/bin`)
4. Set up zsh completions

## Uninstall

```bash
apprunner --uninstall
```

Removes the binary, zsh completions, and all app configurations.

## Usage

```bash
apprunner          # Launch the TUI
```

### Keybindings

#### App List

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down |
| `s` | Start selected app |
| `S` | Start with options (override template variables) |
| `x` | Stop selected app |
| `r` | Restart app |
| `R` | Restart with options (override template variables) |
| `a` | Attach (open in Ghostty) |
| `n` | New app |
| `e` | Edit app |
| `d` | Delete app |
| `Enter` | Focus output pane |
| `q` | Quit |
| `?` | Help |

#### Output Pane

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down/up |
| `G` | Jump to bottom |
| `g` | Jump to top |
| `Esc` | Back to app list |

#### Form

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next/prev field |
| `Ctrl+b` | Browse directory |
| `Enter` | Save |
| `Esc` | Cancel |

#### File Browser

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate |
| `Enter` / `l` | Enter directory |
| `h` / `Backspace` | Go up |
| `.` | Select current directory |
| `Esc` | Cancel |

## Configuration

Apps are configured through the TUI form with:

- **Name** — Unique identifier
- **Working directory** — Where the app runs (supports `{{variables}}`)
- **Command** — Shell command to execute (supports `{{variables}}`)
- **Environment variables** — `KEY=VALUE` format, comma-separated (supports `{{variables}}`)
- **Auto-start** — Start automatically when apprunner launches
- **Max runtime** — Alert threshold in seconds (blank = use global default of 5 hours)
- **Interval** — Run as a cron job at this interval (seconds)

### Template Variables

Use `{{variable_name}}` placeholders in command, working directory, or environment variables to create parameterized app configurations.

```
Command:     cargo run -- --port {{port}}
Directory:   {{project_dir}}/api
Env vars:    API_KEY={{api_key}}
```

Variables are auto-detected when you tab out of a field. For each detected variable you can set:
- **Description** — Human-readable label shown in prompts
- **Default** — Pre-filled value (leave blank to make it required)

**Start behavior:**
- `s` starts immediately if all variables have defaults. Shows an input modal only if required variables exist.
- `S` always shows the input modal so you can override defaults for a one-off run.
- Overrides are ephemeral — configured defaults are always the source of truth.

**Auto-start:** If an app has `auto_start` enabled but has required variables without defaults, it will show as "Pending Input" and trigger an alert instead of starting.

## Requirements

- macOS or Linux
- [Ghostty](https://ghostty.org) (for attach mode)
- Rust toolchain (for building from source)

## Shell Completions

Generated during install. To manually generate:

```bash
apprunner completions zsh > ~/.zfunc/_apprunner
apprunner completions bash > ~/.bash_completions/apprunner
apprunner completions fish > ~/.config/fish/completions/apprunner.fish
```

## License

MIT
