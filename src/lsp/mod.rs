pub mod models;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct LspConfig {
    // Basic LSP configuration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LspStatus {
    Connected,
    Disconnected,
    Error(String),
    Ready,
    Initializing,
    Unavailable,
    Failed,
    Uninitialized,
}

impl Default for LspStatus {
    fn default() -> Self {
        LspStatus::Disconnected
    }
}