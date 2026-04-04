use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Context as AnyhowContext, Result};
use crate::config::SwiftGitConfig;
use crate::git::{GitFile, GitRepo, RemoteFile};
use crate::ui::components::toast::{Toast, ToastType};
use crate::ui::components;

// ── DisplayItem for the collapsible folder tree ───────────────────────────────
#[derive(Debug, Clone)]
pub enum DisplayItem {
    /// A collapsible folder header
    FolderHeader {
        path: String,   // full path e.g. "src/ui"
        count: usize,   // how many DIRECT children (files + subdirs)
        expanded: bool,
        depth: usize,   // nesting level for indent
    },
    /// A single file entry
    FileEntry {
        file_idx: usize, // index into Model::git_files
        depth: usize,    // 0 = root, 1 = one folder deep, etc.
    },
}

impl DisplayItem {
    pub fn file_idx(&self) -> Option<usize> {
        if let DisplayItem::FileEntry { file_idx, .. } = self { Some(*file_idx) } else { None }
    }
}

// ── App modes ─────────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AppMode {
    Auth,
    Dashboard,
    FolderInput,
    CloneInput,
    RepoView,
    Loading,
    RemotePicker,  // selective file picker from GitHub
    PushDialog,
    ForcePushConfirm,
    DeinitConfirm,
    SshSetup,      // SSH key generation + GitHub connection flow
    Editor,        // Task 7: inline file editor (right panel)
    Settings,      // Ctrl+W overlay — edit ~/.swiftgit/config.json
    RecentProjects, // Task: Dialog to pick from recent projects
    Rebase,        // Interactive rebase mode
    Search,        // Search/Filter mode
    Help,          // Help overlay
}

// ── Model ─────────────────────────────────────────────────────────────────────
pub struct Model {
    pub mode: AppMode,
    pub frame_count: u64,
    pub toast: Option<Toast>,

    // Auth
    pub token_input: String,
    pub token_cursor: usize,
    pub auth_status: String,
    pub is_validating: bool,

    // Dashboard
    pub dashboard_cursor: usize,
    pub recent_projects_cursor: usize, // Cursor for recent projects dialog

    // Text input (folder/clone)
    pub text_input: String,
    pub text_cursor: usize,

    // Dir suggestions (FolderInput)
    pub dir_suggestions: Vec<String>,
    pub suggestion_cursor: Option<usize>,

    // Repo view
    pub repo: Option<GitRepo>,
    pub git_files: Vec<GitFile>,
    pub display_items: Vec<DisplayItem>,  // built from git_files + expanded_folders
    pub expanded_folders: HashSet<String>,
    pub repo_cursor: usize,   // indexes into display_items
    pub repo_scroll: usize,
    pub diff_scroll: usize,
    pub diff_content: String,
    pub diff_struct: crate::git::Diff, // Task: Hunk-based staging
    pub hunk_cursor: Option<usize>,   // Task: Hunk-based staging
    pub commit_mode: bool,
    pub commit_input: String,
    pub commit_cursor: usize,
    pub commit_history: Vec<String>,
    pub status_msg: String,
    pub is_loading: bool,
    pub is_diff_loading: bool,  // Task 2: async diff loader
    pub branch: String,
    pub loading_label: String,
    pub filter: String,

    // Remote Picker
    pub remote_url: String,
    pub remote_owner: String,
    pub remote_repo: String,
    pub remote_files: Vec<RemoteFile>,
    pub remote_cursor: usize,
    pub remote_scroll: usize,
    pub remote_selected: HashSet<usize>,
    pub remote_expanded: HashSet<String>,

    pub config: SwiftGitConfig,

    // ── Task 3: Push dialog ───────────────────────────────────────────
    pub push_dlg: components::push_dialog::PushDialogState,
    // ── SSH setup ─────────────────────────────────────────────────────────────
    pub ssh_step:   components::ssh_setup::SshSetupStep,
    pub ssh_pubkey: String,
    pub deinit_confirm_cursor: usize, // 0=Cancel, 1=Deinit
    pub force_push_confirm_cursor: usize, // 0=Cancel, 1=Force Push

