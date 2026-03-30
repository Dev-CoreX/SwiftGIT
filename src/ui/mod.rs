use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::SwiftGitConfig;
use crate::git::{GhFile, GitFile, GitRepo};

pub mod components;
pub mod theme;

use components::toast::{Toast, ToastType};

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));
}

// ── Spinner frames ────────────────────────────────────────────────────────────
pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_char(frame: u64) -> &'static str {
    SPINNER[(frame / 2) as usize % SPINNER.len()]
}

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
        file_idx: usize, // index into AppState::git_files
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
    GhGrab,        // selective file picker from GitHub
    PushDialog,
    ForcePushConfirm,
    DeinitConfirm,
    SshSetup,      // SSH key generation + GitHub connection flow
    Editor,        // Task 7: inline file editor (right panel)
    Settings,      // Ctrl+W overlay — edit ~/.swiftgit/config.json
}

// ── App state ─────────────────────────────────────────────────────────────────
pub struct AppState {
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
    pub diff_content: String,
    pub commit_mode: bool,
    pub commit_input: String,
    pub commit_cursor: usize,
    pub commit_history: Vec<String>,
    pub status_msg: String,
    pub is_loading: bool,
    pub is_diff_loading: bool,  // Task 2: async diff loader
    pub branch: String,
    pub loading_label: String,

    // GhGrab
    pub ghgrab_url: String,
    pub ghgrab_owner: String,
    pub ghgrab_repo: String,
    pub ghgrab_files: Vec<GhFile>,
    pub ghgrab_cursor: usize,
    pub ghgrab_scroll: usize,
    pub ghgrab_selected: HashSet<usize>,
    pub ghgrab_expanded: HashSet<String>,  // Task 2: expanded folder paths in ghgrab

    pub config: SwiftGitConfig,

    // ── Task 3: Push dialog ───────────────────────────────────────────
    pub push_dlg: components::push_dialog::PushDialogState,
    // ── SSH setup ─────────────────────────────────────────────────────────────
    pub ssh_step:   components::ssh_setup::SshSetupStep,
    pub ssh_pubkey: String,
    pub deinit_confirm_cursor: usize, // 0=Cancel, 1=Deinit
    pub force_push_confirm_cursor: usize, // 0=Cancel, 1=Force Push

    // ── Task 7: Inline editor ─────────────────────────────────────────
    pub editor_lines:       Vec<String>,
    pub editor_cursor_line: usize,
    pub editor_cursor_col:  usize,
    pub editor_scroll:      usize,
    pub editor_path:        String,     // relative path inside repo
    pub editor_modified:    bool,

    // ── Task 6: Frame numbers ─────────────────────────────────────────
    // Frames: 1=Tree panel, 2=Diff/Editor panel (tracked for number-key nav)
    pub active_frame: u8,

    // ── Settings dialog (Ctrl+W) ──────────────────────────────────────
    pub settings_dlg: components::settings_dialog::SettingsDialogState,
    pub settings_prev_mode: AppMode,  // mode to return to on close
}

