//! Settings dialog — overlay for editing ~/.swiftgit/config.json
//! Triggered by Ctrl+W (Windows-key equivalent in terminals).
//! Fields: Display Name, GitHub Username, Personal Access Token (masked).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme::*;

// Which field is currently focused
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsField {
    DisplayName,
    Username,
    Token,
}

impl SettingsField {
    pub fn next(&self) -> Self {
        match self {
            Self::DisplayName => Self::Username,
            Self::Username    => Self::Token,
            Self::Token       => Self::DisplayName,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::DisplayName => Self::Token,
            Self::Username    => Self::DisplayName,
            Self::Token       => Self::Username,
        }
    }
}

pub struct SettingsDialogState {
    pub display_name: String,
    pub username:     String,
    pub token:        String,
    pub focused:      SettingsField,
    pub cursor:       usize,   // cursor within the active field
    pub show_token:   bool,    // toggle token visibility with Tab+press
}

impl Default for SettingsDialogState {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            username:     String::new(),
            token:        String::new(),
            focused:      SettingsField::DisplayName,
            cursor:       0,
            show_token:   false,
        }
    }
}

impl SettingsDialogState {
    pub fn active_field_text(&self) -> &str {
        match self.focused {
            SettingsField::DisplayName => &self.display_name,
            SettingsField::Username    => &self.username,
            SettingsField::Token       => &self.token,
        }
    }

    pub fn active_field_text_mut(&mut self) -> &mut String {
        match self.focused {
            SettingsField::DisplayName => &mut self.display_name,
            SettingsField::Username    => &mut self.username,
            SettingsField::Token       => &mut self.token,
        }
    }

    /// Clamp cursor to current field length
    pub fn clamp_cursor(&mut self) {
        let len = self.active_field_text().chars().count();
        if self.cursor > len { self.cursor = len; }
    }

    pub fn type_char(&mut self, c: char) {
        let cursor = self.cursor;
        let text   = self.active_field_text_mut();
        let byte   = text.char_indices().nth(cursor).map(|(i, _)| i).unwrap_or(text.len());
        text.insert(byte, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let cursor = self.cursor;
            let text   = self.active_field_text_mut();
            let byte   = text.char_indices().nth(cursor - 1).map(|(i, _)| i).unwrap_or(0);
            text.remove(byte);
            self.cursor -= 1;
        }
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

/// Renders a centered overlay dialog. Call this AFTER the main screen has been drawn.
pub fn render(f: &mut Frame, area: Rect, s: &SettingsDialogState, frame_count: u64) {
    // ── Center a 54-wide × 16-tall box ───────────────────────────────────────
    let dialog = centered_rect(58, 18, area);

    // Clear background so it truly overlays
    f.render_widget(Clear, dialog);

    // Outer border
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" ⚙  Settings  Ctrl+W close ", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)))
        .border_style(Style::default().fg(ACCENT_COLOR))
        .style(Style::default().bg(BG_COLOR));
    f.render_widget(outer, dialog);

    // Inner layout: padding + 3 fields + hint
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // spacer
            Constraint::Length(3), // Display Name
            Constraint::Length(1), // spacer
            Constraint::Length(3), // GitHub Username
            Constraint::Length(1), // spacer
            Constraint::Length(3), // PAT
            Constraint::Length(1), // spacer
            Constraint::Length(1), // hint line
        ])
        .split(dialog);

    let blink = if (frame_count / 5) % 2 == 0 { "█" } else { " " };

    // ── Field 1: Display Name ─────────────────────────────────────────────────
    render_field(
        f, inner[1],
        "Display Name",
        &s.display_name,
        s.cursor,
        s.focused == SettingsField::DisplayName,
        false,
        blink,
    );

    // ── Field 2: GitHub Username ──────────────────────────────────────────────
    render_field(
        f, inner[3],
        "GitHub Username",
        &s.username,
        s.cursor,
        s.focused == SettingsField::Username,
        false,
        blink,
    );

    // ── Field 3: Personal Access Token (masked unless show_token) ────────────
    let token_title = if s.show_token {
        "Personal Access Token  [visible]"
    } else {
        "Personal Access Token  [hidden — Tab to reveal]"
    };
    render_field(
        f, inner[5],
        token_title,
        &s.token,
        s.cursor,
        s.focused == SettingsField::Token,
        !s.show_token,
        blink,
    );

    // ── Hint ─────────────────────────────────────────────────────────────────
    let hint = Paragraph::new(
        "↑↓ / Tab Move fields   Enter Save   Esc Cancel   Ctrl+W Close"
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(BORDER_COLOR));
    f.render_widget(hint, inner[7]);
}

fn render_field(
    f: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    masked: bool,
    blink: &str,
) {
    let border_col = if focused { ACCENT_COLOR } else { BORDER_COLOR };

    let display: String = if masked {
        "•".repeat(value.chars().count())
    } else {
        value.to_string()
    };

    let content = if focused {
        // Show cursor inside the active field (masked or not)
        let shown: String = if masked {
            "•".repeat(value.chars().count())
        } else {
            value.to_string()
        };
        // Insert blinking cursor at position
        let mut chars: Vec<char> = shown.chars().collect();
        let insert_at = cursor.min(chars.len());
        chars.insert(insert_at, blink.chars().next().unwrap_or('█'));
        format!(" {}", chars.iter().collect::<String>())
    } else {
        format!(" {}", display)
    };

    let para = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(border_col))
                .style(Style::default().bg(BG_COLOR)),
        )
        .style(Style::default().fg(if focused { FG_COLOR } else { BORDER_COLOR }));
    f.render_widget(para, area);
}

/// Returns a centered Rect of the given width/height within `r`
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width:  width.min(r.width),
        height: height.min(r.height),
    }
}
