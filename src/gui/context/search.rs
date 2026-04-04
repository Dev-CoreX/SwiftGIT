use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::render_text_input;
use async_trait::async_trait;

pub struct SearchContext;

#[async_trait]
impl Context for SearchContext {
    fn view_name(&self) -> &str {
        "search"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        render_text_input(
            f, 
            f.size(), 
            "Filter Files",
            "type to filter...", 
            &model.filter, 
            model.frame_count
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc => {
                s.filter.clear();
                s.apply_filter();
                s.mode = AppMode::RepoView;
                return Ok(true);
            }
            KeyCode::Enter => {
                s.mode = AppMode::RepoView;
                return Ok(true);
            }
            KeyCode::Backspace => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    s.filter.clear();
                } else {
                    s.filter.pop();
                }
                s.apply_filter();
            }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                s.filter.push(c);
                s.apply_filter();
            }
            _ => {}
        }
        Ok(false)
    }
}