impl AppState {
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
            diff_content: String::new(),
            commit_mode: false,
            commit_input: String::new(),
            commit_cursor: 0,
            commit_history: Vec::new(),
            status_msg: String::new(),
            is_loading: false,
            is_diff_loading: false,
            branch: String::new(),
            loading_label: String::new(),
            ghgrab_url: String::new(),
            ghgrab_owner: String::new(),
            ghgrab_repo: String::new(),
            ghgrab_files: Vec::new(),
            ghgrab_cursor: 0,
            ghgrab_scroll: 0,
            ghgrab_selected: HashSet::new(),
            ghgrab_expanded: HashSet::new(),
            config,
            push_dlg: components::push_dialog::PushDialogState::default(),
            ssh_step:   components::ssh_setup::SshSetupStep::Detecting,
            ssh_pubkey: String::new(),
            deinit_confirm_cursor: 0,
            force_push_confirm_cursor: 0,
            editor_lines:       Vec::new(),
            editor_cursor_line: 0,
            editor_cursor_col:  0,
            editor_scroll:      0,
            editor_path:        String::new(),
            editor_modified:    false,
            active_frame:       1,
            settings_dlg:       components::settings_dialog::SettingsDialogState::default(),
            settings_prev_mode: AppMode::Dashboard,
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
            // Task 2: use all_files() so committed/clean files stay visible in tree
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
                self.rebuild_display_items();
                if self.repo_cursor >= self.display_items.len() && !self.display_items.is_empty() {
                    self.repo_cursor = self.display_items.len() - 1;
                }
                self.refresh_diff();
                // Task 1: also refresh history when status refreshes
                self.refresh_commit_history();
            }
            None => {}
        }
    }

    pub fn refresh_diff(&mut self) {
        if let Some(repo) = &self.repo {
            if let Some(file) = self.current_file() {
                let path = file.path.clone();
                match repo.diff_file(&path) {
                    Ok(diff) => self.diff_content = diff,
                    Err(_) => self.diff_content = String::new(),
                }
            } else {
                self.diff_content = String::new();
            }
        }
    }

    /// Task 2 & 3: async refresh to prevent UI freeze
    pub async fn async_refresh_status(state: Arc<Mutex<AppState>>) {
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

    pub async fn async_refresh_diff(state: Arc<Mutex<AppState>>) {
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
            let res = tokio::task::spawn_blocking(move || {
                let repo = GitRepo { root };
                repo.diff_file(&path_c)
            }).await;

            if let Ok(Ok(diff)) = res {
                let mut s = state.lock().await;
                // Only update if cursor hasn't moved to a different file
                if let Some(f) = s.current_file() {
                    if f.path == path {
                        s.diff_content = diff;
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

    /// Task 1 fix: Space = stage/unstage with robust fallback cascade.
    pub fn space_stage_unstage(&mut self) {
        match self.display_items.get(self.repo_cursor).cloned() {
            // ── Folder: stage or unstage entire folder ─────────────────
            Some(DisplayItem::FolderHeader { path, .. }) => {
                let files_in: Vec<(usize, bool)> = self.git_files.iter().enumerate()
                    .filter(|(_, f)| f.path.starts_with(&format!("{}/", path)))
                    .map(|(i, f)| (i, f.status.is_staged()))
                    .collect();

                // If ANY are unstaged → stage all; if ALL staged → unstage all
                let any_unstaged = files_in.is_empty() || files_in.iter().any(|(_, s)| !s);

                if let Some(repo) = &self.repo {
                    let result = if any_unstaged {
                        repo.stage_folder(&path)
                    } else {
                        // Task 1 fix: use unstage_folder which handles new files
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

            // ── File: toggle stage / unstage ───────────────────────────
            Some(DisplayItem::FileEntry { file_idx, .. }) => {
                if let Some(file) = self.git_files.get(file_idx) {
                    let path      = file.path.clone();
                    let is_staged = file.status.is_staged();
                    if let Some(repo) = &self.repo {
                        // Task 1 fix: unstage uses robust cascade (restore → rm --cached → reset)
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

    /// Task 4: Enter = ONLY expand/collapse folder. On a file, refresh diff.
    pub fn enter_expand_collapse(&mut self) {
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
                // On a file, Enter refreshes the diff
                self.refresh_diff();
            }
            None => {}
        }
    }

    /// Legacy: kept for compatibility — now unused directly
    pub fn toggle_stage_or_folder(&mut self) {
        self.space_stage_unstage();
    }

    /// Toggle expand/collapse for the folder at the current cursor
    pub fn toggle_folder(&mut self) {
        self.enter_expand_collapse();
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
        self.rebuild_display_items();
        self.repo_cursor = 0;
        self.repo_scroll = 0;
        self.repo = Some(repo);
        self.refresh_diff();
        self.refresh_commit_history();
        self.mode = AppMode::RepoView;
        Ok(was_init)
    }

    /// Update dir suggestions based on current text_input
    pub fn update_dir_suggestions(&mut self) {
        self.dir_suggestions = compute_dir_suggestions(&self.text_input);
        self.suggestion_cursor = None;
    }

    // ── Task 7: Editor helpers ───────────────────────────────────────────────

    /// Open a file in the inline editor
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
                    self.editor_scroll      = 0;
                    self.editor_modified    = false;
                    self.active_frame       = 2;
                    self.mode               = AppMode::Editor;
                }
                Err(e) => self.show_toast(format!("Cannot open: {}", e), ToastType::Error),
            }
        }
    }

    /// Save editor content back to disk
    pub fn editor_save(&mut self) {
        if let Some(repo) = &self.repo {
            let abs_path = repo.root.join(&self.editor_path);
            let content  = self.editor_lines.join("
");
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
        if self.editor_cursor_line < self.editor_scroll {
            self.editor_scroll = self.editor_cursor_line;
        } else if self.editor_cursor_line >= self.editor_scroll + vis {
            self.editor_scroll = self.editor_cursor_line - vis + 1;
        }
    }
}

/// Build the flat display list from files + expanded set
/// Task 2 fix: build display list showing ONLY one level at a time.
/// A folder only shows its DIRECT children (files + sub-folders).
/// Sub-folders are collapsed by default and expand independently.
fn build_display_items(files: &[GitFile], expanded: &HashSet<String>) -> Vec<DisplayItem> {
    let mut items = Vec::new();
    build_level(files, expanded, "", 0, &mut items);
    items
}

/// Recursively build one directory level.
/// `parent` = directory prefix to list children of ("" = root).
fn build_level(
    files:    &[GitFile],
    expanded: &HashSet<String>,
    parent:   &str,
    depth:    usize,
    out:      &mut Vec<DisplayItem>,
) {
    use std::collections::BTreeMap;

    // Separate direct files and direct sub-directories of `parent`
    let mut direct_files:   Vec<usize>          = Vec::new();
    let mut direct_subdirs: BTreeMap<String, usize> = BTreeMap::new(); // subdir_path → child-count

    for (i, f) in files.iter().enumerate() {
        // Strip the parent prefix to get the relative path inside this dir
        let rel = if parent.is_empty() {
            f.path.as_str()
        } else {
            match f.path.strip_prefix(&format!("{}/", parent)) {
                Some(r) => r,
                None    => continue, // not in this subtree
            }
        };

        if let Some(slash) = rel.find('/') {
            // There is at least one more slash — this belongs to a subdir
            let subdir = if parent.is_empty() {
                rel[..slash].to_string()
            } else {
                format!("{}/{}", parent, &rel[..slash])
            };
            *direct_subdirs.entry(subdir).or_insert(0) += 1;
        } else {
            // Direct file of this directory
            direct_files.push(i);
        }
    }

    // 1. Direct files (sorted by name via file index order)
    for idx in direct_files {
        out.push(DisplayItem::FileEntry { file_idx: idx, depth });
    }

    // 2. Direct sub-directories (alphabetical)
    for (subdir_path, count) in &direct_subdirs {
        let is_expanded = expanded.contains(subdir_path);
        out.push(DisplayItem::FolderHeader {
            path:     subdir_path.clone(),
            count:    *count,
            expanded: is_expanded,
            depth,
        });
        // Only recurse one level if THIS folder is expanded
        if is_expanded {
            build_level(files, expanded, subdir_path, depth + 1, out);
        }
    }
}

/// Compute filesystem dir suggestions for a path prefix
fn compute_dir_suggestions(input: &str) -> Vec<String> {
    if input.is_empty() {
        // Show home subdirectories
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
        // Input IS a directory — show its children
        return list_subdirs(&expanded, "");
    }

    // Input is a partial path
    let parent = expanded.parent().unwrap_or(std::path::Path::new("/")).to_path_buf();
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

// ── TUI entry-point ───────────────────────────────────────────────────────────

pub async fn run_tui() -> Result<()> {
    install_panic_hook();
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = SwiftGitConfig::load().unwrap_or_default();
    // Launch flow: SSH setup first (if needed), then PAT auth, then dashboard
    let initial_mode = if !config.ssh_key_added && crate::auth::find_ssh_key().is_none() {
        AppMode::SshSetup
    } else if config.github_token.is_none() {
        AppMode::Auth
    } else {
        AppMode::Dashboard
    };
    let mut state = AppState::new(config);
    state.mode = initial_mode.clone();
    // Trigger SSH detection in background if starting on SshSetup
    if initial_mode == AppMode::SshSetup {
        state.ssh_step = crate::ui::components::ssh_setup::SshSetupStep::Detecting;
    }
    let state = Arc::new(Mutex::new(state));
    let result = event_loop(&mut terminal, state).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<Mutex<AppState>>,
) -> Result<()> {
    loop {
        {
            let mut s = state.lock().await;
            s.frame_count = s.frame_count.wrapping_add(1);
            let fc = s.frame_count;
            if let Some(ref t) = s.toast {
                if t.is_expired() { s.toast = None; }
            }
            terminal.draw(|f| {
                let area = f.size();
                // Ensure the background is always drawn first
                f.render_widget(
                    ratatui::widgets::Block::default()
                        .style(ratatui::style::Style::default().bg(theme::BG_COLOR)),
                    area,
                );
                match &s.mode {
                    AppMode::Auth => {
                        let cv = (fc / 5) % 2 == 0;
                        components::auth::render(f, area, &s.token_input, s.token_cursor,
                            &s.auth_status, cv, s.is_validating);
                    }
                    AppMode::Dashboard => {
                        // Task 1: show display_name on screen, fallback to github username
                        let shown_name = s.config.display_name.as_deref()
                            .or(s.config.username.as_deref());
                        components::dashboard::render(f, area, s.dashboard_cursor,
                            shown_name, &s.config.recent_projects, &s.status_msg);
                    }
                    AppMode::FolderInput => {
                        render_folder_input(f, area, &s.text_input, fc,
                            &s.dir_suggestions, s.suggestion_cursor);
                    }
                    AppMode::CloneInput => {
                        render_text_input(f, area, "Clone Repository",
                            "https://github.com/owner/repo", &s.text_input, fc);
                    }
                    AppMode::Loading => {
                        components::searching::render(f, area, fc, &s.loading_label);
                    }
                    AppMode::GhGrab => {
                        components::ghgrab::render(f, area, fc,
                            &s.ghgrab_owner, &s.ghgrab_repo,
                            &s.ghgrab_files, s.ghgrab_cursor, s.ghgrab_scroll,
                            &s.ghgrab_selected, s.is_loading, &s.ghgrab_expanded);
                    }
                    AppMode::RepoView => {
                        let repo_name = s.repo.as_ref()
                            .map(|r| r.repo_name()).unwrap_or_else(|| "repo".to_string());
                        let status_msg = s.status_msg.clone();
                        let view_state = components::repo_view::RepoViewState {
                            repo_name: &repo_name,
                            branch: &s.branch,
                            files: &s.git_files,
                            display_items: &s.display_items,
                            cursor: s.repo_cursor,
                            scroll_offset: s.repo_scroll,
                            diff_content: &s.diff_content,
                            commit_mode: s.commit_mode,
                            commit_input: &s.commit_input,
                            commit_cursor: s.commit_cursor,
                            commit_history: &s.commit_history,
                            status_msg: &status_msg,
                            is_loading: s.is_loading,
                            is_diff_loading: s.is_diff_loading,
                            frame_count: fc,
                            active_frame: s.active_frame,
                        };
                        components::repo_view::render(f, area, &view_state);
                    }
                    AppMode::Editor => {
                        // Draw repo view underneath, replace diff panel with editor
                        let repo_name = s.repo.as_ref()
                            .map(|r| r.repo_name()).unwrap_or_else(|| "repo".to_string());
                        let status_msg = s.status_msg.clone();
                        let view_state = components::repo_view::RepoViewState {
                            repo_name: &repo_name,
                            branch: &s.branch,
                            files: &s.git_files,
                            display_items: &s.display_items,
                            cursor: s.repo_cursor,
                            scroll_offset: s.repo_scroll,
                            diff_content: "",
                            commit_mode: false,
                            commit_input: "",
                            commit_cursor: 0,
                            commit_history: &s.commit_history,
                            status_msg: &status_msg,
                            is_loading: false,
                            is_diff_loading: false,
                            frame_count: fc,
                            active_frame: 2,
                        };
                        components::repo_view::render_with_editor(f, area, &view_state,
                            &components::editor::EditorState {
                                lines:        &s.editor_lines,
                                cursor_line:  s.editor_cursor_line,
                                cursor_col:   s.editor_cursor_col,
                                scroll_top:   s.editor_scroll,
                                file_path:    &s.editor_path,
                                modified:     s.editor_modified,
                                frame_count:  fc,
                            });
                    }
                    AppMode::PushDialog => {
                        // Overlay the push dialog directly (clean modal over BG)
                        let mut dlg = s.push_dlg.clone();
                        dlg.frame_count = fc;
                        dlg.is_pushing  = s.is_loading;
                        components::push_dialog::render(f, area, &dlg);
                    }
                    AppMode::ForcePushConfirm => {
                        // Overlay push dialog background, then confirm modal
                        let mut dlg = s.push_dlg.clone();
                        dlg.frame_count = fc;
                        dlg.is_pushing  = false;
                        components::push_dialog::render(f, area, &dlg);
                        render_force_push_confirm(f, area, s.force_push_confirm_cursor);
                    }
                    AppMode::SshSetup => {
                        let state_ref = components::ssh_setup::SshSetupState {
                            step:        &s.ssh_step,
                            frame_count: fc,
                        };
                        components::ssh_setup::render(f, area, &state_ref);
                    }
                    AppMode::DeinitConfirm => {
                        // Draw repo view beneath
                        let repo_name = s.repo.as_ref().map(|r| r.repo_name()).unwrap_or_default();
                        let sm = s.status_msg.clone();
                        let vs = components::repo_view::RepoViewState {
                            repo_name: &repo_name, branch: &s.branch,
                            files: &s.git_files, display_items: &s.display_items,
                            cursor: s.repo_cursor, scroll_offset: s.repo_scroll,
                            diff_content: &s.diff_content, commit_mode: false,
                            commit_input: "", commit_cursor: 0,
                            commit_history: &s.commit_history,
                            status_msg: &sm, is_loading: false,
                            is_diff_loading: false,
                            frame_count: fc, active_frame: s.active_frame,
                        };
                        components::repo_view::render(f, area, &vs);
                        render_deinit_confirm(f, area, s.deinit_confirm_cursor);
                    }
                    AppMode::Settings => {
                        // Draw dashboard as background; overlay is rendered after the match
                        // Task 1: show display_name on screen, fallback to github username
                        let shown_name = s.config.display_name.as_deref()
                            .or(s.config.username.as_deref());
                        components::dashboard::render(f, area, s.dashboard_cursor,
                            shown_name, &s.config.recent_projects, &s.status_msg);
                    }
                }
                if let Some(ref toast) = s.toast {
                    components::toast::render(f, area, toast);
                }
                // Global Loading Spinner (bottom-right)
                if s.is_loading {
                    render_global_loading(f, area, s.frame_count, &s.loading_label);
                }
                // Settings overlay draws on top of everything
                if s.mode == AppMode::Settings {
                    components::settings_dialog::render(f, area, &s.settings_dlg, fc);
                }
            })?;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    // Check ctrl+c / ctrl+q while holding lock
                    {
                        let _s = state.lock().await;
                        if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            return Ok(());
                        }
                    }
                    // Global Ctrl+W: open/close Settings overlay from any screen
                    {
                        let mut s = state.lock().await;
                        if key.code == KeyCode::Char('w')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            if s.mode == AppMode::Settings {
                                // Close: return to previous mode
                                let prev = s.settings_prev_mode.clone();
                                s.mode = prev;
                            } else {
                                // Open: populate dialog — display_name and username are separate
                                s.settings_prev_mode = s.mode.clone();
                                s.settings_dlg.display_name = s.config.display_name
                                    .clone().unwrap_or_else(|| s.config.username.clone().unwrap_or_default());
                                s.settings_dlg.username = s.config.username
                                    .clone().unwrap_or_default();
                                s.settings_dlg.token = s.config.github_token
                                    .clone().unwrap_or_default();
                                s.settings_dlg.focused = components::settings_dialog::SettingsField::DisplayName;
                                s.settings_dlg.cursor = s.settings_dlg.display_name.chars().count();
                                s.settings_dlg.show_token = false;
                                s.mode = AppMode::Settings;
                            }
                            continue;
                        }
                    }
                    let quit = handle_key(key, Arc::clone(&state)).await?;
                    if quit { return Ok(()); }
                }
            }
        }
    }
}

// ── Key dispatch ─────────────────────────────────────────────────────────────

async fn handle_key(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    let mode = { state.lock().await.mode.clone() };
    match mode {
        AppMode::Auth => {
            let mut s = state.lock().await;
            handle_auth(key, &mut s).await
        }
        AppMode::Dashboard => {
            let mut s = state.lock().await;
            handle_dashboard(key, &mut s)
        }
        AppMode::FolderInput => {
            let mut s = state.lock().await;
            handle_folder_input(key, &mut s)
        }
        AppMode::CloneInput => {
            // May spawn async fetch — needs Arc
            handle_clone_input(key, state).await
        }
        AppMode::RepoView => {
            // Push/pull release lock while running — needs Arc
            handle_repo_view(key, state).await
        }
        AppMode::GhGrab => {
            handle_ghgrab(key, state).await
        }
        AppMode::PushDialog => {
            handle_push_dialog(key, state).await
        }
        AppMode::ForcePushConfirm => {
            handle_force_push_confirm(key, state).await
        }
        AppMode::SshSetup => {
            handle_ssh_setup(key, state).await
        }
        AppMode::DeinitConfirm => {
            let mut s = state.lock().await;
            Ok(handle_deinit_confirm(key, &mut s))
        }
        AppMode::Editor => {
            let mut s = state.lock().await;
            Ok(handle_editor(key, &mut s))
        }
        AppMode::Loading => {
            if key.code == KeyCode::Esc {
                state.lock().await.mode = AppMode::Dashboard;
            }
            Ok(false)
        }
        AppMode::Settings => {
            let mut s = state.lock().await;
            Ok(handle_settings(key, &mut s))
        }
    }
}

// ── Auth ─────────────────────────────────────────────────────────────────────

async fn handle_auth(key: KeyEvent, s: &mut AppState) -> Result<bool> {
    match key.code {
        KeyCode::Esc => { s.mode = AppMode::Dashboard; }
        KeyCode::Enter => {
            let token = s.token_input.trim().to_string();
            if token.is_empty() { s.mode = AppMode::Dashboard; return Ok(false); }
            s.is_validating = true;
            s.auth_status = "Validating...".to_string();
            match validate_github_token(&token).await {
                Ok(username) => {
                    s.auth_status = format!("Welcome, {}!", username);
                    s.config.github_token = Some(token);
                    s.config.username = Some(username.clone());
                    let _ = s.config.save();
                    s.show_toast(format!("Logged in as @{}", username), ToastType::Success);
                    s.mode = AppMode::Dashboard;
                }
                Err(e) => { s.auth_status = format!("Invalid token: {}", e); }
            }
            s.is_validating = false;
        }
        KeyCode::Backspace => {
            if s.token_cursor > 0 {
                let bp = s.token_input.char_indices().nth(s.token_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                s.token_input.remove(bp);
                s.token_cursor -= 1;
            }
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let bp = s.token_input.char_indices().nth(s.token_cursor).map(|(i,_)| i).unwrap_or(s.token_input.len());
            s.token_input.insert(bp, c);
            s.token_cursor += 1;
            s.auth_status.clear();
        }
        _ => {}
    }
    Ok(false)
}

async fn validate_github_token(token: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client.get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "swiftgit/1.1")
        .send().await.context("Network error")?;
    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await?;
        Ok(json["login"].as_str().unwrap_or("user").to_string())
    } else if resp.status() == 401 {
        anyhow::bail!("Invalid token")
    } else {
        anyhow::bail!("GitHub returned: {}", resp.status())
    }
}

// ── Settings dialog handler ───────────────────────────────────────────────────

fn handle_settings(key: KeyEvent, s: &mut AppState) -> bool {
    use components::settings_dialog::SettingsField;
    match key.code {
        // Close without saving
        KeyCode::Esc => {
            let prev = s.settings_prev_mode.clone();
            s.mode = prev;
        }
        // Save and close
        KeyCode::Enter => {
            let name  = s.settings_dlg.display_name.trim().to_string();
            let user  = s.settings_dlg.username.trim().to_string();
            let token = s.settings_dlg.token.trim().to_string();
            // Task 1: display_name shown on screen; username used for GitHub API
            if !name.is_empty() { s.config.display_name = Some(name); }
            if !user.is_empty() { s.config.username = Some(user.clone()); }
            if !token.is_empty() { s.config.github_token = Some(token); }
            let _ = s.config.save();
            s.show_toast("✅ Settings saved", ToastType::Success);
            let prev = s.settings_prev_mode.clone();
            s.mode = prev;
        }
        // Move focus down / next field
        KeyCode::Tab | KeyCode::Down => {
            let next = s.settings_dlg.focused.next();
            s.settings_dlg.focused = next;
            s.settings_dlg.cursor = s.settings_dlg.active_field_text().chars().count();
            if s.settings_dlg.focused == SettingsField::Token {
                s.settings_dlg.show_token = false;
            }
        }
        // Move focus up / prev field
        KeyCode::Up => {
            let prev = s.settings_dlg.focused.prev();
            s.settings_dlg.focused = prev;
            s.settings_dlg.cursor = s.settings_dlg.active_field_text().chars().count();
        }
        // Shift+Tab: prev field (or toggle token visibility on Token field)
        KeyCode::BackTab => {
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
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            s.settings_dlg.type_char(c);
        }
        _ => {}
    }
    false
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

fn handle_dashboard(key: KeyEvent, s: &mut AppState) -> Result<bool> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Up | KeyCode::Char('k') => {
            if s.dashboard_cursor > 0 { s.dashboard_cursor -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if s.dashboard_cursor < 2 { s.dashboard_cursor += 1; }
        }
        KeyCode::Enter => {
            s.status_msg.clear();
            match s.dashboard_cursor {
                0 => {
                    s.text_input.clear(); s.text_cursor = 0;
                    s.dir_suggestions.clear(); s.suggestion_cursor = None;
                    s.mode = AppMode::FolderInput;
                }
                1 => { s.text_input.clear(); s.text_cursor = 0; s.mode = AppMode::CloneInput; }
                2 => {
                    if let Some(recent) = s.config.recent_projects.first().cloned() {
                        let path = PathBuf::from(&recent.path);
                        match s.open_repo(path) {
                            Ok(was_init) => { if was_init { s.show_toast("Initialized new repo", ToastType::Info); } }
                            Err(e) => s.show_toast(format!("Error: {}", e), ToastType::Error),
                        }
                    } else {
                        s.show_toast("No recent projects yet", ToastType::Info);
                    }
                }
                _ => {}
            }
        }
        KeyCode::Char('1') => {
            s.text_input.clear(); s.text_cursor = 0;
            s.dir_suggestions.clear(); s.suggestion_cursor = None;
            s.mode = AppMode::FolderInput;
        }
        KeyCode::Char('2') => { s.text_input.clear(); s.text_cursor = 0; s.mode = AppMode::CloneInput; }
        _ => {}
    }
    Ok(false)
}

// ── Folder input (with dir suggestions) ──────────────────────────────────────

fn handle_folder_input(key: KeyEvent, s: &mut AppState) -> Result<bool> {
    match key.code {
        KeyCode::Esc => { s.mode = AppMode::Dashboard; }
        KeyCode::Tab => {
            // Cycle through suggestions
            if s.dir_suggestions.is_empty() {
                s.update_dir_suggestions();
            }
            if !s.dir_suggestions.is_empty() {
                let next = match s.suggestion_cursor {
                    None => 0,
                    Some(i) => (i + 1) % s.dir_suggestions.len(),
                };
                s.suggestion_cursor = Some(next);
                s.text_input = s.dir_suggestions[next].clone();
                s.text_cursor = s.text_input.chars().count();
            }
        }
        KeyCode::Up => {
            // Navigate suggestions upward
            if !s.dir_suggestions.is_empty() {
                let next = match s.suggestion_cursor {
                    None | Some(0) => s.dir_suggestions.len().saturating_sub(1),
                    Some(i) => i - 1,
                };
                s.suggestion_cursor = Some(next);
                s.text_input = s.dir_suggestions[next].clone();
                s.text_cursor = s.text_input.chars().count();
            }
        }
        KeyCode::Down => {
            // Navigate suggestions downward
            if !s.dir_suggestions.is_empty() {
                let next = match s.suggestion_cursor {
                    None => 0,
                    Some(i) => (i + 1) % s.dir_suggestions.len(),
                };
                s.suggestion_cursor = Some(next);
                s.text_input = s.dir_suggestions[next].clone();
                s.text_cursor = s.text_input.chars().count();
            }
        }
        KeyCode::Enter => {
            let raw = s.text_input.trim().to_string();
            if raw.is_empty() { s.show_toast("Please enter a path", ToastType::Warning); return Ok(false); }
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
                Ok(was_init) => { if was_init { s.show_toast("Initialized new git repo!", ToastType::Success); } }
                Err(e) => s.show_toast(format!("Error: {}", e), ToastType::Error),
            }
        }
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                s.text_input.clear(); s.text_cursor = 0;
            } else if s.text_cursor > 0 {
                let bp = s.text_input.char_indices().nth(s.text_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                s.text_input.remove(bp); s.text_cursor -= 1;
            }
            s.update_dir_suggestions();
        }
        KeyCode::Left => { if s.text_cursor > 0 { s.text_cursor -= 1; } }
        KeyCode::Right => { if s.text_cursor < s.text_input.chars().count() { s.text_cursor += 1; } }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let bp = s.text_input.char_indices().nth(s.text_cursor).map(|(i,_)| i).unwrap_or(s.text_input.len());
            s.text_input.insert(bp, c); s.text_cursor += 1;
            s.update_dir_suggestions();
        }
        _ => {}
    }
    Ok(false)
}

// ── Clone input (with GhGrab picker) ─────────────────────────────────────────

async fn handle_clone_input(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.lock().await.mode = AppMode::Dashboard;
            return Ok(false);
        }
        KeyCode::Enter => {
            let (url, token) = {
                let s = state.lock().await;
                (s.text_input.trim().to_string(), s.config.github_token.clone())
            };

            if url.is_empty() {
                state.lock().await.show_toast("Please enter a URL", ToastType::Warning);
                return Ok(false);
            }

            // Check if it's a GitHub URL — offer file picker
            if let Some((owner, repo_name)) = crate::git::parse_github_url(&url) {
                {
                    let mut s = state.lock().await;
                    s.ghgrab_url = url.clone();
                    s.ghgrab_owner = owner.clone();
                    s.ghgrab_repo = repo_name.clone();
                    s.ghgrab_files.clear();
                    s.ghgrab_selected.clear();
                    s.ghgrab_cursor = 0;
                    s.ghgrab_scroll = 0;
                    s.is_loading = true;
                    s.loading_label = format!("Fetching file list for {}/{}...", owner, repo_name);
                    s.mode = AppMode::Loading;
                }

                // Fetch file list in a background task
                let state_c = Arc::clone(&state);
                let owner_c = owner.clone();
                let repo_c = repo_name.clone();
                let tok = token.clone();
                tokio::task::spawn(async move {
                    let files_result = tokio::task::spawn_blocking(move || {
                        crate::git::fetch_github_files(&owner_c, &repo_c, tok.as_deref())
                    }).await;

                    let mut s = state_c.lock().await;
                    s.is_loading = false;
                    match files_result {
                        Ok(Ok(files)) => {
                            s.ghgrab_files = files;
                            s.mode = AppMode::GhGrab;
                        }
                        _ => {
                            s.show_toast("Failed to fetch files", ToastType::Error);
                            s.mode = AppMode::CloneInput;
                        }
                    }
                });
            } else {
                // Non-GitHub or SSH URL — full clone immediately
                {
                    let mut s = state.lock().await;
                    s.loading_label = format!("Cloning {}...", url);
                    s.is_loading = true;
                    s.mode = AppMode::Loading;
                }

                let url_c = url.clone();
                let state_c = Arc::clone(&state);
                let clone_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join("swiftgit-repos");
                let _ = std::fs::create_dir_all(&clone_dir);
                let clone_dir_c = clone_dir.clone();

                tokio::task::spawn(async move {
                    let output = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .args(["clone", &url_c])
                            .current_dir(&clone_dir_c)
                            .output()
                    }).await;

                    let mut s = state_c.lock().await;
                    s.is_loading = false;
                    match output {
                        Ok(Ok(out)) if out.status.success() => {
                            let repo_name = url.trim_end_matches('/').split('/').next_back()
                                .unwrap_or("repo").trim_end_matches(".git").to_string();
                            let repo_path = clone_dir.join(&repo_name);
                            match s.open_repo(repo_path) {
                                Ok(_) => s.show_toast(format!("Cloned: {}", repo_name), ToastType::Success),
                                Err(e) => { s.mode = AppMode::Dashboard; s.show_toast(format!("{}", e), ToastType::Warning); }
                            }
                        }
                        _ => {
                            s.mode = AppMode::Dashboard;
                            s.show_toast("Clone failed", ToastType::Error);
                        }
                    }
                });
            }
        }
        KeyCode::Backspace => {
            let mut s = state.lock().await;
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                s.text_input.clear(); s.text_cursor = 0;
            } else if s.text_cursor > 0 {
                let bp = s.text_input.char_indices().nth(s.text_cursor - 1).map(|(i,_)| i).unwrap_or(0);
                s.text_input.remove(bp); s.text_cursor -= 1;
            }
        }
        KeyCode::Left => {
            let mut s = state.lock().await;
            if s.text_cursor > 0 { s.text_cursor -= 1; }
        }
        KeyCode::Right => {
            let mut s = state.lock().await;
            if s.text_cursor < s.text_input.chars().count() { s.text_cursor += 1; }
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.lock().await;
            let bp = s.text_input.char_indices().nth(s.text_cursor).map(|(i,_)| i).unwrap_or(s.text_input.len());
            s.text_input.insert(bp, c); s.text_cursor += 1;
        }
        _ => {}
    }
    Ok(false)
}

