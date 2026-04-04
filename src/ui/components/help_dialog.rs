use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::ui::theme::*;

pub fn render(f: &mut Frame, area: Rect) {
    let mw = 70u16.min(area.width.saturating_sub(4));
    let mh = 24u16.min(area.height.saturating_sub(4));
    let mx = area.x + (area.width.saturating_sub(mw)) / 2;
    let my = area.y + (area.height.saturating_sub(mh)) / 2;
    let modal = Rect::new(mx, my, mw, mh);

    f.render_widget(Clear, modal);

    let items = vec![
        build_item(" Global ", ""),
        build_item("  ? ", "Show this help"),
        build_item("  q / Ctrl+C ", "Quit"),
        build_item("  Ctrl+W ", "Settings"),
        build_item("  1 / 2 ", "Switch active frame (Left/Right)"),
        build_item("", ""),
        build_item(" Navigation ", ""),
        build_item("  ↑ / ↓ / j / k ", "Move cursor"),
        build_item("  Enter ", "Expand/Collapse folder or Refresh Diff"),
        build_item("  Esc ", "Back to Dashboard"),
        build_item("", ""),
        build_item(" Repository Operations ", ""),
        build_item("  Space ", "Stage/Unstage file or folder"),
        build_item("  s ", "Stage ALL changes"),
        build_item("  c ", "Commit staged changes"),
        build_item("  p ", "Push to remote"),
        build_item("  P ", "Pull from remote"),
        build_item("  r ", "Refresh status"),
        build_item("  X ", "Deinitialize repository"),
        build_item("  / ", "Filter files"),
        build_item("  i ", "Interactive Rebase (simulated)"),
        build_item("", ""),
        build_item(" Editor ", ""),
        build_item("  e ", "Open file in editor"),
        build_item("  Ctrl+S ", "Save file"),
        build_item("  Ctrl+Q / Ctrl+X ", "Close editor (checks for unsaved)"),
        build_item("  Esc ", "Discard changes and close"),
        build_item("", ""),
        build_item(" Clone Screen ", ""),
        build_item("  Ctrl+G ", "Auto-fill 'https://github.com/'"),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(" 📖 Help & Keybindings ", Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD)))
            .border_style(Style::default().fg(ACCENT_COLOR))
            .style(Style::default().bg(BG_COLOR)),
    );

    f.render_widget(list, modal);
}

fn build_item(key: &str, desc: &str) -> ListItem<'static> {
    if desc.is_empty() {
        ListItem::new(Line::from(vec![
            Span::styled(key.to_string(), Style::default().fg(ACCENT_COLOR).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:<15}", key), Style::default().fg(SUCCESS_COLOR)),
            Span::styled(desc.to_string(), Style::default().fg(FG_COLOR)),
        ]))
    }
}
