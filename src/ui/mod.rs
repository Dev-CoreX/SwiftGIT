use anyhow::{Context as AnyhowContext, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;

use crate::config::SwiftGitConfig;
pub use crate::gui::model::{AppMode, DisplayItem, Model as AppState};
pub use crate::gui::model::{build_display_items, compute_dir_suggestions};
use crate::gui::Gui;
use crate::gui::context::dashboard::DashboardContext;

pub mod components;
pub mod theme;

pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner_char(frame: u64) -> &'static str {
    SPINNER[(frame / 2) as usize % SPINNER.len()]
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
    let model = AppState::new(config);
    let mut gui = Gui::new(model);
    
    // Set initial mode based on config
    let initial_mode = if !gui.model.lock().await.config.ssh_key_added && crate::auth::find_ssh_key().is_none() {
        AppMode::SshSetup
    } else if gui.model.lock().await.config.github_token.is_none() {
        AppMode::Auth
    } else {
        AppMode::Dashboard
    };
    
    gui.model.lock().await.mode = initial_mode.clone();
    
    // Initialize initial context
    if initial_mode == AppMode::Dashboard {
        gui.context_stack.push(Box::new(DashboardContext), Arc::clone(&gui.model)).await?;
    }
    
    let result = event_loop(&mut terminal, gui).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut gui: Gui,
) -> Result<()> {
    let mut last_mode = gui.model.lock().await.mode.clone();
    
    loop {
        {
            let mut s = gui.model.lock().await;
            
            // Sync context stack with mode if changed
            if s.mode != last_mode {
                let current_view = gui.context_stack.current().map(|c| c.view_name().to_string());
                let target_view = match s.mode {
                    AppMode::Dashboard      => Some("dashboard"),
                    AppMode::FolderInput    => Some("folder_input"),
                    AppMode::CloneInput     => Some("clone_input"),
                    AppMode::Auth           => Some("auth"),
                    AppMode::RecentProjects => Some("recent_projects"),
                    AppMode::RepoView       => Some("repo_view"),
                    AppMode::Editor         => Some("editor"),
                    AppMode::Loading        => Some("loading"),
                    AppMode::Settings       => Some("settings"),
                    AppMode::RemotePicker   => Some("remote_picker"),
                    AppMode::PushDialog | AppMode::ForcePushConfirm => Some("push_dialog"),
                    AppMode::SshSetup       => Some("ssh_setup"),
                    AppMode::DeinitConfirm  => Some("deinit_confirm"),
                    AppMode::Rebase         => Some("rebase"),
                    AppMode::Search         => Some("search"),
                    AppMode::Help           => Some("help"),
                };

                if let Some(target) = target_view {
                    if current_view.as_deref() != Some(target) {
                        // If going to a primary mode, maybe clear or pop?
                        // For now, let's just push if not already there.
                        // Special case: if going back to Dashboard, pop everything above it.
                        if target == "dashboard" {
                            while let Some(ctx) = gui.context_stack.current() {
                                if ctx.view_name() == "dashboard" { break; }
                                gui.context_stack.pop(Arc::clone(&gui.model)).await?;
                            }
                        } else {
                            // If target is already in the stack (but not on top), we might want to pop to it.
                            // Simplified: push new.
                            match s.mode {
                                AppMode::FolderInput => { gui.context_stack.push(Box::new(crate::gui::context::folder_input::FolderInputContext), Arc::clone(&gui.model)).await?; }
                                AppMode::CloneInput  => { gui.context_stack.push(Box::new(crate::gui::context::clone_input::CloneInputContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Auth        => { gui.context_stack.push(Box::new(crate::gui::context::auth::AuthContext), Arc::clone(&gui.model)).await?; }
                                AppMode::RecentProjects => { gui.context_stack.push(Box::new(crate::gui::context::recent_projects::RecentProjectsContext), Arc::clone(&gui.model)).await?; }
                                AppMode::RepoView    => { gui.context_stack.push(Box::new(crate::gui::context::repo::RepoContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Editor      => { gui.context_stack.push(Box::new(crate::gui::context::editor::EditorContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Loading     => { gui.context_stack.push(Box::new(crate::gui::context::loading::LoadingContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Settings    => { gui.context_stack.push(Box::new(crate::gui::context::settings::SettingsContext), Arc::clone(&gui.model)).await?; }
                                AppMode::RemotePicker => {
                                    gui.context_stack.push(Box::new(crate::gui::context::remote_picker::RemotePickerContext), Arc::clone(&gui.model)).await?;
                                }

                                AppMode::PushDialog | AppMode::ForcePushConfirm => { gui.context_stack.push(Box::new(crate::gui::context::push_dialog::PushDialogContext), Arc::clone(&gui.model)).await?; }
                                AppMode::SshSetup    => { gui.context_stack.push(Box::new(crate::gui::context::ssh_setup::SshSetupContext), Arc::clone(&gui.model)).await?; }
                                AppMode::DeinitConfirm => { gui.context_stack.push(Box::new(crate::gui::context::deinit_confirm::DeinitConfirmContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Rebase      => { gui.context_stack.push(Box::new(crate::gui::context::rebase::RebaseContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Search      => { gui.context_stack.push(Box::new(crate::gui::context::search::SearchContext), Arc::clone(&gui.model)).await?; }
                                AppMode::Help        => { gui.context_stack.push(Box::new(crate::gui::context::help::HelpContext), Arc::clone(&gui.model)).await?; }
                                _ => {}
                            }
                        }
                    }
                }
                last_mode = s.mode.clone();
            }

            s.frame_count = s.frame_count.wrapping_add(1);
            let _fc = s.frame_count;

            if let Some(ref t) = s.toast {
                if t.is_expired() { s.toast = None; }
            }
            
            terminal.draw(|f| {
                let area = f.size();
                f.render_widget(
                    ratatui::widgets::Block::default()
                        .style(ratatui::style::Style::default().bg(theme::BG_COLOR)),
                    area,
                );
                
                if let Some(context) = gui.context_stack.current() {
                    if let Err(e) = context.render(f, &s) {
                         s.status_msg = format!("Render error: {}", e);
                    }
                }
                
                if let Some(ref t) = s.toast {
                    components::toast::render(f, area, t);
                }
                
                if s.is_loading {
                    render_global_loading(f, area, s.frame_count, &s.loading_label);
                }
            })?;
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    // Global keys
                    if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        return Ok(());
                    }

                    if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        let mut s = gui.model.lock().await;
                        if s.mode == AppMode::Settings {
                            let prev = s.settings_prev_mode.clone();
                            s.mode = prev;
                        } else {
                            s.settings_prev_mode = s.mode.clone();
                            s.settings_dlg.display_name = s.config.display_name.clone().unwrap_or_default();
                            s.settings_dlg.username = s.config.username.clone().unwrap_or_default();
                            s.settings_dlg.token = s.config.github_token.clone().unwrap_or_default();
                            s.settings_dlg.focused = components::settings_dialog::SettingsField::DisplayName;
                            s.settings_dlg.cursor = s.settings_dlg.display_name.chars().count();
                            s.settings_dlg.show_token = false;
                            s.mode = AppMode::Settings;
                        }
                        continue;
                    }

                    if key.code == KeyCode::Char('?') {
                        let mut s = gui.model.lock().await;
                        if s.mode == AppMode::Help {
                            let prev = s.help_prev_mode.clone();
                            s.mode = prev;
                        } else {
                            s.help_prev_mode = s.mode.clone();
                            s.mode = AppMode::Help;
                        }
                        continue;
                    }

                    if let Some(context) = gui.context_stack.current() {
                        if context.handle_event(key, Arc::clone(&gui.model)).await? {
                            continue;
                        }
                    }
                }
            }
        }
    }
}

pub async fn validate_github_token(token: &str) -> Result<String> {
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

// ── Rendering helpers ─────────────────────────────────────────────────────────

pub fn render_text_input(
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

pub fn render_folder_input(
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

    render_text_input(f, area, "Open Folder", "/path/to/your/project", input, frame_count);

    if suggestions.is_empty() { return; }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
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

    if popup_area.y + popup_area.height > area.height { return; }

    let items: Vec<ListItem> = suggestions.iter().enumerate().map(|(i, s)| {
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

pub fn render_global_loading(f: &mut ratatui::Frame, area: ratatui::layout::Rect, frame_count: u64, label: &str) {
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

pub fn render_recent_projects_dialog(
    f: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    projects: &[crate::config::RecentProject],
    cursor: usize,
) {
    use ratatui::{
        layout::Rect,
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem},
    };

    let mw = 70u16.min(area.width.saturating_sub(4));
    let mh = 14u16.min(area.height.saturating_sub(4));
    let mx = area.x + (area.width.saturating_sub(mw)) / 2;
    let my = area.y + (area.height.saturating_sub(mh)) / 2;
    let modal = Rect::new(mx, my, mw, mh);

    f.render_widget(ratatui::widgets::Clear, modal);

    let items: Vec<ListItem> = projects.iter().enumerate().map(|(i, p)| {
        let selected = i == cursor;
        if selected {
            ListItem::new(Line::from(vec![
                Span::styled(" ▶ ", Style::default().fg(theme::BG_COLOR).bg(theme::ACCENT_COLOR)),
                Span::styled(format!("{:<20} ", p.name), Style::default().fg(theme::BG_COLOR).bg(theme::ACCENT_COLOR).add_modifier(Modifier::BOLD)),
                Span::styled(p.path.clone(), Style::default().fg(theme::BG_COLOR).bg(theme::ACCENT_COLOR).add_modifier(Modifier::ITALIC)),
            ]))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format!("{:<20} ", p.name), Style::default().fg(theme::ACCENT_COLOR).add_modifier(Modifier::BOLD)),
                Span::styled(p.path.clone(), Style::default().fg(theme::BORDER_COLOR)),
            ]))
        }
    }).collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Recent Project — Enter=Open  d=Remove  C=Clear All  Esc=Back ")
            .border_style(Style::default().fg(theme::ACCENT_COLOR))
            .style(Style::default().bg(theme::BG_COLOR)),
    );

    f.render_widget(list, modal);
}

pub fn render_deinit_confirm(f: &mut ratatui::Frame, area: ratatui::layout::Rect, cursor: usize) {
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

    // Hint
    let hint = Paragraph::new("←→ Select   Enter Confirm   ? Help   Esc Cancel")
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(theme::BORDER_COLOR));
    f.render_widget(hint, rows[0]); // Re-use first empty row for hint
}

pub fn render_force_push_confirm(f: &mut ratatui::Frame, area: ratatui::layout::Rect, cursor: usize) {
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

    // Hint
    let hint = Paragraph::new("←→ Select   Enter Confirm   ? Help   Esc Cancel")
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(theme::BORDER_COLOR));
    f.render_widget(hint, rows[0]);
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
