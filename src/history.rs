use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::ProviderKind;

pub type AnswerPayload = BTreeMap<String, String>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputPayload {
    pub prompt: String,
    #[serde(flatten)]
    pub answers: AnswerPayload,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub question: String,
    pub answer: AnswerPayload,
    pub providers: Vec<ProviderKind>,
    pub timestamp: String,
}

pub fn render_output(prompt: &str, answer: &AnswerPayload) -> Result<String> {
    Ok(serde_json::to_string_pretty(&OutputPayload {
        prompt: prompt.to_owned(),
        answers: answer.clone(),
    })?)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::render_output;

    #[test]
    fn render_output_places_prompt_before_provider_entries() {
        let output = render_output(
            "what is LLM?",
            &BTreeMap::from([
                ("openai".to_owned(), "LLM is ...".to_owned()),
                ("claude".to_owned(), "LLM stands for ...".to_owned()),
            ]),
        )
        .unwrap();

        assert_eq!(
            output,
            r#"{
  "prompt": "what is LLM?",
  "claude": "LLM stands for ...",
  "openai": "LLM is ..."
}"#
        );
    }
}
