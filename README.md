# SwiftGit v1.4

> A keyboard-driven, terminal-based Git client built in Rust. Faster than typing, cleaner than a GUI.

SwiftGit is a high-performance TUI (Terminal User Interface) application for managing Git repositories. It integrates directly with GitHub, features a syntax-highlighted diff viewer, an inline editor, and a powerful "GhGrab" tool for selective file downloads.

## 🚀 Key Features in v1.4

- **Hunk-based Staging** — Precision staging is here! Navigate between individual diff hunks (`Tab`/`n`/`p`) and stage/unstage them independently (`Space`). Active hunks are highlighted for clarity.
- **Dashboard** — Quick access to Open, Clone, and Recent Projects.
- **Recent Projects Dialog** — Pressing "Recent Projects" now opens a selection modal to pick from your history. Support for removing individual entries (`d`) or clearing all (`C`).
- **Collapsible File Tree** — Navigate complex repos with ease; status indicators (`[M]`, `[U]`, `[✓]`) show what's happening at a glance.
- **Syntax-Highlighted Diff** — TokyoNight Storm theme with token-level highlighting.
- **Async Diff Loading** — No more UI stutter; diffs load in the background with a smooth spinner.
- **Permanent Commit History** — The left panel now permanently displays recent commits for the current repo.
- **Stage/Unstage** — Spacebar to toggle files, entire folders, or individual hunks.
- **Smart Push/Pull** — Seamlessly handles SSH and HTTPS/PAT authentication. Detects remote changes and offers force-push confirmation if needed.
- **Inline Editor** — Press `e` to edit any file directly within SwiftGit.
- **SSH Setup Wizard** — Step-by-step guidance to generate and register SSH keys on GitHub.
- **GhGrab** — Selective file/folder download from any GitHub repository without cloning.
- **Enhanced Installer** — `install.sh` now features an orange-boxed interactive configuration wizard to set up your GitHub PAT, username, and SSH keys during installation.

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
| `↑` / `↓` | Navigate tree / lists |
| `Tab` / `n` | Next diff hunk |
| `BackTab` / `p` | Previous diff hunk |
| `Space` | Stage / Unstage file, folder, or hunk |
| `Enter` | Expand/Collapse folder / Open selection |
| `s` | Stage all changes |
| `c` | Enter commit message mode |
| `p` | Open Push dialog (if no hunk selected) |
| `P` | Pull from remote |
| `e` | Edit current file inline |
| `r` | Refresh status |
| `Ctrl+W` | Open Settings (Token, Username, Display Name) |
| `1` / `2` | Switch focus between Tree and Diff panel |
| `q` / `Esc`| Back / Quit |

## ⚙️ Configuration

SwiftGit stores its configuration in `~/.swiftgit/config.json`. The `install.sh` wizard handles this automatically, but you can edit it manually or via the `Ctrl+W` settings overlay.

---
Built with 🦀 by SwiftGit Team.