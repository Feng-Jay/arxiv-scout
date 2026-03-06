use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub published: DateTime<Utc>,
    pub url: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedPaper {
    pub paper: Paper,
    /// 0.0 – 1.0
    pub relevance_score: f32,
    /// One-line reason from the filter model
    pub relevance_reason: String,
    pub summary: String,
    pub key_contributions: Vec<String>,
    /// Proposed method, model, or study scenario
    pub methodology: String,
    /// Experimental setup, datasets, metrics, and results
    pub experiments: String,
    /// Insights, open questions, and implications for the researcher
    pub insights: String,
    /// Why this paper was selected — how it advances the researcher's interests
    pub selection_reason: String,
    /// Whether the full PDF text was included in the analysis prompt
    pub deep_analyzed: bool,
}
