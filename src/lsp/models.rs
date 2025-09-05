use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridAnalysisResult {
    pub analysis_type: String,
    pub results: Vec<AnalysisItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisItem {
    pub name: String,
    pub location: String,
    pub details: String,
}