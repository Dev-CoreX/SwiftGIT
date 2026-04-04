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
    Clean,
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
            FileStatus::Clean     => "[ ]", 
            FileStatus::Unknown   => "[ ]", 
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

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteFile {
    pub path: String,
    #[serde(rename = "type")]
    pub kind: String, // "file" or "dir"
    pub size: Option<u64>,
    pub sha:  String,
    pub url:  String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteRepoItem {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,    
    pub lines:  Vec<String>, 
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Diff {
    pub file_header: String, 
    pub hunks: Vec<Hunk>,
}

impl Diff {
    pub fn is_empty(&self) -> bool {
        self.hunks.is_empty()
    }

    pub fn to_string(&self) -> String {
        let mut out = self.file_header.clone();
        for h in &self.hunks {
            out.push_str(&h.header);
            out.push('\n');
            for l in &h.lines {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    }
}

pub struct GitRepo {
    pub root: PathBuf,
}

struct CredFile {
    path: PathBuf,
}

impl CredFile {
    fn new(token: &str) -> Result<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let path = std::env::temp_dir()
            .join(format!("sg_creds_{}_{}", std::process::id(), now));
        let line = format!("https://x-access-token:{}@github.com\n", token);
        std::fs::write(&path, line.as_bytes())
            .context("Failed to write temp credential file")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .context("Failed to chmod cred file")?;
        }
        Ok(Self { path })
    }
}

impl Drop for CredFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
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
                anyhow::bail!("git init failed: {}", redact_tokens(err.trim()));
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

    pub fn git(&self, args: &[&str]) -> Result<String> {
        let out = Command::new("git").args(args).current_dir(&self.root)
            .output().with_context(|| format!("Failed to run: git {}", args.join(" ")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            anyhow::bail!("{}", redact_tokens(err.trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

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

    pub fn all_files(&self) -> Result<Vec<GitFile>> {
        let raw = self.git(&["ls-files", "--cached", "--others", "--exclude-standard"])?;
        let changed = self.status().unwrap_or_default();
        
        let mut files = Vec::new();
        for path in raw.lines() {
            let status = changed.iter()
                .find(|f| f.path == path)
                .map(|f| f.status.clone())
                .unwrap_or(FileStatus::Clean); 
            files.push(GitFile { path: path.to_string(), status });
        }
        Ok(files)
    }

    pub fn current_branch(&self) -> Result<String> {
        let out = self.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    pub fn repo_name(&self) -> String {
        self.root.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "repo".to_string())
    }

    pub fn stage(&self, path: &str) -> Result<()> {
        self.git(&["add", path])?;
        Ok(())
    }

    pub fn unstage(&self, path: &str) -> Result<()> {
        let _ = self.git(&["restore", "--staged", path]);
        let _ = self.git(&["rm", "--cached", path]);
        let _ = self.git(&["reset", "HEAD", "--", path]);
        Ok(())
    }

    pub fn stage_folder(&self, path: &str) -> Result<()> {
        self.git(&["add", path])?;
        Ok(())
    }

    pub fn unstage_folder(&self, path: &str) -> Result<()> {
        let _ = self.git(&["restore", "--staged", path]);
        let _ = self.git(&["reset", "HEAD", "--", path]);
        Ok(())
    }

    pub fn commit(&self, msg: &str) -> Result<String> {
        self.git(&["commit", "-m", msg])
    }

    pub fn get_remote_url(&self) -> Result<String> {
        let out = self.git(&["remote", "get-url", "origin"])?;
        Ok(out.trim().to_string())
    }

    pub fn has_commits(&self) -> bool {
        Command::new("git").args(["rev-parse", "HEAD"])
            .current_dir(&self.root).output()
            .map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn clean_url(url: &str) -> String {
        if url.starts_with("https://") {
            let rest = &url[8..];
            let rest = if let Some(at) = rest.find('@') { &rest[at+1..] } else { rest };
            format!("https://{}", rest)
        } else {
            url.to_string()
        }
    }

    fn push_with_pat(&self, clean_url: &str, token: &str, branch: &str, force: bool) -> Result<String> {
        let cred = CredFile::new(token)?;
        let cred_helper_str = format!("credential.helper=store --file=\"{}\"", cred.path.display());

        let mut args = vec![
            "-c", "credential.helper=",           
            "-c", &cred_helper_str,
            "push", "--set-upstream",
        ];
        if force { args.push("--force"); }
        args.push(clean_url);
        args.push(branch);

        let out = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .context("git push failed")?;

        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();

        if out.status.success() {
            Ok(format!("{}{}", stdout, stderr))
        } else {
            Err(self.format_push_error(stderr.trim()))
        }
    }

    fn format_push_error(&self, err: &str) -> anyhow::Error {
        let err = redact_tokens(err);
        if err.contains("rejected") || err.contains("non-fast-forward") {
            anyhow::anyhow!("Rejected: fetch first or force push")
        } else if err.contains("Everything up-to-date") {
            anyhow::anyhow!("Already up-to-date")
        } else {
            anyhow::anyhow!("{}", err)
        }
    }

    pub fn smart_pull(&self, token: Option<&str>) -> Result<String> {
        if let Some(tok) = token {
            let remote_url = self.get_remote_url().unwrap_or_default();
            if !remote_url.is_empty() && remote_url.starts_with("https://") {
                let cred = CredFile::new(tok)?;
                let cred_helper = format!("store --file={}", cred.path.display());
                let out  = Command::new("git")
                    .args([
                        "-c", "credential.helper=",
                        "-c", &format!("credential.helper={}", cred_helper),
                        "pull",
                    ])
                    .current_dir(&self.root)
                    .env("GIT_TERMINAL_PROMPT", "0")
                    .output()
                    .context("Failed to run git pull")?;
                
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    return Ok(format!("{}{}", stdout, stderr).trim().to_string());
                }
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
            anyhow::bail!("{}", redact_tokens(stderr.trim()))
        }
    }

    pub fn pull(&self) -> Result<String> {
        self.git(&["pull"])
    }

    pub fn deinit(&self) -> Result<()> {
        let dot_git = self.root.join(".git");
        if dot_git.exists() {
            std::fs::remove_dir_all(dot_git)
                .context("Failed to remove .git folder")?;
        }
        Ok(())
    }

    pub fn diff_file(&self, file: &str) -> Result<Diff> {
        let raw = self.git(&["diff", "-U3", "--", file]).unwrap_or_default();
        let staged_raw = self.git(&["diff", "--cached", "-U3", "--", file]).unwrap_or_default();
        
        if raw.is_empty() && staged_raw.is_empty() {
            let mut diff = Diff::default();
            if let Ok(status) = self.status() {
                if let Some(f) = status.iter().find(|f| f.path == file) {
                    match f.status {
                        FileStatus::Untracked | FileStatus::Added => {
                            let content = std::fs::read_to_string(self.root.join(file)).unwrap_or_default();
                            let line_count = content.lines().count();
                            let mut hunk = Hunk {
                                header: format!("@@ -0,0 +1,{} @@", line_count),
                                lines: content.lines().map(|l| format!("+{}", l)).collect(),
                                old_start: 0, old_lines: 0,
                                new_start: 1, new_lines: line_count as u32,
                            };
                            if hunk.lines.is_empty() { hunk.lines.push("+".to_string()); }
                            diff.hunks.push(hunk);
                            return Ok(diff);
                        }
                        _ => {}
                    }
                }
            }
            return Ok(diff);
        }

        let combined = format!("{}{}", staged_raw, raw);
        self.parse_diff(&combined)
    }

    fn parse_diff(&self, input: &str) -> Result<Diff> {
        let mut diff = Diff::default();
        let mut current_hunk: Option<Hunk> = None;
        let mut header_done = false;

        for line in input.lines() {
            if line.starts_with("@@") {
                header_done = true;
                if let Some(h) = current_hunk.take() {
                    diff.hunks.push(h);
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let old = parts[1].trim_start_matches('-').split(',').collect::<Vec<&str>>();
                    let new = parts[2].trim_start_matches('+').split(',').collect::<Vec<&str>>();
                    
                    let old_start = old[0].parse().unwrap_or(0);
                    let old_lines = old.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                    let new_start = new[0].parse().unwrap_or(0);
                    let new_lines = new.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

                    current_hunk = Some(Hunk {
                        header: line.to_string(),
                        lines: Vec::new(),
                        old_start,
                        old_lines,
                        new_start,
                        new_lines,
                    });
                }
            } else if !header_done {
                diff.file_header.push_str(line);
                diff.file_header.push('\n');
            } else if let Some(h) = current_hunk.as_mut() {
                h.lines.push(line.to_string());
            }
        }
        if let Some(h) = current_hunk {
            diff.hunks.push(h);
        }
        Ok(diff)
    }

    pub fn stage_hunk(&self, file: &str, hunk: &Hunk) -> Result<()> {
        let mut patch = self.diff_file(file)?.file_header;
        patch.push_str(&hunk.header);
        patch.push('\n');
        for l in &hunk.lines {
            patch.push_str(l);
            patch.push('\n');
        }

        let mut child = Command::new("git")
            .args(["apply", "--cached", "--unidiff-zero", "-"])
            .current_dir(&self.root)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn git apply")?;

        use std::io::Write;
        let mut stdin = child.stdin.take().context("Failed to open stdin")?;
        stdin.write_all(patch.as_bytes())?;
        drop(stdin);

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("git apply failed");
        }
        Ok(())
    }

    pub fn unstage_hunk(&self, file: &str, hunk: &Hunk) -> Result<()> {
        let mut patch = self.diff_file(file)?.file_header;
        patch.push_str(&hunk.header);
        patch.push('\n');
        for l in &hunk.lines {
            patch.push_str(l);
            patch.push('\n');
        }

        let mut child = Command::new("git")
            .args(["apply", "--cached", "--reverse", "--unidiff-zero", "-"])
            .current_dir(&self.root)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn git apply")?;

        use std::io::Write;
        let mut stdin = child.stdin.take().context("Failed to open stdin")?;
        stdin.write_all(patch.as_bytes())?;
        drop(stdin);

        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("git apply reverse failed");
        }
        Ok(())
    }

    pub fn smart_push(&self, token: Option<&str>, branch: &str, force: bool) -> Result<String> {
        if !self.has_commits() {
            anyhow::bail!("No commits yet — stage and commit files first");
        }
        let actual_branch = self.current_branch().unwrap_or_else(|_| branch.to_string());
        let branch = actual_branch.as_str();

        if let Some(tok) = token {
            let remote_url = self.get_remote_url().unwrap_or_default();
            if !remote_url.is_empty() && remote_url.starts_with("https://") {
                let clean = Self::clean_url(&remote_url);
                return self.push_with_pat(&clean, tok, branch, force);
            }
        }

        let mut args = vec!["push", "--set-upstream", "origin", branch];
        if force { args.push("--force"); }
        self.git(&args)
    }
}

fn parse_porcelain_status(xy: &str) -> FileStatus {
    let x = xy.chars().next().unwrap_or(' ');
    let y = xy.chars().nth(1).unwrap_or(' ');

    match (x, y) {
        (' ', 'M') => FileStatus::Modified,
        ('M', ' ') | ('M', 'M') | ('A', ' ') => FileStatus::Staged,
        ('?', '?') => FileStatus::Untracked,
        ('D', ' ') | (' ', 'D') => FileStatus::Deleted,
        ('R', ' ') => FileStatus::Renamed,
        ('A', 'M') => FileStatus::Added,
        _ => FileStatus::Unknown,
    }
}

pub fn redact_tokens(input: &str) -> String {
    let mut output = input.to_string();
    if let Some(start) = output.find("ghp_") {
        if let Some(end) = output[start..].find(|c: char| !c.is_alphanumeric() && c != '_') {
            output.replace_range(start..start+end, "[REDACTED TOKEN]");
        } else {
            output.replace_range(start.., "[REDACTED TOKEN]");
        }
    }
    output
}

pub fn fetch_github_files(owner: &str, repo: &str, token: Option<&str>) -> Result<Vec<RemoteFile>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://api.github.com/repos/{}/{}/contents", owner, repo);
    let mut req = client.get(url).header("User-Agent", "swiftgit/1.3");
    if let Some(t) = token { req = req.header("Authorization", format!("token {}", t)); }
    
    let resp = req.send().context("Network error reaching GitHub")?;
    if !resp.status().is_success() {
        anyhow::bail!("GitHub API error: {}", resp.status());
    }
    let files: Vec<RemoteFile> = resp.json().context("Failed to parse file list")?;
    Ok(files)
}

pub fn download_github_item(owner: &str, repo: &str, item: &RemoteFile, dest_dir: &Path, token: Option<&str>) -> Result<()> {
    if item.kind == "file" {
        download_github_file(owner, repo, &item.path, dest_dir, token)
    } else {
        download_github_folder(owner, repo, &item.path, dest_dir, token)
    }
}

fn download_github_file(owner: &str, repo: &str, path: &str, dest_dir: &Path, token: Option<&str>) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
    let mut req = client.get(url).header("User-Agent", "swiftgit/1.3").header("Accept", "application/vnd.github.v3.raw");
    if let Some(t) = token { req = req.header("Authorization", format!("token {}", t)); }

    let resp = req.send().context("Network error downloading file")?;
    if !resp.status().is_success() {
        anyhow::bail!("GitHub API error: {}", resp.status());
    }

    let dest_path = dest_dir.join(path);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let mut file = std::fs::File::create(&dest_path)?;
    let mut content = resp;
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

fn download_github_folder(owner: &str, repo: &str, path: &str, dest_dir: &Path, token: Option<&str>) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, path);
    let mut req = client.get(url).header("User-Agent", "swiftgit/1.3");
    if let Some(t) = token { req = req.header("Authorization", format!("token {}", t)); }

    let resp = req.send().context("Network error fetching folder contents")?;
    if !resp.status().is_success() {
        anyhow::bail!("GitHub API error: {}", resp.status());
    }

    let items: Vec<RemoteFile> = resp.json().context("Failed to parse folder contents")?;
    for item in items {
        download_github_item(owner, repo, &item, dest_dir, token)?;
    }
    Ok(())
}

pub fn fetch_user_repos(token: &str) -> Result<Vec<RemoteRepoItem>> {
    let client = reqwest::blocking::Client::new();
    let resp = client.get("https://api.github.com/user/repos?sort=updated&per_page=30")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "swiftgit/1.3")
        .send().context("Network error")?;
    
    if !resp.status().is_success() { anyhow::bail!("API Error: {}", resp.status()); }
    let repos: Vec<RemoteRepoItem> = resp.json()?;
    Ok(repos)
}

pub fn set_remote_and_push(root: &Path, token: &str, owner: &str, repo_name: &str, branch: &str, force: bool) -> Result<String> {
    let repo = GitRepo { root: root.to_path_buf() };
    let https_url = format!("https://github.com/{}/{}.git", owner, repo_name);

    let has_remote = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root).output()
        .map(|o| o.status.success()).unwrap_or(false);

    if has_remote {
        repo.git(&["remote", "set-url", "origin", &https_url])?;
    } else {
        repo.git(&["remote", "add", "origin", &https_url])?;
    }

    repo.push_with_pat(&https_url, token, branch, force)
}

#[derive(Debug, Clone)]
pub struct RebaseCommit {
    pub sha:     String,
    pub action:  String,
    pub message: String,
}

pub fn rebase_todo(root: &Path) -> Result<Vec<RebaseCommit>> {
    let out = Command::new("git")
        .args(["log", "HEAD~15..HEAD", "--oneline", "--no-decorate"])
        .current_dir(root).output().context("Failed to get rebase todo")?;
    
    let mut commits = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if let Some((sha, msg)) = line.split_once(' ') {
            commits.push(RebaseCommit {
                sha: sha.to_string(),
                action: "pick".to_string(),
                message: msg.to_string(),
            });
        }
    }
    Ok(commits)
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
