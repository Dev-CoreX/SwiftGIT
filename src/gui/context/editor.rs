use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::editor, components::toast::ToastType};
use async_trait::async_trait;

pub struct EditorContext;

#[async_trait]
impl Context for EditorContext {
    fn view_name(&self) -> &str {
        "editor"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let editor_state = editor::EditorState {
            lines: &model.editor_lines,
            cursor_line: model.editor_cursor_line,
            cursor_col: model.editor_cursor_col,
            scroll_top: model.editor_scroll_line,
            file_path: &model.editor_path,
            modified: model.editor_modified,
            frame_count: model.frame_count,
        };
        editor::render(f, f.size(), &editor_state);
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        
        // Ctrl shortcuts
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Char('s') => { s.editor_save(); return Ok(true); }
                KeyCode::Char('q') | KeyCode::Char('x') => {
                    if s.editor_modified {
                        s.show_toast("Unsaved changes — Ctrl+S to save, or Esc to discard and exit",
                            ToastType::Warning);
                    } else {
                        s.mode = AppMode::RepoView;
                        s.active_frame = 1;
                    }
                    return Ok(true);
                }
                _ => {}
            }
        }

        match event.code {
            KeyCode::Esc => {
                s.mode = AppMode::RepoView;
                s.active_frame = 1;
                s.editor_modified = false;
                return Ok(true);
            }

            KeyCode::Up => {
                if s.editor_cursor_line > 0 {
                    s.editor_cursor_line -= 1;
                    let cursor_line = s.editor_cursor_line;
                    let line_len = s.editor_lines.get(cursor_line)
                        .map(|l| l.chars().count()).unwrap_or(0);
                    if s.editor_cursor_col > line_len { s.editor_cursor_col = line_len; }
                    s.editor_adjust_scroll();
                }
            }
            KeyCode::Down => {
                if s.editor_cursor_line + 1 < s.editor_lines.len() {
                    s.editor_cursor_line += 1;
                    let cursor_line = s.editor_cursor_line;
                    let line_len = s.editor_lines.get(cursor_line)
                        .map(|l| l.chars().count()).unwrap_or(0);
                    if s.editor_cursor_col > line_len { s.editor_cursor_col = line_len; }
                    s.editor_adjust_scroll();
                }
            }
            KeyCode::Left => {
                if s.editor_cursor_col > 0 {
                    s.editor_cursor_col -= 1;
                } else if s.editor_cursor_line > 0 {
                    s.editor_cursor_line -= 1;
                    let cursor_line = s.editor_cursor_line;
                    s.editor_cursor_col = s.editor_lines.get(cursor_line)
                        .map(|l| l.chars().count()).unwrap_or(0);
                    s.editor_adjust_scroll();
                }
            }
            KeyCode::Right => {
                let cursor_line = s.editor_cursor_line;
                let line_len = s.editor_lines.get(cursor_line)
                    .map(|l| l.chars().count()).unwrap_or(0);
                if s.editor_cursor_col < line_len {
                    s.editor_cursor_col += 1;
                } else if s.editor_cursor_line + 1 < s.editor_lines.len() {
                    s.editor_cursor_line += 1;
                    s.editor_cursor_col = 0;
                    s.editor_adjust_scroll();
                }
            }
            KeyCode::Home => { s.editor_cursor_col = 0; }
            KeyCode::End  => {
                let cursor_line = s.editor_cursor_line;
                s.editor_cursor_col = s.editor_lines.get(cursor_line)
                    .map(|l| l.chars().count()).unwrap_or(0);
            }

            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let col = s.editor_cursor_col;
                let cursor_line = s.editor_cursor_line;
                if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                    let byte_pos = line.char_indices()
                        .nth(col)
                        .map(|(i, _)| i)
                        .unwrap_or(line.len());
                    line.insert(byte_pos, c);
                }
                s.editor_cursor_col += 1;
                s.editor_modified = true;
            }

            KeyCode::Backspace => {
                let col = s.editor_cursor_col;
                let cursor_line = s.editor_cursor_line;
                if col > 0 {
                    if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                        let byte_pos = line.char_indices()
                            .nth(col - 1)
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        line.remove(byte_pos);
                        s.editor_cursor_col -= 1;
                        s.editor_modified = true;
                    }
                } else if cursor_line > 0 {
                    let current_line = s.editor_lines.remove(cursor_line);
                    s.editor_cursor_line -= 1;
                    let new_cursor_line = s.editor_cursor_line;
                    if let Some(prev_line) = s.editor_lines.get_mut(new_cursor_line) {
                        let prev_len = prev_line.chars().count();
                        prev_line.push_str(&current_line);
                        s.editor_cursor_col = prev_len;
                        s.editor_modified = true;
                        s.editor_adjust_scroll();
                    }
                }
            }

            KeyCode::Delete => {
                let col = s.editor_cursor_col;
                let cursor_line = s.editor_cursor_line;
                let line_len = s.editor_lines.get(cursor_line).map(|l| l.chars().count()).unwrap_or(0);
                if col < line_len {
                    if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                        let byte_pos = line.char_indices()
                            .nth(col)
                            .map(|(i, _)| i)
                            .unwrap_or(line.len());
                        line.remove(byte_pos);
                        s.editor_modified = true;
                    }
                } else if cursor_line + 1 < s.editor_lines.len() {
                    let next = s.editor_lines.remove(cursor_line + 1);
                    if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                        line.push_str(&next);
                        s.editor_modified = true;
                    }
                }
            }

            KeyCode::Enter => {
                let col = s.editor_cursor_col;
                let cursor_line = s.editor_cursor_line;
                let rest = if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                    let byte_pos = line.char_indices()
                        .nth(col)
                        .map(|(i, _)| i)
                        .unwrap_or(line.len());
                    let tail = line[byte_pos..].to_string();
                    line.truncate(byte_pos);
                    tail
                } else {
                    String::new()
                };
                s.editor_cursor_line += 1;
                let new_cursor_line = s.editor_cursor_line;
                s.editor_lines.insert(new_cursor_line, rest);
                s.editor_cursor_col = 0;
                s.editor_modified = true;
                s.editor_adjust_scroll();
            }

            KeyCode::Tab => {
                let col = s.editor_cursor_col;
                let cursor_line = s.editor_cursor_line;
                if let Some(line) = s.editor_lines.get_mut(cursor_line) {
                    let byte_pos = line.char_indices()
                        .nth(col)
                        .map(|(i, _)| i)
                        .unwrap_or(line.len());
                    line.insert_str(byte_pos, "    ");
                    s.editor_cursor_col += 4;
                    s.editor_modified = true;
                }
            }

            KeyCode::F(1) => { s.mode = AppMode::RepoView; s.active_frame = 1; }

            _ => {}
        }
        Ok(false)
    }
}