    // ── Rebase ────────────────────────────────────────────────────────
    pub rebase_commits: Vec<crate::git::RebaseCommit>,
    pub rebase_cursor:  usize,

    // ── Task 7: Inline editor ─────────────────────────────────────────
    pub editor_lines:       Vec<String>,
    pub editor_cursor_line: usize,
    pub editor_cursor_col:  usize,
    pub editor_scroll_line: usize,
    pub editor_scroll_col:  usize,
    pub editor_modified:    bool,
    pub editor_path:        String,

    // ── Task 6: active frame (1=left, 2=right) ────────────────────────
    pub active_frame: usize,

    // ── Settings dialog (Ctrl+W) ──────────────────────────────────────
    pub settings_dlg: components::settings_dialog::SettingsDialogState,
    pub settings_prev_mode: AppMode,
    pub help_prev_mode: AppMode,
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("mode", &self.mode)
            .field("token_input", &"[REDACTED]")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Model {
    pub fn new(config: SwiftGitConfig) -> Self {
        Self {
            mode: AppMode::Dashboard,
            frame_count: 0,
            toast: None,
            token_input: String::new(),
            token_cursor: 0,
            auth_status: String::new(),
            is_validating: false,
            dashboard_cursor: 0,
            recent_projects_cursor: 0,
            text_input: String::new(),
            text_cursor: 0,
            dir_suggestions: Vec::new(),
            suggestion_cursor: None,
            repo: None,
            git_files: Vec::new(),
            display_items: Vec::new(),
            expanded_folders: HashSet::new(),
            repo_cursor: 0,
            repo_scroll: 0,
            diff_scroll: 0,
            diff_content: String::new(),
            diff_struct: crate::git::Diff { hunks: Vec::new(), file_header: String::new() },
            hunk_cursor: None,
            commit_mode: false,
            commit_input: String::new(),
            commit_cursor: 0,
            commit_history: Vec::new(),
            status_msg: String::new(),
            is_loading: false,
            is_diff_loading: false,
            branch: String::new(),
            loading_label: String::new(),
            filter: String::new(),
            remote_url: String::new(),
            remote_owner: String::new(),
            remote_repo: String::new(),
            remote_files: Vec::new(),
            remote_cursor: 0,
            remote_scroll: 0,
            remote_selected: HashSet::new(),
            remote_expanded: HashSet::new(),
            config,
            push_dlg: components::push_dialog::PushDialogState::default(),
            ssh_step:   components::ssh_setup::SshSetupStep::Detecting,
            ssh_pubkey: String::new(),
            deinit_confirm_cursor: 0,
            force_push_confirm_cursor: 0,
            rebase_commits: Vec::new(),
            rebase_cursor: 0,
            editor_lines:       Vec::new(),
            editor_cursor_line: 0,
            editor_cursor_col:  0,
            editor_scroll_line: 0,
            editor_scroll_col:  0,
            editor_modified:    false,
            editor_path:        String::new(),
            active_frame: 1,
            settings_dlg: components::settings_dialog::SettingsDialogState::default(),
            settings_prev_mode: AppMode::Dashboard,
            help_prev_mode: AppMode::Dashboard,
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>, t: ToastType) {
        self.toast = Some(Toast::new(msg.into(), t));
    }

    // ── Display-item navigation ───────────────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.repo_cursor > 0 {
            self.repo_cursor -= 1;
            self.adjust_scroll();
        }
    }

    pub fn move_down(&mut self) {
        let max = self.display_items.len().saturating_sub(1);
        if self.repo_cursor < max {
            self.repo_cursor += 1;
            self.adjust_scroll();
        }
    }

    fn adjust_scroll(&mut self) {
        let visible = 20usize;
        if self.repo_cursor < self.repo_scroll {
            self.repo_scroll = self.repo_cursor;
        } else if self.repo_cursor >= self.repo_scroll + visible {
            self.repo_scroll = self.repo_cursor - visible + 1;
        }
    }

    /// Rebuild the display_items tree from git_files + expanded_folders
    pub fn rebuild_display_items(&mut self) {
        self.display_items = build_display_items(&self.git_files, &self.expanded_folders);
        let max = self.display_items.len().saturating_sub(1);
        if self.repo_cursor > max {
            self.repo_cursor = max;
        }
    }

