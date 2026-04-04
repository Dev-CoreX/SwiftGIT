use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{render_folder_input, components::toast::ToastType};
use async_trait::async_trait;

pub struct FolderInputContext;

#[async_trait]
impl Context for FolderInputContext {
    fn view_name(&self) -> &str {
        "folder_input"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        render_folder_input(
            f,
            f.size(),
            &model.text_input,
            model.frame_count,
            &model.dir_suggestions,
            model.suggestion_cursor,
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc => {
                s.mode = AppMode::Dashboard;
                // In a true context-driven system, we'd pop. 
                // But for now, we're still integrating with AppMode.
                return Ok(true); 
            }
            KeyCode::Tab => {
                if s.dir_suggestions.is_empty() {
                    s.update_dir_suggestions();
                }
                if !s.dir_suggestions.is_empty() {
                    let next = match s.suggestion_cursor {
                        None => 0,
                        Some(i) => (i + 1) % s.dir_suggestions.len(),
                    };
                    s.suggestion_cursor = Some(next);
                    if let Some(suggestion) = s.dir_suggestions.get(next) {
                        s.text_input = suggestion.clone();
                        s.text_cursor = s.text_input.chars().count();
                    }
                }
            }
            KeyCode::Up => {
                if !s.dir_suggestions.is_empty() {
                    let next = match s.suggestion_cursor {
                        None | Some(0) => s.dir_suggestions.len().saturating_sub(1),
                        Some(i) => i - 1,
                    };
                    s.suggestion_cursor = Some(next);
                    if let Some(suggestion) = s.dir_suggestions.get(next) {
                        s.text_input = suggestion.clone();
                        s.text_cursor = s.text_input.chars().count();
                    }
                }
            }
            KeyCode::Down => {
                if !s.dir_suggestions.is_empty() {
                    let next = match s.suggestion_cursor {
                        None => 0,
                        Some(i) => (i + 1) % s.dir_suggestions.len(),
                    };
                    s.suggestion_cursor = Some(next);
                    if let Some(suggestion) = s.dir_suggestions.get(next) {
                        s.text_input = suggestion.clone();
                        s.text_cursor = s.text_input.chars().count();
                    }
                }
            }
            KeyCode::Enter => {
                let raw = s.text_input.trim().to_string();
                if raw.is_empty() { 
                    s.show_toast("Please enter a path", ToastType::Warning); 
                    return Ok(false); 
                }
                let path = if raw.starts_with('~') {
                    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(&raw[2..])
                } else {
                    PathBuf::from(&raw)
                };
                if !path.exists() {
                    s.show_toast(format!("Path not found: {}", path.display()), ToastType::Error);
                    return Ok(false);
                }
                s.dir_suggestions.clear();
                s.suggestion_cursor = None;
                match s.open_repo(path) {
                    Ok(was_init) => { 
                        if was_init { s.show_toast("Initialized new git repo!", ToastType::Success); } 
                    }
                    Err(e) => s.show_toast(format!("Error: {}", e), ToastType::Error),
                }
            }
            KeyCode::Backspace => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    s.text_input.clear(); s.text_cursor = 0;
                } else if s.text_cursor > 0 {
                    let bp = s.text_input.char_indices().nth(s.text_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                    s.text_input.remove(bp); s.text_cursor -= 1;
                }
                s.update_dir_suggestions();
            }
            KeyCode::Left => { if s.text_cursor > 0 { s.text_cursor -= 1; } }
            KeyCode::Right => { if s.text_cursor < s.text_input.chars().count() { s.text_cursor += 1; } }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let bp = s.text_input.char_indices().nth(s.text_cursor).map(|(i,_)| i).unwrap_or(s.text_input.len());
                s.text_input.insert(bp, c); s.text_cursor += 1;
                s.update_dir_suggestions();
            }
            _ => {}
        }
        Ok(false)
    }
}
