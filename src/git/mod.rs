use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Untracked,
    Staged,
    Added,
    Deleted,
    Renamed,
    Clean,    // committed and unchanged — shown with dim indicator
    Unknown,
}

impl FileStatus {
    pub fn indicator(&self) -> &'static str {
        match self {
            FileStatus::Staged | FileStatus::Added => "[✓]",
            FileStatus::Modified  => "[M]",
            FileStatus::Untracked => "[U]",
            FileStatus::Deleted   => "[D]",
            FileStatus::Renamed   => "[R]",
            FileStatus::Clean     => "[ ]", // clean committed file — visible but dim
            FileStatus::Unknown   => "[ ]", // fallback
        }
    }
    pub fn is_staged(&self) -> bool {
        matches!(self, FileStatus::Staged | FileStatus::Added)
    }
}

#[derive(Debug, Clone)]
pub struct GitFile {
    pub path:   String,
    pub status: FileStatus,
}

/// A file from GitHub API tree
#[derive(Debug, Clone, Deserialize)]
pub struct GhFile {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub size: Option<u64>,
    pub sha:  String,
}

pub struct GitRepo {
    pub root: PathBuf,
}

impl GitRepo {
    pub fn open(path: &Path) -> Result<Self> {
        let mut current = path.to_path_buf();
        loop {
            if current.join(".git").exists() {
                return Ok(GitRepo { root: current });
            }
            if !current.pop() {
                anyhow::bail!("Not a git repository: {}", path.display());
            }
        }
    }

    pub fn init(path: &Path) -> Result<Self> {
        let out = Command::new("git").args(["init", "-b", "main"])
            .current_dir(path).output().context("Failed to run git init")?;
        if !out.status.success() {
            let out2 = Command::new("git").args(["init"])
                .current_dir(path).output().context("Failed to run git init")?;
            if !out2.status.success() {
                let err = String::from_utf8_lossy(&out2.stderr);
                anyhow::bail!("git init failed: {}", err);
            }
            let _ = Command::new("git")
                .args(["symbolic-ref", "HEAD", "refs/heads/main"])
                .current_dir(path).output();
        }
        Ok(GitRepo { root: path.to_path_buf() })
    }

    pub fn open_or_init(path: &Path) -> Result<(Self, bool)> {
        if path.join(".git").exists() {
            Ok((Self::open(path)?, false))
        } else {
            Ok((Self::init(path)?, true))
        }
    }

