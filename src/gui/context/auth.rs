use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::auth, components::toast::ToastType, validate_github_token};
use async_trait::async_trait;

pub struct AuthContext;

#[async_trait]
impl Context for AuthContext {
    fn view_name(&self) -> &str {
        "auth"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let cv = (model.frame_count / 5) % 2 == 0;
        auth::render(
            f, 
            f.size(), 
            &model.token_input, 
            model.token_cursor,
            &model.auth_status, 
            cv, 
            model.is_validating
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
            KeyCode::Enter => {
                let token = s.token_input.trim().to_string();
                if token.is_empty() { 
                    s.mode = AppMode::Dashboard; 
                    return Ok(true); 
                }
                s.is_validating = true;
                s.auth_status = "Validating...".to_string();
                
                // We need to drop the lock before awaiting an async function that might take time
                // or if we want to allow the UI to continue rendering.
                // But validate_github_token is a simple network call.
                // For better responsiveness, we could spawn a task.
                drop(s);
                let res = validate_github_token(&token).await;
                let mut s = model.lock().await;
                
                match res {
                    Ok(username) => {
                        s.auth_status = format!("Welcome, {}!", username);
                        s.config.github_token = Some(token);
                        s.config.username = Some(username.clone());
                        let _ = s.config.save();
                        s.show_toast(format!("Logged in as @{}", username), ToastType::Success);
                        s.mode = AppMode::Dashboard;
                    }
                    Err(e) => { 
                        s.auth_status = format!("Invalid token: {}", e); 
                    }
                }
                s.is_validating = false;
                return Ok(true);
            }
            KeyCode::Backspace => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    s.token_input.clear(); s.token_cursor = 0;
                } else if s.token_cursor > 0 {
                    let bp = s.token_input.char_indices().nth(s.token_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                    s.token_input.remove(bp); s.token_cursor -= 1;
                }
            }
            KeyCode::Left => { if s.token_cursor > 0 { s.token_cursor -= 1; } }
            KeyCode::Right => { if s.token_cursor < s.token_input.chars().count() { s.token_cursor += 1; } }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let bp = s.token_input.char_indices().nth(s.token_cursor).map(|(i,_)| i).unwrap_or(s.token_input.len());
                s.token_input.insert(bp, c); s.token_cursor += 1;
            }
            _ => {}
        }
        Ok(false)
    }
}
