//! Task 7: Inline file editor rendered in the right panel.
//! Terminal-only. No external processes. Supports basic editing + save.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::*;

pub struct EditorState<'a> {
    pub lines:        &'a [String],   // file content split into lines
    pub cursor_line:  usize,          // line index
    pub cursor_col:   usize,          // column index (chars)
    pub scroll_top:   usize,          // first visible line
    pub file_path:    &'a str,
    pub modified:     bool,
    pub frame_count:  u64,
}

pub fn render(f: &mut Frame, area: Rect, s: &EditorState) {
    f.render_widget(Clear, area); // Ensure no text from underneath is visible

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),    // editor body
            Constraint::Length(1), // status line
        ])
        .split(area);

    let body_h = chunks[0].height.saturating_sub(2) as usize;

    // Build visible lines with line numbers + cursor
    let visible: Vec<Line> = s.lines
        .iter()
        .enumerate()
        .skip(s.scroll_top)
        .take(body_h)
        .map(|(li, line_text)| {
            let is_cursor_line = li == s.cursor_line;
            let line_num_style = if is_cursor_line {
                Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(BORDER_COLOR)
            };
            let line_num = Span::styled(format!(" {:>4} │ ", li + 1), line_num_style);

            if is_cursor_line {
                // Render cursor position as a block highlight
                let chars: Vec<char> = line_text.chars().collect();
                let col = s.cursor_col.min(chars.len());

                let before: String = chars[..col].iter().collect();
                let cursor_ch = if col < chars.len() {
                    chars[col].to_string()
                } else {
                    " ".to_string()
                };
                let after: String = if col + 1 <= chars.len() {
                    chars[col + 1..].iter().collect()
                } else {
                    String::new()
                };

                // Blink: every 5 frames toggle cursor visibility
                let cursor_visible = (s.frame_count / 5) % 2 == 0;
                let cursor_style = if cursor_visible {
                    Style::default().fg(BG_COLOR).bg(FG_COLOR)
                } else {
                    Style::default().fg(FG_COLOR).bg(HIGHLIGHT_BG)
                };

                Line::from(vec![
                    line_num,
                    Span::styled(before, Style::default().fg(FG_COLOR).bg(HIGHLIGHT_BG)),
                    Span::styled(cursor_ch, cursor_style),
                    Span::styled(after, Style::default().fg(FG_COLOR).bg(HIGHLIGHT_BG)),
                ])
            } else {
                let highlighted = highlight_line(line_text);
                let mut spans = vec![line_num];
                spans.extend(highlighted);
                Line::from(spans)
            }
        })
        .collect();

    let modified_marker = if s.modified { " ● " } else { "   " };
    let title = format!(" EDIT {}{} ", modified_marker, s.file_path);
    let border_color = if s.modified { WARNING_COLOR } else { ACCENT_COLOR };

    let editor_widget = Paragraph::new(visible)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(BG_COLOR)),
        );
    f.render_widget(editor_widget, chunks[0]);

    // Status line: position + hints
    let status_text = format!(
        " Ln {}/{}  Col {}    Ctrl+S Save   Ctrl+Q/X Close editor   ↑↓←→ Move",
        s.cursor_line + 1,
        s.lines.len(),
        s.cursor_col + 1,
    );
    f.render_widget(
        Paragraph::new(status_text)
            .style(Style::default().fg(BORDER_COLOR)),
        chunks[1],
    );
}

fn highlight_line(line: &str) -> Vec<Span<'static>> {
    if line.trim().starts_with("//") || line.trim().starts_with("///") || line.trim().starts_with("//!") {
        return vec![Span::styled(line.to_string(), Style::default().fg(SYN_COMMENT))];
    }

    let mut spans = Vec::new();
    let words: Vec<&str> = line.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_').collect();

    for word in words {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let style = if is_keyword(trimmed) {
            Style::default().fg(SYN_KEYWORD)
        } else if is_type(trimmed) {
            Style::default().fg(SYN_TYPE)
        } else if is_function_call(word) {
            Style::default().fg(SYN_FUNCTION)
        } else if trimmed.chars().all(|c| c.is_numeric()) {
            Style::default().fg(SYN_NUMBER)
        } else if word.starts_with('"') || word.ends_with('"') {
            Style::default().fg(SYN_STRING)
        } else if is_operator(word) {
            Style::default().fg(SYN_OPERATOR)
        } else {
            Style::default().fg(FG_COLOR)
        };
        spans.push(Span::styled(word.to_string(), style));
    }
    spans
}

fn is_keyword(w: &str) -> bool {
    matches!(w, "pub" | "fn" | "use" | "let" | "match" | "mut" | "if" | "else" | "impl" | "struct" | "enum" | "trait" | "type" | "for" | "in" | "while" | "loop" | "return" | "break" | "continue" | "as" | "async" | "await" | "mod" | "crate" | "self" | "Self" | "where")
}

fn is_type(w: &str) -> bool {
    matches!(w, "String" | "Result" | "Option" | "Vec" | "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" | "f32" | "f64" | "bool" | "char" | "str" | "Path" | "PathBuf" | "Arc" | "Mutex")
}

fn is_function_call(w: &str) -> bool {
    w.ends_with('(')
}

fn is_operator(w: &str) -> bool {
    w.trim().chars().all(|c| matches!(c, '=' | '>' | '<' | '!' | '&' | '|' | '+' | '-' | '*' | '/' | '%' | '^' | ':'))
}
