use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub interests: InterestsConfig,
    pub sources: SourcesConfig,
    pub llm: LlmConfig,
    pub output: OutputConfig,
    pub email: Option<EmailConfig>,
    #[serde(default)]
    pub schedule: ScheduleConfig,
    #[serde(default)]
    pub retry: RetryConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InterestsConfig {
    pub topics: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default = "default_threshold")]
    pub relevance_threshold: f32,
}

fn default_threshold() -> f32 {
    0.6
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourcesConfig {
    pub arxiv: ArxivConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArxivConfig {
    pub categories: Vec<String>,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_days_back")]
    pub days_back: u32,
}

fn default_max_results() -> usize {
    200
}
fn default_days_back() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// LLM config — three specialised model slots, each with its own provider
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// Cheap model for batch relevance scoring
    pub filter: ModelSlotConfig,
    /// Powerful model for per-paper deep analysis
    pub analysis: ModelSlotConfig,

    /// Download each paper's PDF and feed the first `pdf_chars` characters to the
    /// analysis model for richer analysis. Default: false.
    #[serde(default)]
    pub deep: bool,
    /// How many characters of extracted PDF text to include in the analysis prompt.
    /// Only used when `deep = true`. Default: 20000.
    #[serde(default = "default_pdf_chars")]
    pub pdf_chars: usize,
}

fn default_pdf_chars() -> usize {
    20_000
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelSlotConfig {
    /// "openai" | "anthropic" | "custom"
    pub provider: String,
    pub api_key: String,
    /// Required for "custom"; optional base-URL override for "openai"
    pub base_url: Option<String>,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_tokens() -> u32 {
    2048
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct OutputConfig {
    pub output_dir: String,
}

// ---------------------------------------------------------------------------
// Email
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password_env: String,
    pub from: String,
    pub to: Vec<String>,
}

// ---------------------------------------------------------------------------
// Schedule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleConfig {
    /// HH:MM in UTC
    #[serde(default = "default_time")]
    pub time: String,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            time: default_time(),
        }
    }
}

fn default_time() -> String {
    "08:00".to_string()
}

// ---------------------------------------------------------------------------
// Retry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of attempts for external calls (arXiv, LLM APIs).
    /// Must be ≥ 1. Default: 3.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self { max_attempts: default_max_attempts() }
    }
}

fn default_max_attempts() -> u32 {
    3
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            anyhow::anyhow!(
                "Cannot read config file '{}': {}",
                path.as_ref().display(),
                e
            )
        })?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
