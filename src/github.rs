//! github.rs — GitHub API via PAT (repository management only)
//!
//! Responsibilities:
//!   - list user repos
//!   - create repo
//!   - get user info
//!   - add SSH key to GitHub account
//!
//! All Git operations (clone/push/pull) are handled by auth.rs + git.rs via SSH.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── API types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GhUser {
    pub login:      String,
    pub name:       Option<String>,
    pub email:      Option<String>,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhRepo {
    pub full_name:    String,
    pub name:         String,
    pub private:      bool,
    pub ssh_url:      String,    // git@github.com:owner/repo.git  ← we always use this
    pub clone_url:    String,    // https:// — kept for display only
    pub description:  Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhSshKey {
    pub id:    u64,
    pub title: String,
    pub key:   String,
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct GithubClient {
    token:  String,
    client: reqwest::blocking::Client,
}

impl GithubClient {
    pub fn new(token: &str) -> Self {
        Self {
            token:  token.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    fn get(&self, endpoint: &str) -> Result<reqwest::blocking::Response> {
        let url = format!("https://api.github.com{}", endpoint);
        let resp = self.client
            .get(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("User-Agent", "swiftgit/1.3")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .with_context(|| format!("GET {} failed", url))?;
        Ok(resp)
    }

    fn post<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<reqwest::blocking::Response> {
        let url = format!("https://api.github.com{}", endpoint);
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .header("User-Agent", "swiftgit/1.3")
            .header("Accept", "application/vnd.github.v3+json")
            .json(body)
            .send()
            .with_context(|| format!("POST {} failed", url))?;
        Ok(resp)
    }

    // ── User ─────────────────────────────────────────────────────────────────

    pub fn get_user(&self) -> Result<GhUser> {
        let resp = self.get("/user")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to get user: {}", resp.status());
        }
        resp.json::<GhUser>().context("Failed to parse user")
    }

    // ── Repositories ──────────────────────────────────────────────────────────

    /// List all repos for the authenticated user (sorted by recent activity)
    pub fn list_repos(&self) -> Result<Vec<GhRepo>> {
        let resp = self.get("/user/repos?per_page=100&sort=updated&affiliation=owner")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to list repos: {}", resp.status());
        }
        resp.json::<Vec<GhRepo>>().context("Failed to parse repos")
    }

    /// Create a new GitHub repository.
    /// Returns the created repo with its SSH URL already populated.
    pub fn create_repo(&self, name: &str, private: bool, description: &str) -> Result<GhRepo> {
        #[derive(Serialize)]
        struct CreateRepoPayload<'a> {
            name:        &'a str,
            private:     bool,
            description: &'a str,
            auto_init:   bool,   // create initial commit so we can push immediately
        }

        let payload = CreateRepoPayload {
            name,
            private,
            description,
            auto_init: false,    // empty repo — user will push their own first commit
        };

        let resp = self.post("/user/repos", &payload)?;
        let status = resp.status().as_u16();

        match status {
            201 => resp.json::<GhRepo>().context("Failed to parse created repo"),
            422 => anyhow::bail!("Repo '{}' already exists on your account", name),
            403 => anyhow::bail!("PAT lacks 'repo' scope — update via Ctrl+W"),
            _   => anyhow::bail!("Create repo failed: {}", status),
        }
    }

    // ── SSH keys ──────────────────────────────────────────────────────────────

    /// List all SSH keys registered on the GitHub account
    pub fn list_ssh_keys(&self) -> Result<Vec<GhSshKey>> {
        let resp = self.get("/user/keys")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to list SSH keys: {}", resp.status());
        }
        resp.json::<Vec<GhSshKey>>().context("Failed to parse SSH keys")
    }

    /// Check if a given public key is already registered on GitHub
    pub fn is_key_registered(&self, pubkey: &str) -> Result<bool> {
        let keys  = self.list_ssh_keys()?;
        let clean = pubkey.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
        Ok(keys.iter().any(|k| {
            let k_clean = k.key.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
            k_clean == clean
        }))
    }

    /// Add an SSH public key to the GitHub account programmatically.
    /// This is optional — users can also add it manually at github.com/settings/keys.
    pub fn add_ssh_key(&self, title: &str, pubkey: &str) -> Result<GhSshKey> {
        #[derive(Serialize)]
        struct AddKeyPayload<'a> {
            title: &'a str,
            key:   &'a str,
        }

        let resp = self.post("/user/keys", &AddKeyPayload { title, key: pubkey })?;
        let status = resp.status().as_u16();

        match status {
            201 => resp.json::<GhSshKey>().context("Failed to parse added key"),
            422 => anyhow::bail!("Key already exists on your GitHub account"),
            403 => anyhow::bail!("PAT needs 'admin:public_key' scope to add SSH keys"),
            _   => anyhow::bail!("Failed to add SSH key: {}", status),
        }
    }
}

// ── Convenience functions ─────────────────────────────────────────────────────

/// Try to auto-register the local SSH pubkey on GitHub.
/// Silently succeeds if key already exists.
pub fn auto_register_ssh_key(token: &str, pubkey: &str) -> Result<String> {
    let client = GithubClient::new(token);

    // Check if already registered
    if client.is_key_registered(pubkey).unwrap_or(false) {
        return Ok("SSH key already registered on GitHub".to_string());
    }

    // Title: "SwiftGit on <hostname>"
    let hostname = std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let title = format!("SwiftGit on {}", hostname);

    client.add_ssh_key(&title, pubkey)?;
    Ok(format!("SSH key '{}' added to GitHub", title))
}
