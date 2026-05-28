# TICKET-05: VT Parser + Scrollback Buffer

## Priority: High
## Dependencies: TICKET-01
## Blocks: TICKET-07

## Description
Implement a VT (virtual terminal) parser using the `vte` crate that processes raw PTY output bytes into styled text lines suitable for rendering in ratatui.

## Acceptance Criteria
- [ ] Parse raw bytes through `vte` parser
- [ ] Convert ANSI color codes to ratatui `Style` attributes
- [ ] Maintain a ring buffer of parsed lines (configurable size, default 1000 lines)
- [ ] Support basic ANSI: colors (16 + 256 + RGB), bold, italic, underline, reset
- [ ] Handle cursor movement sequences (at minimum: newline, carriage return)
- [ ] Provide API to get last N lines for rendering
- [ ] Provide API to get lines at an offset (for scrolling)

## API

```rust
pub struct StyledChar {
    pub ch: char,
    pub style: ratatui::style::Style,
}

pub struct TerminalLine {
    pub chars: Vec<StyledChar>,
}

pub struct Scrollback {
    lines: VecDeque<TerminalLine>,
    max_lines: usize,
    parser: vte::Parser,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self;
    pub fn feed(&mut self, bytes: &[u8]);
    pub fn get_lines(&self, offset: usize, count: usize) -> &[TerminalLine];
    pub fn total_lines(&self) -> usize;
}
```

## Files
- `src/vt/mod.rs`

## Tests
- Test plain text parsing
- Test ANSI color parsing (foreground, background)
- Test bold/italic/underline attributes
- Test color reset sequences
- Test ring buffer overflow (oldest lines dropped)
- Test newline handling
- Test carriage return (overwrites current line)
- Test get_lines with offset and count
- Test 256-color and RGB color codes
