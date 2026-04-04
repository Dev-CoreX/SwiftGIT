pub mod dashboard;
pub mod folder_input;
pub mod clone_input;
pub mod auth;
pub mod recent_projects;
pub mod repo;
pub mod editor;
pub mod loading;
pub mod settings;
pub mod remote_picker;
pub mod push_dialog;
pub mod ssh_setup;
pub mod deinit_confirm;
pub mod rebase;
pub mod search;
pub mod help;

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::model::Model;

use async_trait::async_trait;

#[async_trait]
pub trait Context: Send + Sync {
    fn view_name(&self) -> &str;
    fn render(&self, f: &mut Frame, model: &Model) -> Result<()>;
    async fn handle_event(&self, event: KeyEvent, model: Arc<Mutex<Model>>) -> Result<bool>;
    
    async fn on_focus(&self, _model: Arc<Mutex<Model>>) -> Result<()> { Ok(()) }
    async fn on_focus_lost(&self, _model: Arc<Mutex<Model>>) -> Result<()> { Ok(()) }
}

pub struct ContextStack {
    stack: Vec<Box<dyn Context>>,
}

impl ContextStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub async fn push(&mut self, context: Box<dyn Context>, model: Arc<Mutex<Model>>) -> Result<()> {
        if let Some(current) = self.stack.last() {
            current.on_focus_lost(Arc::clone(&model)).await?;
        }
        context.on_focus(model).await?;
        self.stack.push(context);
        Ok(())
    }

    pub async fn pop(&mut self, model: Arc<Mutex<Model>>) -> Result<Option<Box<dyn Context>>> {
        let popped = self.stack.pop();
        if let Some(ref context) = popped {
            context.on_focus_lost(Arc::clone(&model)).await?;
        }
        if let Some(current) = self.stack.last() {
            current.on_focus(model).await?;
        }
        Ok(popped)
    }

    pub fn current(&self) -> Option<&dyn Context> {
        self.stack.last().map(|c| c.as_ref())
    }
}
