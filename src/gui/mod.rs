pub mod context;
pub mod controllers;
pub mod helpers;
pub mod model;

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::gui::model::Model;
use crate::gui::context::ContextStack;

pub struct Gui {
    pub model: Arc<Mutex<Model>>,
    pub context_stack: ContextStack,
}

impl Gui {
    pub fn new(model: Model) -> Self {
        Self {
            model: Arc::new(Mutex::new(model)),
            context_stack: ContextStack::new(),
        }
    }
}