// ── GhGrab file picker ────────────────────────────────────────────────────────

/// Build tree items (same logic as ghgrab component) to know what's at cursor
fn ghgrab_tree_cursor_path(files: &[crate::git::GhFile], expanded: &std::collections::HashSet<String>, cursor: usize) -> Option<GhGrabCursorItem> {
    let mut items = Vec::new();
    ghgrab_build_level(files, expanded, "", &mut items);
    items.into_iter().nth(cursor)
}

#[derive(Clone)]
enum GhGrabCursorItem {
    Folder(String),       // folder path
    File(usize),          // file_idx into files slice
}

fn ghgrab_build_level(
    files: &[crate::git::GhFile],
    expanded: &std::collections::HashSet<String>,
    parent: &str,
    out: &mut Vec<GhGrabCursorItem>,
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

    for idx in direct_files { out.push(GhGrabCursorItem::File(idx)); }
    for (dir_path, _) in &subdirs {
        out.push(GhGrabCursorItem::Folder(dir_path.clone()));
        if expanded.contains(dir_path) {
            ghgrab_build_level(files, expanded, dir_path, out);
        }
    }
}

fn ghgrab_tree_len(files: &[crate::git::GhFile], expanded: &std::collections::HashSet<String>) -> usize {
    let mut items = Vec::new();
    ghgrab_build_level(files, expanded, "", &mut items);
    items.len()
}

