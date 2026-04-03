use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::ProviderKind;

pub type AnswerPayload = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub question: String,
    pub answer: AnswerPayload,
    pub providers: Vec<ProviderKind>,
    pub timestamp: String,
}

pub fn render_answer(answer: &AnswerPayload) -> Result<String> {
    Ok(serde_json::to_string_pretty(answer)?)
}

pub fn load_history(path: &Path) -> Result<HistoryEntry> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read history file: {}", path.display()))?;
    let entry = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse history file: {}", path.display()))?;
    Ok(entry)
}

pub fn save_history(path: &Path, entry: &HistoryEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create history directory: {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(entry)?;
    fs::write(path, body)
        .with_context(|| format!("failed to write history file: {}", path.display()))
}
