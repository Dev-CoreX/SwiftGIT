//! Task 7: Inline file editor rendered in the right panel.
//! Terminal-only. No external processes. Supports basic editing + save.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
                Line::from(vec![
                    line_num,
                    Span::styled(line_text.to_string(), Style::default().fg(FG_COLOR)),
                ])
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
        " Ln {}/{}  Col {}    Ctrl+S Save   Ctrl+X Close editor   ↑↓←→ Move",
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
