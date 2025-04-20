pub mod api;
pub mod config;
pub mod error;
pub mod llm;
pub mod scraper;

use std::sync::Arc;
use config::Config;

/// Application state that will be shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
} 