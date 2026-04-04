use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::repo_view, components::toast::ToastType, spinner_char};
use async_trait::async_trait;

pub struct RepoContext;

#[async_trait]
impl Context for RepoContext {
    fn view_name(&self) -> &str {
        "repo_view"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let repo_name = model.repo.as_ref()
            .map(|r| r.repo_name()).unwrap_or_else(|| "repo".to_string());
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
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let commit_mode = { model.lock().await.commit_mode };
        if commit_mode {
            return self.handle_commit_input(event, Arc::clone(&model)).await;
        }

        match event.code {
            KeyCode::Char('q') => return Ok(false), // Let global handler or event loop deal with it
            KeyCode::Esc => {
                let mut s = model.lock().await;
                s.mode = AppMode::Dashboard;
                s.status_msg.clear();
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let mut s = model.lock().await;
                if s.active_frame == 2 {
                    if s.diff_scroll > 0 { s.diff_scroll -= 1; }
                } else {
                    s.move_up();
                    s.hunk_cursor = None;
                    s.diff_scroll = 0;
                    drop(s);
                    Model::async_refresh_diff(Arc::clone(&model)).await;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let mut s = model.lock().await;
                if s.active_frame == 2 {
                    s.diff_scroll += 1;
                } else {
                    s.move_down();
                    s.hunk_cursor = None;
                    s.diff_scroll = 0;
                    drop(s);
                    Model::async_refresh_diff(Arc::clone(&model)).await;
                }
            }
            KeyCode::Tab | KeyCode::Char('n') => {
                let mut s = model.lock().await;
                if !s.diff_struct.hunks.is_empty() {
                    let next = match s.hunk_cursor {
                        None => 0,
                        Some(i) => (i + 1) % s.diff_struct.hunks.len(),
                    };
                    s.hunk_cursor = Some(next);
                }
            }
            KeyCode::BackTab | KeyCode::Char('p') if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut s = model.lock().await;
                if !s.diff_struct.hunks.is_empty() {
                    let prev = match s.hunk_cursor {
                        None | Some(0) => s.diff_struct.hunks.len().saturating_sub(1),
                        Some(i) => i - 1,
                    };
                    s.hunk_cursor = Some(prev);
                }
            }
            KeyCode::Char(' ') => {
                {
                    let mut s = model.lock().await;
                    if let Some(hc) = s.hunk_cursor {
                        if let Some(file) = s.current_file() {
                            let path = file.path.clone();
                            let is_staged = file.status.is_staged();
                            
                            if let Some(hunk) = s.diff_struct.hunks.get(hc).cloned() {
                                if let Some(repo) = &s.repo {
                                    let res = if is_staged {
                                        repo.unstage_hunk(&path, &hunk)
                                    } else {
                                        repo.stage_hunk(&path, &hunk)
                                    };
                                    
                                    match res {
                                        Ok(_) => s.show_toast(format!("Hunk {} {}", hc+1, if is_staged { "unstaged" } else { "staged" }), ToastType::Success),
                                        Err(e) => s.show_toast(format!("Error: {}", e), ToastType::Error),
                                    }
                                }
                            }
                        }
                    } else {
                        s.space_stage_unstage();
                    }
                }
                Model::async_refresh_status(Arc::clone(&model)).await;
            }
            KeyCode::Enter => {
                let mut s = model.lock().await;
                s.enter_expand_collapse();
            }
            KeyCode::Char('c') => {
                let mut s = model.lock().await;
                let has_staged = s.git_files.iter().any(|f| f.status.is_staged());
                if has_staged {
                    s.commit_mode = true;
                    s.commit_input.clear();
                    s.commit_cursor = 0;
                    if let Some(repo) = &s.repo {
                        s.commit_history = crate::git::recent_commits(&repo.root, 5);
                    }
                } else {
                    s.show_toast("No staged files — Space to stage", ToastType::Warning);
                }
            }
            KeyCode::Char('s') => {
                let root = {
                    let s = model.lock().await;
                    s.repo.as_ref().map(|r| r.root.clone())
                };
                if let Some(root) = root {
                    let out = std::process::Command::new("git")
                        .args(["add", "-A"])
                        .current_dir(&root)
                        .output();
                    {
                        let mut s = model.lock().await;
                        match out {
                            Ok(o) if o.status.success() => {
                                s.show_toast("✅ Staged all changes", ToastType::Success);
                            }
                            Ok(o) => {
                                let err = String::from_utf8_lossy(&o.stderr).to_string();
                                s.show_toast(format!("❌ Stage all failed: {}", err.trim()), ToastType::Error);
                            }
                            Err(e) => s.show_toast(format!("❌ {}", e), ToastType::Error),
                        }
                    }
                    Model::async_refresh_status(Arc::clone(&model)).await;
                }
            }
            KeyCode::Char('p') => {
                // For now, we still trigger AppMode::PushDialog
                let mut s = model.lock().await;
                s.mode = AppMode::PushDialog;
                return Ok(false); 
            }
            KeyCode::Char('P') => {
                let (token, root, already_loading) = {
                    let mut s = model.lock().await;
                    if s.is_loading {
                        (None, None, true)
                    } else {
                        s.status_msg = format!("{} Pulling…", spinner_char(s.frame_count));
                        s.loading_label = "Pulling from remote".to_string();
                        s.is_loading = true;
                        (s.config.github_token.clone(), s.repo.as_ref().map(|r| r.root.clone()), false)
                    }
                };

                if already_loading {
                    return Ok(true);
                }

                if let Some(root_c) = root {
                    let tok  = token.clone();
                    let state_c = Arc::clone(&model);

                    tokio::task::spawn(async move {
                        let ssh_ready = crate::auth::test_ssh_github().is_ok();
                        let result = if ssh_ready {
                            tokio::task::spawn_blocking(move || crate::auth::pull_via_ssh(&root_c)).await
                        } else {
                            tokio::task::spawn_blocking(move || {
                                crate::git::GitRepo { root: root_c }.smart_pull(tok.as_deref())
                            }).await
                        };

                        let mut s = state_c.lock().await;
                        s.is_loading = false;
                        match result {
                            Ok(Ok(out)) => {
                                let first = out.lines().next().unwrap_or("Pulled").to_string();
                                let msg = if first.contains("Already up to date") || first.contains("up-to-date") {
                                    "✅ Already up-to-date".to_string()
                                } else {
                                    format!("✅ {}", first)
                                };
                                s.status_msg.clear();
                                s.show_toast(msg, ToastType::Success);
                                drop(s);
                                Model::async_refresh_status(Arc::clone(&state_c)).await;
                            }
                            _ => {
                                let msg = "❌ Pull failed".to_string();
                                s.status_msg = msg.clone();
                                s.show_toast(msg, ToastType::Error);
                            }
                        }
                    });
                }
            }
            KeyCode::Char('r') => {
                Model::async_refresh_status(Arc::clone(&model)).await;
                let mut s = model.lock().await;
                s.show_toast("Refreshed", ToastType::Info);
            }
            KeyCode::Char('X') => {
                let mut s = model.lock().await;
                s.deinit_confirm_cursor = 0;
                s.mode = AppMode::DeinitConfirm;
            }
            KeyCode::Char('e') => {
                let mut s = model.lock().await;
                let path_opt = s.current_file().map(|f| f.path.clone());
                if let Some(path) = path_opt {
                    s.editor_open(&path);
                } else {
                    s.show_toast("Navigate to a file first, then press e", ToastType::Info);
                }
            }
            KeyCode::Char('i') => {
                let root = {
                    let s = model.lock().await;
                    s.repo.as_ref().map(|r| r.root.clone())
                };
                if let Some(root) = root {
                    match crate::git::rebase_todo(&root) {
                        Ok(commits) => {
                            let mut s = model.lock().await;
                            s.rebase_commits = commits;
                            s.rebase_cursor = 0;
                            s.mode = AppMode::Rebase;
                        }
                        Err(e) => {
                            model.lock().await.show_toast(format!("❌ Failed to get rebase todo: {}", e), ToastType::Error);
                        }
                    }
                }
            }
            KeyCode::Char('/') => {
                let mut s = model.lock().await;
                s.filter.clear();
                s.mode = AppMode::Search;
            }
            KeyCode::Char('1') => { model.lock().await.active_frame = 1; }
            KeyCode::Char('2') => { model.lock().await.active_frame = 2; }
            _ => {}
        }
        Ok(false)
    }
}

