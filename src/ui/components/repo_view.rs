//! Repo view: collapsible tree (left) + diff/editor (right)
//! v1.4: TokyoNight Storm diff theme — syntax-aware coloring, gutter line numbers

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::git::FileStatus;
use crate::ui::theme::*;
use crate::ui::{DisplayItem, spinner_char};

pub struct RepoViewState<'a> {
    pub repo_name:     &'a str,
    pub branch:        &'a str,
    pub files:         &'a [crate::git::GitFile],
    pub display_items: &'a [DisplayItem],
    pub cursor:        usize,
    pub scroll_offset: usize,
    pub diff_content:  &'a str,
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

// ── Task 1: compute folder status for green tick ──────────────────────────────

enum FolderStatus { AllClean, AllStaged, Mixed, HasChanges }

fn folder_status(folder_path: &str, files: &[crate::git::GitFile]) -> FolderStatus {
    use crate::git::FileStatus;
    let children: Vec<_> = files.iter()
        .filter(|f| f.path.starts_with(&format!("{}/", folder_path)))
        .collect();
    if children.is_empty() { return FolderStatus::AllClean; }

    let all_clean   = children.iter().all(|f| matches!(f.status, FileStatus::Clean | FileStatus::Unknown));
    let all_staged  = children.iter().all(|f| f.status.is_staged());
    let any_staged  = children.iter().any(|f| f.status.is_staged());


    if all_clean   { FolderStatus::AllClean }
    else if all_staged { FolderStatus::AllStaged }
    else if any_staged { FolderStatus::Mixed }
    else               { FolderStatus::HasChanges }
}

// ── Main render ───────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(f, main[0], s);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main[1]);

    render_tree(f, cols[0], s);
    render_diff(f, cols[1], s);
    render_footer(f, main[2], s, false);
}

pub fn render_with_editor(
    f: &mut Frame,
    area: Rect,
    s: &RepoViewState,
    editor: &super::editor::EditorState,
) {
    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(f, main[0], s);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main[1]);

    let tree_s = RepoViewState { active_frame: 1, ..*s };
    render_tree(f, cols[0], &tree_s);
    super::editor::render(f, cols[1], editor);
    render_footer(f, main[2], s, true);
}

// ── Header bar ────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, area: Rect, s: &RepoViewState) {
    let spin = if s.is_loading {
        format!(" {} ", spinner_char(s.frame_count))
    } else {
        "  ".to_string()
    };
    let text = format!("{}  {}  [{}]", spin, s.repo_name, s.branch);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" SwiftGit v1.3 ")
                    .border_style(Style::default().fg(ACCENT_COLOR))
                    .style(Style::default().bg(BG_COLOR)),
            )
            .style(Style::default().fg(FG_COLOR).add_modifier(Modifier::BOLD)),
        area,
    );
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
    let file_count = s.display_items.iter()
        .filter(|d| matches!(d, DisplayItem::FileEntry { .. }))
        .count();
    let title = if s.is_loading {
        format!(" {} 1  {} Working Tree ", active_dot, spinner_char(s.frame_count))
    } else if s.display_items.is_empty() {
        format!(" {} 1  Working Tree (clean ✓) ", active_dot)
    } else {
        format!(" {} 1  Working Tree ({}) ", active_dot, file_count)
    };

    f.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
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
            let name   = path.split('/').next_back().unwrap_or(path.as_str());

            // Task 1: show status indicator for folders same as files
            let (indicator, ind_color) = match folder_status(path, s.files) {
                FolderStatus::AllClean   => ("[ ]", BORDER_COLOR),   // all committed & clean
                FolderStatus::AllStaged  => ("[✓]", SUCCESS_COLOR),  // all staged
                FolderStatus::Mixed      => ("[~]", WARNING_COLOR),   // some staged
                FolderStatus::HasChanges => ("[M]", WARNING_COLOR),   // has changes not staged
            };

            let label = format!(" {}{}{} 📁 {}/  ({})", indent, arrow, indicator, name, count);

            if selected {
                ListItem::new(Line::from(Span::styled(
                    label,
                    Style::default().fg(BG_COLOR).bg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
                )))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {}{}", indent, arrow),
                        Style::default().fg(BORDER_COLOR),
                    ),
                    Span::styled(
                        format!("{} ", indicator),
                        Style::default().fg(ind_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("📁 {}/  ({})", name, count),
                        Style::default().fg(FOLDER_COLOR).add_modifier(Modifier::BOLD),
                    ),
                ]))
            }
        }

        DisplayItem::FileEntry { file_idx, depth } => {
            if let Some(file) = s.files.get(*file_idx) {
                let indicator = file.status.indicator();
                let ind_color = match &file.status {
                    FileStatus::Staged | FileStatus::Added => SUCCESS_COLOR,
                    FileStatus::Modified                    => WARNING_COLOR,
                    FileStatus::Untracked                   => BORDER_COLOR,
                    FileStatus::Deleted                     => ERROR_COLOR,
                    FileStatus::Clean                       => BORDER_COLOR, // dim — clean file
                    FileStatus::Unknown                     => BORDER_COLOR,
                    _                                       => FG_COLOR,
                };
                let indent = "  ".repeat(*depth);
                let fname  = file.path.split('/').next_back().unwrap_or(&file.path);
                let icon   = file_icon(fname);

                if selected {
                    ListItem::new(Line::from(Span::styled(
                        format!(" {} {} {} {}", indent, icon, indicator, fname),
                        Style::default().fg(BG_COLOR).bg(ACCENT_COLOR).add_modifier(Modifier::BOLD),
                    )))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!(" {} {} ", indent, icon),
                            Style::default().fg(BORDER_COLOR),
                        ),
                        Span::styled(
                            format!("{} ", indicator),
                            Style::default().fg(ind_color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(fname.to_string(), Style::default().fg(FG_COLOR)),
                    ]))
                }
            } else {
                ListItem::new("")
            }
        }
    }
}

