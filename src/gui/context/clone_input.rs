use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{render_text_input, components::toast::ToastType};
use async_trait::async_trait;

pub struct CloneInputContext;

#[async_trait]
impl Context for CloneInputContext {
    fn view_name(&self) -> &str {
        "clone_input"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        render_text_input(
            f, 
            f.size(), 
            "Clone Repository",
            "https://github.com/owner/repo", 
            &model.text_input, 
            model.frame_count
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        // Ctrl shortcuts
        if event.modifiers.contains(KeyModifiers::CONTROL) {
            match event.code {
                KeyCode::Char('g') => {
                    s.text_input = "https://github.com/".to_string();
                    s.text_cursor = s.text_input.chars().count();
                    return Ok(true);
                }
                _ => {}
            }
        }

        match event.code {
            KeyCode::Esc => {
                s.mode = AppMode::Dashboard;
                return Ok(true);
            }
            KeyCode::Enter => {
                let (url, token, already_loading) = {
                    if s.is_loading {
                        (String::new(), None, true)
                    } else {
                        let url = s.text_input.trim().to_string();
                        let token = s.config.github_token.clone();
                        (url, token, false)
                    }
                };

                if already_loading {
                    return Ok(true);
                }

                if url.is_empty() {
                    s.show_toast("Please enter a URL", ToastType::Warning);
                    return Ok(true);
                }

                // Check if it's a GitHub URL — offer file picker
                if let Some((owner, repo_name)) = crate::auth::parse_github_url(&url) {
                    {
                        s.remote_url = url.clone();
                        s.remote_owner = owner.to_string();
                        s.remote_repo = repo_name.to_string();
                        s.remote_files.clear();
                        s.remote_selected.clear();
                        s.remote_cursor = 0;
                        s.remote_scroll = 0;
                        s.is_loading = true;
                        s.loading_label = format!("Fetching file list for {}/{}...", owner, repo_name);
                        s.mode = AppMode::Loading;
                    }

                    let model_c = Arc::clone(&model);
                    let owner_c = owner.to_string();
                    let repo_c = repo_name.to_string();
                    let tok = token.clone();
                    
                    tokio::task::spawn(async move {
                        let files_result = tokio::task::spawn_blocking(move || {
                            crate::git::fetch_github_files(&owner_c, &repo_c, tok.as_deref())
                        }).await;

                        let mut s = model_c.lock().await;
                        s.is_loading = false;
                        match files_result {
                            Ok(Ok(files)) => {
                                s.remote_files = files;
                                s.mode = AppMode::RemotePicker;
                            }
                            Ok(Err(e)) => {
                                s.show_toast(format!("Failed to fetch files: {}", e), ToastType::Error);
                                s.mode = AppMode::CloneInput;
                            }
                            Err(_) => {
                                s.show_toast("Task panicked", ToastType::Error);
                                s.mode = AppMode::CloneInput;
                            }
                        }
                    });
                } else {
                    // Non-GitHub or SSH URL — full clone immediately
                    {
                        s.loading_label = format!("Cloning {}...", url);
                        s.is_loading = true;
                        s.mode = AppMode::Loading;
                    }

                    let url_c = url.clone();
                    let model_c = Arc::clone(&model);
                    let clone_base = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("swiftgit-repos");
                    let _ = std::fs::create_dir_all(&clone_base);
                    
                    let repo_name = url.trim_end_matches('/').split('/').next_back()
                        .unwrap_or("repo").trim_end_matches(".git").to_string();
                    let repo_path = clone_base.join(&repo_name);

                    if repo_path.join(".git").exists() {
                        s.is_loading = false;
                        match s.open_repo(repo_path) {
                            Ok(_) => s.show_toast(format!("Opened existing repo: {}", repo_name), ToastType::Success),
                            Err(e) => { s.mode = AppMode::Dashboard; s.show_toast(format!("{}", e), ToastType::Warning); }
                        }
                        return Ok(true);
                    }

                    tokio::task::spawn(async move {
                        let url_captured = url_c.clone();
                        let clone_dir_captured = clone_base.clone();
                        let output = tokio::task::spawn_blocking(move || {
                            std::process::Command::new("git")
                                .args(["clone", &url_captured])
                                .current_dir(&clone_dir_captured)
                                .output()
                        }).await;

                        let mut s = model_c.lock().await;
                        s.is_loading = false;
                        match output {
                            Ok(Ok(out)) if out.status.success() => {
                                let rpath = clone_base.join(&repo_name);
                                match s.open_repo(rpath) {
                                    Ok(_) => s.show_toast(format!("Cloned: {}", repo_name), ToastType::Success),
                                    Err(e) => { s.mode = AppMode::Dashboard; s.show_toast(format!("{}", e), ToastType::Warning); }
                                }
                            }
                            Ok(Ok(out)) => {
                                let err = String::from_utf8_lossy(&out.stderr);
                                s.mode = AppMode::Dashboard;
                                s.show_toast(format!("Clone failed: {}", err.trim()), ToastType::Error);
                            }
                            Ok(Err(e)) => {
                                s.mode = AppMode::Dashboard;
                                s.show_toast(format!("Command error: {}", e), ToastType::Error);
                            }
                            Err(_) => {
                                s.mode = AppMode::Dashboard;
                                s.show_toast("Task panicked", ToastType::Error);
                            }
                        }
                    });
                }
                return Ok(true);
            }
            KeyCode::Backspace => {
                if event.modifiers.contains(KeyModifiers::CONTROL) {
                    s.text_input.clear(); s.text_cursor = 0;
                } else if s.text_cursor > 0 {
                    let bp = s.text_input.char_indices().nth(s.text_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                    s.text_input.remove(bp); s.text_cursor -= 1;
                }
            }
            KeyCode::Left => { if s.text_cursor > 0 { s.text_cursor -= 1; } }
            KeyCode::Right => { if s.text_cursor < s.text_input.chars().count() { s.text_cursor += 1; } }
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
                let bp = s.text_input.char_indices().nth(s.text_cursor).map(|(i,_)| i).unwrap_or(s.text_input.len());
                s.text_input.insert(bp, c); s.text_cursor += 1;
            }
            _ => {}
        }
        Ok(false)
    }
}
