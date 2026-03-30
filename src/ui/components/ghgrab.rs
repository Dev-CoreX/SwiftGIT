//! GhGrab file picker — one-level collapsible tree, file icons, MB sizes.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use std::collections::HashSet;
use crate::git::GhFile;
use crate::ui::theme::*;
use crate::ui::spinner_char;

// ── File-type icon (same palette as repo_view) ───────────────────────────────

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs"                              => "🦀",
        "js" | "mjs" | "cjs"             => "JS",
        "ts" | "tsx"                      => "TS",
        "jsx"                             => "⚛ ",
        "html" | "htm"                    => "🌐",
        "css" | "scss" | "sass"           => "🎨",
        "json"                            => "{}",
        "toml" | "ini" | "cfg"            => "⚙ ",
        "yaml" | "yml"                    => "📋",
        "env"                             => "🔑",
        "md" | "mdx"                      => "📝",
        "txt"                             => "📃",
        "sh" | "bash" | "zsh" | "fish"   => "⚡",
        "py"                              => "🐍",
        "rb"                              => "💎",
        "go"                              => "🐹",
        "java" | "kt"                     => "☕",
        "c" | "h"                         => "C ",
        "cpp" | "cc" | "cxx" | "hpp"     => "C+",
        "cs"                              => "C#",
        "php"                             => "🐘",
        "swift"                           => "🍎",
        "lock"                            => "🔒",
        "log"                             => "📜",
        "png"|"jpg"|"jpeg"|"gif"|"svg"
        |"webp"|"ico"                     => "🖼 ",
        "zip"|"tar"|"gz"|"bz2"           => "📦",
        "sql"|"db"|"sqlite"              => "🗃 ",
        "xml"                             => "📄",
        "pdf"                             => "📕",
        _                                 => "  ",
    }
}

// ── Task 2: MB formatter (2 decimal places) ───────────────────────────────────

fn format_size(bytes: u64) -> String {
    if bytes == 0 { return "  0.00 MB".to_string(); }
    if bytes < 1024 {
        return format!("  0.00 MB"); // sub-1KB shows as 0.00 MB
    }
    let kb = bytes as f64 / 1024.0;
    if kb < 1024.0 {
        return format!("{:>6.2} KB", kb);
    }
    let mb = kb / 1024.0;
    format!("{:>6.2} MB", mb)
}

// ── Tree item for display ─────────────────────────────────────────────────────

#[derive(Clone)]
enum TreeItem {
    Folder { name: String, file_count: usize, total_bytes: u64, expanded: bool },
    File   { file_idx: usize, depth: usize },
}

/// Build one-level-at-a-time tree from flat file list + expanded set
fn build_tree(files: &[GhFile], expanded: &HashSet<String>) -> Vec<TreeItem> {
    let mut items = Vec::new();
    build_tree_level(files, expanded, "", 0, &mut items);
    items
}

fn build_tree_level(
    files:    &[GhFile],
    expanded: &HashSet<String>,
    parent:   &str,
    depth:    usize,
    out:      &mut Vec<TreeItem>,
) {
    use std::collections::BTreeMap;

    let mut direct_files: Vec<usize> = Vec::new();
    let mut subdirs: BTreeMap<String, (usize, u64)> = BTreeMap::new(); // name → (count, total_bytes)

    for (i, f) in files.iter().enumerate() {
        let rel = if parent.is_empty() {
            f.path.as_str()
        } else {
            match f.path.strip_prefix(&format!("{}/", parent)) {
                Some(r) => r,
                None    => continue,
            }
        };

        if let Some(slash) = rel.find('/') {
            let subdir = if parent.is_empty() {
                rel[..slash].to_string()
            } else {
                format!("{}/{}", parent, &rel[..slash])
            };
            let entry = subdirs.entry(subdir).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += f.size.unwrap_or(0);
        } else {
            direct_files.push(i);
        }
    }

    // Root-level files first
    for idx in direct_files {
        out.push(TreeItem::File { file_idx: idx, depth });
    }

    // Then folders, collapsed by default
    for (dir_path, (count, total_bytes)) in &subdirs {
        let name     = dir_path.split('/').next_back().unwrap_or(dir_path).to_string();
        let is_open  = expanded.contains(dir_path);
        out.push(TreeItem::Folder {
            name,
            file_count: *count,
            total_bytes: *total_bytes,
            expanded: is_open,
        });
        if is_open {
            build_tree_level(files, expanded, dir_path, depth + 1, out);
        }
    }
}

// ── Main render ───────────────────────────────────────────────────────────────

