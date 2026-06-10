# TICKET-22: Template Variables for App Commands

## Priority: Medium
## Dependencies: TICKET-08, TICKET-02
## Blocks: None

## Description
Add parameterized template variables to app configurations. Users can include `{{variable_name}}` placeholders in `command`, `working_dir`, and `env_vars` fields. Variables are auto-detected on field exit, and the user provides metadata (description, default) in the same create/edit form. At start time, apps with all defaults start immediately; apps with required (no-default) variables show an override modal. A separate keybinding (`S`) forces the modal for optional overrides.

## Syntax
- Mustache-style double braces: `{{variable_name}}`
- No escaping mechanism (literal `{{` not supported)
- Variables are deduplicated across all three fields (`command`, `working_dir`, `env_vars`)
- Same variable name = same value everywhere it appears

## Variable Metadata
Each detected variable stores:
- `name` — extracted from the `{{...}}` pattern
- `description` — human-readable label shown in prompts
- `default` — optional default value; null means required

## Database Changes
Add column to `apps` table:
```sql
ALTER TABLE apps ADD COLUMN template_vars TEXT DEFAULT '[]'
```

JSON format:
```json
[
  {"name": "port", "description": "Server port", "default": "3000"},
  {"name": "project_dir", "description": "Project directory", "default": null}
]
```

Migration uses existing PRAGMA table_info check pattern (same as `interval_seconds`).

## Detection Behavior
- Variables are parsed from `command`, `working_dir`, and `env_vars` on field exit (tab out)
- Regex: `\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}`
- Deduplicated across all three fields
- Orphaned variables (no longer referenced in any field) are auto-removed on save
- Renaming a variable = old removed + new created (no heuristic carry-over)

## Form UX (Create/Edit)
- When user tabs out of command/working_dir/env_vars, detected variables appear as dynamic fields below
- Each variable gets two sub-fields: Description (text), Default (text, can be empty)
- Focus stays in the current field during typing; user tabs into variable fields after
- On save, `template_vars` JSON is built from the detected variables + user-provided metadata

### Layout
```
┌───────────────────────────────────────────┐
│  New App                                  │
│                                           │
│  Name:        [my-api                ]    │
│  Directory:   [{{project_dir}}/api   ] ^B │
│  Command:     [cargo run -- --port {{port}}] │
│  Env vars:    [API_KEY={{api_key}}    ]    │
│  Auto-start:  [ ] yes                     │
│  Max runtime: [300s                  ]    │
│                                           │
│  ── Template Variables ──────────────     │
│  port                                     │
│    Description: [Server port         ]    │
│    Default:     [3000                ]    │
│  project_dir                              │
│    Description: [Project directory    ]    │
│    Default:     [                    ]    │
│  api_key                                  │
│    Description: [API key for service ]    │
│    Default:     [                    ]    │
│                                           │
│  [Save]  [Cancel]                         │
└───────────────────────────────────────────┘
```

## Start-Time Override Modal
- Shown as inline modal (reuses existing form infrastructure)
- Pre-filled with configured defaults from `template_vars`
- Override values are ephemeral (not persisted); config defaults are always source of truth

### Layout
```
┌───────────────────────────────────────┐
│  Start: my-api                        │
│                                       │
│  port (Server port)                   │
│    Value: [3000                   ]   │
│  project_dir (Project directory)      │
│    Value: [                       ]   │
│  api_key (API key for service)        │
│    Value: [                       ]   │
│                                       │
│  [Start]  [Cancel]                    │
└───────────────────────────────────────┘
```

## Keybindings (AppList mode)
| Key | Action | Behavior |
|-----|--------|----------|
| `s` | StartApp | Start immediately if all vars have defaults. Show modal only if required vars exist. |
| `S` | StartAppWithOptions | Always show override modal, pre-filled with defaults. |
| `r` | RestartApp | Reuse current run's variable values silently. |
| `R` | RestartAppWithOptions | Show override modal (works on running or stopped apps). Pre-fills with configured defaults. |

## Auto-Start with Missing Defaults
- App status set to "Pending Input" (distinct from Running/Stopped)
- Alert notification via existing alert system: "App '{name}' requires input before starting"
- User must manually trigger with `s` or `S` to provide values

## Substitution Logic
At process start:
1. Collect resolved values (defaults merged with any overrides from modal)
2. Replace all `{{variable_name}}` occurrences in `command`, `working_dir`, and `env_vars`
3. Pass resolved strings to the process spawner

## Acceptance Criteria
- [ ] DB migration: add `template_vars TEXT DEFAULT '[]'` to `apps` table
- [ ] Update `AppConfig` and `NewApp` models to include `template_vars`
- [ ] Template variable parser: extract `{{var}}` patterns from strings
- [ ] Auto-detection on field exit in app form (command, working_dir, env_vars)
- [ ] Dynamic variable metadata fields in create/edit form (description + default)
- [ ] Orphan removal: variables not in any field are removed on save
- [ ] Deduplication: same variable name across fields = single entry
- [ ] Start-time override modal component
- [ ] `s` starts immediately with defaults if all vars have defaults; shows modal if required vars exist
- [ ] `S` always shows override modal
- [ ] `r` restarts with current run's values
- [ ] `R` shows override modal (works on any app state)
- [ ] "Pending Input" status for auto-start apps with missing defaults
- [ ] Alert notification for apps that couldn't auto-start
- [ ] Substitution engine replaces `{{var}}` in command, working_dir, env_vars before spawn

## Files
- `src/db/mod.rs` (migration)
- `src/db/models.rs` (add `template_vars` field)
- `src/db/operations.rs` (read/write template_vars)
- `src/tui/form.rs` (dynamic variable fields, detection on field exit)
- `src/tui/input.rs` (add `S` and `R` keybindings, new actions)
- `src/tui/ui.rs` ("Pending Input" status rendering)
- `src/app.rs` (wire override modal, substitution before start)
- `src/process/mod.rs` (accept resolved values for spawn)
- `src/alerts.rs` (pending input alert)
- `src/template.rs` (new — parser + substitution engine)

## Tests
- Parser extracts variables from strings with mixed content
- Parser handles multiple variables in one string
- Parser deduplicates across multiple strings
- Parser ignores incomplete patterns (`{{`, `{{port`, `{port}}`)
- Substitution replaces all occurrences correctly
- Substitution with missing value returns error (not silent empty string)
- Orphan detection removes variables not in any field
- Form detects new variables on field exit
- Form removes variable fields when variable is deleted from text
- Override modal pre-fills with configured defaults
- Auto-start skipped for apps with required (no-default) variables
- "Pending Input" status set correctly
- `S` keybinding triggers modal regardless of defaults
- `R` keybinding works on both running and stopped apps
