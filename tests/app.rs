use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use qql::app::{Clock, run};
use qql::cli::{Cli, Command};
use qql::config::{AppPaths, Config, ProviderKind, ResolvedProviderConfig};
use qql::history::{AnswerPayload, HistoryEntry};
use qql::provider::{Provider, ProviderFactory};
use tempfile::tempdir;

#[derive(Default)]
struct MockFactory {
    answers: Mutex<HashMap<ProviderKind, String>>,
    build_log: Mutex<Vec<(ProviderKind, ResolvedProviderConfig)>>,
    ask_count: AtomicUsize,
}

impl MockFactory {
    fn with_answer(self, kind: ProviderKind, answer: &str) -> Self {
        self.answers.lock().unwrap().insert(kind, answer.to_owned());
        self
    }

    fn ask_count(&self) -> usize {
        self.ask_count.load(Ordering::SeqCst)
    }

    fn build_log(&self) -> Vec<(ProviderKind, ResolvedProviderConfig)> {
        self.build_log.lock().unwrap().clone()
    }
}

struct MockProvider {
    answer: String,
    ask_count: Arc<AtomicUsize>,
}

impl Provider for MockProvider {
    fn ask(&self, _question: &str) -> Result<String> {
        self.ask_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.answer.clone())
    }
}

impl ProviderFactory for MockFactory {
    fn build(
        &self,
        kind: ProviderKind,
        config: &ResolvedProviderConfig,
    ) -> Result<Arc<dyn Provider>> {
        self.build_log.lock().unwrap().push((kind, config.clone()));
        let answer = self
            .answers
            .lock()
            .unwrap()
            .get(&kind)
            .cloned()
            .ok_or_else(|| anyhow!("missing mock answer"))?;
        Ok(Arc::new(MockProvider {
            answer,
            ask_count: Arc::new(AtomicUsize::new(self.ask_count())),
        }))
    }
}

struct FixedClock;

impl Clock for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-04-03T12:00:00Z".to_owned()
    }
}

fn write_config(dir: &std::path::Path, body: &str) {
    fs::write(dir.join("config.json"), body).unwrap();
}

fn read_history(dir: &std::path::Path) -> HistoryEntry {
    serde_json::from_str(&fs::read_to_string(dir.join("history.json")).unwrap()).unwrap()
}

