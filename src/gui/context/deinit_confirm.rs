use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::repo_view, components::toast::ToastType, render_deinit_confirm};
use async_trait::async_trait;

pub struct DeinitConfirmContext;

#[async_trait]
impl Context for DeinitConfirmContext {
    fn view_name(&self) -> &str {
        "deinit_confirm"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let repo_name = model.repo.as_ref().map(|r| r.repo_name()).unwrap_or_default();
        let view_state = repo_view::RepoViewState {
            repo_name: &repo_name,
            branch: &model.branch,
            files: &model.git_files,
            display_items: &model.display_items,
            cursor: model.repo_cursor,
            scroll_offset: model.repo_scroll,
            diff_scroll: model.diff_scroll,
            diff_content: &model.diff_content,
            diff_struct: &model.diff_struct,
            hunk_cursor: model.hunk_cursor,
            commit_mode: false,
            commit_input: "",
            commit_cursor: 0,
            commit_history: &model.commit_history,
            status_msg: &model.status_msg,
            is_loading: false,
            is_diff_loading: false,
            frame_count: model.frame_count,
            active_frame: model.active_frame as u8,
        };
        repo_view::render(f, f.size(), &view_state);
        render_deinit_confirm(f, f.size(), model.deinit_confirm_cursor);
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.mode = AppMode::RepoView;
                s.show_toast("Cancelled", ToastType::Info);
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::BackTab => {
                s.deinit_confirm_cursor = 0; // Cancel
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                s.deinit_confirm_cursor = 1; // Deinit
            }
            KeyCode::Enter => {
                if s.deinit_confirm_cursor == 1 {
                    // Confirmed — remove .git
                    if let Some(repo) = &s.repo {
                        match repo.deinit() {
                            Ok(_) => {
                                s.repo = None;
                                s.git_files.clear();
                                s.display_items.clear();
                                s.branch.clear();
                                s.diff_content.clear();
                                s.mode = AppMode::Dashboard;
                                s.status_msg.clear();
                                s.show_toast(
                                    "✅ Deinitialized — .git removed. Folder still intact.",
                                    ToastType::Success,
                                );
                            }
                            Err(e) => {
                                s.mode = AppMode::RepoView;
                                s.show_toast(
                                    format!("❌ Deinit failed: {}", e),
                                    ToastType::Error,
                                );
                            }
                        }
                    }
                } else {
                    s.mode = AppMode::RepoView;
                    s.show_toast("Cancelled", ToastType::Info);
                }
            }
            _ => {}
        }
        Ok(true)
    }
}
