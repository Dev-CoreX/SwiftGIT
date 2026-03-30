use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::*;

pub fn render(
    f: &mut Frame,
    area: Rect,
    token_input: &str,
    _cursor_pos: usize,
    status_msg: &str,
    cursor_visible: bool,
    is_validating: bool,
) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(3),  // title
            Constraint::Length(2),  // desc
            Constraint::Length(2),  // spacer
            Constraint::Length(3),  // input
            Constraint::Length(2),  // spacer
            Constraint::Length(3),  // hint
            Constraint::Min(0),
        ])
        .split(area);

    let _col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(area);

    // Title
    let title = Paragraph::new("🔐  GitHub Authentication")
        .alignment(Alignment::Center)
        .style(Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD));
    f.render_widget(title, vertical[1]);

    // Description
    let desc = Paragraph::new("Enter your GitHub Personal Access Token (PAT) to get started.")
        .alignment(Alignment::Center)
        .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC));
    f.render_widget(desc, vertical[2]);

    // Input box
    let input_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(vertical[4]);

    let display_len = token_input.chars().count();
    let masked: String = if display_len > 0 {
        let visible_start = token_input.chars().take(4).collect::<String>();
        let dots = "•".repeat(display_len.saturating_sub(4).min(20));
        format!("{}{}", visible_start, dots)
    } else {
        String::new()
    };

    let cursor_char = if cursor_visible && !is_validating { "█" } else { "" };

    let input_text = if is_validating {
        " Validating token...".to_string()
    } else if masked.is_empty() {
        format!(" {}", cursor_char)
    } else {
        format!(" {}{}", masked, cursor_char)
    };

    let input_color = if is_validating { WARNING_COLOR } else { ACCENT_COLOR };

    let input = Paragraph::new(input_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Token ")
                .border_style(Style::default().fg(input_color))
                .style(Style::default().bg(BG_COLOR)),
        )
        .style(Style::default().fg(FG_COLOR));
    f.render_widget(input, input_col[1]);

    // Status / error
    if !status_msg.is_empty() {
        let status_col = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[6]);

        let color = if status_msg.starts_with("✅") {
            SUCCESS_COLOR
        } else if status_msg.starts_with("❌") {
            ERROR_COLOR
        } else {
            WARNING_COLOR
        };

        let status = Paragraph::new(status_msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD));
        f.render_widget(status, status_col[1]);
    } else {
        let hint_col = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical[6]);

        let hint_lines = vec![
            Line::from(Span::styled(
                "Generate at: github.com/settings/tokens",
                Style::default().fg(BORDER_COLOR),
            )),
            Line::from(Span::styled(
                "Press Enter to confirm  •  Esc to skip (public repos only)",
                Style::default().fg(BORDER_COLOR),
            )),
        ];
        let hint = Paragraph::new(hint_lines).alignment(Alignment::Center);
        f.render_widget(hint, hint_col[1]);
    }
}
