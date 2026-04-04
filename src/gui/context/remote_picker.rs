use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::{components::remote_picker, components::toast::ToastType};
use async_trait::async_trait;

pub struct RemotePickerContext;

#[async_trait]
impl Context for RemotePickerContext {
    fn view_name(&self) -> &str {
        "remote_picker"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        remote_picker::render(
            f, 
            f.size(), 
            model.frame_count,
            &model.remote_owner, 
            &model.remote_repo,
            &model.remote_files, 
            model.remote_cursor, 
            model.remote_scroll,
            &model.remote_selected, 
            model.is_loading, 
            &model.remote_expanded
        );
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let mut s = model.lock().await;
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.mode = AppMode::CloneInput;
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.remote_cursor > 0 {
                    s.remote_cursor -= 1;
                    if s.remote_cursor < s.remote_scroll { s.remote_scroll = s.remote_cursor; }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let tree_len = tree_len(&s.remote_files, &s.remote_expanded);
                let max = tree_len.saturating_sub(1);
                if s.remote_cursor < max {
                    s.remote_cursor += 1;
                    let visible = 25usize;
                    if s.remote_cursor >= s.remote_scroll + visible {
                        s.remote_scroll = s.remote_cursor - visible + 1;
                    }
                }
            }
            KeyCode::Enter => {
                let cursor = s.remote_cursor;
                if let Some(item) = tree_cursor_path(&s.remote_files, &s.remote_expanded, cursor) {
                    match item {
                        CursorItem::Folder(path) => {
                            if s.remote_expanded.contains(&path) {
                                s.remote_expanded.remove(&path);
                            } else {
                                s.remote_expanded.insert(path);
                            }
                        }
                        CursorItem::File(_) => {}
                    }
                }
            }
            KeyCode::Char(' ') => {
                let cursor = s.remote_cursor;
                if let Some(item) = tree_cursor_path(&s.remote_files, &s.remote_expanded, cursor) {
                    match item {
                        CursorItem::File(file_idx) => {
                            if s.remote_selected.contains(&file_idx) {
                                s.remote_selected.remove(&file_idx);
                            } else {
                                s.remote_selected.insert(file_idx);
                            }
                        },
                        CursorItem::Folder(path) => {
                            // Select/Deselect ALL files under this path
                            let prefix = format!("{}/", path);
                            let mut all_under_selected = true;
                            let mut found_any = false;
                            
                            for (i, file) in s.remote_files.iter().enumerate() {
                                if file.path == path || file.path.starts_with(&prefix) {
                                    found_any = true;
                                    if !s.remote_selected.contains(&i) {
                                        all_under_selected = false;
                                        break;
                                    }
                                }
                            }

                            if found_any {
                                let target_indices: Vec<usize> = s.remote_files.iter().enumerate()
                                    .filter(|(_, file)| file.path == path || file.path.starts_with(&prefix))
                                    .map(|(i, _)| i)
                                    .collect();

                                if all_under_selected {
                                    for i in target_indices {
                                        s.remote_selected.remove(&i);
                                    }
                                } else {
                                    for i in target_indices {
                                        s.remote_selected.insert(i);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('a') => {
                let (url, already_loading) = {
                    if s.is_loading {
                        (String::new(), true)
                    } else {
                        s.loading_label = format!("Cloning all from {}...", s.remote_url);
                        s.is_loading = true;
                        s.mode = AppMode::Loading;
                        (s.remote_url.clone(), false)
                    }
                };

                if already_loading {
                    return Ok(true);
                }

                let model_c = Arc::clone(&model);
                let url_c = url.clone();
                let dest_dir = dirs::home_dir().unwrap_or_default().join("swiftgit-repos");
                let _ = std::fs::create_dir_all(&dest_dir);
                let dest_dir_c = dest_dir.clone();

                tokio::task::spawn(async move {
                    let url_captured = url_c.clone();
                    let dest_captured = dest_dir_c.clone();
                    let output = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .args(["clone", &url_captured])
                            .current_dir(&dest_captured)
                            .output()
                    }).await;

                    let mut s = model_c.lock().await;
                    s.is_loading = false;
                    match output {
                        Ok(Ok(out)) if out.status.success() => {
                            let rname = url_c.trim_end_matches('/').split('/').next_back().unwrap_or("repo").trim_end_matches(".git").to_string();
                            let rpath = dest_dir_c.join(&rname);
                            match s.open_repo(rpath) {
                                Ok(_) => s.show_toast(format!("Cloned all: {}", rname), ToastType::Success),
                                Err(e) => { s.mode = AppMode::Dashboard; s.show_toast(format!("{}", e), ToastType::Warning); }
                            }
                        }
                        Ok(Ok(out)) => {
                            let err = String::from_utf8_lossy(&out.stderr);
                            s.mode = AppMode::Dashboard;
                            s.show_toast(format!("Clone all failed: {}", err.trim()), ToastType::Error);
                        }
                        _ => {
                            s.mode = AppMode::Dashboard;
                            s.show_toast("Clone all task failed or panicked", ToastType::Error);
                        }
                    }
                });
            }
            KeyCode::Char('d') => {
                let (owner, repo_name, token, items_to_download, already_loading) = {
                    if s.is_loading {
                        (String::new(), String::new(), None, Vec::new(), true)
                    } else if s.remote_selected.is_empty() {
                        s.show_toast("No files selected — Space to select, 'a' for all", ToastType::Warning);
                        return Ok(true);
                    } else {
                        let selected_items: Vec<crate::git::RemoteFile> = s.remote_selected.iter()
                            .filter_map(|&i| s.remote_files.get(i))
                            .cloned()
                            .collect();
                        s.is_loading = true;
                        s.loading_label = format!("Downloading {} item(s)...", selected_items.len());
                        s.mode = AppMode::Loading;
                        (s.remote_owner.clone(), s.remote_repo.clone(), s.config.github_token.clone(), selected_items, false)
                    }
                };

                if already_loading {
                    return Ok(true);
                }

                let model_c = Arc::clone(&model);
                let dest_dir = dirs::home_dir().unwrap_or_default().join("swiftgit-repos").join(&repo_name);
                let _ = std::fs::create_dir_all(&dest_dir);
                let dest_dir_c = dest_dir.clone();
                let repo_name_c = repo_name.clone();

                tokio::task::spawn(async move {
                    let owner_captured = owner.clone();
                    let repo_captured = repo_name.clone();
                    let dest_captured = dest_dir_c.clone();
                    let token_captured = token.clone();
                    let items_captured = items_to_download.clone();

                    let result = tokio::task::spawn_blocking(move || {
                        let mut errors = Vec::new();
                        for item in &items_captured {
                            if let Err(e) = crate::git::download_github_item(&owner_captured, &repo_captured, item, &dest_captured, token_captured.as_deref()) {
                                errors.push(format!("{}: {}", item.path, e));
                            }
                        }
                        errors
                    }).await;

                    let mut s = model_c.lock().await;
                    s.is_loading = false;
                    match result {
                        Ok(errors) => {
                            if !errors.is_empty() {
                                s.mode = AppMode::Dashboard;
                                s.show_toast(format!("{} error(s) during download", errors.len()), ToastType::Warning);
                            } else {
                                s.show_toast(format!("Downloaded items to ~/swiftgit-repos/{}", repo_name_c), ToastType::Success);
                                s.mode = AppMode::Dashboard;
                            }
                        }
                        _ => {
                            s.mode = AppMode::Dashboard;
                            s.show_toast("Download task failed or panicked", ToastType::Error);
                        }
                    }
                });
            }
            _ => {}
        }
        Ok(false)
    }
}

#[derive(Clone)]
enum CursorItem {
    Folder(String),
    File(usize),
}

fn build_level(
    files: &[crate::git::RemoteFile],
    expanded: &std::collections::HashSet<String>,
    parent: &str,
    out: &mut Vec<CursorItem>,
) {
    use std::collections::BTreeMap;
    let mut direct_files = Vec::new();
    let mut subdirs: BTreeMap<String, ()> = BTreeMap::new();

    for (i, f) in files.iter().enumerate() {
        let rel = if parent.is_empty() { f.path.as_str() }
                  else { match f.path.strip_prefix(&format!("{}/", parent)) { Some(r) => r, None => continue } };
        if let Some(slash) = rel.find('/') {
            let sd = if parent.is_empty() { rel[..slash].to_string() }
                     else { format!("{}/{}", parent, &rel[..slash]) };
            subdirs.entry(sd).or_insert(());
        } else {
            direct_files.push(i);
        }
    }

    for idx in direct_files { out.push(CursorItem::File(idx)); }
    for (dir_path, _) in &subdirs {
        out.push(CursorItem::Folder(dir_path.clone()));
        if expanded.contains(dir_path) {
            build_level(files, expanded, dir_path, out);
        }
    }
}

fn tree_len(files: &[crate::git::RemoteFile], expanded: &std::collections::HashSet<String>) -> usize {
    let mut items = Vec::new();
    build_level(files, expanded, "", &mut items);
    items.len()
}

fn tree_cursor_path(files: &[crate::git::RemoteFile], expanded: &std::collections::HashSet<String>, cursor: usize) -> Option<CursorItem> {
    let mut items = Vec::new();
    build_level(files, expanded, "", &mut items);
    items.into_iter().nth(cursor)
}
