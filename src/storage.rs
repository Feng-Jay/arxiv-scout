use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct StorageData {
    seen: HashMap<String, DateTime<Utc>>,
}

pub struct Storage {
    path: PathBuf,
    data: StorageData,
}

impl Storage {
    /// Load from disk; creates an empty store if the file does not exist yet.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)?
        } else {
            StorageData::default()
        };
        Ok(Self { path, data })
    }

    pub fn is_seen(&self, paper_id: &str) -> bool {
        self.data.seen.contains_key(paper_id)
    }

    pub fn mark_seen(&mut self, paper_id: &str) {
        self.data.seen.insert(paper_id.to_string(), Utc::now());
    }

    /// Remove entries older than `days` to keep the file compact.
    pub fn cleanup_old(&mut self, days: u32) {
        let cutoff = Utc::now() - chrono::TimeDelta::days(days as i64);
        self.data.seen.retain(|_, ts| *ts > cutoff);
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}