impl RepoContext {
    async fn handle_commit_input(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc => {
                s.commit_mode = false;
                s.commit_input.clear();
            }
            KeyCode::Enter => {
                let msg = s.commit_input.trim().to_string();
                if msg.is_empty() {
                    s.show_toast("Commit message cannot be empty", ToastType::Warning);
                    return Ok(false);
                }
                if let Some(repo) = &s.repo {
                    match repo.commit(&msg) {
                        Ok(out) => {
                            let summary = out.lines().next().unwrap_or("Committed").to_string();
                            s.commit_mode = false;
                            s.commit_input.clear();
                            s.commit_cursor = 0;
                            s.status_msg.clear();
                            s.show_toast(format!("✅ {}", summary), ToastType::Success);
                            s.repo_cursor = 0;
                            s.repo_scroll = 0;
                            drop(s);
                            Model::async_refresh_status(Arc::clone(&model)).await;
                            return Ok(true);
                        }
                        Err(e) => {
                            s.show_toast(format!("❌ Commit failed: {}", e), ToastType::Error);
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if s.commit_cursor > 0 {
                    let bp = s.commit_input.char_indices().nth(s.commit_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                    s.commit_input.remove(bp);
                    s.commit_cursor -= 1;
                }
            }
            KeyCode::Left  => { if s.commit_cursor > 0 { s.commit_cursor -= 1; } }
            KeyCode::Right => {
                if s.commit_cursor < s.commit_input.chars().count() { s.commit_cursor += 1; }
            }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let bp = s.commit_input.char_indices().nth(s.commit_cursor).map(|(i,_)| i).unwrap_or(s.commit_input.len());
                s.commit_input.insert(bp, c);
                s.commit_cursor += 1;
            }
            _ => {}
        }
        Ok(false)
    }
}
