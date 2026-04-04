use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::components::searching;
use async_trait::async_trait;

pub struct LoadingContext;

#[async_trait]
impl Context for LoadingContext {
    fn view_name(&self) -> &str {
        "loading"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        searching::render(f, f.size(), model.frame_count, &model.loading_label);
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        if event.code == KeyCode::Esc {
            model.lock().await.mode = AppMode::Dashboard;
            return Ok(true);
        }
        Ok(false)
    }
}
