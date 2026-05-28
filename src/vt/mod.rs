//! VT parser and scrollback ring buffer for terminal output rendering.

use ratatui::style::{Color, Modifier, Style};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct StyledChar {
    pub ch: char,
    pub style: Style,
}

#[derive(Debug, Clone, Default)]
pub struct TerminalLine {
    pub chars: Vec<StyledChar>,
}

impl TerminalLine {
    pub fn new() -> Self {
        Self { chars: Vec::new() }
    }

    /// For testing - extracts just the text
    pub fn plain_text(&self) -> String {
        self.chars.iter().map(|sc| sc.ch).collect()
    }
}

/// Actions produced by the performer to be applied to scrollback state.
#[derive(Debug)]
enum TerminalAction {
    Print(char, Style),
    Newline,
    CarriageReturn,
    Tab,
}

/// Implements `vte::Perform` and accumulates actions with current style state.
struct TerminalPerformer {
    actions: Vec<TerminalAction>,
    current_style: Style,
}

impl TerminalPerformer {
    fn new() -> Self {
        Self {
            actions: Vec::new(),
            current_style: Style::default(),
        }
    }

    /// Map SGR parameter value (30-37) to a ratatui Color.
    fn standard_color(n: u16) -> Color {
        match n {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::Gray,
            _ => Color::Reset,
        }
    }

    /// Map SGR parameter value (90-97) to bright ratatui Color.
    fn bright_color(n: u16) -> Color {
        match n {
            0 => Color::DarkGray,
            1 => Color::LightRed,
            2 => Color::LightGreen,
            3 => Color::LightYellow,
            4 => Color::LightBlue,
            5 => Color::LightMagenta,
            6 => Color::LightCyan,
            7 => Color::White,
            _ => Color::Reset,
        }
    }

