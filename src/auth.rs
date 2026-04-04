//! auth.rs — Hybrid SSH + PAT authentication for SwiftGit
//!
//! Responsibilities:
//!   SSH  → all Git operations (clone, push, pull) — no passwords, no prompts
//!   PAT  → GitHub API (list repos, create repo, user info)
//!
//! Flow:
//!   1. On first launch, check SSH keys → generate if missing
//!   2. Display public key, guide user to github.com/settings/keys
//!   3. Validate PAT via GET /user
//!   4. All subsequent operations use SSH silently

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// ── SSH key locations ─────────────────────────────────────────────────────────

const KEY_TYPES: &[(&str, &str)] = &[
    ("id_ed25519", "ed25519"),   // preferred: smaller, faster, secure
    ("id_rsa",     "rsa"),       // fallback for older systems
    ("id_ecdsa",   "ecdsa"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshStatus {
    /// Key exists and ssh-agent/known hosts ready
    Ready { key_path: PathBuf, pubkey: String },
    /// Key exists but not added to GitHub yet (user must add it)
    PubkeyPending { key_path: PathBuf, pubkey: String },
    /// No key found — needs generation
    NoKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatStatus {
    Valid { username: String },
    Invalid,
    NotSet,
}

// ── SSH detection ─────────────────────────────────────────────────────────────

/// Returns the first existing SSH private key, or None
pub fn find_ssh_key() -> Option<(PathBuf, PathBuf)> {
    let ssh_dir = dirs::home_dir()?.join(".ssh");
    for (name, _) in KEY_TYPES {
        let priv_key = ssh_dir.join(name);
        let pub_key  = ssh_dir.join(format!("{}.pub", name));
        if priv_key.exists() && pub_key.exists() {
            return Some((priv_key, pub_key));
        }
    }
    None
}

/// Read the public key content from disk
pub fn read_pubkey(pub_path: &Path) -> Result<String> {
    std::fs::read_to_string(pub_path)
        .map(|s| s.trim().to_string())
        .with_context(|| format!("Failed to read public key: {}", pub_path.display()))
}

/// Generate a new ed25519 SSH key pair non-interactively
/// Saves to ~/.ssh/id_ed25519 — does NOT overwrite existing keys
pub fn generate_ssh_key(email: &str) -> Result<(PathBuf, String)> {
    let ssh_dir  = dirs::home_dir()
        .context("Cannot find home directory")?
        .join(".ssh");
    std::fs::create_dir_all(&ssh_dir)
        .context("Failed to create ~/.ssh")?;

    // Set permissions on .ssh dir (ssh is strict about this)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ssh_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let priv_path = ssh_dir.join("id_ed25519");
    let pub_path  = ssh_dir.join("id_ed25519.pub");

    // Safety: never overwrite an existing key
    if priv_path.exists() {
        anyhow::bail!("SSH key already exists at {}", priv_path.display());
    }

    let out = Command::new("ssh-keygen")
        .args([
            "-t", "ed25519",
            "-C", email,          // comment = email for identification on GitHub
            "-f", &priv_path.to_string_lossy(),
            "-N", "",             // empty passphrase for seamless operation
            "-q",                 // quiet
        ])
        .output()
        .context("ssh-keygen not found — install OpenSSH")?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("ssh-keygen failed: {}", err.trim());
    }

    let pubkey = read_pubkey(&pub_path)?;
    Ok((priv_path, pubkey))
}

// ── SSH connectivity check ────────────────────────────────────────────────────

/// Test if GitHub accepts our SSH key.
/// Returns Ok(username) on success, Err with message on failure.
pub fn test_ssh_github() -> Result<String> {
    // ssh -T git@github.com exits with code 1 but prints "Hi username!"
    let out = Command::new("ssh")
        .args([
            "-T",
            "-o", "StrictHostKeyChecking=accept-new",
            "-o", "BatchMode=yes",
            "-o", "ConnectTimeout=10",
            "git@github.com",
        ])
        .output()
        .context("ssh not found")?;

    // GitHub always exits 1 on -T but the message tells us if auth worked
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    if combined.contains("Hi ") && combined.contains("!") {
        // Extract username from "Hi username! You have..."
        let username = combined
            .split("Hi ").nth(1)
            .and_then(|s| s.split('!').next())
            .unwrap_or("user")
            .trim()
            .to_string();
        Ok(username)
    } else if combined.contains("Permission denied") {
        anyhow::bail!("SSH key not added to GitHub yet")
    } else if combined.contains("Could not resolve hostname") {
        anyhow::bail!("No internet connection")
    } else {
        anyhow::bail!("SSH auth failed: {}", combined.trim())
    }
}

/// Detect full SSH status (key presence + GitHub connectivity)
pub fn detect_ssh_status() -> SshStatus {
    match find_ssh_key() {
        None => SshStatus::NoKey,
        Some((_priv, pub_path)) => {
            let pubkey = read_pubkey(&pub_path).unwrap_or_default();
            let priv_path = pub_path.with_extension("");
            match test_ssh_github() {
                Ok(_) => SshStatus::Ready { key_path: priv_path, pubkey },
                Err(_) => SshStatus::PubkeyPending { key_path: priv_path, pubkey },
            }
        }
    }
}

// ── ssh-agent helpers ─────────────────────────────────────────────────────────

/// Ensure the private key is loaded into ssh-agent for the session.
/// This prevents repeated passphrase prompts (though we use empty passphrase).
pub fn ensure_key_in_agent(key_path: &Path) -> Result<()> {
    // Check if already loaded
    let list_out = Command::new("ssh-add").args(["-l"]).output();
    if let Ok(o) = list_out {
        let out_str = String::from_utf8_lossy(&o.stdout).to_string();
        let key_str = key_path.to_string_lossy().to_string();
        if out_str.contains(&key_str) {
            return Ok(()); // already in agent
        }
    }

    // Add the key
    let add_out = Command::new("ssh-add")
        .arg(key_path)
        .output()
        .context("ssh-add failed")?;

    if !add_out.status.success() {
        let err = String::from_utf8_lossy(&add_out.stderr);
        anyhow::bail!("ssh-add failed: {}", err.trim());
    }
    Ok(())
}

// ── SSH URL conversion ────────────────────────────────────────────────────────

/// Convert any GitHub URL to its SSH equivalent.
///
/// https://github.com/user/repo.git  →  git@github.com:user/repo.git
/// https://github.com/user/repo      →  git@github.com:user/repo.git
/// git@github.com:user/repo.git      →  (unchanged)
pub fn to_ssh_url(url: &str) -> String {
    let url = crate::git::GitRepo::clean_url(url.trim().trim_end_matches('/'));

    // Already SSH
    if url.starts_with("git@github.com:") {
        return ensure_git_suffix(&url);
    }

    // HTTPS → SSH
    if let Some(path) = url.strip_prefix("https://github.com/") {
        return format!("git@github.com:{}", ensure_git_suffix(path));
    }

    // Unknown format — return as-is
    url.to_string()
}

fn ensure_git_suffix(s: &str) -> String {
    if s.ends_with(".git") {
        s.to_string()
    } else {
        format!("{}.git", s)
    }
}

/// Extract (owner, repo) from any GitHub URL format
pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/').trim_end_matches(".git");

    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = url.strip_prefix("https://github.com/") {
        rest
    } else {
        return None;
    };

    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

// ── PAT validation ────────────────────────────────────────────────────────────

/// Validate a GitHub PAT by calling GET /user.
/// Returns the GitHub username on success.
pub fn validate_pat_blocking(token: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "swiftgit/1.3")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .context("Network error reaching GitHub API")?;

    match resp.status().as_u16() {
        200 => {
            let json: serde_json::Value = resp.json()
                .context("Failed to parse GitHub user response")?;
            let username = json["login"]
                .as_str()
                .unwrap_or("user")
                .to_string();
            Ok(username)
        }
        401 => anyhow::bail!("Invalid token — check it and try again"),
        403 => anyhow::bail!("Token has no API access — regenerate with 'repo' scope"),
        _ => anyhow::bail!("GitHub API returned {}", resp.status()),
    }
}

/// Async PAT validation (used from TUI event loop)
pub async fn validate_pat(token: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token))
        .header("User-Agent", "swiftgit/1.3")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("Network error")?;

    match resp.status().as_u16() {
        200 => {
            let json: serde_json::Value = resp.json().await?;
            Ok(json["login"].as_str().unwrap_or("user").to_string())
        }
        401 => anyhow::bail!("Invalid token"),
        403 => anyhow::bail!("Token lacks permissions"),
        _ => anyhow::bail!("GitHub returned {}", resp.status()),
    }
}

