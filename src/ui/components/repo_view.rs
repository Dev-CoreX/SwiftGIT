//! Repo view: collapsible tree (left) + diff/editor (right)
//! v1.4: TokyoNight Storm diff theme — syntax-aware coloring, gutter line numbers

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::ui::theme::*;
use crate::ui::{spinner_char, DisplayItem};

pub struct RepoViewState<'a> {
    pub repo_name:     &'a str,
    pub branch:        &'a str,
    pub files:         &'a [crate::git::GitFile],
    pub display_items: &'a [DisplayItem],
    pub cursor:        usize,
    pub scroll_offset: usize,
    pub diff_scroll:   usize,
    pub diff_content:  &'a str,
    pub diff_struct:   &'a crate::git::Diff,
    pub hunk_cursor:   Option<usize>,
    pub commit_mode:   bool,
    pub commit_input:  &'a str,
    pub commit_cursor: usize,
    pub commit_history: &'a [String],
    pub status_msg:    &'a str,
    pub is_loading:    bool,
    pub is_diff_loading: bool,
    pub frame_count:   u64,
    pub active_frame:  u8,
}

// ── File-type icons ───────────────────────────────────────────────────────────

fn file_icon(path: &str) -> &'static str {
    let p = path.to_lowercase();
    if p.ends_with(".rs") { "🦀" }
    else if p.ends_with(".py") { "🐍" }
    else if p.ends_with(".js") || p.ends_with(".ts") { "JS" }
    else if p.ends_with(".json") { "JSON" }
    else if p.ends_with(".md") { "📝" }
    else if p.ends_with(".toml") { "⚙️" }
    else if p.ends_with(".sh") || p.ends_with(".bash") { "🐚" }
    else if p.ends_with(".zip") || p.ends_with(".tar") || p.ends_with(".gz") { "📦" }
    else if p.ends_with(".png") || p.ends_with(".jpg") || p.ends_with(".svg") { "🖼️" }
    else { "📄" }
}

// ── Main Render ───────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    render_header(f, chunks[0], s);
    
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Tree + History
            Constraint::Percentage(70), // Diff
        ])
        .split(chunks[1]);

    render_tree(f, body[0], s);
    render_diff(f, body[1], s);
    render_footer(f, chunks[2], s);
}

pub fn render_with_editor(f: &mut Frame, area: Rect, s: &RepoViewState, ed: &crate::ui::components::editor::EditorState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, chunks[0], s);
    
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(chunks[1]);

    render_tree(f, body[0], s);
    crate::ui::components::editor::render(f, body[1], ed);
    render_footer(f, chunks[2], s);
}

// ── Header ────────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let title = format!(" SwiftGit v1.4 │ {} ", s.repo_name);
    let branch = format!(" 🌿 {} ", s.branch);
    
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_COLOR))
        .style(Style::default().bg(BG_COLOR));

    let header_text = vec![
        Line::from(vec![
            Span::styled(title, Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(branch, Style::default().fg(SUCCESS_COLOR)),
        ])
    ];

    f.render_widget(Paragraph::new(header_text).block(block), area);
}

// ── Tree panel ────────────────────────────────────────────────────────────────

fn render_tree(f: &mut Frame, area: Rect, s: &RepoViewState) {
    // Permanent split: Tree (top) and Commits (bottom)
    let h = if s.commit_mode { 14 } else { 10 }; // commit history section height
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(h)])
        .split(area);
    let list_area = v[0];
    let commit_area = v[1];

    if s.commit_mode {
        let commit_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input
                Constraint::Min(0),    // History
            ])
            .split(commit_area);

        let blink = if (s.frame_count / 5) % 2 == 0 { "█" } else { "" };
        f.render_widget(
            Paragraph::new(format!(" {}{}", s.commit_input, blink))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" New Commit Message  Enter=confirm  Esc=cancel ")
                        .border_style(Style::default().fg(SUCCESS_COLOR))
                        .style(Style::default().bg(BG_COLOR)),
                )
                .style(Style::default().fg(FG_COLOR)),
            commit_layout[0],
        );

        render_commit_history(f, commit_layout[1], s);
    } else {
        render_commit_history(f, commit_area, s);
    }

    let visible_h = list_area.height.saturating_sub(2) as usize;
    let items: Vec<ListItem> = s.display_items
        .iter()
        .enumerate()
        .skip(s.scroll_offset)
        .take(visible_h)
        .map(|(i, item)| build_item(item, i == s.cursor, s))
        .collect();

    let active_dot = if s.active_frame == 1 { "◉" } else { "○" };
    let border_col = if s.active_frame == 1 { ACCENT_COLOR } else { BORDER_COLOR };

    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} 1  Working Tree ", active_dot))
                .border_style(Style::default().fg(border_col))
                .style(Style::default().bg(BG_COLOR)),
        ),
        list_area,
    );
}