    /// Current file under cursor (if cursor is on a file entry)
    pub fn current_file(&self) -> Option<&GitFile> {
        match self.display_items.get(self.repo_cursor) {
            Some(DisplayItem::FileEntry { file_idx, .. }) => self.git_files.get(*file_idx),
            _ => None,
        }
    }

    pub fn refresh_commit_history(&mut self) {
        if let Some(repo) = &self.repo {
            self.commit_history = crate::git::recent_commits(&repo.root, 8);
        }
    }

    pub fn refresh_status(&mut self) {
        let result = self.repo.as_ref().map(|repo| {
            let files = if repo.has_commits() {
                repo.all_files().unwrap_or_else(|_| repo.status().unwrap_or_default())
            } else {
                repo.status().unwrap_or_default()
            };
            (files, repo.current_branch().unwrap_or_default())
        });
        match result {
            Some((files, branch)) => {
                self.git_files = files;
                self.branch    = branch;
                self.apply_filter();
                self.refresh_diff();
                self.refresh_commit_history();
            }
            None => {}
        }
    }

    pub fn apply_filter(&mut self) {
        let mut filtered = self.git_files.clone();
        if !self.filter.is_empty() {
            let f = self.filter.to_lowercase();
            filtered.retain(|file| file.path.to_lowercase().contains(&f));
        }
        self.display_items = build_display_items(&filtered, &self.expanded_folders);
        let max = self.display_items.len().saturating_sub(1);
        if self.repo_cursor > max {
            self.repo_cursor = max;
        }
    }

    pub fn refresh_diff(&mut self) {
        if let Some(repo) = &self.repo {
            if let Some(file) = self.current_file() {
                let path = file.path.clone();
                match repo.diff_file(&path) {
                    Ok(diff) => {
                        self.diff_content = diff.to_string();
                        self.diff_struct = diff;
                    }
                    Err(_) => {
                        self.diff_content = String::new();
                        self.diff_struct = crate::git::Diff { hunks: Vec::new(), file_header: String::new() };
                    }
                }
            } else {
                self.diff_content = String::new();
                self.diff_struct = crate::git::Diff { hunks: Vec::new(), file_header: String::new() };
            }
        }
    }

    /// Task 2 & 3: async refresh to prevent UI freeze
    pub async fn async_refresh_status(state: Arc<Mutex<Model>>) {
        let (repo_opt, _has_commits) = {
            let s = state.lock().await;
            let repo = s.repo.as_ref().map(|r| r.root.clone());
            let hc = s.repo.as_ref().map(|r| r.has_commits()).unwrap_or(false);
            (repo, hc)
        };

        if let Some(root) = repo_opt {
            let res = tokio::task::spawn_blocking(move || {
                let repo = GitRepo { root };
                let files = if repo.has_commits() {
                    repo.all_files().unwrap_or_else(|_| repo.status().unwrap_or_default())
                } else {
                    repo.status().unwrap_or_default()
                };
                let branch = repo.current_branch().unwrap_or_default();
                let history = crate::git::recent_commits(&repo.root, 8);
                (files, branch, history)
            }).await;

            if let Ok((files, branch, history)) = res {
                let mut s = state.lock().await;
                s.git_files = files;
                s.branch = branch;
                s.commit_history = history;
                s.rebuild_display_items();
                s.refresh_diff();
            }
        }
    }

    pub async fn async_refresh_diff(state: Arc<Mutex<Model>>) {
        let (repo_opt, file_opt) = {
            let s = state.lock().await;
            let repo = s.repo.as_ref().map(|r| r.root.clone());
            let file = s.current_file().map(|f| f.path.clone());
            (repo, file)
        };

        if let (Some(root), Some(path)) = (repo_opt, file_opt) {
            {
                let mut s = state.lock().await;
                s.is_diff_loading = true;
            }
            let path_c = path.clone();
            let res: Result<Result<crate::git::Diff, anyhow::Error>, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
                let repo = GitRepo { root };
                repo.diff_file(&path_c)
            }).await;