async fn handle_ghgrab(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            let mut s = state.lock().await;
            s.mode = AppMode::CloneInput;
            return Ok(false);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let mut s = state.lock().await;
            if s.ghgrab_cursor > 0 {
                s.ghgrab_cursor -= 1;
                if s.ghgrab_cursor < s.ghgrab_scroll { s.ghgrab_scroll = s.ghgrab_cursor; }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let mut s = state.lock().await;
            let tree_len = ghgrab_tree_len(&s.ghgrab_files, &s.ghgrab_expanded);
            let max = tree_len.saturating_sub(1);
            if s.ghgrab_cursor < max {
                s.ghgrab_cursor += 1;
                let visible = 25usize;
                if s.ghgrab_cursor >= s.ghgrab_scroll + visible {
                    s.ghgrab_scroll = s.ghgrab_cursor - visible + 1;
                }
            }
        }
        // Task 2: Enter = expand/collapse folder
        KeyCode::Enter => {
            let mut s = state.lock().await;
            let cursor = s.ghgrab_cursor;
            if let Some(item) = ghgrab_tree_cursor_path(&s.ghgrab_files, &s.ghgrab_expanded, cursor) {
                match item {
                    GhGrabCursorItem::Folder(path) => {
                        if s.ghgrab_expanded.contains(&path) {
                            s.ghgrab_expanded.remove(&path);
                        } else {
                            s.ghgrab_expanded.insert(path);
                        }
                    }
                    GhGrabCursorItem::File(_) => {}
                }
            }
        }
        KeyCode::Char(' ') => {
            let mut s = state.lock().await;
            let cursor = s.ghgrab_cursor;
            if let Some(item) = ghgrab_tree_cursor_path(&s.ghgrab_files, &s.ghgrab_expanded, cursor) {
                if let GhGrabCursorItem::File(file_idx) = item {
                    if s.ghgrab_selected.contains(&file_idx) {
                        s.ghgrab_selected.remove(&file_idx);
                    } else {
                        s.ghgrab_selected.insert(file_idx);
                    }
                }
            }
        }
        KeyCode::Char('a') => {
            // Select ALL — trigger full clone
            let (url, _token) = {
                let s = state.lock().await;
                (s.ghgrab_url.clone(), s.config.github_token.clone())
            };
            {
                let mut s = state.lock().await;
                s.is_loading = true;
                s.loading_label = "Cloning entire repo...".to_string();
                s.mode = AppMode::Loading;
            }

            let state_c = Arc::clone(&state);
            let clone_dir = dirs::home_dir().unwrap_or_default().join("swiftgit-repos");
            let _ = std::fs::create_dir_all(&clone_dir);
            let clone_dir_c = clone_dir.clone();
            let url_c = url.clone();

            tokio::task::spawn(async move {
                let output = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("git")
                        .args(["clone", &url_c])
                        .current_dir(&clone_dir_c)
                        .output()
                }).await;

                let mut s = state_c.lock().await;
                s.is_loading = false;
                match output {
                    Ok(Ok(out)) if out.status.success() => {
                        let rname = url.trim_end_matches('/').split('/').next_back()
                            .unwrap_or("repo").trim_end_matches(".git").to_string();
                        let rpath = clone_dir.join(&rname);
                        match s.open_repo(rpath) {
                            Ok(_) => s.show_toast(format!("Cloned all: {}", rname), ToastType::Success),
                            Err(e) => { s.mode = AppMode::Dashboard; s.show_toast(format!("{}", e), ToastType::Warning); }
                        }
                    }
                    _ => {
                        s.mode = AppMode::Dashboard;
                        s.show_toast("Clone failed", ToastType::Error);
                    }
                }
            });
        }
        KeyCode::Char('d') => {
            // 'd' = Download only selected files
            let (owner, repo_name, selected, token) = {
                let s = state.lock().await;
                if s.ghgrab_selected.is_empty() {
                    drop(s);
                    state.lock().await.show_toast("No files selected — Space to select, 'a' for all", ToastType::Warning);
                    return Ok(false);
                }
                let paths: Vec<String> = s.ghgrab_selected.iter()
                    .filter_map(|&i| s.ghgrab_files.get(i))
                    .map(|f| f.path.clone())
                    .collect();
                (s.ghgrab_owner.clone(), s.ghgrab_repo.clone(), paths, s.config.github_token.clone())
            };

            {
                let mut s = state.lock().await;
                s.is_loading = true;
                s.loading_label = format!("Downloading {} file(s)...", selected.len());
                s.mode = AppMode::Loading;
            }

            let state_c = Arc::clone(&state);
            let dest_dir = dirs::home_dir().unwrap_or_default()
                .join("swiftgit-repos")
                .join(&repo_name);
            let _ = std::fs::create_dir_all(&dest_dir);

            let owner_c = owner.clone();
            let repo_c = repo_name.clone();
            let tok_c = token.clone();
            let dest_c = dest_dir.clone();
            let selected_c = selected.clone();

            tokio::task::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    let mut errors = Vec::new();
                    for path in &selected_c {
                        if let Err(e) = crate::git::download_github_file(&owner_c, &repo_c, path, &dest_c, tok_c.as_deref()) {
                            errors.push(format!("{}: {}", path, e));
                        }
                    }
                    errors
                }).await;

                let mut s = state_c.lock().await;
                s.is_loading = false;

                match result {
                    Ok(errors) => {
                        if !errors.is_empty() {
                            s.mode = AppMode::Dashboard;
                            s.show_toast(format!("{} error(s) during download", errors.len()), ToastType::Warning);
                        } else {
                            s.show_toast(format!("Downloaded {} file(s) to ~/swiftgit-repos/{}", selected.len(), repo_name), ToastType::Success);
                            s.mode = AppMode::Dashboard;
                        }
                    }
                    _ => {
                        s.mode = AppMode::Dashboard;
                        s.show_toast("Download failed", ToastType::Error);
                    }
                }
            });
        }
        _ => {}
    }
    Ok(false)
}