// ── Combined auth status ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub ssh:      SshStatus,
    pub pat_user: Option<String>,   // None = no PAT stored / not validated
}

impl AuthStatus {
    pub fn ssh_ready(&self) -> bool {
        matches!(self.ssh, SshStatus::Ready { .. })
    }

    pub fn ssh_label(&self) -> &'static str {
        match &self.ssh {
            SshStatus::Ready { .. }        => "SSH ✅ connected",
            SshStatus::PubkeyPending { .. } => "SSH ⚠ key not added to GitHub",
            SshStatus::NoKey               => "SSH ❌ no key",
        }
    }

    pub fn pat_label(&self) -> String {
        match &self.pat_user {
            Some(u) => format!("API ✅ @{}", u),
            None    => "API ○ not connected".to_string(),
        }
    }
}

/// Run full auth detection (blocking — call from spawn_blocking)
pub fn detect_auth_status(token: Option<&str>) -> AuthStatus {
    let ssh     = detect_ssh_status();
    let pat_user = token.and_then(|t| validate_pat_blocking(t).ok());
    AuthStatus { ssh, pat_user }
}

// ── Git clone via SSH ─────────────────────────────────────────────────────────

/// Clone a repo using SSH URL, into dest_dir.
/// Automatically converts HTTPS URLs to SSH.
pub fn clone_via_ssh(url: &str, dest_dir: &Path) -> Result<String> {
    let ssh_url = to_ssh_url(url);

    let out = Command::new("git")
        .args(["clone", &ssh_url])
        .current_dir(dest_dir)
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new")
        .output()
        .context("git clone failed")?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if out.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        let e = stderr.trim();
        if e.contains("Permission denied") {
            anyhow::bail!(
                "SSH key not authorised on GitHub.\n\
                 Add your public key at: github.com/settings/keys"
            )
        } else if e.contains("not found") || e.contains("does not exist") {
            anyhow::bail!("Repository not found: {}", ssh_url)
        } else {
            anyhow::bail!("{}", e)
        }
    }
}

