use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect, frame_count: u64, status_msg: &str) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Length(8),  // logo
            Constraint::Length(2),  // status
            Constraint::Length(3),  // spinner
            Constraint::Min(0),
        ])
        .split(area);

    let header_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical[1]);

    let header_lines = vec![
        Line::from(Span::styled(
            " ███████╗██╗    ██╗██╗███████╗████████╗ ██████╗ ██╗████████╗",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ██╔════╝██║    ██║██║██╔════╝╚══██╔══╝██╔════╝ ██║╚══██╔══╝",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ███████╗██║ █╗ ██║██║█████╗     ██║   ██║  ███╗██║   ██║   ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ╚════██║██║███╗██║██║██╔══╝     ██║   ██║   ██║██║   ██║   ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ███████║╚███╔███╔╝██║██║        ██║   ╚██████╔╝██║   ██║   ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " ╚══════╝ ╚══╝╚══╝ ╚═╝╚═╝        ╚═╝    ╚═════╝ ╚═╝   ╚═╝   ",
            Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
        )),
    ];

    let header = Paragraph::new(header_lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(header, header_area[1]);

    let msg = if status_msg.is_empty() { "Working..." } else { status_msg };
    let status = Paragraph::new(Span::styled(
        msg,
        Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC),
    ))
    .alignment(Alignment::Center)
    .style(Style::default().bg(BG_COLOR));
    f.render_widget(status, vertical[2]);

    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner_char = spinner_frames[(frame_count / 2) as usize % spinner_frames.len()];

    let spinner_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(10), Constraint::Min(0)])
        .split(vertical[3]);

    let spinner = Paragraph::new(Span::styled(
        spinner_char,
        Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::NONE).style(Style::default().bg(BG_COLOR)));
    f.render_widget(spinner, spinner_col[1]);
}
