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

TICKET-15 (Add Proxy Dependencies)
├── TICKET-16 (Mask DB Model)
│   ├── TICKET-17 (Proxy Engine Core) [also needs TICKET-15]
│   │   └── TICKET-20 (Mask Lifecycle Management) [also needs TICKET-18]
│   ├── TICKET-18 (TUI Tab Navigation)
│   │   ├── TICKET-19 (Mask Form) [also needs TICKET-16, TICKET-17]
│   │   └── TICKET-20 (Mask Lifecycle Management)
│   └── TICKET-19 (Mask Form)
└── TICKET-17 (Proxy Engine Core)

TICKET-21 (Mask Integration Testing) depends on TICKET-19, TICKET-20

TICKET-22 (Template Variables) depends on TICKET-08, TICKET-02
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

---

## API Mask Feature (TICKET-15 through TICKET-21)

### Mask Phase 1 (No dependencies on existing tickets)
- **TICKET-15**: Add Proxy Dependencies

### Mask Phase 2 (Parallel — both depend on TICKET-15)
- **TICKET-16**: Mask DB Model (needs TICKET-15)
- **TICKET-17**: Proxy Engine Core (needs TICKET-15, TICKET-16)

### Mask Phase 3 (Parallel — depend on Phase 2)
- **TICKET-18**: TUI Tab Navigation (needs TICKET-16)
- **TICKET-17**: Proxy Engine Core (if not started in Phase 2)

### Mask Phase 4 (Depends on Phase 3)
- **TICKET-19**: Mask Form (needs TICKET-16, TICKET-17, TICKET-18)
- **TICKET-20**: Mask Lifecycle Management (needs TICKET-17, TICKET-18)

### Mask Phase 5 (Final)
- **TICKET-21**: Mask Integration Testing (needs TICKET-19, TICKET-20)

---

## Template Variables Feature (TICKET-22)

### Can start after Phase 5 of the main track (needs TICKET-08, TICKET-02)
- **TICKET-22**: Template Variables (needs TICKET-08, TICKET-02)
