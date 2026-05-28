# TICKET-09: TUI File Browser Overlay

## Priority: Medium
## Dependencies: TICKET-07
## Blocks: None

## Description
Implement a directory picker overlay widget that allows users to browse the filesystem and select a directory for the app's working_dir.

## Acceptance Criteria
- [ ] Opens as an overlay on top of the form
- [ ] Starts at `~` or current value of the working_dir field
- [ ] Shows only directories (filters out files)
- [ ] Breadcrumb path displayed at top
- [ ] j/k to navigate directory entries
- [ ] Enter to descend into selected directory
- [ ] h/Backspace to go up one level
- [ ] `.` to select current directory and close
- [ ] Esc to cancel and close without selection
- [ ] Entries sorted alphabetically
- [ ] Hidden directories (starting with `.`) shown but at the bottom

## Layout

```
┌─────────────────────────────────┐
│  Select Directory               │
│  ~/work/my-project              │
│  ─────────────────────────────  │
│  > src/                         │
│    tests/                       │
│    docs/                        │
│    .git/                        │
│                                 │
│  [.] select  [Esc] cancel      │
└─────────────────────────────────┘
```

## Files
- `src/tui/file_browser.rs`

## Tests
- Test directory listing (only dirs, no files)
- Test navigation up/down wrapping
- Test descend into directory
- Test go up from subdirectory
- Test go up from root (stays at root)
- Test hidden directories sorted last
- Test breadcrumb path updates on navigation
