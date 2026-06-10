# TICKET-19: Mask Form with Dynamic Header Table

## Priority: High
## Dependencies: TICKET-16, TICKET-17, TICKET-18
## Blocks: TICKET-21

## Description
Implement the TUI form for creating and editing API masks. Includes standard text fields (name, target_url, port) and a dynamic key-value table for headers with vim-style bindings.

## Form Fields
1. `name` — text input
2. `target_url` — text input (e.g. `https://api.openai.com`)
3. `port` — numeric text input
4. `auto_start` — toggle (boolean)
5. `headers` — dynamic key-value table

## Header Table Vim Bindings
- `j` / `k` — move between rows
- `h` / `l` — move between key and value columns
- `a` — add new row at bottom, enter edit mode on key field
- `dd` — delete selected row
- `Enter` — enter edit mode on selected cell
- `Esc` — exit edit mode → navigation mode → exit table back to form

## Acceptance Criteria
- [ ] `src/tui/mask_form.rs` — mask form component
- [ ] Text fields for name, target_url, port with validation
- [ ] Toggle for auto_start
- [ ] Header table widget with row selection highlighting
- [ ] Header table supports: navigate rows (j/k), navigate columns (h/l), add row (a), delete row (dd), edit cell (Enter), exit (Esc)
- [ ] Form submits to DB via `insert_mask` or `update_mask`
- [ ] Form pre-fills values when editing existing mask
- [ ] Port validation: must be a number 1-65535
- [ ] Header values masked in display (show `••••••••` instead of actual value)
- [ ] Form accessible from MaskList via `n` (new) and `e` (edit)

## Files
- `src/tui/mask_form.rs` (new file)
- `src/tui/mod.rs` (add module)
- `src/tui/input.rs` (form key handling for mask form context)
- `src/app.rs` (wire form open/submit/cancel)

## Tests
- Header table add/delete operations
- Port validation rejects non-numeric and out-of-range values
- Form serializes headers to JSON correctly
- Edit mode pre-populates existing mask data
