# SwiftGit v1.2 — Setup & Usage Guide

## Quick Install

```bash
unzip swiftgit-v1.2.zip
cd swiftgit-v1.2
cargo build --release
sudo cp target/release/swiftgit /usr/local/bin/
swiftgit
```

Requires: Rust 1.74+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

---

## First Launch

On first launch you'll see the **GitHub Auth** screen.

- Paste your **Personal Access Token** from github.com/settings/tokens
  - Scope needed: `repo` (for private repos) or none (public only)
- Press **Enter** to validate — SwiftGit calls the GitHub API and saves it
- Press **Esc** to skip (local repos only, push dialog won't load your repos)

Token saved to: `~/.swiftgit/config.json`

---

## Dashboard

```
  ▶ Open Folder       ← opens a file browser to pick your project
    Clone Repo        ← paste a GitHub URL to clone
    Recent Projects   ← reopen last used repos
```

Navigate with `↑↓` or `j/k`, select with `Enter`. Press `1` or `2` as shortcuts.

---

## Repo View — All Keybindings

### Navigation
| Key | Action |
|-----|--------|
| `↑↓` / `j k` | Move cursor |
| `Enter` | Expand / Collapse folder |
| `Space` | Stage / Unstage (file or entire folder) |
| `s` | Stage ALL changes (`git add -A`) |

### Panels (Task 6)
| Key | Action |
|-----|--------|
| `1` | Focus **Tree panel** (left) — `◉ 1 Working Tree` |
| `2` | Focus **Diff/Editor panel** (right) — `◉ 2 Diff` |

### Git Operations
| Key | Action |
|-----|--------|
| `c` | Commit staged files (type message → Enter) |
| `p` | Push — opens **Push Dialog** (Task 5) |
| `P` | Pull latest changes |
| `r` | Refresh git status |

### Editor (Task 7)
| Key | Action |
|-----|--------|
| `e` | Open current file in inline editor |
| `Ctrl+S` | Save file to disk |
| `Ctrl+X` | Close editor |
| `↑↓←→` | Move cursor |
| `Home`/`End` | Start/end of line |
| `Tab` | Insert 4 spaces |
| `Enter` | New line |
| `Backspace`/`Delete` | Delete character |

### General
| Key | Action |
|-----|--------|
| `Esc` | Back to Dashboard |
| `q` | Quit |
| `Ctrl+C` | Force quit |

---

## Push Dialog (Task 5)

Press `p` → a modal opens showing:

**Left panel:** Your GitHub repositories
- Pre-filtered to match your local repo name
- Type to search/filter
- `🔒` = private repo, `🌐` = public repo
- `↑↓` to navigate

**Right panel:** Your last 10 commits with hash + message

Press **Enter** to push to the selected repo.
SwiftGit will:
1. Set `origin` to `https://github.com/owner/repo.git`
2. Push using your PAT embedded in the URL (never stored in `.git/config`)
3. Show success/error in the status bar

---

## File Icons (Task 1)

| Icon | Type |
|------|------|
| 🦀 | Rust `.rs` |
| 🐍 | Python `.py` |
| `JS` | JavaScript `.js` |
| `TS` | TypeScript `.ts` |
| `{}` | JSON |
| ⚙  | TOML/INI config |
| 📝 | Markdown |
| ⚡ | Shell scripts |
| 📁 | Folder |
| 🔒 | Lock files |
| 🖼  | Images |
| 🐳 | Dockerfile |
| + 20 more types | |

---

## Config File

`~/.swiftgit/config.json`

```json
{
  "github_token": "ghp_...",
  "username": "yourname",
  "recent_projects": [
    { "path": "/home/user/myproject", "name": "myproject" }
  ]
}
```

To reset auth: `rm ~/.swiftgit/config.json` then relaunch.