pub fn render(
    f: &mut Frame,
    area: Rect,
    frame_count: u64,
    owner: &str,
    repo: &str,
    files: &[GhFile],
    cursor: usize,
    scroll: usize,
    selected: &HashSet<usize>,
    is_loading: bool,
    expanded: &HashSet<String>,
) {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    // Header
    let spin = if is_loading { format!(" {} ", spinner_char(frame_count)) } else { String::new() };
    f.render_widget(
        Paragraph::new(format!("  {}/{} — {} files{}  (Space=select  Enter=expand  a=clone all)", owner, repo, files.len(), spin))
            .block(Block::default().borders(Borders::ALL)
                .title(" GhGrab — File Picker ")
                .border_style(Style::default().fg(ACCENT_COLOR))
                .style(Style::default().bg(BG_COLOR)))
            .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD)),
        main[0],
    );

    // Split: tree left, info right
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(main[1]);

    // Build tree
    let tree = build_tree(files, expanded);
    let visible_h = cols[0].height.saturating_sub(2) as usize;

    let items: Vec<ListItem> = tree.iter().enumerate()
        .skip(scroll)
        .take(visible_h)
        .map(|(i, node)| {
            let is_cursor = i == cursor;
            match node {
                TreeItem::Folder { name, file_count, total_bytes, expanded: is_open } => {
                    let arrow  = if *is_open { "▼ " } else { "▶ " };
                    let label  = format!(" {}📁 {}/  ({} files, {})",
                        arrow, name, file_count, format_size(*total_bytes));
                    if is_cursor {
                        ListItem::new(Line::from(Span::styled(label,
                            Style::default().fg(BG_COLOR).bg(ACCENT_COLOR).add_modifier(Modifier::BOLD))))
                    } else {
                        ListItem::new(Line::from(Span::styled(label,
                            Style::default().fg(FOLDER_COLOR).add_modifier(Modifier::BOLD))))
                    }
                }
                TreeItem::File { file_idx, depth } => {
                    if let Some(file) = files.get(*file_idx) {
                        let fname     = file.path.split('/').next_back().unwrap_or(&file.path);
                        let icon      = file_icon(fname);
                        let indent    = "  ".repeat(*depth);
                        let is_sel    = selected.contains(file_idx);
                        let check     = if is_sel { "◉" } else { "○" };
                        let check_col = if is_sel { SUCCESS_COLOR } else { BORDER_COLOR };
                        let size_str  = file.size.map(|s| format!(" {}", format_size(s))).unwrap_or_default();

                        if is_cursor {
                            ListItem::new(Line::from(Span::styled(
                                format!(" {}{} {} {} {}{}", indent, check, icon, fname, size_str, ""),
                                Style::default().fg(BG_COLOR).bg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
                            )))
                        } else {
                            ListItem::new(Line::from(vec![
                                Span::styled(format!(" {}{}", indent, check),
                                    Style::default().fg(check_col).add_modifier(Modifier::BOLD)),
                                Span::styled(format!(" {} ", icon),
                                    Style::default().fg(BORDER_COLOR)),
                                Span::styled(fname.to_string(),
                                    Style::default().fg(FG_COLOR)),
                                Span::styled(size_str,
                                    Style::default().fg(BORDER_COLOR)),
                            ]))
                        }
                    } else { ListItem::new("") }
                }
            }
        })
        .collect();

    f.render_widget(
        List::new(items).block(
            Block::default().borders(Borders::ALL)
                .title(format!(" Files ({} selected) ", selected.len()))
                .border_style(Style::default().fg(BORDER_COLOR))
                .style(Style::default().bg(BG_COLOR)),
        ),
        cols[0],
    );

    // Info panel for selected file
    let info_lines: Vec<Line> = if let Some(TreeItem::File { file_idx, .. }) = tree.get(cursor) {
        if let Some(file) = files.get(*file_idx) {
            let fname = file.path.split('/').next_back().unwrap_or(&file.path);
            let dir   = file.path.rfind('/').map(|i| &file.path[..i]).unwrap_or("/");
            let icon  = file_icon(fname);
            let size  = file.size.map(|s| format_size(s)).unwrap_or_else(|| "unknown".into());
            vec![
                Line::from(""),
                Line::from(Span::styled("  File", Style::default().fg(BORDER_COLOR))),
                Line::from(Span::styled(format!("  {} {}", icon, fname),
                    Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD))),
                Line::from(""),
                Line::from(Span::styled("  Directory", Style::default().fg(BORDER_COLOR))),
                Line::from(Span::styled(format!("  /{}", dir),
                    Style::default().fg(ACCENT_COLOR))),
                Line::from(""),
                Line::from(Span::styled("  Size", Style::default().fg(BORDER_COLOR))),
                Line::from(Span::styled(format!("  {}", size),
                    Style::default().fg(FG_COLOR))),
                Line::from(""),
                Line::from(Span::styled("  SHA", Style::default().fg(BORDER_COLOR))),
                Line::from(Span::styled(
                    format!("  {}…", &file.sha[..8.min(file.sha.len())]),
                    Style::default().fg(BORDER_COLOR))),
            ]
        } else { vec![] }
    } else if let Some(TreeItem::Folder { name, file_count, total_bytes, .. }) = tree.get(cursor) {
        vec![
            Line::from(""),
            Line::from(Span::styled("  Folder", Style::default().fg(BORDER_COLOR))),
            Line::from(Span::styled(format!("  📁 {}/", name),
                Style::default().fg(FOLDER_COLOR).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("  Files", Style::default().fg(BORDER_COLOR))),
            Line::from(Span::styled(format!("  {}", file_count), Style::default().fg(FG_COLOR))),
            Line::from(""),
            Line::from(Span::styled("  Total Size", Style::default().fg(BORDER_COLOR))),
            Line::from(Span::styled(format!("  {}", format_size(*total_bytes)),
                Style::default().fg(FG_COLOR))),
        ]
    } else {
        vec![Line::from(Span::styled("  Select a file", Style::default().fg(BORDER_COLOR)))]
    };

    f.render_widget(
        Paragraph::new(info_lines)
            .block(Block::default().borders(Borders::ALL)
                .title(" Info ")
                .border_style(Style::default().fg(BORDER_COLOR))
                .style(Style::default().bg(BG_COLOR))),
        cols[1],
    );

    // Footer
    f.render_widget(
        Paragraph::new("↑↓ Navigate   Enter Expand/Collapse folder   Space Select file   d Download selected   a Clone all   Esc Back")
            .alignment(Alignment::Center)
            .style(Style::default().fg(BORDER_COLOR)),
        main[2],
    );
}
