# TICKET-08: TUI Form (New/Edit App)

## Priority: Medium
## Dependencies: TICKET-07, TICKET-03
## Blocks: TICKET-09

## Description
Implement the inline form for creating and editing apps within the TUI. Includes field navigation, input handling, inline validation via health checks, and save/cancel actions.

## Acceptance Criteria
- [ ] Form overlay for creating a new app
- [ ] Form overlay for editing an existing app (pre-filled)
- [ ] Fields: name, working_dir, command, env_vars, auto_start, max_runtime_secs
- [ ] Tab/Shift+Tab to navigate between fields
- [ ] Inline validation errors shown below fields
- [ ] Health check runs on Save (validates dir, command, env vars)
- [ ] Ctrl+b on working_dir field opens file browser (TICKET-09)
- [ ] Esc cancels and returns to app list
- [ ] After save: app appears in sidebar list

## Layout

```
┌─────────────────────────────────────┐
│  New App                            │
│                                     │
│  Name:        [my-api          ]    │
│  Directory:   [~/work/my-api   ] ^B │
│  Command:     [cargo run       ]    │
│  Env vars:    [PORT=3000       ]    │
│  Auto-start:  [ ] yes               │
│  Max runtime: [300s         ]       │
│                                     │
│  ⚠ Directory does not exist         │
│                                     │
│  [Save]  [Cancel]                   │
└─────────────────────────────────────┘
```

## Files
- `src/tui/form.rs`

## Tests
- Test form field navigation (Tab cycles through fields)
- Test text input handling (character insertion, deletion, cursor movement)
- Test validation triggers on save attempt
- Test validation error display
- Test pre-fill on edit mode
- Test env vars parsing (KEY=VALUE format to JSON)