    fn git(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("git").args(args).current_dir(&self.root)
            .output().with_context(|| format!("Failed to run: git {}", args.join(" ")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            anyhow::bail!("{}", err.trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    // ── Status ────────────────────────────────────────────────────────────────

    pub fn status(&self) -> Result<Vec<GitFile>> {
        let raw = self.git(&["status", "--porcelain"])?;
        let mut files = Vec::new();
        for line in raw.lines() {
            if line.len() < 3 { continue; }
            let xy   = &line[..2];
            let path = line[3..].trim().trim_matches('"').to_string();
            files.push(GitFile { path, status: parse_porcelain_status(xy) });
        }
        Ok(files)
    }

    /// Task 2: list ALL tracked files in the repo (clean + changed)
    pub fn all_files(&self) -> Result<Vec<GitFile>> {
        // First get tracked committed files
        let tracked_raw = Command::new("git")
            .args(["ls-files", "--cached", "--others", "--exclude-standard"])
            .current_dir(&self.root)
            .output()
            .context("Failed to run git ls-files")?;

        let tracked: Vec<String> = if tracked_raw.status.success() {
            String::from_utf8_lossy(&tracked_raw.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect()
        } else {
            vec![]
        };

        // Get changed-file statuses for overlay
        let changed = self.status().unwrap_or_default();

        let mut files: Vec<GitFile> = tracked.into_iter().map(|path| {
            // Check if this file has a changed status
            let status = changed.iter()
                .find(|f| f.path == path)
                .map(|f| f.status.clone())
                .unwrap_or(FileStatus::Clean); // Clean = committed and unchanged
            GitFile { path, status }
        }).collect();

        // Add any untracked files that aren't already in the list
        for f in &changed {
            if matches!(f.status, FileStatus::Untracked) {
                if !files.iter().any(|e| e.path == f.path) {
                    files.push(f.clone());
                }
            }
        }

        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    // ── Staging ───────────────────────────────────────────────────────────────

    pub fn stage(&self, file: &str) -> Result<()> {
        self.git(&["add", "--", file])?;
        Ok(())
    }

    pub fn stage_folder(&self, folder: &str) -> Result<()> {
        let arg = if folder.ends_with('/') { folder.to_string() } else { format!("{}/", folder) };
        self.git(&["add", "--", &arg])?;
        Ok(())
    }

    pub fn unstage(&self, file: &str) -> Result<()> {
        // Try restore --staged first, fallback to rm --cached, then reset HEAD
        if self.git(&["restore", "--staged", "--", file]).is_ok() { return Ok(()); }
        if self.git(&["rm", "--cached", "--", file]).is_ok() { return Ok(()); }
        self.git(&["reset", "HEAD", "--", file]).map(|_| ())
    }

    pub fn unstage_folder(&self, folder: &str) -> Result<()> {
        if self.git(&["restore", "--staged", "--", folder]).is_ok() { return Ok(()); }
        if self.git(&["reset", "HEAD", "--", folder]).is_ok() { return Ok(()); }
        self.git(&["rm", "--cached", "-r", "--", folder]).map(|_| ())
    }

    // ── Commit ────────────────────────────────────────────────────────────────

    pub fn commit(&self, message: &str) -> Result<String> {
        let out = self.git(&["commit", "-m", message])?;
        Ok(out.trim().to_string())
    }

    // ── Remote ───────────────────────────────────────────────────────────────

    pub fn get_remote_url(&self) -> Result<String> {
        let out = self.git(&["remote", "get-url", "origin"])?;
        Ok(out.trim().to_string())
    }

    pub fn get_remote_url_safe(&self) -> String {
        self.get_remote_url().unwrap_or_default()
    }

    // ── Push — via temporary git-credential-store file ────────────────────────
    //
    // This is the same technique GitHub Actions uses internally.
    // Write a temp credential file git can read, push, delete it immediately.
    // Never stored in .git/config; never embedded in process command line.

    /// Strip any embedded credentials from an HTTPS URL.
    pub fn clean_url(url: &str) -> String {
        if url.starts_with("https://") {
            let rest = &url[8..];
            let rest = if let Some(at) = rest.find('@') { &rest[at+1..] } else { rest };
            format!("https://{}", rest)
        } else {
            url.to_string()
        }
    }

    /// Write a temporary git credentials file and return its path.
    /// Format: https://x-access-token:TOKEN@github.com
    fn write_cred_file(token: &str) -> Result<PathBuf> {
        let path = std::env::temp_dir()
            .join(format!("sg_creds_{}", std::process::id()));
        // git credential store format: scheme://user:pass@host
        let line = format!("https://x-access-token:{}@github.com\n", token);
        std::fs::write(&path, line.as_bytes())
            .context("Failed to write temp credential file")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .context("Failed to chmod cred file")?;
        }
        Ok(path)
    }

    /// Push to a clean HTTPS URL using a temporary credential file.
    fn push_with_pat(&self, clean_url: &str, token: &str, branch: &str, force: bool) -> Result<String> {
        let cred = Self::write_cred_file(token)?;
        let cred_helper_str = format!("credential.helper=store --file=\"{}\"", cred.display());

        let mut args = vec![
            "-c", "credential.helper=",           // disable system helper first
            "-c", &cred_helper_str,
            "push", "--set-upstream",
        ];
        if force { args.push("--force"); }
        args.push(clean_url);
        args.push(branch);

        let result = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .context("git push failed");

        let _ = std::fs::remove_file(&cred);  // always clean up

        let out    = result?;
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();

        if out.status.success() {
            Ok(format!("{}{}", stdout, stderr))
        } else {
            Err(self.format_push_error(stderr.trim()))
        }
    }


    pub fn smart_push(&self, token: Option<&str>, branch: &str, force: bool) -> Result<String> {
        if !self.has_commits() {
            anyhow::bail!("No commits yet — stage and commit files first");
        }
        let actual_branch = self.current_branch().unwrap_or_else(|_| branch.to_string());
        let branch = actual_branch.as_str();

        // Before pushing, check if remote branch exists and is ahead of us.
        // This catches the "fetch first" rejection before it happens.
        if !force {
            if let Ok(remote_url) = self.get_remote_url() {
                if !remote_url.is_empty() {
                    // Check if remote branch is ahead (would cause non-fast-forward)
                    let fetch_check = Command::new("git")
                        .args(["fetch", "--dry-run", "origin"])
                        .current_dir(&self.root)
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .output();
                    if let Ok(fc) = fetch_check {
                        // If fetch finds new remote commits, our push will be rejected
                        let fc_out = String::from_utf8_lossy(&fc.stderr).to_string();
                        if fc_out.contains(branch) {
                            anyhow::bail!(
                                "Remote has new commits your local branch doesn't have.\n\
                                 Run 'Pull' first to sync, then push.\n\
                                 (Or use force push to overwrite — data may be lost.)"
                            );
                        }
                    }
                }
            }
        }

        if let Some(tok) = token {
            let remote_url = self.get_remote_url().unwrap_or_default();
            if !remote_url.is_empty() && remote_url.starts_with("https://") {
                return self.push_with_pat(&Self::clean_url(&remote_url), tok, branch, force);
            }
        }

        // SSH remote or no token — plain push
        let mut args = vec!["push", "-u", "origin", branch];
        if force { args.push("--force"); }

        let out = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output().context("git push failed")?;
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if out.status.success() { Ok(format!("{}{}", stdout, stderr)) }
        else { Err(self.format_push_error(stderr.trim())) }
    }

    pub fn push_with_token(&self, token: &str, force: bool) -> Result<String> {
        let branch = self.current_branch().unwrap_or_else(|_| "main".to_string());
        self.smart_push(Some(token), &branch, force)
    }

    pub fn push(&self, force: bool) -> Result<String> {
        let branch = self.current_branch().unwrap_or_else(|_| "main".to_string());
        self.smart_push(None, &branch, force)
    }

    fn format_push_error(&self, err: &str) -> anyhow::Error {
        if err.contains("denied") || err.contains("403") || err.contains("Authentication failed") {
            anyhow::anyhow!(
                "Authentication failed.\n\
                 Make sure your PAT has 'repo' write scope.\n\
                 Update it: Ctrl+W → Token field."
            )
        } else if err.contains("not found") || err.contains("404") || err.contains("does not exist") {
            anyhow::anyhow!(
                "Repository not found on GitHub.\n\
                 Create it first at github.com/new, then push."
            )
        } else if err.contains("does not match any") || err.contains("src refspec") {
            anyhow::anyhow!(
                "No commits on this branch yet.\n\
                 Stage files, commit them, then push."
            )
        } else if err.contains("non-fast-forward") || err.contains("fetch first")
               || err.contains("rejected") || err.contains("behind") {
            anyhow::anyhow!(
                "Push rejected — remote has commits your local branch doesn't have.\n\
                 Pull first to sync changes, then push.\n\
                 (Use force push only if you want to overwrite the remote.)"
            )
        } else if err.contains("Everything up-to-date") {
            anyhow::anyhow!("Already up-to-date")
        } else {
            anyhow::anyhow!("{}", err)
        }
    }

    // ── Pull ─────────────────────────────────────────────────────────────────

    pub fn smart_pull(&self, token: Option<&str>) -> Result<String> {
        if let Some(tok) = token {
            let remote_url = self.get_remote_url().unwrap_or_default();
            if !remote_url.is_empty() && remote_url.starts_with("https://") {
                let cred = Self::write_cred_file(tok).unwrap_or_default();
                let cred_helper = format!("store --file={}", cred.display());
                let out  = Command::new("git")
                    .args([
                        "-c", "credential.helper=",
                        "-c", &format!("credential.helper={}", cred_helper),
                        "pull",
                    ])
                    .current_dir(&self.root)
                    .env("GIT_TERMINAL_PROMPT", "0")
                    .output()
                    .context("Failed to run git pull");
                let _ = std::fs::remove_file(&cred);
                let out = out?;
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                }
                // fall through to plain pull
            }
        }

        let out = Command::new("git")
            .args(["pull"])
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .context("Failed to run git pull")?;

        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if out.status.success() {
            Ok(format!("{}{}", stdout, stderr).trim().to_string())
        } else {
            anyhow::bail!("{}", stderr.trim())
        }
    }

    pub fn pull(&self) -> Result<String> {
        self.smart_pull(None)
    }

    // ── Diff ─────────────────────────────────────────────────────────────────

    pub fn diff_file(&self, file: &str) -> Result<String> {
        let staged   = self.git(&["diff", "--cached", "--", file]).unwrap_or_default();
        let unstaged = self.git(&["diff", "--", file]).unwrap_or_default();
        let combined = format!("{}{}", staged, unstaged);

        if combined.is_empty() {
            // Check if it's an untracked file
            let status = self.status().unwrap_or_default();
            if status.iter().any(|f| f.path == file && f.status == FileStatus::Untracked) {
                // Show the file content as if it were all added
                match std::fs::read_to_string(self.root.join(file)) {
                    Ok(content) => {
                        let mut diff = format!("--- /dev/null\n+++ b/{}\n@@ -0,0 +1,{} @@\n", file, content.lines().count());
                        for line in content.lines() {
                            diff.push_str("+");
                            diff.push_str(line);
                            diff.push_str("\n");
                        }
                        return Ok(diff);
                    }
                    Err(_) => return Ok("(untracked file — cannot read content)".to_string()),
                }
            }
            Ok("(no changes — file is committed and clean)".to_string())
        } else {
            Ok(combined)
        }
    }

    // ── Branch / meta ─────────────────────────────────────────────────────────

    pub fn current_branch(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    pub fn list_branches(&self) -> Vec<String> {
        let out = Command::new("git")
            .args(["for-each-ref", "--format=%(refname:short)", "refs/heads/"])
            .current_dir(&self.root).output();
        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect(),
            _ => vec![],
        }
    }

    pub fn has_commits(&self) -> bool {
        Command::new("git").args(["rev-parse", "HEAD"])
            .current_dir(&self.root).output()
            .map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn deinit(&self) -> Result<()> {
        let git_dir = self.root.join(".git");
        std::fs::remove_dir_all(&git_dir)
            .with_context(|| format!("Failed to remove {}", git_dir.display()))?;
        Ok(())
    }

    pub fn repo_name(&self) -> String {
        self.root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "repo".to_string())
    }
}

// ── Status parser ─────────────────────────────────────────────────────────────

fn parse_porcelain_status(xy: &str) -> FileStatus {
    let x = xy.chars().next().unwrap_or(' ');
    let y = xy.chars().nth(1).unwrap_or(' ');
    match (x, y) {
        ('?', '?')            => FileStatus::Untracked,
        ('A', _)              => FileStatus::Added,
        ('M', ' ')            => FileStatus::Staged,
        (' ', 'M')            => FileStatus::Modified,
        ('M', 'M')            => FileStatus::Modified,
        ('D', _) | (' ', 'D') => FileStatus::Deleted,
        ('R', _)              => FileStatus::Renamed,
        _ => if x != ' ' && x != '?' { FileStatus::Staged } else { FileStatus::Modified },
    }
}

// ── GitHub API ────────────────────────────────────────────────────────────────

pub fn inject_token_into_url(url: &str, _token: &str) -> String {
    GitRepo::clean_url(url)
}

pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let prefix = "https://github.com/";
    if !url.starts_with(prefix) { return None; }
    let rest = &url[prefix.len()..];
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

pub fn fetch_github_files(owner: &str, repo: &str, token: Option<&str>) -> Result<Vec<GhFile>> {
    let url = format!("https://api.github.com/repos/{}/{}/git/trees/HEAD?recursive=1", owner, repo);
    let client = reqwest::blocking::Client::new();
    let mut req = client.get(&url)
        .header("User-Agent", "swiftgit/1.3")
        .header("Accept", "application/vnd.github.v3+json");
    if let Some(t) = token { req = req.header("Authorization", format!("token {}", t)); }
    let resp = req.send().context("GitHub API request failed")?;
    if !resp.status().is_success() { anyhow::bail!("GitHub API returned {}", resp.status()); }

    #[derive(Deserialize)]
    struct TreeResponse { tree: Vec<GhFile> }
    let body: TreeResponse = resp.json().context("Failed to parse GitHub tree response")?;
    Ok(body.tree.into_iter().filter(|f| f.kind == "blob").collect())
}

pub fn download_github_file(owner: &str, repo: &str, path: &str, dest_dir: &Path, token: Option<&str>) -> Result<()> {
    let raw_url = format!("https://raw.githubusercontent.com/{}/{}/main/{}", owner, repo, path);
    let client  = reqwest::blocking::Client::new();
    let mut req = client.get(&raw_url).header("User-Agent", "swiftgit/1.3");
    if let Some(t) = token { req = req.header("Authorization", format!("token {}", t)); }
    let resp = req.send().context("Failed to request file")?;
    if !resp.status().is_success() { anyhow::bail!("Download failed ({}): {}", resp.status(), path); }
    let bytes = resp.bytes().context("Failed to read response")?;
    let dest  = dest_dir.join(path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir -p {}", parent.display()))?;
    }
    std::fs::write(&dest, &bytes).with_context(|| format!("write {}", dest.display()))?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRepo {
    pub full_name: String,
    pub name:      String,
    pub private:   bool,
    pub ssh_url:   String,
    pub clone_url: String,
}

pub fn fetch_user_repos(token: &str) -> Result<Vec<GhRepo>> {
    let client = reqwest::blocking::Client::new();
    let resp = client.get("https://api.github.com/user/repos?per_page=100&sort=updated")
        .header("User-Agent", "swiftgit/1.3")
        .header("Accept", "application/vnd.github.v3+json")
        .header("Authorization", format!("token {}", token))
        .send().context("GitHub API request failed")?;
    if !resp.status().is_success() { anyhow::bail!("GitHub API returned {}", resp.status()); }
    let repos: Vec<GhRepo> = resp.json().context("Failed to parse repos")?;
    Ok(repos)
}

pub fn set_remote_and_push(root: &Path, token: &str, owner: &str, repo_name: &str, branch: &str, force: bool) -> Result<String> {
    let repo = GitRepo { root: root.to_path_buf() };

    if !repo.has_commits() {
        anyhow::bail!("No commits yet — stage and commit files first");
    }

    let actual_branch = repo.current_branch().unwrap_or_else(|_| branch.to_string());
    let branch = actual_branch.as_str();

    // Build clean HTTPS URL (no token — stored separately for security)
    let https_url = format!("https://github.com/{}/{}.git", owner, repo_name);
    // Auth URL uses x-access-token format which GitHub supports for PATs

    // Set origin to the clean URL (no token in .git/config)
    let has_remote = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root).output()
        .map(|o| o.status.success()).unwrap_or(false);

    if has_remote {
        repo.git(&["remote", "set-url", "origin", &https_url])?;
    } else {
        repo.git(&["remote", "add", "origin", &https_url])?;
    }

    // Push via credential-store file — reliable, no credentials in .git/config
    repo.push_with_pat(&https_url, token, branch, force)
}

pub fn recent_commits(root: &Path, n: usize) -> Vec<String> {
    let out = Command::new("git")
        .args(["log", &format!("-{}", n), "--oneline", "--no-decorate"])
        .current_dir(root).output();
    match out {
        Ok(o) if o.status.success() =>
            String::from_utf8_lossy(&o.stdout).lines().map(|l| l.to_string()).collect(),
        _ => vec![],
    }
}