#[test]
fn uses_default_provider_and_persists_history() {
    let dir = tempdir().unwrap();
    write_config(
        dir.path(),
        r#"{
          "default_providers": ["claude"],
          "providers": {
            "claude": { "api_key": "test-key" }
          }
        }"#,
    );

    let factory = MockFactory::default()
        .with_answer(ProviderKind::Claude, "LLM stands for large language model.");
    let output = run(
        Cli {
            command: None,
            question: Some("what is LLM?".to_owned()),
            providers: vec![],
            last: false,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap();

    assert_eq!(output, "LLM stands for large language model.");
    assert_eq!(
        factory.build_log(),
        vec![(
            ProviderKind::Claude,
            ResolvedProviderConfig {
                api_key: "test-key".to_owned(),
                model: "claude-haiku-4-5".to_owned(),
            }
        )]
    );

    let history = read_history(dir.path());
    assert_eq!(history.question, "what is LLM?");
    assert_eq!(
        history.answer,
        AnswerPayload::Single("LLM stands for large language model.".to_owned())
    );
    assert_eq!(history.providers, vec![ProviderKind::Claude]);
    assert_eq!(history.timestamp, "2026-04-03T12:00:00Z");
}

#[test]
fn emits_json_for_multiple_providers() {
    let dir = tempdir().unwrap();
    write_config(
        dir.path(),
        r#"{
          "default_providers": ["openai", "claude"],
          "providers": {
            "openai": { "api_key": "openai-key" },
            "claude": { "api_key": "claude-key", "model": "claude-sonnet-4-5" }
          }
        }"#,
    );

    let factory = MockFactory::default()
        .with_answer(ProviderKind::Openai, "LLM is a machine learning model.")
        .with_answer(ProviderKind::Claude, "LLM stands for large language model.");
    let output = run(
        Cli {
            command: None,
            question: Some("what is LLM?".to_owned()),
            providers: vec![],
            last: false,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap();

    let parsed: BTreeMap<String, String> = serde_json::from_str(&output).unwrap();
    assert_eq!(
        parsed,
        BTreeMap::from([
            (
                "claude".to_owned(),
                "LLM stands for large language model.".to_owned()
            ),
            (
                "openai".to_owned(),
                "LLM is a machine learning model.".to_owned()
            ),
        ])
    );

    let history = read_history(dir.path());
    assert_eq!(history.answer, AnswerPayload::Multiple(parsed));
}

#[test]
fn provider_flag_overrides_default_providers() {
    let dir = tempdir().unwrap();
    write_config(
        dir.path(),
        r#"{
          "default_providers": ["openai", "claude"],
          "providers": {
            "openai": { "api_key": "openai-key" },
            "gemini": { "api_key": "gemini-key" }
          }
        }"#,
    );

    let factory = MockFactory::default().with_answer(ProviderKind::Gemini, "Gemini answer");
    let output = run(
        Cli {
            command: None,
            question: Some("what is LLM?".to_owned()),
            providers: vec![ProviderKind::Gemini],
            last: false,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap();

    assert_eq!(output, "Gemini answer");
    assert_eq!(factory.build_log().len(), 1);
    assert_eq!(factory.build_log()[0].0, ProviderKind::Gemini);
}

#[test]
fn last_reads_history_without_calling_provider() {
    let dir = tempdir().unwrap();
    write_config(
        dir.path(),
        r#"{
          "default_providers": ["openai"],
          "providers": {
            "openai": { "api_key": "openai-key" }
          }
        }"#,
    );
    fs::write(
        dir.path().join("history.json"),
        r#"{
          "question": "what is LLM?",
          "answer": {
            "openai": "LLM is ..."
          },
          "providers": ["openai"],
          "timestamp": "2026-04-03T12:00:00Z"
        }"#,
    )
    .unwrap();

    let factory = MockFactory::default();
    let output = run(
        Cli {
            command: None,
            question: None,
            providers: vec![],
            last: true,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap();

    let parsed: BTreeMap<String, String> = serde_json::from_str(&output).unwrap();
    assert_eq!(
        parsed,
        BTreeMap::from([("openai".to_owned(), "LLM is ...".to_owned())])
    );
    assert_eq!(factory.build_log().len(), 0);
}

#[test]
fn init_creates_config_template_without_calling_provider() {
    let dir = tempdir().unwrap();
    let factory = MockFactory::default();

    let output = run(
        Cli {
            question: None,
            providers: vec![],
            last: false,
            command: Some(Command::Init),
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap();

    assert!(output.contains("Created config file"));
    assert!(dir.path().join("config.json").exists());
    assert_eq!(factory.build_log().len(), 0);

    let config: Config =
        serde_json::from_str(&fs::read_to_string(dir.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(config.default_providers, vec![ProviderKind::Openai]);
    assert!(config.providers.contains_key(&ProviderKind::Openai));
    assert!(config.providers.contains_key(&ProviderKind::Claude));
    assert!(config.providers.contains_key(&ProviderKind::Gemini));
}

#[test]
fn init_fails_when_config_already_exists() {
    let dir = tempdir().unwrap();
    write_config(
        dir.path(),
        r#"{
          "default_providers": ["openai"],
          "providers": {
            "openai": { "api_key": "openai-key" }
          }
        }"#,
    );
    let factory = MockFactory::default();

    let error = run(
        Cli {
            question: None,
            providers: vec![],
            last: false,
            command: Some(Command::Init),
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
    )
    .unwrap_err();

    assert!(error.to_string().contains("already exists"));
    assert_eq!(factory.build_log().len(), 0);
}
