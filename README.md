# SwiftGit v2.0

> A keyboard-driven, terminal-based Git client built in Rust. Faster than typing, cleaner than a GUI.

SwiftGit is a high-performance TUI (Terminal User Interface) application for managing Git repositories. It integrates directly with GitHub, features a syntax-highlighted diff viewer, an inline editor, and a powerful "RemotePicker" tool for selective file downloads.

## 🚀 Key Features in v2.0

- **Modular Context Architecture** — Completely rewritten backend using a context-driven stack, inspired by LazyGit. Each view is now an independent, highly responsive module.
- **Hunk-based Staging** — Precision staging is here! Navigate between individual diff hunks (`Tab`/`n`) and stage/unstage them independently (`Space`). Active hunks are highlighted for clarity.
- **Visual Interactive Rebase** — Press `i` to enter a visual rebase editor. Pick, drop, reword, and fixup commits with a real-time preview (simulated backend).
- **Global Search & Filter** — Press `/` anywhere in the Repo View to instantly filter your file tree with case-insensitive search.
- **Intelligent Push Dialog** — Press `p` to open a unified push dialog. It pre-fills your last commit message and can automatically create a new commit if you have staged changes.
- **TokyoNight IDE Theme** — A beautiful, professional "Minimal IDE" aesthetic across the entire app, including token-level syntax highlighting in the editor and diff viewer.
- **Recent Projects Selection** — Quickly switch between repositories with a dedicated modal (`d` to remove, `C` to clear history).
- **RemotePicker** — Recursive selective file/folder download from any GitHub repository without cloning.
- **SSH Setup Wizard** — Step-by-step guidance to generate and register SSH keys on GitHub.
- **Async Power** — All heavy operations (clone, fetch, push, pull) run in background tasks with smooth loading spinners, keeping the UI at 60FPS.

## 🛠 Installation

Requirements: Rust 1.74+, Git, and SSH.

```bash
# Automated install (Wizard included)
./install.sh

# Manual build
cargo build --release
cp target/release/swiftgit /usr/local/bin/
```

## ⌨️ Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Navigate tree / lists |
| `Tab` / `n` | Next diff hunk |
| `BackTab` | Previous diff hunk |
| `Space` | Stage / Unstage file, folder, or hunk |
| `Enter` | Expand/Collapse folder / Open selection |
| `s` | Stage ALL changes |
| `c` | Enter commit message mode |
| `p` | Open Push dialog |
| `P` | Pull from remote |
| `e` | Edit current file inline |
| `r` | Refresh status |
| `/` | Filter file tree |
| `i` | Visual Interactive Rebase |
| `?` | Show Help Overlay |
| `Ctrl+W` | Open Settings |
| `1` / `2` | Switch focus between Tree and Diff panel |
| `q` / `Esc`| Back / Quit |

## ⚙️ Configuration

SwiftGit stores its configuration in `~/.swiftgit/config.json`. The `install.sh` wizard handles this automatically, but you can edit it manually or via the `Ctrl+W` settings overlay.

---
Built with 🦀 by SwiftGit Team.