fn render_commit_history(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let history_items: Vec<ListItem> = if s.commit_history.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(" No commits yet ", Style::default().fg(BORDER_COLOR))))]
    } else {
        s.commit_history.iter()
            .map(|line| {
                if let Some((hash, msg)) = line.split_once(' ') {
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {} ", hash), Style::default().fg(ACCENT_COLOR)),
                        Span::styled(msg.to_string(), Style::default().fg(BORDER_COLOR)),
                    ]))
                } else {
                    ListItem::new(Line::from(Span::styled(line, Style::default().fg(BORDER_COLOR))))
                }
            })
            .collect()
    };

    f.render_widget(
        List::new(history_items)
            .block(Block::default().borders(Borders::ALL).title(" Recent Commits "))
            .style(Style::default().bg(BG_COLOR)),
        area,
    );
}

fn build_item(item: &DisplayItem, selected: bool, s: &RepoViewState) -> ListItem<'static> {
    match item {
        DisplayItem::FolderHeader { path, count, expanded, depth } => {
            let arrow  = if *expanded { "▼ " } else { "▶ " };
            let indent = "  ".repeat(*depth);
            let name   = path.split('/').last().unwrap_or(path);
            let style  = if selected { Style::default().fg(BG_COLOR).bg(FOLDER_COLOR) } 
                         else { Style::default().fg(FOLDER_COLOR) };
            
            ListItem::new(Line::from(vec![
                Span::raw(indent),
                Span::styled(arrow, style),
                Span::styled(format!("{}/", name), style.add_modifier(Modifier::BOLD)),
                Span::styled(format!(" ({})", count), Style::default().fg(BORDER_COLOR)),
            ]))
        }
        DisplayItem::FileEntry { file_idx, depth } => {
            let indent = "  ".repeat(*depth + 1);
            let file = match s.files.get(*file_idx) {
                Some(f) => f,
                None => {
                    // Fallback for ghost entries during rapid refreshes
                    return ListItem::new(" (updating...)");
                }
            };
            let name = file.path.split('/').last().unwrap_or(&file.path);
            let icon = file_icon(name);
            let status = &file.status;
            
            let (status_color, indicator) = match status {
                crate::git::FileStatus::Staged | crate::git::FileStatus::Added => (SUCCESS_COLOR, status.indicator()),
                crate::git::FileStatus::Modified => (WARNING_COLOR, status.indicator()),
                crate::git::FileStatus::Untracked => (ACCENT_COLOR, status.indicator()),
                crate::git::FileStatus::Deleted => (ERROR_COLOR, status.indicator()),
                crate::git::FileStatus::Clean => (BORDER_COLOR, status.indicator()),
                _ => (FG_COLOR, "[ ]"),
            };

            let mut spans = vec![
                Span::raw(indent),
                Span::styled(format!("{} ", indicator), Style::default().fg(status_color)),
                Span::raw(format!("{} ", icon)),
            ];

            if selected {
                spans.push(Span::styled(name.to_string(), Style::default().fg(BG_COLOR).bg(FG_COLOR).add_modifier(Modifier::BOLD)));
            } else {
                let mut style = Style::default().fg(FG_COLOR);
                if matches!(status, crate::git::FileStatus::Clean) {
                    style = style.add_modifier(Modifier::DIM);
                }
                spans.push(Span::styled(name.to_string(), style));
            }

            ListItem::new(Line::from(spans))
        }
    }
}

// ── Task 4: Diff panel with minimal readable theme ───────────────────────────