// ── Task 4: Diff panel with minimal readable theme ───────────────────────────

fn render_diff(f: &mut Frame, area: Rect, s: &RepoViewState) {
    // Task 4: split diff into header (file/hunk info) + body (code lines)
    let _inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1)])
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
    } else if s.diff_content.is_empty() || s.diff_content == "(no changes)" {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No changes in this file.",
                Style::default().fg(BORDER_COLOR).add_modifier(Modifier::ITALIC),
            )),
        ]
    } else {
        render_diff_lines(s.diff_content)
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
            .wrap(ratatui::widgets::Wrap { trim: false }),
        area,
    );
}

/// Diff theme renderer — all colors sourced from src/ui/theme.rs.
/// To change any color: edit the corresponding SYN_* or DIFF_* constant in theme.rs, rebuild.
fn render_diff_lines(content: &str) -> Vec<Line<'static>> {

    // Short aliases pointing to theme.rs public constants
    let add_bg    = DIFF_ADD_BG;
    let add_fg    = DIFF_ADD_FG;
    let add_sym   = DIFF_ADD_SYM;
    let del_bg    = DIFF_DEL_BG;
    let del_fg    = DIFF_DEL_FG;
    let del_sym   = DIFF_DEL_SYM;
    let hunk_fg   = DIFF_HUNK_FG;
    let hunk_bg   = DIFF_HUNK_BG;
    let file_fg   = DIFF_FILE_FG;
    let ctx_fg    = DIFF_CTX_FG;
    let gutter_fg = DIFF_GUTTER_FG;
    let meta_fg   = DIFF_META_FG;
    let kw_fg     = SYN_KEYWORD;
    let ty_fg     = SYN_TYPE;
    let fn_fg     = SYN_FUNCTION;
    let st_fg     = SYN_STRING;
    let cm_fg     = SYN_COMMENT;
    let nu_fg     = SYN_NUMBER;
    let mc_fg     = SYN_MACRO;
    let at_fg     = SYN_ATTRIBUTE;
    let op_fg     = SYN_OPERATOR;

    // Rust keyword list
    let keywords: &[&str] = &[
        "pub", "fn", "use", "let", "mut", "const", "static", "struct", "enum",
        "impl", "trait", "type", "where", "for", "in", "if", "else", "match",
        "return", "self", "Self", "super", "crate", "mod", "ref", "async",
        "await", "loop", "while", "break", "continue", "true", "false",
        "unsafe", "extern", "dyn", "move", "box",
    ];
    let builtin_types: &[&str] = &[
        "Option", "Result", "String", "Vec", "HashMap", "HashSet",
        "PathBuf", "Path", "Box", "Arc", "Rc", "Mutex", "bool",
        "u8","u16","u32","u64","u128","usize",
        "i8","i16","i32","i64","i128","isize",
        "f32","f64","str","char",
    ];

    // Colors bundled for passing into inner fns (avoids closure capture issues)
    #[allow(clippy::too_many_arguments)]
    fn classify_token(
        token: &str, kws: &[&str], tys: &[&str],
        kw: ratatui::style::Color, ty: ratatui::style::Color,
        fn_c: ratatui::style::Color, st: ratatui::style::Color,
        cm: ratatui::style::Color, nu: ratatui::style::Color,
        mc: ratatui::style::Color, at: ratatui::style::Color,
        op: ratatui::style::Color, ctx: ratatui::style::Color,
    ) -> ratatui::style::Color {
        if token.starts_with("//") { return cm; }
        if token.starts_with('"') || token.ends_with('"') { return st; }
        if token.starts_with('#') { return at; }
        if token.ends_with('!') { return mc; }
        let bare = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if kws.contains(&bare) { return kw; }
        if tys.contains(&bare) { return ty; }
        if token.ends_with("()") || token.ends_with('(') { return fn_c; }
        if token.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) { return nu; }
        if ["=>", "->", "!=", "==", "&&", "||", "|", "&"].contains(&token) { return op; }
        let _ = (fn_c, nu); // suppress unused warnings
        ctx
    }

    #[allow(clippy::too_many_arguments)]
    fn build_spans(
        code: &str,
        base_fg: ratatui::style::Color,
        base_bg: Option<ratatui::style::Color>,
        kws: &[&str], tys: &[&str],
        kw: ratatui::style::Color, ty: ratatui::style::Color,
        fn_c: ratatui::style::Color, st: ratatui::style::Color,
        cm: ratatui::style::Color, nu: ratatui::style::Color,
        mc: ratatui::style::Color, at: ratatui::style::Color,
        op: ratatui::style::Color, ctx: ratatui::style::Color,
    ) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let apply_bg = |s: Style| if let Some(bg) = base_bg { s.bg(bg) } else { s };

        let trimmed = code.trim_start();
        if trimmed.starts_with("//") {
            spans.push(Span::styled(code.to_string(),
                apply_bg(Style::default().fg(cm).add_modifier(Modifier::ITALIC))));
            return spans;
        }
        if trimmed.starts_with("#[") || trimmed.starts_with("#!") {
            spans.push(Span::styled(code.to_string(), apply_bg(Style::default().fg(at))));
            return spans;
        }
        if let Some(sq) = code.find('"') {
            if sq > 0 {
                spans.extend(build_spans(&code[..sq], base_fg, base_bg,
                    kws, tys, kw, ty, fn_c, st, cm, nu, mc, at, op, ctx));
            }
            let rest = &code[sq+1..];
            let end = rest.find('"').map(|e| sq + 1 + e + 1).unwrap_or(code.len());
            spans.push(Span::styled(code[sq..end].to_string(), apply_bg(Style::default().fg(st))));
            if end < code.len() {
                spans.extend(build_spans(&code[end..], base_fg, base_bg,
                    kws, tys, kw, ty, fn_c, st, cm, nu, mc, at, op, ctx));
            }
            return spans;
        }
        let mut current = String::new();
        for ch in code.chars() {
            if ch.is_alphanumeric() || ch == '_' || ch == '!' {
                current.push(ch);
            } else {
                if !current.is_empty() {
                    let col = classify_token(&current, kws, tys,
                        kw, ty, fn_c, st, cm, nu, mc, at, op, ctx);
                    spans.push(Span::styled(current.clone(), apply_bg(Style::default().fg(col))));
                    current.clear();
                }
                let op_col = match ch {
                    '=' | '>' | '<' | '|' | '&' | '!' | '+' | '-' | '*' | '/' => op,
                    _ => base_fg,
                };
                spans.push(Span::styled(ch.to_string(), apply_bg(Style::default().fg(op_col))));
            }
        }
        if !current.is_empty() {
            let col = classify_token(&current, kws, tys,
                kw, ty, fn_c, st, cm, nu, mc, at, op, ctx);
            spans.push(Span::styled(current, apply_bg(Style::default().fg(col))));
        }
        spans
    }

    // Convenience macro to call build_spans with all color args
    macro_rules! syn {
        ($code:expr, $fg:expr, $bg:expr) => {
            build_spans($code, $fg, $bg, keywords, builtin_types,
                kw_fg, ty_fg, fn_fg, st_fg, cm_fg, nu_fg, mc_fg, at_fg, op_fg, ctx_fg)
        };
    }

    // ── Track old/new line numbers across hunks ───────────────────────────────
    let mut old_ln: u32 = 0;
    let mut new_ln: u32 = 0;

    content.lines().map(|raw_line| {
        let line = raw_line.to_string();

        // ── File header (diff --git, ---, +++) ───────────────────────────────
        if line.starts_with("diff ") || line.starts_with("index ")
            || line.starts_with("--- ") || line.starts_with("+++ ")
        {
            return Line::from(vec![
                Span::styled("      ".to_string(), Style::default().fg(gutter_fg)),
                Span::styled("  ".to_string(),      Style::default().fg(gutter_fg)),
                Span::styled(line, Style::default().fg(file_fg).add_modifier(Modifier::BOLD)),
            ]);
        }

        // ── Hunk header (@@ … @@) ────────────────────────────────────────────
        if line.starts_with("@@") {
            // Parse old/new start numbers from @@ -old,n +new,n @@
            let parse = |s: &str, prefix: char| -> u32 {
                s.split_whitespace()
                 .find(|t| t.starts_with(prefix))
                 .and_then(|t| t.trim_start_matches(prefix).split(',').next())
                 .and_then(|n| n.parse::<u32>().ok())
                 .unwrap_or(1)
                 .saturating_sub(1)
            };
            old_ln = parse(&line, '-');
            new_ln = parse(&line, '+');

            return Line::from(vec![
                Span::styled("      ".to_string(), Style::default().fg(gutter_fg).bg(hunk_bg)),
                Span::styled("  ".to_string(),      Style::default().fg(gutter_fg).bg(hunk_bg)),
                Span::styled(line, Style::default().fg(hunk_fg).bg(hunk_bg).add_modifier(Modifier::BOLD)),
            ]);
        }

        // ── "\ No newline at end of file" ────────────────────────────────────
        if line.starts_with('\\') {
            return Line::from(Span::styled(
                line,
                Style::default().fg(meta_fg).add_modifier(Modifier::ITALIC),
            ));
        }

        // ── Added line (+) ────────────────────────────────────────────────────
        if line.starts_with('+') {
            new_ln += 1;
            let code = line[1..].to_string();
            let gutter = format!("{:>3}   ", new_ln);
            let mut spans = vec![
                Span::styled(gutter, Style::default().fg(add_fg).bg(add_bg)),
                Span::styled("+ ".to_string(), Style::default().fg(add_sym).bg(add_bg)
                    .add_modifier(Modifier::BOLD)),
            ];
            spans.extend(syn!(&code, add_fg, Some(add_bg)));
            return Line::from(spans);
        }

        // ── Deleted line (-) ─────────────────────────────────────────────────
        if line.starts_with('-') {
            old_ln += 1;
            let code = line[1..].to_string();
            let gutter = format!("{:>3}   ", old_ln);
            let mut spans = vec![
                Span::styled(gutter, Style::default().fg(del_fg).bg(del_bg)),
                Span::styled("- ".to_string(), Style::default().fg(del_sym).bg(del_bg)
                    .add_modifier(Modifier::BOLD)),
            ];
            spans.extend(syn!(&code, del_fg, Some(del_bg)));
            return Line::from(spans);
        }

        // ── Context line (space) ──────────────────────────────────────────────
        old_ln += 1;
        new_ln += 1;
        let code = if line.starts_with(' ') { line[1..].to_string() } else { line.clone() };
        let gutter = format!("{:>3} {:>3}", old_ln, new_ln);
        let mut spans = vec![
            Span::styled(gutter, Style::default().fg(gutter_fg)),
            Span::styled("   ".to_string(), Style::default().fg(gutter_fg)),
        ];
        spans.extend(syn!(&code, ctx_fg, None));
        Line::from(spans)

    }).collect()
}

