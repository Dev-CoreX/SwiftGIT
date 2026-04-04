use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::Model;
use crate::ui::components::help_dialog;
use async_trait::async_trait;

pub struct HelpContext;

#[async_trait]
impl Context for HelpContext {
    fn view_name(&self) -> &str {
        "help"
    }

    fn render(&self, f: &mut Frame, _model: &Model) -> Result<()> {
        help_dialog::render(f, f.size());
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                let prev = s.help_prev_mode.clone();
                s.mode = prev;
                return Ok(true);
            }
            _ => {}
        }
        Ok(true) // Consume all events while help is open
    }
}