    /// Process SGR parameters from a CSI sequence.
    fn handle_sgr(&mut self, params: &vte::Params) {
        let mut iter = params.iter();

        while let Some(subparams) = iter.next() {
            // Each param group is a slice of u16 (subparams separated by ':')
            // For standard SGR, we only look at the first value per semicolon-separated group.
            // But for 38;5;N and 38;2;R;G;B we need to consume subsequent groups.
            let param = subparams[0];

            match param {
                0 => self.current_style = Style::default(),
                1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
                3 => self.current_style = self.current_style.add_modifier(Modifier::ITALIC),
                4 => self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED),
                22 => self.current_style = self.current_style.remove_modifier(Modifier::BOLD),
                23 => self.current_style = self.current_style.remove_modifier(Modifier::ITALIC),
                24 => self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED),
                30..=37 => {
                    self.current_style = self.current_style.fg(Self::standard_color(param - 30));
                }
                38 => {
                    // Extended foreground color
                    // Check if subparams contains the color spec (colon-separated: 38:5:N or 38:2:R:G:B)
                    if subparams.len() >= 3 && subparams[1] == 5 {
                        let idx = subparams[2];
                        self.current_style = self.current_style.fg(Color::Indexed(idx as u8));
                    } else if subparams.len() >= 5 && subparams[1] == 2 {
                        let r = subparams[2] as u8;
                        let g = subparams[3] as u8;
                        let b = subparams[4] as u8;
                        self.current_style = self.current_style.fg(Color::Rgb(r, g, b));
                    } else {
                        // Semicolon-separated: 38;5;N or 38;2;R;G;B
                        if let Some(mode_params) = iter.next() {
                            match mode_params[0] {
                                5 => {
                                    if let Some(color_params) = iter.next() {
                                        let idx = color_params[0];
                                        self.current_style =
                                            self.current_style.fg(Color::Indexed(idx as u8));
                                    }
                                }
                                2 => {
                                    let r_param = iter.next();
                                    let g_param = iter.next();
                                    let b_param = iter.next();
                                    if let (Some(r), Some(g), Some(b)) = (r_param, g_param, b_param)
                                    {
                                        self.current_style = self
                                            .current_style
                                            .fg(Color::Rgb(r[0] as u8, g[0] as u8, b[0] as u8));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                39 => {
                    self.current_style = self.current_style.fg(Color::Reset);
                }
                40..=47 => {
                    self.current_style = self.current_style.bg(Self::standard_color(param - 40));
                }
                48 => {
                    // Extended background color
                    if subparams.len() >= 3 && subparams[1] == 5 {
                        let idx = subparams[2];
                        self.current_style = self.current_style.bg(Color::Indexed(idx as u8));
                    } else if subparams.len() >= 5 && subparams[1] == 2 {
                        let r = subparams[2] as u8;
                        let g = subparams[3] as u8;
                        let b = subparams[4] as u8;
                        self.current_style = self.current_style.bg(Color::Rgb(r, g, b));
                    } else {
                        // Semicolon-separated
                        if let Some(mode_params) = iter.next() {
                            match mode_params[0] {
                                5 => {
                                    if let Some(color_params) = iter.next() {
                                        let idx = color_params[0];
                                        self.current_style =
                                            self.current_style.bg(Color::Indexed(idx as u8));
                                    }
                                }
                                2 => {
                                    let r_param = iter.next();
                                    let g_param = iter.next();
                                    let b_param = iter.next();
                                    if let (Some(r), Some(g), Some(b)) = (r_param, g_param, b_param)
                                    {
                                        self.current_style = self
                                            .current_style
                                            .bg(Color::Rgb(r[0] as u8, g[0] as u8, b[0] as u8));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                49 => {
                    self.current_style = self.current_style.bg(Color::Reset);
                }
                90..=97 => {
                    self.current_style = self.current_style.fg(Self::bright_color(param - 90));
                }
                _ => {} // Ignore unhandled SGR params
            }
        }
    }
}

impl vte::Perform for TerminalPerformer {
    fn print(&mut self, c: char) {
        self.actions
            .push(TerminalAction::Print(c, self.current_style));
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0A => self.actions.push(TerminalAction::Newline),
            0x0D => self.actions.push(TerminalAction::CarriageReturn),
            0x09 => self.actions.push(TerminalAction::Tab),
            _ => {} // Ignore other control chars
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action == 'm' {
            self.handle_sgr(params);
        }
        // Ignore non-SGR CSI sequences for now
    }
}

pub struct Scrollback {
    lines: VecDeque<TerminalLine>,
    current_line: TerminalLine,
    max_lines: usize,
    cursor_col: usize,
    parser: vte::Parser,
    performer: TerminalPerformer,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            current_line: TerminalLine::new(),
            max_lines,
            cursor_col: 0,
            parser: vte::Parser::new(),
            performer: TerminalPerformer::new(),
        }
    }

    /// Feed raw bytes from PTY into the parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.parser.advance(&mut self.performer, byte);
        }

        // Drain and apply accumulated actions
        let actions: Vec<TerminalAction> = self.performer.actions.drain(..).collect();
        for action in actions {
            match action {
                TerminalAction::Print(ch, style) => {
                    let sc = StyledChar { ch, style };
                    if self.cursor_col < self.current_line.chars.len() {
                        self.current_line.chars[self.cursor_col] = sc;
                    } else {
                        // Fill gap with spaces if needed
                        while self.current_line.chars.len() < self.cursor_col {
                            self.current_line.chars.push(StyledChar {
                                ch: ' ',
                                style: Style::default(),
                            });
                        }
                        self.current_line.chars.push(sc);
                    }
                    self.cursor_col += 1;
                }
                TerminalAction::Newline => {
                    let finished_line = std::mem::take(&mut self.current_line);
                    self.lines.push_back(finished_line);
                    if self.lines.len() > self.max_lines {
                        self.lines.pop_front();
                    }
                    self.cursor_col = 0;
                }
                TerminalAction::CarriageReturn => {
                    self.cursor_col = 0;
                }
                TerminalAction::Tab => {
                    let next_tab_stop = (self.cursor_col / 8 + 1) * 8;
                    let spaces_needed = next_tab_stop - self.cursor_col;
                    for _ in 0..spaces_needed {
                        let sc = StyledChar {
                            ch: ' ',
                            style: Style::default(),
                        };
                        if self.cursor_col < self.current_line.chars.len() {
                            self.current_line.chars[self.cursor_col] = sc;
                        } else {
                            self.current_line.chars.push(sc);
                        }
                        self.cursor_col += 1;
                    }
                }
            }
        }
    }

    /// Get a slice of lines for rendering (offset from the top, count lines).
    pub fn get_lines(&self, offset: usize, count: usize) -> Vec<&TerminalLine> {
        self.lines.iter().skip(offset).take(count).collect()
    }

    /// Total number of completed lines.
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Get the current (incomplete) line.
    pub fn current_line(&self) -> &TerminalLine {
        &self.current_line
    }

    /// Clear the scrollback buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_line = TerminalLine::new();
        self.cursor_col = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_parsing() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"hello world");
        assert_eq!(sb.current_line().plain_text(), "hello world");
    }

    #[test]
    fn test_newline_creates_new_line() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"line one\nline two");
        assert_eq!(sb.total_lines(), 1);
        assert_eq!(sb.lines[0].plain_text(), "line one");
        assert_eq!(sb.current_line().plain_text(), "line two");
    }

    #[test]
    fn test_carriage_return_and_overwrite() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"hello\rworld");
        assert_eq!(sb.current_line().plain_text(), "world");
    }