// ── Git push/pull via SSH ─────────────────────────────────────────────────────

/// Set the remote URL to SSH and push — no credentials, no prompts.
pub fn push_via_ssh(repo_root: &Path, branch: &str, force: bool) -> Result<String> {
    // Convert any existing HTTPS remote to SSH
    ensure_remote_is_ssh(repo_root)?;

    let mut args = vec!["push", "--set-upstream", "origin", branch];
    if force { args.push("--force"); }

    let out = Command::new("git")
        .args(&args)
        .current_dir(repo_root)
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("git push failed")?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if out.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        let e = stderr.trim();
        if e.contains("Permission denied") {
            anyhow::bail!("SSH push denied — add your key at github.com/settings/keys")
        } else if e.contains("does not match any") || e.contains("src refspec") {
            anyhow::bail!("No commits to push — commit something first")
        } else {
            anyhow::bail!("{}", e)
        }
    }
}

pub fn pull_via_ssh(repo_root: &Path) -> Result<String> {
    ensure_remote_is_ssh(repo_root)?;

    let out = Command::new("git")
        .args(["pull"])
        .current_dir(repo_root)
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("git pull failed")?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    if out.status.success() {
        Ok(format!("{}{}", stdout, stderr).trim().to_string())
    } else {
        anyhow::bail!("{}", stderr.trim())
    }
}

/// Convert an HTTPS remote to SSH in-place (modifies .git/config)
pub fn ensure_remote_is_ssh(repo_root: &Path) -> Result<()> {
    let current = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_root)
        .output();

    match current {
        Ok(o) if o.status.success() => {
            let url = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if url.starts_with("https://") {
                let ssh = to_ssh_url(&url);
                Command::new("git")
                    .args(["remote", "set-url", "origin", &ssh])
                    .current_dir(repo_root)
                    .output()?;
            }
            Ok(())
        }
        _ => Ok(()), // no remote yet — that's fine
    }
}

/// Set origin to SSH URL (used after API-create-repo)
pub fn set_remote_ssh(repo_root: &Path, owner: &str, repo: &str) -> Result<()> {
    let ssh_url = format!("git@github.com:{}/{}.git", owner, repo);

    let has_remote = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_remote {
        Command::new("git")
            .args(["remote", "set-url", "origin", &ssh_url])
            .current_dir(repo_root)
            .output()?;
    } else {
        Command::new("git")
            .args(["remote", "add", "origin", &ssh_url])
            .current_dir(repo_root)
            .output()?;
    }
    Ok(())
}
