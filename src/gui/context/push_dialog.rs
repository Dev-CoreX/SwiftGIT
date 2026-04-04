use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::push_dialog, components::repo_view, components::toast::ToastType, render_force_push_confirm, spinner_char};
use crate::ui::components::push_dialog::PushField;
use async_trait::async_trait;

pub struct PushDialogContext;

#[async_trait]
impl Context for PushDialogContext {
    fn view_name(&self) -> &str {
        "push_dialog"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let repo_name = model.repo.as_ref()
            .map(|r| r.repo_name()).unwrap_or_else(|| "repo".to_string());
        
        // Background RepoView
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
            commit_mode: model.commit_mode,
            commit_input: &model.commit_input,
            commit_cursor: model.commit_cursor,
            commit_history: &model.commit_history,
            status_msg: &model.status_msg,
            is_loading: model.is_loading,
            is_diff_loading: model.is_diff_loading,
            frame_count: model.frame_count,
            active_frame: model.active_frame as u8,
        };
        repo_view::render(f, f.size(), &view_state);

        let mut dlg = model.push_dlg.clone();
        dlg.frame_count = model.frame_count;
        dlg.is_pushing = model.is_loading;
        push_dialog::render(f, f.size(), &dlg);

        if model.mode == AppMode::ForcePushConfirm {
            render_force_push_confirm(f, f.size(), model.force_push_confirm_cursor);
        }

        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        
        if s.mode == AppMode::ForcePushConfirm {
            match event.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Tab => {
                    s.force_push_confirm_cursor = 1 - s.force_push_confirm_cursor;
                }
                KeyCode::Esc | KeyCode::Char('n') => {
                    s.mode = AppMode::PushDialog;
                }
                KeyCode::Enter | KeyCode::Char('y') => {
                    if s.force_push_confirm_cursor == 1 {
                        s.push_dlg.force_push = true;
                        s.mode = AppMode::PushDialog;
                        drop(s);
                        self.trigger_push(Arc::clone(&model)).await;
                    } else {
                        s.mode = AppMode::PushDialog;
                    }
                }
                _ => {}
            }
            return Ok(true);
        }

        match event.code {
            KeyCode::Esc => {
                if s.push_dlg.branch_open {
                    s.push_dlg.branch_open = false;
                } else {
                    s.mode = AppMode::RepoView;
                }
            }
            KeyCode::Tab | KeyCode::Down => {
                if s.push_dlg.branch_open {
                    let max = s.push_dlg.branch_list.len().saturating_sub(1);
                    if s.push_dlg.branch_cursor < max { s.push_dlg.branch_cursor += 1; }
                } else {
                    let next = s.push_dlg.focused.next();
                    s.push_dlg.focused = next;
                    s.push_dlg.clamp_cursor();
                }
            }
            KeyCode::BackTab | KeyCode::Up => {
                if s.push_dlg.branch_open {
                    if s.push_dlg.branch_cursor > 0 { s.push_dlg.branch_cursor -= 1; }
                } else {
                    let prev = s.push_dlg.focused.prev();
                    s.push_dlg.focused = prev;
                    s.push_dlg.clamp_cursor();
                }
            }
            KeyCode::Enter => {
                if s.push_dlg.branch_open {
                    if let Some(b) = s.push_dlg.branch_list.get(s.push_dlg.branch_cursor).cloned() {
                        s.push_dlg.branch = b;
                        s.push_dlg.branch_open = false;
                        s.push_dlg.update_origin();
                    }
                } else {
                    drop(s);
                    self.trigger_push(Arc::clone(&model)).await;
                }
            }
            KeyCode::Char(' ') if s.push_dlg.focused == PushField::Branch => {
                s.push_dlg.branch_open = !s.push_dlg.branch_open;
                if s.push_dlg.branch_open { s.push_dlg.sync_branch_cursor(); }
            }
            KeyCode::Left => { if s.push_dlg.cursor > 0 { s.push_dlg.cursor -= 1; } }
            KeyCode::Right => {
                let len = s.push_dlg.active_text().chars().count();
                if s.push_dlg.cursor < len { s.push_dlg.cursor += 1; }
            }
            KeyCode::Backspace => { s.push_dlg.backspace(); }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                s.push_dlg.type_char(c);
            }
            _ => {}
        }
        Ok(false)
    }
}

