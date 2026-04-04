use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::components::{dashboard, toast::ToastType};
use async_trait::async_trait;

pub struct DashboardContext;

#[async_trait]
impl Context for DashboardContext {
    fn view_name(&self) -> &str {
        "dashboard"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        dashboard::render(
            f,
            f.size(),
            model.dashboard_cursor,
            model.config.display_name.as_deref().or(model.config.username.as_deref()),
            &model.config.recent_projects,
            &model.status_msg,
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut model = model.lock().await;
        match event.code {
            KeyCode::Up => {
                if model.dashboard_cursor > 0 {
                    model.dashboard_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if model.dashboard_cursor < dashboard::MENU_ITEMS.len() - 1 {
                    model.dashboard_cursor += 1;
                }
            }
            KeyCode::Enter => {
                model.status_msg.clear();
                match model.dashboard_cursor {
                    0 => {
                        model.text_input.clear(); 
                        model.text_cursor = 0;
                        model.dir_suggestions.clear(); 
                        model.suggestion_cursor = None;
                        model.mode = AppMode::FolderInput;
                    }
                    1 => {
                        model.text_input.clear(); 
                        model.text_cursor = 0;
                        model.mode = AppMode::CloneInput;
                    }
                    2 => {
                        if !model.config.recent_projects.is_empty() {
                            model.recent_projects_cursor = 0;
                            model.mode = AppMode::RecentProjects;
                        } else {
                            model.show_toast("No recent projects yet", ToastType::Info);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }
}