// ── Repo view ─────────────────────────────────────────────────────────────────

async fn handle_repo_view(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    // Check commit mode first
    let commit_mode = { state.lock().await.commit_mode };
    if commit_mode {
        return handle_commit_input(key, state).await;
    }

    match key.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc => {
            let mut s = state.lock().await;
            s.mode = AppMode::Dashboard;
            s.status_msg.clear();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            {
                let mut s = state.lock().await;
                s.move_up();
                // Task 2: clear diff before loading async (optional)
                // s.diff_content.clear();
            }
            AppState::async_refresh_diff(Arc::clone(&state)).await;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            {
                let mut s = state.lock().await;
                s.move_down();
            }
            AppState::async_refresh_diff(Arc::clone(&state)).await;
        }
        // Task 4: Space = stage/unstage ONLY (file or entire folder)
        KeyCode::Char(' ') => {
            {
                let mut s = state.lock().await;
                s.space_stage_unstage();
            }
            AppState::async_refresh_status(Arc::clone(&state)).await;
        }
        // Task 4: Enter = expand/collapse folder ONLY (on file: refresh diff)
        KeyCode::Enter => {
            let mut s = state.lock().await;
            s.enter_expand_collapse();
        }
        KeyCode::Char('c') => {
            let mut s = state.lock().await;
            let has_staged = s.git_files.iter().any(|f| f.status.is_staged());
            if has_staged {
                s.commit_mode = true;
                s.commit_input.clear();
                s.commit_cursor = 0;
                // Show recent commits in history
                if let Some(repo) = &s.repo {
                    s.commit_history = crate::git::recent_commits(&repo.root, 5);
                }
            } else {
                s.show_toast("No staged files — Space to stage", ToastType::Warning);
            }
        }
        // 's' = stage ALL changed files at once (git add -A)
        KeyCode::Char('s') => {
            let root = {
                let s = state.lock().await;
                s.repo.as_ref().map(|r| r.root.clone())
            };
            if let Some(root) = root {
                let out = std::process::Command::new("git")
                    .args(["add", "-A"])
                    .current_dir(&root)
                    .output();
                {
                    let mut s = state.lock().await;
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
                AppState::async_refresh_status(Arc::clone(&state)).await;
            }
        }
        // Task 5: 'p' opens push dialog (repo picker + commit log)
        // 'p' = push via SSH if key ready, else PAT dialog
        KeyCode::Char('p') => {
            let (_token, root, repo_name, branch, has_remote) = {
                let s = state.lock().await;
                let token  = s.config.github_token.clone();
                let root   = s.repo.as_ref().map(|r| r.root.clone());
                let name   = s.repo.as_ref().map(|r| r.repo_name()).unwrap_or_default();
                let branch = s.branch.clone();
                let has_remote = root.as_ref().map(|r| {
                    crate::git::GitRepo { root: r.clone() }
                        .get_remote_url()
                        .map(|u| !u.trim().is_empty())
                        .unwrap_or(false)
                }).unwrap_or(false);
                (token, root, name, branch, has_remote)
            };

            if has_remote {
                // ── Remote set: check if SSH key ready → use SSH directly ──────
                let remote_url = {
                    let s = state.lock().await;
                    s.repo.as_ref().map(|r| r.get_remote_url_safe()).unwrap_or_default()
                };
                let remote_repo = remote_url.trim_end_matches('/')
                    .trim_end_matches(".git")
                    .split('/').next_back().unwrap_or(&repo_name).to_string();

                // If remote is HTTPS and SSH key is ready, convert and push via SSH
                let ssh_ready = crate::auth::test_ssh_github().is_ok();
                if ssh_ready && !remote_url.starts_with("git@") {
                    // Convert to SSH and push
                    let root_c   = root.clone().unwrap();
                    let branch_c = branch.clone();
                    {
                        let mut s = state.lock().await;
                        s.status_msg = "Pushing via SSH…".to_string();
                        s.loading_label = "Pushing via SSH".to_string();
                        s.is_loading = true;
                    }
                    let result = tokio::task::spawn_blocking(move || {
                        crate::auth::push_via_ssh(&root_c, &branch_c, false)
                    }).await?;
                    let mut s = state.lock().await;
                    s.is_loading = false;
                    match result {
                        Ok(_)  => {
                            s.status_msg.clear();
                            s.show_toast("✅ Pushed via SSH!", ToastType::Success);
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            let is_rejected = err_str.contains("rejected") ||
                                              err_str.contains("non-fast-forward") ||
                                              err_str.contains("fetch first");

                            if is_rejected {
                                // Populate push_dlg state before switching to ForcePushConfirm
                                s.push_dlg.repo_name   = remote_repo;
                                s.push_dlg.branch      = branch.clone();
                                s.push_dlg.username    = s.config.username.clone().unwrap_or_default();
                                s.push_dlg.origin      = remote_url;
                                s.push_dlg.force_push  = false;
                                s.push_dlg.status_msg  = "⚠ Push rejected — force push?".to_string();

                                s.force_push_confirm_cursor = 0;
                                s.mode = AppMode::ForcePushConfirm;
                            } else {
                                s.status_msg = format!("❌ {}", e);
                                s.show_toast(format!("❌ Push failed: {}", e), ToastType::Error);
                            }
                        }
                    }
                    return Ok(false);
                }
                // Extract repo name from remote URL
                let (recent, branches, has_commits_flag) = if let Some(r) = &root {
                    let repo = crate::git::GitRepo { root: r.clone() };
                    (crate::git::recent_commits(r, 5), repo.list_branches(), repo.has_commits())
                } else { (vec![], vec!["main".to_string()], false) };

                let branches = if branches.is_empty() { vec!["main".to_string()] } else { branches };

                let mut s = state.lock().await;
                s.push_dlg.repo_name      = remote_repo;
                s.push_dlg.branch         = branch.clone();
                s.push_dlg.branch_list    = branches;
                s.push_dlg.branch_open    = false;
                s.push_dlg.username       = s.config.username.clone().unwrap_or_default();
                s.push_dlg.commit_msg     = recent.first().cloned().unwrap_or_default();
                s.push_dlg.recent_commits = recent;
                s.push_dlg.origin         = remote_url;
                s.push_dlg.cursor         = s.push_dlg.repo_name.chars().count();
                s.push_dlg.focused        = components::push_dialog::PushField::RepoName;
                s.push_dlg.status_msg.clear();
                s.push_dlg.is_pushing     = false;
                s.push_dlg.has_commits    = has_commits_flag;
                s.push_dlg.sync_branch_cursor();
                s.mode = AppMode::PushDialog;
            } else {
                // ── No remote: open lean push dialog ──────────────────────────
                let (recent, branches, has_commits_flag) = if let Some(r) = &root {
                    let repo = crate::git::GitRepo { root: r.clone() };
                    (crate::git::recent_commits(r, 5), repo.list_branches(), repo.has_commits())
                } else { (vec![], vec!["main".to_string()], false) };

                let branches = if branches.is_empty() { vec!["main".to_string()] } else { branches };

                let mut s = state.lock().await;
                s.push_dlg.repo_name      = repo_name.clone();
                s.push_dlg.branch         = branch.clone();
                s.push_dlg.branch_list    = branches;
                s.push_dlg.branch_open    = false;
                s.push_dlg.username       = s.config.username.clone().unwrap_or_default();
                s.push_dlg.commit_msg     = recent.first().cloned().unwrap_or_default();
                s.push_dlg.recent_commits = recent;
                s.push_dlg.cursor         = s.push_dlg.repo_name.chars().count();
                s.push_dlg.focused        = components::push_dialog::PushField::RepoName;
                s.push_dlg.status_msg.clear();
                s.push_dlg.is_pushing     = false;
                s.push_dlg.has_commits    = has_commits_flag;
                s.push_dlg.sync_branch_cursor();
                s.push_dlg.update_origin();
                s.mode = AppMode::PushDialog;
            }
        }
        // 'P' = pull via SSH if key ready, else via PAT
        KeyCode::Char('P') => {
            let (token, root) = {
                let mut s = state.lock().await;
                s.status_msg = format!("{} Pulling…", spinner_char(s.frame_count));
                s.loading_label = "Pulling from remote".to_string();
                s.is_loading = true;
                (s.config.github_token.clone(), s.repo.as_ref().map(|r| r.root.clone()))
            };

            let root = match root { Some(r) => r, None => { return Ok(false); } };
            let tok  = token.clone();
            let root_c = root.clone();
            let state_c = Arc::clone(&state);

            tokio::task::spawn(async move {
                // Prefer SSH if key is available
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
                        let msg = if first.contains("Already up to date")
                                  || first.contains("up-to-date") {
                            "✅ Already up-to-date".to_string()
                        } else {
                            format!("✅ {}", first)
                        };
                        s.status_msg.clear(); // Task 2: toast only, don't block keybinds
                        s.show_toast(msg, ToastType::Success);
                        drop(s);
                        AppState::async_refresh_status(Arc::clone(&state_c)).await;
                    }
                    _ => {
                        let msg = "❌ Pull failed".to_string();
                        s.status_msg = msg.clone();
                        s.show_toast(msg, ToastType::Error);
                    }
                }
            });
        }
        KeyCode::Char('r') => {
            AppState::async_refresh_status(Arc::clone(&state)).await;
            state.lock().await.show_toast("Refreshed", ToastType::Info);
        }

        // Task 1: 'X' (shift+x) = deinit confirmation
        KeyCode::Char('X') => {
            let mut s = state.lock().await;
            s.deinit_confirm_cursor = 0; // default to Cancel (safe)
            s.mode = AppMode::DeinitConfirm;
        }

        // Task 7: 'e' opens inline editor for the current file
        KeyCode::Char('e') => {
            let path_opt = {
                let s = state.lock().await;
                s.current_file().map(|f| f.path.clone())
            };
            if let Some(path) = path_opt {
                let mut s = state.lock().await;
                s.editor_open(&path.clone());
            } else {
                state.lock().await.show_toast(
                    "Navigate to a file first, then press e", ToastType::Info);
            }
        }

        // Task 6: number keys 1/2 switch active frame
        KeyCode::Char('1') => {
            let mut s = state.lock().await;
            s.active_frame = 1;
        }
        KeyCode::Char('2') => {
            let mut s = state.lock().await;
            s.active_frame = 2;
        }

        _ => {}
    }
    Ok(false)
}