fn render_diff(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // body
            Constraint::Length(1), // hunk keybinds
        ])
        .split(area);

    let diff_lines: Vec<Line> = if s.is_diff_loading {
        vec![Line::from(vec![
            Span::styled(format!("  {} ", spinner_char(s.frame_count)), Style::default().fg(ACCENT_COLOR)),
            Span::styled("loading diff...", Style::default().fg(BORDER_COLOR)),
        ])]
    } else if s.is_loading {
        vec![Line::from(Span::styled(
            format!("  {} loading…", spinner_char(s.frame_count)),
            Style::default().fg(BORDER_COLOR),
        ))]
    } else if s.diff_struct.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No changes in this file.",
                Style::default().fg(BORDER_COLOR).add_modifier(Modifier::ITALIC),
            )),
        ]
    } else {
        render_diff_struct(s.diff_struct, s.hunk_cursor)
    };

    let active_dot = if s.active_frame == 2 { "◉" } else { "○" };
    let border_col = if s.active_frame == 2 { ACCENT_COLOR } else { BORDER_COLOR };
    let diff_title = match s.display_items.get(s.cursor) {
        Some(DisplayItem::FileEntry { file_idx, .. }) => {
            if let Some(f) = s.files.get(*file_idx) {
                format!(" {} 2  {} ", active_dot, f.path)
            } else {
                format!(" {} 2  Diff ", active_dot)
            }
        }
        _ => format!(" {} 2  Diff — navigate to a file ", active_dot),
    };

    f.render_widget(
        Paragraph::new(diff_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(diff_title)
                    .border_style(Style::default().fg(border_col))
                    .style(Style::default().bg(BG_COLOR)),
            )
            .scroll((s.diff_scroll as u16, 0))
            .wrap(ratatui::widgets::Wrap { trim: false }),
        chunks[0],
    );

    if let Some(hc) = s.hunk_cursor {
        let hunk_msg = format!(" Hunk {}/{}  Space: Stage/Unstage Hunk  Tab: Next Hunk  n/p: Nav ", 
            hc + 1, s.diff_struct.hunks.len());
        f.render_widget(
            Paragraph::new(hunk_msg).style(Style::default().fg(ACCENT_COLOR)),
            chunks[1]
        );
    }
}

fn render_diff_struct<'a>(diff: &'a crate::git::Diff, hunk_cursor: Option<usize>) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    
    // Header lines
    for line in diff.file_header.lines() {
        let s: String = line.to_string();
        lines.push(Line::from(Span::styled(s, Style::default().fg(DIFF_FILE_FG))));
    }

    for (i, hunk) in diff.hunks.iter().enumerate() {
        let is_active = hunk_cursor == Some(i);
        let header_style = if is_active {
            Style::default().fg(BG_COLOR).bg(DIFF_HUNK_FG).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIFF_HUNK_FG).bg(DIFF_HUNK_BG)
        };
        
        lines.push(Line::from(Span::styled(&hunk.header, header_style)));
        
        for line in &hunk.lines {
            lines.push(render_single_diff_line(line, is_active));
        }
    }
    lines
}

fn render_single_diff_line<'a>(line: &'a str, is_active: bool) -> Line<'a> {
    let style = if line.starts_with('+') {
        Style::default().fg(DIFF_ADD_FG).bg(if is_active { DIFF_ADD_BG } else { BG_COLOR })
    } else if line.starts_with('-') {
        Style::default().fg(DIFF_DEL_FG).bg(if is_active { DIFF_DEL_BG } else { BG_COLOR })
    } else {
        Style::default().fg(DIFF_CTX_FG).bg(if is_active { HIGHLIGHT_BG } else { BG_COLOR })
    };
    Line::from(Span::styled(line.to_string(), style))
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let keybinds = " Space Stage/Unstage  s Stage All  c Commit  p Push  P Pull  e Edit  r Refresh  q Quit ";
    let (text, color) = if !s.status_msg.is_empty() {
        let c = if s.status_msg.contains("❌") { ERROR_COLOR } else { SUCCESS_COLOR };
        (format!("{}   │   {}", s.status_msg, keybinds), c)
    } else {
        (keybinds.to_string(), BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(color)),
        area,
    );
}
