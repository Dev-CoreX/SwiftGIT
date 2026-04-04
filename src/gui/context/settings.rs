use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::Model;
use crate::ui::components::{settings_dialog, dashboard};
use crate::ui::components::settings_dialog::SettingsField;
use async_trait::async_trait;

pub struct SettingsContext;

#[async_trait]
impl Context for SettingsContext {
    fn view_name(&self) -> &str {
        "settings"
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

        settings_dialog::render(f, f.size(), &model.settings_dlg, model.frame_count);
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        
        // Global Ctrl+W to close
        if event.code == KeyCode::Char('w') && event.modifiers.contains(KeyModifiers::CONTROL) {
            let prev = s.settings_prev_mode.clone();
            s.mode = prev;
            return Ok(true);
        }

        match event.code {
            KeyCode::Esc => {
                let prev = s.settings_prev_mode.clone();
                s.mode = prev;
            }
            KeyCode::Enter => {
                let name  = s.settings_dlg.display_name.trim().to_string();
                let user  = s.settings_dlg.username.trim().to_string();
                let token = s.settings_dlg.token.trim().to_string();

                s.config.display_name = if name.is_empty() { None } else { Some(name) };
                s.config.username     = if user.is_empty() { None } else { Some(user) };
                s.config.github_token = if token.is_empty() { None } else { Some(token) };
                let _ = s.config.save();

                let prev = s.settings_prev_mode.clone();
                s.mode = prev;
            }
            KeyCode::Tab | KeyCode::Down => {
                let next = s.settings_dlg.focused.next();
                s.settings_dlg.focused = next;
                s.settings_dlg.cursor = s.settings_dlg.active_field_text().chars().count();
                if s.settings_dlg.focused == SettingsField::Token {
                    s.settings_dlg.show_token = false;
                }
            }
            KeyCode::BackTab | KeyCode::Up => {
                let prev = s.settings_dlg.focused.prev();
                s.settings_dlg.focused = prev;
                s.settings_dlg.cursor = s.settings_dlg.active_field_text().chars().count();
                if s.settings_dlg.focused == SettingsField::Token {
                    s.settings_dlg.show_token = false;
                }
            }
            KeyCode::Char('v') if event.modifiers.contains(KeyModifiers::CONTROL) => {
                if s.settings_dlg.focused == SettingsField::Token {
                    s.settings_dlg.show_token = !s.settings_dlg.show_token;
                } else {
                    let prev = s.settings_dlg.focused.prev();
                    s.settings_dlg.focused = prev;
                    s.settings_dlg.cursor = s.settings_dlg.active_field_text().chars().count();
                }
            }
            KeyCode::Left  => { if s.settings_dlg.cursor > 0 { s.settings_dlg.cursor -= 1; } }
            KeyCode::Right => {
                let len = s.settings_dlg.active_field_text().chars().count();
                if s.settings_dlg.cursor < len { s.settings_dlg.cursor += 1; }
            }
            KeyCode::Backspace => { s.settings_dlg.backspace(); }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                s.settings_dlg.type_char(c);
            }
            _ => {}
        }
        Ok(true)
    }
}
