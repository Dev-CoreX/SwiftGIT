//! Push dialog: repo name, commit msg, branch dropdown, origin — all editable.
//! Task 2: branch shown as a dropdown of existing local branches.
//! Task 3: push guarded so no "src refspec does not match" errors.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::ui::theme::*;
use crate::ui::spinner_char;

// ── Focused field ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushField {
    RepoName,
    CommitMsg,
    Branch,   // dropdown
    Origin,
}

impl PushField {
    pub fn next(&self) -> Self {
        match self {
            Self::RepoName  => Self::CommitMsg,
            Self::CommitMsg => Self::Branch,
            Self::Branch    => Self::Origin,
            Self::Origin    => Self::RepoName,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            Self::RepoName  => Self::Origin,
            Self::CommitMsg => Self::RepoName,
            Self::Branch    => Self::CommitMsg,
            Self::Origin    => Self::Branch,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PushDialogState {
    pub repo_name:      String,
    pub commit_msg:     String,
    pub branch:         String,         // currently selected branch name
    pub branch_list:    Vec<String>,    // all local branches from git
    pub branch_cursor:  usize,          // cursor inside branch_list dropdown
    pub branch_open:    bool,           // is dropdown open?
    pub origin:         String,
    pub focused:        PushField,
    pub cursor:         usize,          // text cursor for non-branch fields
    pub is_pushing:     bool,
    pub status_msg:     String,
    pub frame_count:    u64,
    pub username:       String,
    pub recent_commits: Vec<String>,
    pub has_commits:    bool,           // Task 3: guard flag
    pub force_push:     bool,           // New: force push flag
}

impl Default for PushDialogState {
    fn default() -> Self {
        Self {
            repo_name:     String::new(),
            commit_msg:    String::new(),
            branch:        "main".to_string(),
            branch_list:   vec!["main".to_string()],
            branch_cursor: 0,
            branch_open:   false,
            origin:        String::new(),
            focused:       PushField::RepoName,
            cursor:        0,
            is_pushing:    false,
            status_msg:    String::new(),
            frame_count:   0,
            username:      String::new(),
            recent_commits: Vec::new(),
            has_commits:   true,
            force_push:    false,
        }
    }
}

impl PushDialogState {
    pub fn update_origin(&mut self) {
        if !self.username.is_empty() && !self.repo_name.is_empty() {
            self.origin = format!(
                "https://github.com/{}/{}.git",
                self.username.trim(),
                self.repo_name.trim()
            );
        } else {
            self.origin.clear();
        }
    }

    pub fn active_text(&self) -> &str {
        match self.focused {
            PushField::RepoName  => &self.repo_name,
            PushField::CommitMsg => &self.commit_msg,
            PushField::Branch    => &self.branch,
            PushField::Origin    => &self.origin,
        }
    }

    pub fn active_text_mut(&mut self) -> &mut String {
        match self.focused {
            PushField::RepoName  => &mut self.repo_name,
            PushField::CommitMsg => &mut self.commit_msg,
            PushField::Branch    => &mut self.branch,
            PushField::Origin    => &mut self.origin,
        }
    }

    pub fn clamp_cursor(&mut self) {
        let len = self.active_text().chars().count();
        if self.cursor > len { self.cursor = len; }
    }

    pub fn type_char(&mut self, c: char) {
        let cur  = self.cursor;
        let text = self.active_text_mut();
        let byte = text.char_indices().nth(cur).map(|(i, _)| i).unwrap_or(text.len());
        text.insert(byte, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let cur  = self.cursor;
            let text = self.active_text_mut();
            let byte = text.char_indices().nth(cur - 1).map(|(i, _)| i).unwrap_or(0);
            text.remove(byte);
            self.cursor -= 1;
        }
    }

    /// Sync branch_cursor to the currently selected branch in branch_list
    pub fn sync_branch_cursor(&mut self) {
        if let Some(i) = self.branch_list.iter().position(|b| b == &self.branch) {
            self.branch_cursor = i;
        } else {
            self.branch_cursor = 0;
        }
    }

    /// Select branch from dropdown at current cursor
    pub fn select_branch(&mut self) {
        if let Some(b) = self.branch_list.get(self.branch_cursor) {
            self.branch = b.clone();
            self.cursor = self.branch.chars().count();
        }
        self.branch_open = false;
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, s: &PushDialogState) {
    let mw = 74u16.min(area.width.saturating_sub(2));
    let mh = if s.branch_open { 28u16 } else { 24u16 }.min(area.height.saturating_sub(2));
    let mx = area.x + (area.width.saturating_sub(mw)) / 2;
    let my = area.y + (area.height.saturating_sub(mh)) / 2;
    let modal = Rect::new(mx, my, mw, mh);

    f.render_widget(Clear, modal);

    let spin = if s.is_pushing { format!(" {} ", spinner_char(s.frame_count)) }
               else { "  ".to_string() };

    // Warn if no commits
    let warn = if !s.has_commits { " ⚠ No commits yet! " } else { "" };
    let force = if s.force_push { " 🔥 FORCE PUSH ACTIVE " } else { "" };

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" 🚀 Push{}— @{}  {}{}", spin, s.username, warn, force))
            .border_style(Style::default().fg(if s.force_push { ERROR_COLOR } else if !s.has_commits { WARNING_COLOR } else { ACCENT_COLOR }))
            .style(Style::default().bg(BG_COLOR)),
        modal,
    );

    let inner = Rect::new(modal.x + 1, modal.y + 1, modal.width - 2, modal.height - 2);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Repo Name
            Constraint::Length(1),
            Constraint::Length(3),  // Commit Message
            Constraint::Length(1),
            Constraint::Length(3),  // Branch (+ dropdown below if open)
            Constraint::Length(1),
            Constraint::Length(3),  // Origin
            Constraint::Length(1),
            Constraint::Min(2),     // Recent commits
            Constraint::Length(1),  // Hint
        ])
        .split(inner);

    let blink = if (s.frame_count / 5) % 2 == 0 { "█" } else { " " };

    // ── Repo Name ─────────────────────────────────────────────────────────────
    render_text_field(f, layout[0], "Repo Name  (required)",
        &s.repo_name, s.cursor, s.focused == PushField::RepoName, blink,
        if s.repo_name.is_empty() { "e.g. my-project" } else { "" });

    // ── Commit Message ────────────────────────────────────────────────────────
    render_text_field(f, layout[2], "Commit Message  (editable)",
        &s.commit_msg, s.cursor, s.focused == PushField::CommitMsg, blink, "");

    // ── Branch dropdown ───────────────────────────────────────────────────────
    render_branch_field(f, layout[4], s, blink);

    // If dropdown is open, render it on top overlapping subsequent rows
    if s.branch_open {
        render_branch_dropdown(f, layout[4], s);
    }

    // ── Origin ────────────────────────────────────────────────────────────────
    render_text_field(f, layout[6], "Remote Origin  (auto-set, editable)",
        &s.origin, s.cursor, s.focused == PushField::Origin, blink, "");

    // ── Recent commits ────────────────────────────────────────────────────────
    if !s.recent_commits.is_empty() {
        let lines: Vec<Line> = s.recent_commits.iter().enumerate().map(|(i, c)| {
            let (hash, msg) = c.split_once(' ').unwrap_or(("", c.as_str()));
            Line::from(vec![
                Span::styled(format!(" {:>2}  ", i + 1), Style::default().fg(BORDER_COLOR)),
                Span::styled(format!("{} ", hash), Style::default().fg(ACCENT_COLOR)),
                Span::styled(msg.to_string(), Style::default().fg(FG_COLOR)),
            ])
        }).collect();

        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL)
                    .title(" Recent Commits ")
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .style(Style::default().bg(BG_COLOR))),
            layout[8],
        );
    }

    // ── Status / hint ─────────────────────────────────────────────────────────
    let (hint_text, hint_color) = if !s.has_commits {
        ("⚠  Stage and commit files first, then push", WARNING_COLOR)
    } else if !s.status_msg.is_empty() {
        let c = if s.status_msg.contains('✅') { SUCCESS_COLOR }
                else if s.status_msg.contains('❌') { ERROR_COLOR }
                else { WARNING_COLOR };
        (s.status_msg.as_str(), c)
    } else {
        ("Tab/↑↓ Fields   Enter/Space Branch   Ctrl+F Force Push   Enter Push   Esc Cancel", BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new(hint_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(hint_color)),
        layout[9],
    );
}