// ── Footer ────────────────────────────────────────────────────────────────────

fn render_footer(f: &mut Frame, area: Rect, s: &RepoViewState, editor_mode: bool) {
    use ratatui::widgets::Paragraph;

    if editor_mode {
        f.render_widget(
            Paragraph::new("Ctrl+S Save   Ctrl+X Close   ↑↓←→ Move   Tab Indent   1 Tree focus")
                .style(Style::default().fg(BORDER_COLOR)),
            area,
        );
        return;
    }

    if s.commit_mode {
        f.render_widget(
            Paragraph::new("Type commit message   Enter Confirm   Esc Cancel")
                .style(Style::default().fg(ACCENT_COLOR)),
            area,
        );
        return;
    }

    // Task 2 fix: always show keybinds. If there's a status_msg, show it on the
    // same line using colour only — keybinds are always visible.
    let keybinds = "↑↓ Nav   Enter Expand   Space Stage   s StageAll   e Edit   c Commit   p Push   P Pull   r Refresh   X Deinit   q Quit";

    let (text, color) = if !s.status_msg.is_empty() {
        let c = if s.status_msg.contains('✅') { SUCCESS_COLOR }
                else if s.status_msg.contains('❌') { ERROR_COLOR }
                else { WARNING_COLOR };
        // Show status msg, keybinds in dimmer colour after a separator
        (format!("{}   │   {}", s.status_msg, keybinds), c)
    } else {
        (keybinds.to_string(), BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(color)),
        area,
    );
}
