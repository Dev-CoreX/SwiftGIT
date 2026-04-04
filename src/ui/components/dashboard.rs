use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::ui::theme::*;
use crate::config::RecentProject;

pub const MENU_ITEMS: &[&str] = &[
    "  Open Folder",
    "  Clone Repo",
    "  Recent Projects",
];

pub fn render(
    f: &mut Frame,
    area: Rect,
    cursor: usize,
    username: Option<&str>,
    recent: &[RecentProject],
    status_msg: &str,
) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10), // top pad
            Constraint::Length(8),      // header
            Constraint::Length(2),      // tagline
            Constraint::Length(1),      // spacer
            Constraint::Length(7),      // menu
            Constraint::Length(1),      // spacer
            Constraint::Min(6),         // recent / status
            Constraint::Length(2),      // controls
        ])
        .split(area);

    // ── Username badge top-right ──────────────────────────────────────
    let top_bar = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(24)])
        .split(vertical[0]);

    if let Some(user) = username {
        let badge = Paragraph::new(format!("@{}", user))
            .alignment(Alignment::Right)
            .style(Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD));
        f.render_widget(badge, top_bar[1]);
    }

    // ── ASCII Header ─────────────────────────────────────────────────
    let header_cols = Layout::default()
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
    f.render_widget(header, header_cols[1]);

    // ── Tagline ───────────────────────────────────────────────────────
    let tagline = Paragraph::new("A minimal Git client. Fast, clean, terminal-native.")
        .alignment(Alignment::Center)
        .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC));
    f.render_widget(tagline, vertical[2]);

    // ── Main Menu ────────────────────────────────────────────────────
    let menu_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(vertical[4]);

    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == cursor {
                ListItem::new(Line::from(Span::styled(
                    format!("▶ {}", label.trim()),
                    Style::default()
                        .fg(BG_COLOR)
                        .bg(ACCENT_COLOR)
                        .add_modifier(Modifier::BOLD),
                )))
            } else {
                ListItem::new(Line::from(Span::styled(
                    format!("  {}", label.trim()),
                    Style::default().fg(FG_COLOR),
                )))
            }
        })
        .collect();

    let menu = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Menu ")
            .border_style(Style::default().fg(BORDER_COLOR))
            .style(Style::default().bg(BG_COLOR)),
    );
    f.render_widget(menu, menu_col[1]);

    // ── Recent / Status area ─────────────────────────────────────────
    let bottom_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(vertical[6]);

    if !status_msg.is_empty() {
        let status = Paragraph::new(status_msg)
            .alignment(Alignment::Center)
            .style(Style::default().fg(WARNING_COLOR));
        f.render_widget(status, bottom_col[1]);
    } else if !recent.is_empty() {
        let recent_items: Vec<ListItem> = recent
            .iter()
            .take(4)
            .map(|p| {
                ListItem::new(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        format!("{:<20}", p.name),
                        Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!(" {}", p.path),
                        Style::default().fg(BORDER_COLOR),
                    ),
                ]))
            })
            .collect();

        let recent_list = List::new(recent_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Recent Projects ")
                .border_style(Style::default().fg(BORDER_COLOR))
                .style(Style::default().bg(BG_COLOR)),
        );
        f.render_widget(recent_list, bottom_col[1]);
    }

    // ── Controls ─────────────────────────────────────────────────────
    let controls = Paragraph::new("↑↓ Navigate   Enter Select   Ctrl+W Settings   ? Help   q Quit")
        .alignment(Alignment::Center)
        .style(Style::default().fg(BORDER_COLOR));
    f.render_widget(controls, vertical[7]);
}
