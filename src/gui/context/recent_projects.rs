use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{render_recent_projects_dialog, components::toast::ToastType, components::dashboard};
use async_trait::async_trait;

pub struct RecentProjectsContext;

#[async_trait]
impl Context for RecentProjectsContext {
    fn view_name(&self) -> &str {
        "recent_projects"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        // Draw dashboard as background
        let shown_name = model.config.display_name.as_deref().or(model.config.username.as_deref());
        dashboard::render(
            f, 
            f.size(), 
            model.dashboard_cursor,
            shown_name, 
            &model.config.recent_projects, 
            &model.status_msg
        );
        
        render_recent_projects_dialog(
            f, 
            f.size(), 
            &model.config.recent_projects, 
            model.recent_projects_cursor
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc => {
                s.mode = AppMode::Dashboard;
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.recent_projects_cursor > 0 {
                    s.recent_projects_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = s.config.recent_projects.len().saturating_sub(1);
                if s.recent_projects_cursor < max {
                    s.recent_projects_cursor += 1;
                }
            }
            KeyCode::Enter => {
                let path = s.config.recent_projects.get(s.recent_projects_cursor).map(|p| PathBuf::from(&p.path));
                if let Some(p) = path {
                    match s.open_repo(p) {
                        Ok(_) => {
                            s.show_toast("Opened project", ToastType::Success);
                            drop(s);
                            Model::async_refresh_status(Arc::clone(&model)).await;
                            return Ok(true);
                        }
                        Err(e) => s.show_toast(format!("Error: {}", e), ToastType::Error),
                    }
                }
            }
            KeyCode::Char('d') => {
                if !s.config.recent_projects.is_empty() {
                    let idx = s.recent_projects_cursor;
                    s.config.recent_projects.remove(idx);
                    let _ = s.config.save();
                    let max = s.config.recent_projects.len().saturating_sub(1);
                    if s.recent_projects_cursor > max {
                        s.recent_projects_cursor = max;
                    }
                    if s.config.recent_projects.is_empty() {
                        s.mode = AppMode::Dashboard;
                    }
                }
            }
            KeyCode::Char('C') => {
                s.config.recent_projects.clear();
                let _ = s.config.save();
                s.recent_projects_cursor = 0;
                s.mode = AppMode::Dashboard;
            }
            _ => {}
        }
        Ok(false)
    }
}