            if let Ok(Ok(diff)) = res {
                let mut s = state.lock().await;
                if let Some(f) = s.current_file() {
                    if f.path == path {
                        s.diff_content = diff.to_string();
                        s.diff_struct = diff;
                        if let Some(hc) = s.hunk_cursor {
                             if hc >= s.diff_struct.hunks.len() {
                                 s.hunk_cursor = if s.diff_struct.hunks.is_empty() { None } else { Some(0) };
                             }
                        }
                    }
                }
                s.is_diff_loading = false;
            } else {
                state.lock().await.is_diff_loading = false;
            }
        } else {
            let mut s = state.lock().await;
            s.diff_content = String::new();
            s.is_diff_loading = false;
        }
    }

    pub fn space_stage_unstage(&mut self) {
        match self.display_items.get(self.repo_cursor).cloned() {
            Some(DisplayItem::FolderHeader { path, .. }) => {
                let files_in: Vec<(usize, bool)> = self.git_files.iter().enumerate()
                    .filter(|(_, f)| f.path.starts_with(&format!("{}/", path)))
                    .map(|(i, f)| (i, f.status.is_staged()))
                    .collect();

                let any_unstaged = files_in.is_empty() || files_in.iter().any(|(_, s)| !s);

                if let Some(repo) = &self.repo {
                    let result = if any_unstaged {
                        repo.stage_folder(&path)
                    } else {
                        repo.unstage_folder(&path)
                    };
                    match result {
                        Ok(_) => {
                            let verb = if any_unstaged { "✅ Staged" } else { "Unstaged" };
                            self.show_toast(format!("{} folder: {}/", verb, path), ToastType::Success);
                            self.refresh_status();
                        }
                        Err(e) => self.show_toast(format!("❌ Error: {}", e), ToastType::Error),
                    }
                }
            }

            Some(DisplayItem::FileEntry { file_idx, .. }) => {
                if let Some(file) = self.git_files.get(file_idx) {
                    let path      = file.path.clone();
                    let is_staged = file.status.is_staged();
                    if let Some(repo) = &self.repo {
                        let result = if is_staged { repo.unstage(&path) } else { repo.stage(&path) };
                        match result {
                            Ok(_) => {
                                let msg = if is_staged {
                                    format!("Unstaged: {}", path)
                                } else {
                                    format!("✅ Staged: {}", path)
                                };
                                self.show_toast(msg, ToastType::Success);
                                self.refresh_status();
                            }
                            Err(e) => self.show_toast(format!("❌ Error: {}", e), ToastType::Error),
                        }
                    }
                }
            }
            None => {}
        }
    }

    pub fn enter_expand_collapse(&mut self) {
        self.hunk_cursor = None;
        match self.display_items.get(self.repo_cursor).cloned() {
            Some(DisplayItem::FolderHeader { path, expanded, .. }) => {
                if expanded {
                    self.expanded_folders.remove(&path);
                } else {
                    self.expanded_folders.insert(path);
                }
                self.rebuild_display_items();
            }
            Some(DisplayItem::FileEntry { .. }) => {
                self.diff_scroll = 0;
                self.refresh_diff();
            }
            None => {}
        }
    }

    pub fn editor_open(&mut self, rel_path: &str) {
        if let Some(repo) = &self.repo {
            let abs_path = repo.root.join(rel_path);
            match std::fs::read_to_string(&abs_path) {
                Ok(content) => {
                    self.editor_lines = content.lines().map(|l| l.to_string()).collect();
                    if self.editor_lines.is_empty() { self.editor_lines.push(String::new()); }
                    self.editor_path        = rel_path.to_string();
                    self.editor_cursor_line = 0;
                    self.editor_cursor_col  = 0;
                    self.editor_scroll_line = 0;
                    self.editor_scroll_col  = 0;
                    self.editor_modified    = false;
                    self.active_frame       = 2;
                    self.mode               = AppMode::Editor;
                }
                Err(e) => self.show_toast(format!("Cannot open: {}", e), ToastType::Error),
            }
        }
    }

    pub fn editor_save(&mut self) {
        if let Some(repo) = &self.repo {
            let abs_path = repo.root.join(&self.editor_path);
            let content  = self.editor_lines.join("\n");
            match std::fs::write(&abs_path, content) {
                Ok(_) => {
                    self.editor_modified = false;
                    self.show_toast(format!("✅ Saved: {}", self.editor_path), ToastType::Success);
                    self.refresh_status();
                }
                Err(e) => self.show_toast(format!("❌ Save failed: {}", e), ToastType::Error),
            }
        }
    }

    pub fn editor_adjust_scroll(&mut self) {
        let vis = 30usize;
        if self.editor_cursor_line < self.editor_scroll_line {
            self.editor_scroll_line = self.editor_cursor_line;
        } else if self.editor_cursor_line >= self.editor_scroll_line + vis {
            self.editor_scroll_line = self.editor_cursor_line - vis + 1;
        }
    }

    pub fn update_dir_suggestions(&mut self) {
        self.dir_suggestions = compute_dir_suggestions(&self.text_input);
        self.suggestion_cursor = None;
    }

    pub fn open_repo(&mut self, path: PathBuf) -> Result<bool> {
        let (repo, was_init) = GitRepo::open_or_init(&path)
            .with_context(|| format!("Cannot open: {}", path.display()))?;
        let branch = repo.current_branch().unwrap_or_else(|_| "main".to_string());
        let files = repo.status().unwrap_or_default();
        let name = repo.repo_name();
        self.config.add_recent_project(path.to_string_lossy().to_string(), name);
        let _ = self.config.save();
        self.branch = branch;
        self.git_files = files;
        self.expanded_folders.clear();
        self.filter.clear();
        self.rebuild_display_items();
        self.repo_cursor = 0;
        self.repo_scroll = 0;
        self.repo = Some(repo);
        self.refresh_diff();
        self.refresh_commit_history();
        self.mode = AppMode::RepoView;
        Ok(was_init)
    }
}

