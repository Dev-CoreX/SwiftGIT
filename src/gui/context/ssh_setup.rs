use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::context::Context;
use crate::gui::model::{Model, AppMode};
use crate::ui::components::ssh_setup;
use crate::ui::components::ssh_setup::SshSetupStep;
use async_trait::async_trait;

pub struct SshSetupContext;

#[async_trait]
impl Context for SshSetupContext {
    fn view_name(&self) -> &str {
        "ssh_setup"
    }

    fn render(&self, f: &mut Frame, model: &Model) -> Result<()> {
        let ssh_state = ssh_setup::SshSetupState {
            step: &model.ssh_step,
            frame_count: model.frame_count,
        };
        ssh_setup::render(f, f.size(), &ssh_state);
        Ok(())
    }

    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool> {
        let current_step = {
            let s = model.lock().await;
            s.ssh_step.clone()
        };

        match current_step {
            SshSetupStep::Detecting => {
                let model_c = Arc::clone(&model);
                tokio::spawn(async move {
                    let ssh_status = tokio::task::spawn_blocking(|| {
                        crate::auth::detect_ssh_status()
                    }).await.unwrap_or(crate::auth::SshStatus::NoKey);

                    let mut s = model_c.lock().await;
                    match ssh_status {
                        crate::auth::SshStatus::Ready { pubkey, .. } => {
                            s.ssh_pubkey = pubkey;
                            s.ssh_step = SshSetupStep::Connected { username: "GitHub".to_string() };
                            s.config.ssh_key_added = true;
                            let _ = s.config.save();
                        }
                        crate::auth::SshStatus::PubkeyPending { pubkey, .. } => {
                            s.ssh_pubkey = pubkey.clone();
                            let token = s.config.github_token.clone();
                            if let Some(tok) = token {
                                let pk = pubkey.clone();
                                drop(s);
                                let auto_result = tokio::task::spawn_blocking(move || {
                                    crate::github::auto_register_ssh_key(&tok, &pk)
                                }).await;
                                let mut s2 = model_c.lock().await;
                                let auto_added = auto_result.map(|r| r.is_ok()).unwrap_or(false);
                                s2.ssh_step = SshSetupStep::ShowPubkey { pubkey: s2.ssh_pubkey.clone(), auto_added };
                            } else {
                                s.ssh_step = SshSetupStep::ShowPubkey { pubkey, auto_added: false };
                            }
                        }
                        crate::auth::SshStatus::NoKey => {
                            s.ssh_step = SshSetupStep::NeedGenerate;
                        }
                    }
                });
            }
            SshSetupStep::NeedGenerate => {
                if event.code == KeyCode::Enter {
                    let mut s = model.lock().await;
                    s.ssh_step = SshSetupStep::Generating;
                    let email = s.config.username.clone().unwrap_or_else(|| "swiftgit@local".to_string());
                    drop(s);

                    let model_c = Arc::clone(&model);
                    tokio::spawn(async move {
                        let result = tokio::task::spawn_blocking(move || {
                            crate::auth::generate_ssh_key(&email)
                        }).await;

                        let mut s = model_c.lock().await;
                        match result {
                            Ok(Ok((_path, pubkey))) => {
                                let pk = pubkey.clone();
                                s.ssh_pubkey = pubkey;
                                let token = s.config.github_token.clone();
                                if let Some(tok) = token {
                                    drop(s);
                                    let auto = tokio::task::spawn_blocking(move || {
                                        crate::github::auto_register_ssh_key(&tok, &pk)
                                    }).await;
                                    let auto_added = auto.map(|r| r.is_ok()).unwrap_or(false);
                                    let mut s2 = model_c.lock().await;
                                    s2.ssh_step = SshSetupStep::ShowPubkey { pubkey: s2.ssh_pubkey.clone(), auto_added };
                                } else {
                                    s.ssh_step = SshSetupStep::ShowPubkey { pubkey: s.ssh_pubkey.clone(), auto_added: false };
                                }
                            }
                            Ok(Err(e)) => s.ssh_step = SshSetupStep::Error(e.to_string()),
                            Err(e)    => s.ssh_step = SshSetupStep::Error(e.to_string()),
                        }
                    });
                } else if event.code == KeyCode::Esc {
                    let mut s = model.lock().await;
                    s.mode = if s.config.github_token.is_none() { AppMode::Auth } else { AppMode::Dashboard };
                }
            }
            SshSetupStep::ShowPubkey { .. } => {
                if event.code == KeyCode::Enter {
                    let mut s = model.lock().await;
                    s.ssh_step = SshSetupStep::Testing;
                    drop(s);

                    let model_c = Arc::clone(&model);
                    tokio::spawn(async move {
                        let result = tokio::task::spawn_blocking(|| crate::auth::test_ssh_github()).await;
                        let mut s = model_c.lock().await;
                        match result {
                            Ok(Ok(username)) => {
                                s.ssh_step = SshSetupStep::Connected { username };
                                s.config.ssh_key_added = true;
                                let _ = s.config.save();
                            }
                            Ok(Err(e)) => s.ssh_step = SshSetupStep::Error(e.to_string()),
                            Err(e)    => s.ssh_step = SshSetupStep::Error(e.to_string()),
                        }
                    });
                } else if event.code == KeyCode::Esc {
                    let mut s = model.lock().await;
                    s.mode = if s.config.github_token.is_none() { AppMode::Auth } else { AppMode::Dashboard };
                }
            }
            SshSetupStep::Connected { .. } => {
                if event.code == KeyCode::Enter || event.code == KeyCode::Esc {
                    let mut s = model.lock().await;
                    s.mode = if s.config.github_token.is_none() { AppMode::Auth } else { AppMode::Dashboard };
                }
            }
            SshSetupStep::Error(_) => {
                if event.code == KeyCode::Enter {
                    model.lock().await.ssh_step = SshSetupStep::Detecting;
                } else if event.code == KeyCode::Esc {
                    model.lock().await.mode = AppMode::Dashboard;
                }
            }
            _ => {}
        }
        Ok(true)
    }
}