async fn handle_commit_input(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    let mut s = state.lock().await;
    match key.code {
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
                        // Task 2 fix: show toast but clear status_msg so keybinds reappear
                        s.status_msg.clear();
                        s.show_toast(format!("✅ {}", summary), ToastType::Success);
                        // Reset cursor so tree renders from top after commit
                        s.repo_cursor = 0;
                        s.repo_scroll = 0;
                        drop(s);
                        AppState::async_refresh_status(Arc::clone(&state)).await;
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
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let bp = s.commit_input.char_indices().nth(s.commit_cursor).map(|(i,_)| i).unwrap_or(s.commit_input.len());
            s.commit_input.insert(bp, c);
            s.commit_cursor += 1;
        }
        _ => {}
    }
    Ok(false)
}

// ── Rendering helpers ─────────────────────────────────────────────────────────

fn render_text_input(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    title: &str,
    placeholder: &str,
    input: &str,
    frame_count: u64,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(35), Constraint::Length(3), Constraint::Length(3), Constraint::Length(2), Constraint::Min(0)])
        .split(area);
    let title_widget = Paragraph::new(format!("  {}", title))
        .style(Style::default().fg(theme::ACCENT_COLOR).add_modifier(Modifier::BOLD));
    f.render_widget(title_widget, vertical[1]);
    let input_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(70), Constraint::Percentage(15)])
        .split(vertical[2]);
    let cursor_char = if (frame_count / 5) % 2 == 0 { "█" } else { "" };
    let display = if input.is_empty() { format!(" {}{}", placeholder, cursor_char) } else { format!(" {}{}", input, cursor_char) };
    let text_color = if input.is_empty() { theme::BORDER_COLOR } else { theme::FG_COLOR };
    let input_widget = Paragraph::new(display)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme::ACCENT_COLOR)).style(Style::default().bg(theme::BG_COLOR)))
        .style(Style::default().fg(text_color));
    f.render_widget(input_widget, input_col[1]);
    let hint_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(70), Constraint::Percentage(15)])
        .split(vertical[3]);
    let hint = Paragraph::new("Enter to confirm  •  Esc to cancel  •  Ctrl+Backspace to clear")
        .style(Style::default().fg(theme::BORDER_COLOR));
    f.render_widget(hint, hint_col[1]);
}

/// Like render_text_input but also shows a directory suggestion dropdown
fn render_folder_input(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    input: &str,
    frame_count: u64,
    suggestions: &[String],
    suggestion_cursor: Option<usize>,
) {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::Style,
        text::{Line, Span},
        widgets::{Block, Borders, Clear, List, ListItem},
    };

    // Reuse the standard input render
    render_text_input(f, area, "Open Folder", "/path/to/your/project", input, frame_count);

    if suggestions.is_empty() { return; }

    // Position the suggestion box just below the input field
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(3),  // title
            Constraint::Length(3),  // input
            Constraint::Length(2),  // hint
            Constraint::Min(0),
        ])
        .split(area);

    let input_col = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(70), Constraint::Percentage(15)])
        .split(vertical[2]);

    let list_height = (suggestions.len().min(8) + 2) as u16;
    let popup_area = ratatui::layout::Rect {
        x: input_col[1].x,
        y: input_col[1].y + input_col[1].height,
        width: input_col[1].width,
        height: list_height,
    };

    // Don't overflow the terminal
    if popup_area.y + popup_area.height > area.height { return; }

    let items: Vec<ListItem> = suggestions.iter().enumerate().map(|(i, s)| {
        // Show just the last component for readability
        let display = s.split('/').next_back().unwrap_or(s);
        let full = s.as_str();
        let selected = suggestion_cursor == Some(i);
        if selected {
            ListItem::new(Line::from(vec![
                Span::styled(" ▶ ", Style::default().fg(theme::BG_COLOR).bg(theme::ACCENT_COLOR)),
                Span::styled(format!("{:<20} {}", display, full),
                    Style::default().fg(theme::BG_COLOR).bg(theme::ACCENT_COLOR)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{:<20} {}", display, full),
                    Style::default().fg(theme::FG_COLOR)),
            ]))
        }
    }).collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Directories (Tab to cycle) ")
            .border_style(Style::default().fg(theme::BORDER_COLOR))
            .style(Style::default().bg(theme::BG_COLOR)),
    );

    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
}

fn render_global_loading(f: &mut ratatui::Frame, area: ratatui::layout::Rect, frame_count: u64, label: &str) {
    use ratatui::{
        layout::{Alignment, Rect},
        widgets::{Block, Borders, Paragraph},
        style::Style,
    };

    let text = if label.is_empty() {
        format!(" {} Loading… ", spinner_char(frame_count))
    } else {
        format!(" {} {}… ", spinner_char(frame_count), label)
    };

    let width = (text.chars().count() + 4) as u16;
    let loading_area = Rect::new(
        area.width.saturating_sub(width + 2),
        area.height.saturating_sub(3),
        width,
        3,
    );

    f.render_widget(ratatui::widgets::Clear, loading_area);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme::ACCENT_COLOR))
                    .style(Style::default().bg(theme::BG_COLOR)),
            )
            .alignment(Alignment::Center),
        loading_area,
    );
}

// ── Task 3: Push dialog handler ───────────────────────────────────────────────

