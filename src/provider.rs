use std::collections::BTreeMap;
use std::sync::Arc;
use std::thread;

use anyhow::{Result, anyhow};

use crate::claude::ClaudeProvider;
use crate::config::{ProviderKind, ResolvedProviderConfig};
use crate::gemini::GeminiProvider;
use crate::history::AnswerPayload;
use crate::openai::OpenAiProvider;

pub trait Provider: Send + Sync {
    fn ask(&self, question: &str) -> Result<String>;
}

pub trait ProviderFactory {
    fn build(
        &self,
        kind: ProviderKind,
        config: &ResolvedProviderConfig,
    ) -> Result<Arc<dyn Provider>>;
}

pub struct AskResult {
    pub answers: AnswerPayload,
    pub errors: Vec<(String, anyhow::Error)>,
}

pub fn ask_providers(
    question: &str,
    providers: Vec<(ProviderKind, Arc<dyn Provider>)>,
) -> Result<AskResult> {
    if providers.is_empty() {
        return Err(anyhow!("no providers selected"));
    }

    let mut handles = Vec::new();
    for (kind, provider) in providers {
        let question = question.to_owned();
        handles.push((
            kind,
            thread::spawn(move || -> Result<String> { provider.ask(&question) }),
        ));
    }

    let mut answers = BTreeMap::new();
    let mut errors = Vec::new();
    for (kind, handle) in handles {
        match handle
            .join()
            .map_err(|_| anyhow!("provider `{}` thread panicked", kind.as_str()))
            .and_then(|r| r)
        {
            Ok(answer) => {
                answers.insert(kind.as_str().to_owned(), answer);
            }
            Err(e) => {
                errors.push((kind.as_str().to_owned(), e));
            }
        }
    }

    Ok(AskResult { answers, errors })
}

pub struct RealProviderFactory;

impl ProviderFactory for RealProviderFactory {
    fn build(
        &self,
        kind: ProviderKind,
        config: &ResolvedProviderConfig,
    ) -> Result<Arc<dyn Provider>> {
        let provider: Arc<dyn Provider> = match kind {
            ProviderKind::Openai => Arc::new(OpenAiProvider::new(config.clone())),
            ProviderKind::Claude => Arc::new(ClaudeProvider::new(config.clone())),
            ProviderKind::Gemini => Arc::new(GeminiProvider::new(config.clone())),
        };
        Ok(provider)
    }
}
