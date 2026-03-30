use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SwiftGitConfig {
    pub github_token: Option<String>,
    /// GitHub username — used for API calls
    pub username: Option<String>,
    /// Display name — shown on dashboard
    pub display_name: Option<String>,
    /// Whether the SSH key has been confirmed added to GitHub
    pub ssh_key_added: bool,
    pub recent_projects: Vec<RecentProject>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
}

impl SwiftGitConfig {
    pub fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".swiftgit").join("config.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Self = serde_json::from_str(&contents)
            .with_context(|| "Failed to parse config JSON")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
        }
        let contents = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, contents)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;
        Ok(())
    }

    pub fn add_recent_project(&mut self, path: String, name: String) {
        // Remove if already exists
        self.recent_projects.retain(|p| p.path != path);
        // Add to front
        self.recent_projects.insert(0, RecentProject { path, name });
        // Keep max 10
        self.recent_projects.truncate(10);
    }
}