async fn handle_push_dialog(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    use components::push_dialog::PushField;

    match key.code {
        KeyCode::Esc => {
            let mut s = state.lock().await;
            if s.push_dlg.branch_open {
                // Close dropdown without selecting
                s.push_dlg.branch_open = false;
            } else {
                s.mode = AppMode::RepoView;
            }
        }

        // Tab / Down — next field, or navigate dropdown
        KeyCode::Tab | KeyCode::Down => {
            let mut s = state.lock().await;
            if s.push_dlg.branch_open {
                // Navigate down in dropdown
                let max = s.push_dlg.branch_list.len().saturating_sub(1);
                if s.push_dlg.branch_cursor < max { s.push_dlg.branch_cursor += 1; }
            } else {
                let next = s.push_dlg.focused.next();
                s.push_dlg.focused = next;
                s.push_dlg.clamp_cursor();
            }
        }
        // Shift+Tab / Up — prev field, or navigate dropdown up
        KeyCode::BackTab | KeyCode::Up => {
            let mut s = state.lock().await;
            if s.push_dlg.branch_open {
                if s.push_dlg.branch_cursor > 0 { s.push_dlg.branch_cursor -= 1; }
            } else {
                let prev = s.push_dlg.focused.prev();
                s.push_dlg.focused = prev;
                s.push_dlg.clamp_cursor();
            }
        }

        KeyCode::Left => {
            let mut s = state.lock().await;
            if !s.push_dlg.branch_open && s.push_dlg.cursor > 0 { s.push_dlg.cursor -= 1; }
        }
        KeyCode::Right => {
            let mut s = state.lock().await;
            if !s.push_dlg.branch_open {
                let len = s.push_dlg.active_text().chars().count();
                if s.push_dlg.cursor < len { s.push_dlg.cursor += 1; }
            }
        }
        KeyCode::Backspace => {
            let mut s = state.lock().await;
            if s.push_dlg.branch_open {
                s.push_dlg.branch_open = false;
            } else {
                s.push_dlg.backspace();
                if matches!(s.push_dlg.focused, PushField::RepoName) {
                    s.push_dlg.update_origin();
                }
            }
        }

        // Ctrl+F = Toggle Force Push
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.lock().await;
            s.push_dlg.force_push = !s.push_dlg.force_push;
        }

        // Space on Branch field = toggle dropdown
        KeyCode::Char(' ') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.lock().await;
            if s.push_dlg.focused == PushField::Branch {
                s.push_dlg.branch_open = !s.push_dlg.branch_open;
            } else if !s.push_dlg.branch_open {
                s.push_dlg.type_char(' ');
            }
        }

        // Typing into text fields (not branch)
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let mut s = state.lock().await;
            if s.push_dlg.branch_open {
                // ignore typing while dropdown open
            } else if s.push_dlg.focused == PushField::Branch {
                // Open dropdown instead of typing
                s.push_dlg.branch_open = true;
            } else {
                s.push_dlg.type_char(c);
                if matches!(s.push_dlg.focused, PushField::RepoName) {
                    s.push_dlg.update_origin();
                }
            }
        }

        // Enter — select from dropdown OR push
        KeyCode::Enter => {
            {
                let mut s = state.lock().await;
                if s.push_dlg.branch_open {
                    s.push_dlg.select_branch();
                    return Ok(false);
                }
            }
            let (token, root, repo_name, branch, username, force_push) = {
                let s = state.lock().await;
                let repo_name = s.push_dlg.repo_name.trim().to_string();
                let branch    = s.push_dlg.branch.trim().to_string();
                let token     = s.config.github_token.clone();
                let root      = s.repo.as_ref().map(|r| r.root.clone());
                let username  = s.config.username.clone().unwrap_or_default();
                let force_push = s.push_dlg.force_push;
                (token, root, repo_name, branch, username, force_push)
            };

            if repo_name.is_empty() {
                state.lock().await.push_dlg.status_msg =
                    "❌ Repo name is required".to_string();
                return Ok(false);
            }

            let tok = match token {
                Some(t) => t,
                None => {
                    state.lock().await.push_dlg.status_msg =
                        "❌ No GitHub token — set one via Ctrl+W".to_string();
                    return Ok(false);
                }
            };

            let root = match root {
                Some(r) => r,
                None => {
                    state.lock().await.push_dlg.status_msg = "❌ No repo open".to_string();
                    return Ok(false);
                }
            };

            {
                let mut s = state.lock().await;
                s.push_dlg.status_msg = format!("{} Pushing…", spinner_char(s.push_dlg.frame_count));
                s.loading_label = "Pushing to remote".to_string();
                s.is_loading = true;
            }

            let state_c    = Arc::clone(&state);
            let tok_c      = tok.clone();
            let root_c     = root.clone();
            let branch_c   = branch.clone();
            let username_c = username.clone();
            let repo_c     = repo_name.clone();

            tokio::task::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    // If SSH key ready, prioritize SSH push
                    if crate::auth::test_ssh_github().is_ok() {
                        let _ = crate::auth::set_remote_ssh(&root_c, &username_c, &repo_c);
                        crate::auth::push_via_ssh(&root_c, &branch_c, force_push)
                    } else {
                        // Fallback to PAT via HTTPS
                        crate::git::set_remote_and_push(&root_c, &tok_c, &username_c, &repo_c, &branch_c, force_push)
                    }
                }).await;

                let mut s = state_c.lock().await;
                s.is_loading = false;
                match result {
                    Ok(Ok(out)) => {
                        let msg = if out.contains("Everything up-to-date") || out.contains("up to date") {
                            format!("✅ Already up-to-date")
                        } else {
                            format!("✅ Pushed to {}/{}!", username, repo_name)
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
                            s.force_push_confirm_cursor = 0; // default to Cancel
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

        _ => {}
    }
    Ok(false)
}

// ── Task 7: Inline editor handler ────────────────────────────────────────────

fn handle_editor(key: KeyEvent, s: &mut AppState) -> bool {
    use crossterm::event::KeyModifiers;

    // Ctrl+S = save
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => { s.editor_save(); return false; }
            KeyCode::Char('x') => {
                // Close editor, go back to RepoView
                if s.editor_modified {
                    s.show_toast("Unsaved changes — Ctrl+S to save first, or Ctrl+X again to discard",
                        ToastType::Warning);
                    s.editor_modified = false; // second Ctrl+X will close
                } else {
                    s.mode = AppMode::RepoView;
                    s.active_frame = 1;
                }
                return false;
            }
            _ => {}
        }
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            if s.editor_modified {
                s.show_toast("Unsaved — Ctrl+S to save, Ctrl+X to discard", ToastType::Warning);
            } else {
                s.mode = AppMode::RepoView;
                s.active_frame = 1;
            }
        }

        // Cursor movement
        KeyCode::Up => {
            if s.editor_cursor_line > 0 {
                s.editor_cursor_line -= 1;
                let line_len = s.editor_lines.get(s.editor_cursor_line)
                    .map(|l| l.chars().count()).unwrap_or(0);
                if s.editor_cursor_col > line_len { s.editor_cursor_col = line_len; }
                s.editor_adjust_scroll();
            }
        }
        KeyCode::Down => {
            if s.editor_cursor_line + 1 < s.editor_lines.len() {
                s.editor_cursor_line += 1;
                let line_len = s.editor_lines.get(s.editor_cursor_line)
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
                s.editor_cursor_col = s.editor_lines.get(s.editor_cursor_line)
                    .map(|l| l.chars().count()).unwrap_or(0);
                s.editor_adjust_scroll();
            }
        }
        KeyCode::Right => {
            let line_len = s.editor_lines.get(s.editor_cursor_line)
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
            s.editor_cursor_col = s.editor_lines.get(s.editor_cursor_line)
                .map(|l| l.chars().count()).unwrap_or(0);
        }

        // Typing
        KeyCode::Char(c) => {
            let col = s.editor_cursor_col;
            if let Some(line) = s.editor_lines.get_mut(s.editor_cursor_line) {
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
            if s.editor_cursor_col > 0 {
                let line = &mut s.editor_lines[s.editor_cursor_line];
                let byte_pos = line.char_indices()
                    .nth(s.editor_cursor_col - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                line.remove(byte_pos);
                s.editor_cursor_col -= 1;
                s.editor_modified = true;
            } else if s.editor_cursor_line > 0 {
                // Merge with previous line
                let current_line = s.editor_lines.remove(s.editor_cursor_line);
                s.editor_cursor_line -= 1;
                let prev_len = s.editor_lines[s.editor_cursor_line].chars().count();
                s.editor_lines[s.editor_cursor_line].push_str(&current_line);
                s.editor_cursor_col = prev_len;
                s.editor_modified = true;
                s.editor_adjust_scroll();
            }
        }

        KeyCode::Delete => {
            let line_len = s.editor_lines[s.editor_cursor_line].chars().count();
            if s.editor_cursor_col < line_len {
                let line = &mut s.editor_lines[s.editor_cursor_line];
                let byte_pos = line.char_indices()
                    .nth(s.editor_cursor_col)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                line.remove(byte_pos);
                s.editor_modified = true;
            } else if s.editor_cursor_line + 1 < s.editor_lines.len() {
                // Join with next line
                let next = s.editor_lines.remove(s.editor_cursor_line + 1);
                s.editor_lines[s.editor_cursor_line].push_str(&next);
                s.editor_modified = true;
            }
        }

        KeyCode::Enter => {
            // Split line at cursor
            let rest = {
                let line = &mut s.editor_lines[s.editor_cursor_line];
                let byte_pos = line.char_indices()
                    .nth(s.editor_cursor_col)
                    .map(|(i, _)| i)
                    .unwrap_or(line.len());
                let tail = line[byte_pos..].to_string();
                line.truncate(byte_pos);
                tail
            };
            s.editor_cursor_line += 1;
            s.editor_lines.insert(s.editor_cursor_line, rest);
            s.editor_cursor_col = 0;
            s.editor_modified = true;
            s.editor_adjust_scroll();
        }

        KeyCode::Tab => {
            // Insert 4 spaces
            let line = &mut s.editor_lines[s.editor_cursor_line];
            let byte_pos = line.char_indices()
                .nth(s.editor_cursor_col)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line.insert_str(byte_pos, "    ");
            s.editor_cursor_col += 4;
            s.editor_modified = true;
        }

        // Task 6: 1/2 switch frames while in editor
        KeyCode::F(1) => { s.mode = AppMode::RepoView; s.active_frame = 1; }
        KeyCode::F(2) => {} // already in frame 2 (editor)

        _ => {}
    }
    false
}
// ── Force Push confirmation dialog ───────────────────────────────────────────

fn render_force_push_confirm(f: &mut ratatui::Frame, area: ratatui::layout::Rect, cursor: usize) {
    use ratatui::{
        layout::{Constraint, Direction, Layout, Rect},
        style::{Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let mw = 60u16.min(area.width.saturating_sub(4));
    let mh = 12u16.min(area.height.saturating_sub(4));
    let mx = area.x + (area.width.saturating_sub(mw)) / 2;
    let my = area.y + (area.height.saturating_sub(mh)) / 2;
    let modal = Rect::new(mx, my, mw, mh);

    f.render_widget(ratatui::widgets::Clear, modal);

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" 🔥 Force Push Confirmation ")
            .border_style(Style::default().fg(theme::ERROR_COLOR))
            .style(Style::default().bg(theme::BG_COLOR)),
        modal,
    );

    let inner = Rect::new(modal.x + 2, modal.y + 1, modal.width - 4, modal.height - 2);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new("The normal push was rejected by GitHub.")
            .style(Style::default().fg(theme::WARNING_COLOR).add_modifier(Modifier::BOLD)),
        rows[1],
    );
    f.render_widget(
        Paragraph::new("Your local history diverges from the remote.")
            .style(Style::default().fg(theme::FG_COLOR)),
        rows[2],
    );
    f.render_widget(
        Paragraph::new("Force pushing will OVERWRITE remote changes.")
            .style(Style::default().fg(theme::ERROR_COLOR)),
        rows[3],
    );

    let btn_row = rows[4];
    let btns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(btn_row);

    let cancel_style = if cursor == 0 {
        Style::default().fg(theme::BG_COLOR).bg(theme::SUCCESS_COLOR).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BORDER_COLOR)
    };
    let force_style = if cursor == 1 {
        Style::default().fg(theme::BG_COLOR).bg(theme::ERROR_COLOR).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new("  ← Cancel (safe)")
            .style(cancel_style),
        btns[0],
    );
    f.render_widget(
        Paragraph::new("  Force Push →")
            .style(force_style),
        btns[1],
    );
}

