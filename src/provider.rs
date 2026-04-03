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

pub fn ask_providers(
    question: &str,
    providers: Vec<(ProviderKind, Arc<dyn Provider>)>,
) -> Result<AnswerPayload> {
    if providers.is_empty() {
        return Err(anyhow!("no providers selected"));
    }

    if providers.len() == 1 {
        let (kind, provider) = providers.into_iter().next().expect("checked len");
        return Ok(BTreeMap::from([(
            kind.as_str().to_owned(),
            provider.ask(question)?,
        )]));
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
    for (kind, handle) in handles {
        let answer = handle
            .join()
            .map_err(|_| anyhow!("provider `{}` thread panicked", kind.as_str()))??;
        answers.insert(kind.as_str().to_owned(), answer);
    }

    Ok(answers)
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