fn render_text_field(
    f: &mut Frame, area: Rect,
    title: &str, value: &str,
    cursor: usize, focused: bool, _blink: &str, placeholder: &str,
) {
    let border_col = if focused { ACCENT_COLOR } else { BORDER_COLOR };

    let line = if focused {
        let chars: Vec<char> = value.chars().collect();
        let pos    = cursor.min(chars.len());
        let before: String = chars[..pos].iter().collect();
        let cur_ch = if pos < chars.len() { chars[pos].to_string() } else { " ".to_string() };
        let after:  String = if pos + 1 <= chars.len() { chars[pos+1..].iter().collect() } else { String::new() };
        Line::from(vec![
            Span::raw(" "),
            Span::styled(before, Style::default().fg(FG_COLOR)),
            Span::styled(cur_ch, Style::default().fg(BG_COLOR).bg(FG_COLOR)),
            Span::styled(after, Style::default().fg(FG_COLOR)),
        ])
    } else if value.is_empty() && !placeholder.is_empty() {
        Line::from(Span::styled(format!(" {}", placeholder),
            Style::default().fg(BORDER_COLOR).add_modifier(Modifier::ITALIC)))
    } else {
        Line::from(Span::styled(format!(" {}", value),
            Style::default().fg(if focused { FG_COLOR } else { BORDER_COLOR })))
    };

    f.render_widget(
        Paragraph::new(line)
            .block(Block::default().borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(border_col))
                .style(Style::default().bg(BG_COLOR))),
        area,
    );
}

