//! SSH setup screen — shown when no SSH key exists or key not yet on GitHub.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::ui::theme::*;
use crate::ui::spinner_char;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshSetupStep {
    /// Checking for existing keys
    Detecting,
    /// No key found — offering to generate
    NeedGenerate,
    /// Key being generated
    Generating,
    /// Key exists, show pubkey and wait for user to add to GitHub
    ShowPubkey { pubkey: String, auto_added: bool },
    /// Testing connectivity to GitHub
    Testing,
    /// Done
    Connected { username: String },
    /// Error state
    Error(String),
}

pub struct SshSetupState<'a> {
    pub step:        &'a SshSetupStep,
    pub frame_count: u64,
}

pub fn render(f: &mut Frame, area: Rect, s: &SshSetupState) {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(8),
            Constraint::Length(3),   // title
            Constraint::Length(2),   // subtitle
            Constraint::Length(1),
            Constraint::Min(8),      // content
            Constraint::Length(2),   // hint
        ])
        .split(area);

    let col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
        .split(vert[1]);

    // Title
    f.render_widget(
        Paragraph::new("🔐  SSH Setup")
            .alignment(Alignment::Center)
            .style(Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)),
        col[1],
    );

    let col2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
        .split(vert[2]);

    f.render_widget(
        Paragraph::new("SwiftGit uses SSH for all Git operations — no passwords, ever.")
            .alignment(Alignment::Center)
            .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::ITALIC)),
        col2[1],
    );

    let content_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
        .split(vert[4]);

    match s.step {
        SshSetupStep::Detecting => {
            let spin = spinner_char(s.frame_count);
            f.render_widget(
                Paragraph::new(format!("\n  {} Checking for SSH keys…", spin))
                    .style(Style::default().fg(FG_COLOR)),
                content_col[1],
            );
        }

        SshSetupStep::NeedGenerate => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No SSH key found on this machine.",
                    Style::default().fg(WARNING_COLOR).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  SwiftGit will generate an ed25519 key at:",
                    Style::default().fg(FG_COLOR),
                )),
                Line::from(Span::styled(
                    "  ~/.ssh/id_ed25519",
                    Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to generate key, or Esc to skip (HTTPS mode)",
                    Style::default().fg(BORDER_COLOR),
                )),
            ];
            f.render_widget(
                Paragraph::new(lines).block(
                    Block::default().borders(Borders::ALL)
                        .title(" Generate SSH Key ")
                        .border_style(Style::default().fg(ACCENT_COLOR))
                        .style(Style::default().bg(BG_COLOR))
                ),
                content_col[1],
            );
        }

        SshSetupStep::Generating => {
            let spin = spinner_char(s.frame_count);
            f.render_widget(
                Paragraph::new(format!("\n  {} Generating ed25519 SSH key…", spin))
                    .style(Style::default().fg(WARNING_COLOR)),
                content_col[1],
            );
        }

        SshSetupStep::ShowPubkey { pubkey, auto_added } => {
            let auto_msg = if *auto_added {
                "  ✅ Key automatically added to your GitHub account!"
            } else {
                "  Add this key to GitHub → github.com/settings/keys → New SSH key"
            };

            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(auto_msg,
                    Style::default().fg(if *auto_added { SUCCESS_COLOR } else { WARNING_COLOR })
                        .add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled("  Your public key (copy this):",
                    Style::default().fg(BORDER_COLOR))),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", pubkey),
                    Style::default().fg(ACCENT_COLOR),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  After adding the key, press Enter to test connection.",
                    Style::default().fg(FG_COLOR),
                )),
            ];

            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL)
                        .title(" Public SSH Key ")
                        .border_style(Style::default().fg(ACCENT_COLOR))
                        .style(Style::default().bg(BG_COLOR)))
                    .wrap(Wrap { trim: false }),
                content_col[1],
            );
        }

        SshSetupStep::Testing => {
            let spin = spinner_char(s.frame_count);
            f.render_widget(
                Paragraph::new(format!("\n  {} Testing SSH connection to GitHub…", spin))
                    .style(Style::default().fg(WARNING_COLOR)),
                content_col[1],
            );
        }

        SshSetupStep::Connected { username } => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  ✅ SSH connected!  Hi, {}!", username),
                    Style::default().fg(SUCCESS_COLOR).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  All Git operations will now use SSH automatically.",
                    Style::default().fg(FG_COLOR),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to continue →",
                    Style::default().fg(ACCENT_COLOR),
                )),
            ];
            f.render_widget(Paragraph::new(lines), content_col[1]);
        }

        SshSetupStep::Error(msg) => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ❌ Error",
                    Style::default().fg(ERROR_COLOR).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", msg),
                    Style::default().fg(FG_COLOR),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to retry, Esc to skip",
                    Style::default().fg(BORDER_COLOR),
                )),
            ];
            f.render_widget(Paragraph::new(lines), content_col[1]);
        }
    }

    // Hint bar
    let hint = match s.step {
        SshSetupStep::Connected { .. } => "Enter Continue",
        SshSetupStep::Detecting | SshSetupStep::Generating | SshSetupStep::Testing => "",
        _ => "Enter Confirm   Esc Skip SSH (use HTTPS instead)",
    };
    f.render_widget(
        Paragraph::new(hint).alignment(Alignment::Center)
            .style(Style::default().fg(BORDER_COLOR)),
        vert[5],
    );
}
