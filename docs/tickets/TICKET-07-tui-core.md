# TICKET-07: TUI Core (Sidebar + Output + Status Bar)

## Priority: High
## Dependencies: TICKET-02, TICKET-04, TICKET-05, TICKET-06
## Blocks: TICKET-08, TICKET-09, TICKET-10, TICKET-11

## Description
Implement the core TUI layout with ratatui: sidebar app list, output preview pane, and status bar. Wire up the event loop and vim-like keybindings.

## Acceptance Criteria
- [ ] Terminal setup/teardown (alternate screen, raw mode)
- [ ] Layout: left sidebar (30%) + right output pane (70%)
- [ ] Bottom status bar with keybind hints and alerts
- [ ] Sidebar shows: status icon (●/○/✗), app name, CPU%, memory
- [ ] Output pane shows: scrollback from selected app (styled via VT parser)
- [ ] Output pane header shows app name
- [ ] Vim keybindings for navigation (j/k, Enter, Esc)
- [ ] Action keybindings (s, x, r, a, n, e, d, q, ?)
- [ ] Help overlay on `?`
- [ ] Event loop: handle keyboard input, tick events (for metrics refresh)
- [ ] 2-second tick for metrics polling
- [ ] Focus modes: AppList, OutputPane

## Layout

```
┌─────────────────────────────────────────────────┐
│  Apps               │  Output: my-api           │
│                     │                           │
│  ● my-api  2.1% 64M│  $ cargo run              │
│  ● web-ui  0.4% 12M│  Compiling my-api v0.1    │
│  ○ worker  —    —  │  Finished release         │
│  ✗ migrate —    —  │  Listening on 0.0.0.0:8080│
│                     │                           │
├─────────────────────┴───────────────────────────┤
│ [s]tart [x]top [a]ttach [n]ew [e]dit [d]el [?] │
└─────────────────────────────────────────────────┘
```

## Keybindings

### App List Mode
| Key | Action |
|-----|--------|
| j/k | Navigate |
| s | Start |
| x | Stop |
| r | Restart |
| a | Attach (Ghostty) |
| n | New app |
| e | Edit app |
| d | Delete (confirm) |
| Enter | Focus output pane |
| q | Quit |
| ? | Help overlay |

### Output Pane Mode
| Key | Action |
|-----|--------|
| j/k | Scroll down/up |
| G | Jump to bottom |
| g | Jump to top |
| Esc | Back to app list |

## Files
- `src/app.rs`
- `src/tui/mod.rs`
- `src/tui/ui.rs`
- `src/tui/input.rs`

## Tests
- Test keybinding dispatch maps correct keys to actions
- Test focus mode transitions
- Test app list selection wrapping
- Test scroll offset calculations
