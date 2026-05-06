use std::sync::Arc;

use crate::{config::Config, tunnel::registry::Registry};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            registry: Arc::new(Registry::new()),
        }
    }
}