fn render_branch_field(f: &mut Frame, area: Rect, s: &PushDialogState, _blink: &str) {
    let focused    = s.focused == PushField::Branch;
    let border_col = if focused { ACCENT_COLOR } else { BORDER_COLOR };
    let arrow      = if s.branch_open { "▼" } else { "▶" };

    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(format!("{} ", arrow), Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)),
        Span::styled(
            s.branch.clone(),
            Style::default().fg(if focused { FG_COLOR } else { BORDER_COLOR }).add_modifier(
                if focused { Modifier::BOLD } else { Modifier::empty() }
            ),
        ),
        Span::styled(
            format!("  ({} branches)", s.branch_list.len()),
            Style::default().fg(BORDER_COLOR),
        ),
    ]);

    f.render_widget(
        Paragraph::new(line)
            .block(Block::default().borders(Borders::ALL)
                .title(" Branch  (Enter/Space to open dropdown) ")
                .border_style(Style::default().fg(border_col))
                .style(Style::default().bg(BG_COLOR))),
        area,
    );
}

fn render_branch_dropdown(f: &mut Frame, anchor: Rect, s: &PushDialogState) {
    // Draw dropdown directly below the branch field
    let drop_h = (s.branch_list.len().min(6) + 2) as u16;
    let drop_area = Rect::new(
        anchor.x,
        anchor.y + anchor.height,
        anchor.width,
        drop_h,
    );

    f.render_widget(Clear, drop_area);

    let items: Vec<ListItem> = s.branch_list.iter().enumerate().map(|(i, b)| {
        let is_cur = i == s.branch_cursor;
        let marker = if b == &s.branch { "* " } else { "  " };
        if is_cur {
            ListItem::new(Line::from(Span::styled(
                format!(" {}{}", marker, b),
                Style::default().fg(BG_COLOR).bg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
            )))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {}", marker), Style::default().fg(SUCCESS_COLOR)),
                Span::styled(b.clone(), Style::default().fg(FG_COLOR)),
            ]))
        }
    }).collect();

    f.render_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL)
                .title(" Select Branch ")
                .border_style(Style::default().fg(ACCENT_COLOR))
                .style(Style::default().bg(BG_COLOR))),
        drop_area,
    );
}
