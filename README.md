# 🚀 SwiftGit v1.3.7

>**A minimal, fast, keyboard-driven Git client for your terminal.**

SwiftGit is a high-performance terminal user interface (TUI) that streamlines your everyday Git workflows into a clean, intuitive experience. Built with **Rust** and **Ratatui**, it focuses on speed, simplicity, and a "no-nonsense" approach to version control.

---

## ✨ New in v1.3

- **🔐 Smart Authentication**: SwiftGit now automatically detects your SSH keys. It prioritizes **SSH** for lightning-fast, prompt-less pushes/pulls and intelligently falls back to **PAT (HTTPS)** only when needed.
- **🔥 Force Push Support**: Stuck with a non-fast-forward rejection? You can now toggle **Force Push** by pressing `Ctrl+F` inside the push dialog. A clear visual indicator will let you know when it's active.
- **📜 Commit History at a Glance**: When you're drafting a new commit, SwiftGit shows you the **last 5 commits** along with their hashes right below the input box, so you never lose track of your progress.
- **⏳ Global Loading Spinner**: No more wondering if a process is hung. A sleek loading box appears in the bottom-right corner with **real-time status labels** (e.g., "Cloning entire repo...", "Pushing via SSH...") during long-running tasks.
- **📁 Enhanced Tree Navigation**: Folders now show status indicators (like `[✓]` for all staged) just like files.
- **📝 Built-in Editor**: Quick fix needed? Press `e` on any file to open the internal editor and make changes without leaving SwiftGit.
- **🌐 GhGrab (GitHub Explorer)**: Browse and download specific files or folders from any GitHub repository directly from the dashboard—perfect for grabbing just what you need without a full clone.

---

## 🛠️ Key Features

- **⚡ Instant Dashboard**: Quick access to open folders, clone repositories, or jump back into one of your last 10 projects.
- **📂 Smart Path Suggestions**: Navigate your local filesystem with live directory suggestions and `Tab` completion.
- **🏗️ Powerful Repository View**:
  - **Live Status**: Instantly see `[M]` Modified, `[U]` Untracked, and `[✓]` Staged files.
  - **Stage/Unstage**: Toggle file status with a single `Space` or stage everything at once with `s`.
  - **Diff Preview**: Syntax-highlighted diffs in a dedicated, high-contrast panel.
- **⚙️ Integrated Settings**: Press `Ctrl+W` anywhere to manage your GitHub token and profile. Credentials are automatically masked for your security.
- **🌈 Universal Emoji Icons**: Works on every terminal and OS without needing special "Nerd Fonts"—just clean, beautiful icons out of the box.

---

## 🚀 Installation (Automated)

The included installer checks for system dependencies and handles the build process for you on **Debian/Ubuntu** or **Arch Linux**.

```bash
git clone https://github.com/Dev-CoreX/SwiftGIT.git
cd SwiftGIT
./install.sh
```

## 🏗️ Installation (Manual)

If you prefer to build manually, ensure you have **Rust** and **Git** installed:

```bash
cargo build --release
sudo cp target/release/swiftgit /usr/local/bin/
```

---

## ⌨️ Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Ctrl+W` | Open / Close Settings |
| `Ctrl+C` | Force Quit |
| `Ctrl+Q` | Quit App |

### Dashboard
| Key | Action |
|-----|--------|
| `↑↓` / `j/k` | Navigate Menu |
| `Enter` | Select / Open Project |
| `1` / `2` | Quick access: Open Folder / Clone Repo |
| `g` | Open GhGrab (GitHub Explorer) |

### Repository View
| Key | Action |
|-----|--------|
| `↑↓` / `j/k` | Navigate File Tree |
| `Enter` | Expand/Collapse Folder |
| `Space` | Stage / Unstage File |
| `s` | Stage ALL changes (`git add -A`) |
| `e` | Open Built-in Editor |
| `c` | Commit Mode (shows history) |
| `p` | Open Push Dialog |
| `P` | Smart Pull (auto-detects SSH/PAT) |
| `r` | Refresh Status |
| `Esc` | Back to Dashboard |

### Push Dialog
| Key | Action |
|-----|--------|
| `Ctrl+F` | **Toggle Force Push** |
| `Tab` / `↑↓` | Switch Fields (Repo Name / Branch) |
| `Enter` | Confirm Push |
| `Esc` | Cancel |

---

## ⚙️ Configuration

SwiftGit keeps things simple in `~/.swiftgit/config.json`.

```json
{
  "github_token": "ghp_your_token_here",
  "username": "your_github_username",
  "display_name": "Your Name",
  "recent_projects": []
}
```

---

## 📄 License

**MIT License** - Free to use, modify, and share.

Built with ❤️ by the SwiftGit team for developers who live in the terminal.