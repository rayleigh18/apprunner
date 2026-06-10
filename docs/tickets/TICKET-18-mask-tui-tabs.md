# TICKET-18: TUI Tab Navigation (Apps / Masks)

## Priority: High
## Dependencies: TICKET-16
## Blocks: TICKET-19, TICKET-20, TICKET-21

## Description
Add tab-based navigation to the TUI, switching between "Apps" view (existing) and "Masks" view (new). The status bar shows tab indicators. Pressing `1` switches to Apps, `2` switches to Masks.

## Acceptance Criteria
- [ ] New `FocusMode` variants: `MaskList`, `MaskLog`
- [ ] New `Action` variants: `SwitchToApps`, `SwitchToMasks`, `ActivateMask`, `DeactivateMask`, `NewMask`, `EditMask`, `DeleteMask`, `FocusMaskLog`
- [ ] `1` key switches to Apps tab (from any list view)
- [ ] `2` key switches to Masks tab (from any list view)
- [ ] Status bar renders tab indicator: `[1:Apps] 2:Masks` or `1:Apps [2:Masks]`
- [ ] When on Masks tab, list shows mask entries (name, port, target_url, status)
- [ ] MaskList keybindings mirror AppList: `j`/`k` navigate, `s` activate, `x` deactivate, `n` new, `e` edit, `d` delete, `Enter` focus log
- [ ] `Esc` from MaskLog returns to MaskList
- [ ] App state tracks which tab is active
- [ ] Existing app list functionality unchanged when on Apps tab

## Files
- `src/tui/input.rs` (new FocusMode variants, new Actions, key handlers)
- `src/tui/ui.rs` (tab indicator rendering, mask list rendering, mask log rendering)
- `src/app.rs` (active tab state, mask list state)

## Tests
- Tab switching keybindings work correctly
- MaskList keybindings map to correct actions
- MaskLog scroll bindings work
- Existing AppList tests still pass
