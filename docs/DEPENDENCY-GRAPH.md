# Ticket Dependency Graph

## Overview

```
TICKET-01 (Project Scaffold)
├── TICKET-02 (DB Module)
│   ├── TICKET-04 (Process Module) [also needs TICKET-03]
│   │   ├── TICKET-07 (TUI Core) [also needs TICKET-05, TICKET-06]
│   │   │   ├── TICKET-08 (TUI Form) [also needs TICKET-03]
│   │   │   │   └── TICKET-09 (File Browser)
│   │   │   ├── TICKET-10 (Ghostty Attach)
│   │   │   └── TICKET-11 (Runtime Alerts) [also needs TICKET-06]
│   │   └── TICKET-10 (Ghostty Attach)
│   └── TICKET-12 (CLI + Completions + Uninstall)
│       └── TICKET-13 (Install Script)
├── TICKET-03 (Health Check)
│   ├── TICKET-04 (Process Module)
│   └── TICKET-08 (TUI Form)
├── TICKET-05 (VT Parser)
│   └── TICKET-07 (TUI Core)
└── TICKET-06 (Metrics Module)
    ├── TICKET-07 (TUI Core)
    └── TICKET-11 (Runtime Alerts)

TICKET-14 (Integration Testing) depends on all above
```

## Parallelization Strategy

### Phase 1 (Sequential — must be first)
- **TICKET-01**: Project Scaffold

### Phase 2 (Parallel — no interdependencies)
- **TICKET-02**: DB Module
- **TICKET-03**: Health Check
- **TICKET-05**: VT Parser
- **TICKET-06**: Metrics Module

### Phase 3 (Depends on Phase 2)
- **TICKET-04**: Process Module (needs TICKET-02, TICKET-03)
- **TICKET-12**: CLI + Completions (needs TICKET-02)

### Phase 4 (Depends on Phase 3)
- **TICKET-07**: TUI Core (needs TICKET-02, TICKET-04, TICKET-05, TICKET-06)

### Phase 5 (Parallel — all depend on TICKET-07)
- **TICKET-08**: TUI Form (needs TICKET-07, TICKET-03)
- **TICKET-09**: File Browser (needs TICKET-07)
- **TICKET-10**: Ghostty Attach (needs TICKET-04, TICKET-07)
- **TICKET-11**: Runtime Alerts (needs TICKET-04, TICKET-06, TICKET-07)
- **TICKET-13**: Install Script (needs TICKET-12)

### Phase 6 (Final)
- **TICKET-14**: Integration Testing (needs all)