async fn handle_force_push_confirm(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    match key.code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
            let mut s = state.lock().await;
            s.force_push_confirm_cursor = 1 - s.force_push_confirm_cursor;
        }
        KeyCode::Esc => {
            let mut s = state.lock().await;
            s.mode = AppMode::PushDialog;
        }
        KeyCode::Enter => {
            let cursor = state.lock().await.force_push_confirm_cursor;
            if cursor == 0 {
                let mut s = state.lock().await;
                s.mode = AppMode::PushDialog;
            } else {
                // User confirmed force push!
                {
                    let mut s = state.lock().await;
                    s.push_dlg.force_push = true;
                    s.mode = AppMode::PushDialog;
                }
                // Re-trigger push (this will now use force_push=true)
                return handle_push_dialog(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()), state).await;
            }
        }
        _ => {}
    }
    Ok(false)
}

// ── Task 1: Deinit confirmation dialog ───────────────────────────────────────

fn render_deinit_confirm(f: &mut ratatui::Frame, area: ratatui::layout::Rect, cursor: usize) {
    use ratatui::{
        layout::{Constraint, Direction, Layout, Rect},
        style::{Modifier, Style},
        widgets::{Block, Borders, Paragraph},
    };

    let mw = 60u16.min(area.width.saturating_sub(4));
    let mh = 12u16.min(area.height.saturating_sub(4));
    let mx = area.x + (area.width.saturating_sub(mw)) / 2;
    let my = area.y + (area.height.saturating_sub(mh)) / 2;
    let modal = Rect::new(mx, my, mw, mh);

    f.render_widget(ratatui::widgets::Clear, modal);

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" ⚠  Deinitialize Repository ")
            .border_style(Style::default().fg(theme::ERROR_COLOR))
            .style(Style::default().bg(theme::BG_COLOR)),
        modal,
    );

    let inner = Rect::new(modal.x + 2, modal.y + 1, modal.width - 4, modal.height - 2);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new("This will permanently delete the .git folder.")
            .style(Style::default().fg(theme::WARNING_COLOR).add_modifier(Modifier::BOLD)),
        rows[1],
    );
    f.render_widget(
        Paragraph::new("All history, branches, and staged changes will be lost.")
            .style(Style::default().fg(theme::FG_COLOR)),
        rows[2],
    );
    f.render_widget(
        Paragraph::new("The folder itself and your files will NOT be deleted.")
            .style(Style::default().fg(theme::BORDER_COLOR)),
        rows[3],
    );

    // Two buttons: Cancel (default, safe) and Deinit (destructive)
    let btn_row = rows[4];
    let btns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(btn_row);

    let cancel_style = if cursor == 0 {
        Style::default().fg(theme::BG_COLOR).bg(theme::SUCCESS_COLOR).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BORDER_COLOR)
    };
    let deinit_style = if cursor == 1 {
        Style::default().fg(theme::BG_COLOR).bg(theme::ERROR_COLOR).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::BORDER_COLOR)
    };

    f.render_widget(
        Paragraph::new("  ← Cancel (safe)")
            .style(cancel_style),
        btns[0],
    );
    f.render_widget(
        Paragraph::new("  Deinitialize →")
            .style(deinit_style),
        btns[1],
    );
}

fn handle_deinit_confirm(key: KeyEvent, s: &mut AppState) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            s.mode = AppMode::RepoView;
            s.show_toast("Cancelled", components::toast::ToastType::Info);
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
                                components::toast::ToastType::Success,
                            );
                        }
                        Err(e) => {
                            s.mode = AppMode::RepoView;
                            s.show_toast(
                                format!("❌ Deinit failed: {}", e),
                                components::toast::ToastType::Error,
                            );
                        }
                    }
                }
            } else {
                s.mode = AppMode::RepoView;
                s.show_toast("Cancelled", components::toast::ToastType::Info);
            }
        }
        _ => {}
    }
    false
}

// ── SSH Setup handler ─────────────────────────────────────────────────────────

async fn handle_ssh_setup(key: KeyEvent, state: Arc<Mutex<AppState>>) -> Result<bool> {
    use components::ssh_setup::SshSetupStep;

    let current_step = {
        let s = state.lock().await;
        s.ssh_step.clone()
    };

    match current_step {
        SshSetupStep::Detecting => {
            // Spawn detection task
            let state_c = state.clone();
            tokio::spawn(async move {
                let ssh_state = tokio::task::spawn_blocking(|| {
                    crate::auth::detect_ssh_status()
                }).await.unwrap_or(crate::auth::SshStatus::NoKey);

                let mut s = state_c.lock().await;
                match ssh_state {
                    crate::auth::SshStatus::Ready { pubkey, .. } => {
                        s.ssh_pubkey = pubkey.clone();
                        s.ssh_step = SshSetupStep::Connected {
                            username: "GitHub".to_string(),
                        };
                        s.config.ssh_key_added = true;
                        let _ = s.config.save();
                    }
                    crate::auth::SshStatus::PubkeyPending { pubkey, .. } => {
                        s.ssh_pubkey = pubkey.clone();
                        // Try auto-add via PAT if we have one
                        let token = s.config.github_token.clone();
                        let pk    = pubkey.clone();
                        if let Some(tok) = token {
                            drop(s);
                            let auto_result = tokio::task::spawn_blocking(move || {
                                crate::github::auto_register_ssh_key(&tok, &pk)
                            }).await;
                            let mut s2 = state_c.lock().await;
                            let auto_added = auto_result.map(|r| r.is_ok()).unwrap_or(false);
                            s2.ssh_step = SshSetupStep::ShowPubkey {
                                pubkey: s2.ssh_pubkey.clone(),
                                auto_added,
                            };
                        } else {
                            s.ssh_step = SshSetupStep::ShowPubkey { pubkey, auto_added: false };
                        }
                    }
                    crate::auth::SshStatus::NoKey => {
                        s.ssh_step = SshSetupStep::NeedGenerate;
                    }
                }
            });
        }

        SshSetupStep::NeedGenerate => {
            if key.code == KeyCode::Enter {
                // Generate key
                let mut s = state.lock().await;
                s.ssh_step = SshSetupStep::Generating;
                drop(s);

                let email = {
                    let s = state.lock().await;
                    s.config.username.clone().unwrap_or_else(|| "swiftgit@local".to_string())
                };

                let state_c = state.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        crate::auth::generate_ssh_key(&email)
                    }).await;

                    let mut s = state_c.lock().await;
                    match result {
                        Ok(Ok((_path, pubkey))) => {
                            let pk = pubkey.clone();
                            s.ssh_pubkey = pubkey;
                            // Try auto-add the key via PAT
                            let token   = s.config.github_token.clone();
                            if let Some(tok) = token {
                                drop(s);
                                let auto = tokio::task::spawn_blocking(move || {
                                    crate::github::auto_register_ssh_key(&tok, &pk)
                                }).await;
                                let auto_added = auto.map(|r| r.is_ok()).unwrap_or(false);
                                let mut s2 = state_c.lock().await;
                                s2.ssh_step = SshSetupStep::ShowPubkey {
                                    pubkey: s2.ssh_pubkey.clone(),
                                    auto_added,
                                };
                            } else {
                                s.ssh_step = SshSetupStep::ShowPubkey {
                                    pubkey: s.ssh_pubkey.clone(),
                                    auto_added: false,
                                };
                            }
                        }
                        Ok(Err(e)) => s.ssh_step = SshSetupStep::Error(e.to_string()),
                        Err(e)    => s.ssh_step = SshSetupStep::Error(e.to_string()),
                    }
                });

            } else if key.code == KeyCode::Esc {
                // Skip SSH — go to PAT auth or dashboard
                let mut s = state.lock().await;
                s.mode = if s.config.github_token.is_none() {
                    AppMode::Auth
                } else {
                    AppMode::Dashboard
                };
            }
        }

        SshSetupStep::ShowPubkey { .. } => {
            if key.code == KeyCode::Enter {
                // Test the connection
                let mut s = state.lock().await;
                s.ssh_step = SshSetupStep::Testing;
                drop(s);

                let state_c = state.clone();
                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(|| {
                        crate::auth::test_ssh_github()
                    }).await;

                    let mut s = state_c.lock().await;
                    match result {
                        Ok(Ok(username)) => {
                            s.ssh_step = SshSetupStep::Connected { username };
                            s.config.ssh_key_added = true;
                            let _ = s.config.save();
                        }
                        Ok(Err(e)) => s.ssh_step = SshSetupStep::Error(e.to_string()),
                        Err(e)    => s.ssh_step = SshSetupStep::Error(e.to_string()),
                    }
                });

            } else if key.code == KeyCode::Esc {
                let mut s = state.lock().await;
                s.mode = if s.config.github_token.is_none() {
                    AppMode::Auth
                } else {
                    AppMode::Dashboard
                };
            }
        }

        SshSetupStep::Connected { .. } => {
            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                let mut s = state.lock().await;
                s.mode = if s.config.github_token.is_none() {
                    AppMode::Auth
                } else {
                    AppMode::Dashboard
                };
            }
        }

        SshSetupStep::Error(_) => {
            if key.code == KeyCode::Enter {
                let mut s = state.lock().await;
                s.ssh_step = SshSetupStep::Detecting;
            } else if key.code == KeyCode::Esc {
                let mut s = state.lock().await;
                s.mode = AppMode::Dashboard;
            }
        }

        SshSetupStep::Generating | SshSetupStep::Testing => {
            // Busy — ignore keypresses
        }
    }

    Ok(false)
}
