use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::model::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefreshableView {
    Files,
    Commits,
    Branches,
    Status,
    All,
}

pub struct Refresher;

impl Refresher {
    pub async fn refresh(_model: Arc<Mutex<Model>>, scope: RefreshableView) {
        // Implementation of refresh logic per scope
        match scope {
            RefreshableView::Files => {
                // Fetch files from git
            }
            RefreshableView::Status => {
                // Fetch git status
            }
            _ => {
                // Handle other scopes
            }
        }
    }
}