impl PushDialogContext {
    async fn trigger_push(&self, model: Arc<Mutex<Model>>) {
        let (token, root, repo_name, branch, username, force_push, _frame_count, already_loading) = {
            let mut s = model.lock().await;
            if s.is_loading {
                (None, None, String::new(), String::new(), String::new(), false, 0, true)
            } else {
                let repo_name = s.push_dlg.repo_name.trim().to_string();
                let branch    = s.push_dlg.branch.trim().to_string();
                let token     = s.config.github_token.clone();
                let root      = s.repo.as_ref().map(|r| r.root.clone());
                let username  = s.config.username.clone().unwrap_or_default();
                let force_push = s.push_dlg.force_push;
                let fc        = s.frame_count;
                s.push_dlg.status_msg = format!("{} Pushing…", spinner_char(fc));
                s.loading_label = "Pushing to remote".to_string();
                s.is_loading = true;
                (token, root, repo_name, branch, username, force_push, fc, false)
            }
        };

        if already_loading {
            return;
        }

        if repo_name.is_empty() {
            let mut s = model.lock().await;
            s.push_dlg.status_msg = "❌ Repo name is required".to_string();
            s.is_loading = false;
            return;
        }

        let tok = match token {
            Some(t) => t,
            None => {
                let mut s = model.lock().await;
                s.push_dlg.status_msg = "❌ No GitHub token".to_string();
                s.is_loading = false;
                return;
            }
        };

        let root = match root {
            Some(r) => r,
            None => {
                let mut s = model.lock().await;
                s.push_dlg.status_msg = "❌ No repo open".to_string();
                s.is_loading = false;
                return;
            }
        };

        let model_c    = Arc::clone(&model);
        let tok_c      = tok.clone();
        let root_c     = root.clone();
        let branch_c   = branch.clone();
        let username_c = username.clone();
        let repo_c     = repo_name.clone();

        tokio::task::spawn(async move {
            let u_captured = username_c.clone();
            let r_captured = repo_c.clone();
            let result = tokio::task::spawn_blocking(move || {
                if crate::auth::test_ssh_github().is_ok() {
                    let _ = crate::auth::set_remote_ssh(&root_c, &u_captured, &r_captured);
                    crate::auth::push_via_ssh(&root_c, &branch_c, force_push)
                } else {
                    crate::git::set_remote_and_push(&root_c, &tok_c, &u_captured, &r_captured, &branch_c, force_push)
                }
            }).await;

            let mut s = model_c.lock().await;
            s.is_loading = false;
            match result {
                Ok(Ok(out)) => {
                    let msg = if out.contains("Everything up-to-date") || out.contains("up to date") {
                        format!("✅ Already up-to-date")
                    } else {
                        format!("✅ Pushed to {}/{}!", username_c, repo_c)
                    };
                    s.push_dlg.status_msg = msg.clone();
                    s.status_msg          = msg.clone();
                    s.show_toast(msg, ToastType::Success);
                    s.mode = AppMode::RepoView;
                }
                Ok(Err(e)) => {
                    let err_str = e.to_string();
                    let is_rejected = err_str.contains("rejected") ||
                                      err_str.contains("non-fast-forward") ||
                                      err_str.contains("fetch first");

                    if is_rejected && !force_push {
                        s.force_push_confirm_cursor = 0;
                        s.mode = AppMode::ForcePushConfirm;
                        s.push_dlg.status_msg = "⚠ Push rejected — force push?".to_string();
                    } else {
                        let msg = format!("❌ Push failed: {}", e);
                        s.push_dlg.status_msg = msg.clone();
                        s.show_toast(msg, ToastType::Error);
                    }
                }
                _ => {
                    s.show_toast("Push failed", ToastType::Error);
                }
            }
        });
    }
}