    #[test]
    fn test_ansi_color_red_foreground() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[31mhello");
        let line = sb.current_line();
        assert_eq!(line.plain_text(), "hello");
        for sc in &line.chars {
            assert_eq!(sc.style.fg, Some(Color::Red));
        }
    }

    #[test]
    fn test_bold() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[1mbold text");
        let line = sb.current_line();
        assert_eq!(line.plain_text(), "bold text");
        for sc in &line.chars {
            assert!(sc.style.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn test_reset() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[1;31mcolored\x1b[0m normal");
        let line = sb.current_line();
        assert_eq!(line.plain_text(), "colored normal");

        // First 7 chars should be bold+red
        for sc in &line.chars[..7] {
            assert_eq!(sc.style.fg, Some(Color::Red));
            assert!(sc.style.add_modifier.contains(Modifier::BOLD));
        }
        // ' normal' (chars 7..) should be default style
        for sc in &line.chars[7..] {
            assert_eq!(sc.style, Style::default());
        }
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let max = 5;
        let mut sb = Scrollback::new(max);
        // Feed 7 lines (6 newlines + content creates 6 completed lines)
        sb.feed(b"line1\nline2\nline3\nline4\nline5\nline6\nline7");
        assert_eq!(sb.total_lines(), max);
        // Oldest lines should be dropped
        assert_eq!(sb.lines[0].plain_text(), "line2");
        assert_eq!(sb.lines[4].plain_text(), "line6");
        assert_eq!(sb.current_line().plain_text(), "line7");
    }

    #[test]
    fn test_get_lines_with_offset_and_count() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"a\nb\nc\nd\ne\n");
        assert_eq!(sb.total_lines(), 5);

        let slice = sb.get_lines(1, 2);
        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].plain_text(), "b");
        assert_eq!(slice[1].plain_text(), "c");
    }

    #[test]
    fn test_total_lines_count() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"a\nb\nc\n");
        assert_eq!(sb.total_lines(), 3);
    }

    #[test]
    fn test_256_color() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[38;5;196mred");
        let line = sb.current_line();
        assert_eq!(line.plain_text(), "red");
        for sc in &line.chars {
            assert_eq!(sc.style.fg, Some(Color::Indexed(196)));
        }
    }

    #[test]
    fn test_rgb_color() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[38;2;255;128;0morange");
        let line = sb.current_line();
        assert_eq!(line.plain_text(), "orange");
        for sc in &line.chars {
            assert_eq!(sc.style.fg, Some(Color::Rgb(255, 128, 0)));
        }
    }

    #[test]
    fn test_tab_stops() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"a\tb");
        let text = sb.current_line().plain_text();
        // 'a' at position 0, then tab to position 8, then 'b' at position 8
        assert_eq!(text.len(), 9); // 'a' + 7 spaces + 'b'
        assert_eq!(&text[0..1], "a");
        assert_eq!(&text[1..8], "       ");
        assert_eq!(&text[8..9], "b");
    }

    #[test]
    fn test_bright_foreground_colors() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[91mhi");
        let line = sb.current_line();
        for sc in &line.chars {
            assert_eq!(sc.style.fg, Some(Color::LightRed));
        }
    }

    #[test]
    fn test_background_color() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"\x1b[42mgreen bg");
        let line = sb.current_line();
        for sc in &line.chars {
            assert_eq!(sc.style.bg, Some(Color::Green));
        }
    }

    #[test]
    fn test_clear() {
        let mut sb = Scrollback::new(1000);
        sb.feed(b"some\ntext\nhere");
        sb.clear();
        assert_eq!(sb.total_lines(), 0);
        assert_eq!(sb.current_line().plain_text(), "");
    }
}
