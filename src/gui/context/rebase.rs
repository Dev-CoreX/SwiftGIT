use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::theme;
use async_trait::async_trait;

pub struct RebaseContext;

#[async_trait]
impl Context for RebaseContext {
    fn view_name(&self) -> &str {
        "rebase"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let area = f.size();
        
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // List
                Constraint::Length(3), // Controls
            ])
            .split(area);

        // Title
        let title = Paragraph::new(" 🔀 Interactive Rebase (Simulated) ")
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme::ACCENT_COLOR)))
            .style(Style::default().fg(theme::ACCENT_COLOR).add_modifier(Modifier::BOLD));
        f.render_widget(title, vertical[0]);

        // List of commits
        let items: Vec<ListItem> = model.rebase_commits.iter().enumerate().map(|(i, c)| {
            let selected = i == model.rebase_cursor;
            let action_color = match c.action.as_str() {
                "pick" => Color::Green,
                "drop" => Color::Red,
                "edit" | "reword" => Color::Yellow,
                "fixup" | "squash" => Color::Cyan,
                _ => Color::White,
            };

            let mut style = Style::default();
            if selected {
                style = style.bg(theme::HIGHLIGHT_BG).fg(theme::ACCENT_COLOR);
            }

            let line = Line::from(vec![
                Span::styled(format!(" {:<7} ", c.action), Style::default().fg(action_color).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", c.sha), Style::default().fg(Color::DarkGray)),
                Span::styled(c.message.clone(), Style::default()),
            ]);

            ListItem::new(line).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(list, vertical[1]);

        // Controls
        let controls = Paragraph::new(" ↑↓ Navigate  •  p pick  •  d drop  •  r reword  •  f fixup  •  Ctrl+j/k Move  •  Enter Confirm  •  Esc Cancel ")
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme::BORDER_COLOR)))
            .style(Style::default().fg(theme::BORDER_COLOR));
        f.render_widget(controls, vertical[2]);

        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        
        match event.code {
            KeyCode::Esc => {
                s.mode = AppMode::RepoView;
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    // Move commit up
                    let idx = s.rebase_cursor;
                    if idx > 0 {
                        s.rebase_commits.swap(idx, idx - 1);
                        s.rebase_cursor -= 1;
                    }
                } else if s.rebase_cursor > 0 {
                    s.rebase_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = s.rebase_commits.len().saturating_sub(1);
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    // Move commit down
                    let idx = s.rebase_cursor;
                    if idx < max {
                        s.rebase_commits.swap(idx, idx + 1);
                        s.rebase_cursor += 1;
                    }
                } else if s.rebase_cursor < max {
                    s.rebase_cursor += 1;
                }
            }
            KeyCode::Char('p') => {
                let idx = s.rebase_cursor;
                if let Some(c) = s.rebase_commits.get_mut(idx) { c.action = "pick".to_string(); }
            }
            KeyCode::Char('d') => {
                let idx = s.rebase_cursor;
                if let Some(c) = s.rebase_commits.get_mut(idx) { c.action = "drop".to_string(); }
            }
            KeyCode::Char('r') => {
                let idx = s.rebase_cursor;
                if let Some(c) = s.rebase_commits.get_mut(idx) { c.action = "reword".to_string(); }
            }
            KeyCode::Char('f') => {
                let idx = s.rebase_cursor;
                if let Some(c) = s.rebase_commits.get_mut(idx) { c.action = "fixup".to_string(); }
            }
            KeyCode::Enter => {
                s.show_toast("Rebase applied (simulated)", crate::ui::components::toast::ToastType::Success);
                s.mode = AppMode::RepoView;
                return Ok(true);
            }
            _ => {}
        }
        
        Ok(false)
    }
}