pub fn build_display_items(files: &[GitFile], expanded: &HashSet<String>) -> Vec<DisplayItem> {
    let mut items = Vec::new();
    build_level(files, expanded, "", 0, &mut items);
    items
}

fn build_level(
    files:    &[GitFile],
    expanded: &HashSet<String>,
    parent:   &str,
    depth:    usize,
    out:      &mut Vec<DisplayItem>,
) {
    use std::collections::BTreeMap;
    let mut direct_files:   Vec<usize>          = Vec::new();
    let mut direct_subdirs: BTreeMap<String, usize> = BTreeMap::new();

    for (i, f) in files.iter().enumerate() {
        let rel = if parent.is_empty() {
            f.path.as_str()
        } else {
            match f.path.strip_prefix(&format!("{}/", parent)) {
                Some(r) => r,
                None    => continue,
            }
        };

        if let Some(slash) = rel.find('/') {
            let subdir = if parent.is_empty() {
                rel[..slash].to_string()
            } else {
                format!("{}/{}", parent, &rel[..slash])
            };
            *direct_subdirs.entry(subdir).or_insert(0) += 1;
        } else {
            direct_files.push(i);
        }
    }

    for (subdir_path, count) in direct_subdirs {
        let is_expanded = expanded.contains(&subdir_path);
        out.push(DisplayItem::FolderHeader {
            path: subdir_path.clone(),
            count,
            expanded: is_expanded,
            depth,
        });
        if is_expanded {
            build_level(files, expanded, &subdir_path, depth + 1, out);
        }
    }

    for f_idx in direct_files {
        out.push(DisplayItem::FileEntry { file_idx: f_idx, depth });
    }
}

pub fn compute_dir_suggestions(input: &str) -> Vec<String> {
    if input.is_empty() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        return list_subdirs(&home, "");
    }

    let expanded = if input.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(&input[2..])
    } else {
        PathBuf::from(input)
    };

    if expanded.is_dir() {
        return list_subdirs(&expanded, "");
    }

    let parent = expanded.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let prefix = expanded
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    list_subdirs(&parent, &prefix)
}

fn list_subdirs(dir: &std::path::Path, prefix: &str) -> Vec<String> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            if prefix.is_empty() || name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                results.push(entry.path().to_string_lossy().to_string());
            }
        }
    }
    results.sort();
    results.truncate(8);
    results
}
