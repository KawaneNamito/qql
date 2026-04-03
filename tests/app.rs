use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use qql::app::{Clock, run};
use qql::cli::{Cli, Command};
use qql::config::{AppPaths, Config, ProviderKind, ResolvedProviderConfig};
use qql::history::{AnswerPayload, HistoryEntry};
use qql::init::{InitUi, ModelCatalog, ModelSelection};
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

struct NoopInitUi;

impl InitUi for NoopInitUi {
    fn confirm_overwrite(&mut self, _path: &std::path::Path) -> Result<bool> {
        unreachable!("init UI should not be used in this test")
    }

    fn select_providers(&mut self, _available: &[ProviderKind]) -> Result<Vec<ProviderKind>> {
        unreachable!("init UI should not be used in this test")
    }

    fn input_api_key(&mut self, _provider: ProviderKind) -> Result<String> {
        unreachable!("init UI should not be used in this test")
    }

    fn select_model(
        &mut self,
        _provider: ProviderKind,
        _available: &[String],
    ) -> Result<ModelSelection> {
        unreachable!("init UI should not be used in this test")
    }

    fn input_custom_model(&mut self, _provider: ProviderKind) -> Result<String> {
        unreachable!("init UI should not be used in this test")
    }
}

struct NoopModelCatalog;

impl ModelCatalog for NoopModelCatalog {
    fn list_models(&self, _provider: ProviderKind, _api_key: &str) -> Result<Vec<String>> {
        unreachable!("model catalog should not be used in this test")
    }
}

struct MockInitUi {
    overwrite_confirmation: Option<bool>,
    providers: Vec<ProviderKind>,
    api_keys: HashMap<ProviderKind, String>,
    models: HashMap<ProviderKind, ModelSelection>,
    custom_models: HashMap<ProviderKind, String>,
    offered_models: HashMap<ProviderKind, Vec<String>>,
}

impl MockInitUi {
    fn new(providers: Vec<ProviderKind>) -> Self {
        Self {
            overwrite_confirmation: None,
            providers,
            api_keys: HashMap::new(),
            models: HashMap::new(),
            custom_models: HashMap::new(),
            offered_models: HashMap::new(),
        }
    }

    fn with_api_key(mut self, provider: ProviderKind, api_key: &str) -> Self {
        self.api_keys.insert(provider, api_key.to_owned());
        self
    }

    fn with_model(mut self, provider: ProviderKind, model: ModelSelection) -> Self {
        self.models.insert(provider, model);
        self
    }

    fn with_custom_model(mut self, provider: ProviderKind, model: &str) -> Self {
        self.custom_models.insert(provider, model.to_owned());
        self
    }

    fn with_overwrite_confirmation(mut self, confirmed: bool) -> Self {
        self.overwrite_confirmation = Some(confirmed);
        self
    }

    fn offered_models(&self, provider: ProviderKind) -> Option<&Vec<String>> {
        self.offered_models.get(&provider)
    }
}

impl InitUi for MockInitUi {
    fn confirm_overwrite(&mut self, _path: &std::path::Path) -> Result<bool> {
        Ok(self.overwrite_confirmation.unwrap_or(false))
    }

    fn select_providers(&mut self, _available: &[ProviderKind]) -> Result<Vec<ProviderKind>> {
        Ok(self.providers.clone())
    }

    fn input_api_key(&mut self, provider: ProviderKind) -> Result<String> {
        self.api_keys
            .get(&provider)
            .cloned()
            .ok_or_else(|| anyhow!("missing mock api key"))
    }

    fn select_model(
        &mut self,
        provider: ProviderKind,
        available: &[String],
    ) -> Result<ModelSelection> {
        self.offered_models.insert(provider, available.to_vec());
        self.models
            .get(&provider)
            .cloned()
            .ok_or_else(|| anyhow!("missing mock model"))
    }

    fn input_custom_model(&mut self, provider: ProviderKind) -> Result<String> {
        self.custom_models
            .get(&provider)
            .cloned()
            .ok_or_else(|| anyhow!("missing mock custom model"))
    }
}

#[derive(Default)]
struct MockModelCatalog {
    models: HashMap<ProviderKind, Result<Vec<String>, String>>,
    calls: Mutex<Vec<(ProviderKind, String)>>,
}

impl MockModelCatalog {
    fn with_models(mut self, provider: ProviderKind, models: &[&str]) -> Self {
        self.models.insert(
            provider,
            Ok(models.iter().map(|model| (*model).to_owned()).collect()),
        );
        self
    }

    fn with_error(mut self, provider: ProviderKind, error: &str) -> Self {
        self.models.insert(provider, Err(error.to_owned()));
        self
    }

    fn calls(&self) -> Vec<(ProviderKind, String)> {
        self.calls.lock().unwrap().clone()
    }
}

impl ModelCatalog for MockModelCatalog {
    fn list_models(&self, provider: ProviderKind, api_key: &str) -> Result<Vec<String>> {
        self.calls
            .lock()
            .unwrap()
            .push((provider, api_key.to_owned()));
        match self.models.get(&provider) {
            Some(Ok(models)) => Ok(models.clone()),
            Some(Err(error)) => Err(anyhow!(error.clone())),
            None => Err(anyhow!("missing mock catalog entry")),
        }
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
    let mut init_ui = NoopInitUi;
    let model_catalog = NoopModelCatalog;
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
        &mut init_ui,
        &model_catalog,
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
    let mut init_ui = NoopInitUi;
    let model_catalog = NoopModelCatalog;
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
        &mut init_ui,
        &model_catalog,
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
    let mut init_ui = NoopInitUi;
    let model_catalog = NoopModelCatalog;
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
        &mut init_ui,
        &model_catalog,
    )
    .unwrap();

    assert_eq!(output, "Gemini answer");
    assert_eq!(factory.build_log().len(), 1);
    assert_eq!(factory.build_log()[0].0, ProviderKind::Gemini);
}

#[test]
fn last_reads_history_without_calling_provider() {
    let dir = tempdir().unwrap();
    let mut init_ui = NoopInitUi;
    let model_catalog = NoopModelCatalog;
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
        &mut init_ui,
        &model_catalog,
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
fn init_interactively_creates_config_for_selected_providers() {
    let dir = tempdir().unwrap();
    let factory = MockFactory::default();
    let model_catalog = MockModelCatalog::default()
        .with_models(
            ProviderKind::Claude,
            &["claude-opus-4-1-20250805", "claude-sonnet-4-20250514"],
        )
        .with_models(
            ProviderKind::Gemini,
            &["gemini-2.5-pro", "gemini-2.5-flash"],
        );
    let mut init_ui = MockInitUi::new(vec![ProviderKind::Claude, ProviderKind::Gemini])
        .with_api_key(ProviderKind::Claude, "sk-ant-test")
        .with_api_key(ProviderKind::Gemini, "AIza-test")
        .with_model(
            ProviderKind::Claude,
            ModelSelection::Preset("claude-opus-4-1-20250805".to_owned()),
        )
        .with_model(
            ProviderKind::Gemini,
            ModelSelection::Preset("gemini-2.5-pro".to_owned()),
        );

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
        &mut init_ui,
        &model_catalog,
    )
    .unwrap();

    assert!(output.contains("Created config file"));
    assert!(dir.path().join("config.json").exists());
    assert_eq!(factory.build_log().len(), 0);

    let config: Config =
        serde_json::from_str(&fs::read_to_string(dir.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(
        config.default_providers,
        vec![ProviderKind::Claude, ProviderKind::Gemini]
    );
    assert_eq!(config.providers.len(), 2);
    assert_eq!(
        config.providers.get(&ProviderKind::Claude).unwrap().api_key,
        "sk-ant-test"
    );
    assert_eq!(
        config
            .providers
            .get(&ProviderKind::Claude)
            .unwrap()
            .model
            .as_deref(),
        Some("claude-opus-4-1-20250805")
    );
    assert_eq!(
        config.providers.get(&ProviderKind::Gemini).unwrap().api_key,
        "AIza-test"
    );
    assert_eq!(
        config
            .providers
            .get(&ProviderKind::Gemini)
            .unwrap()
            .model
            .as_deref(),
        Some("gemini-2.5-pro")
    );
    assert_eq!(
        init_ui.offered_models(ProviderKind::Claude),
        Some(&vec![
            "claude-opus-4-1-20250805".to_owned(),
            "claude-sonnet-4-20250514".to_owned(),
        ])
    );
    assert_eq!(
        init_ui.offered_models(ProviderKind::Gemini),
        Some(&vec![
            "gemini-2.5-pro".to_owned(),
            "gemini-2.5-flash".to_owned()
        ])
    );
    assert_eq!(
        model_catalog.calls(),
        vec![
            (ProviderKind::Claude, "sk-ant-test".to_owned()),
            (ProviderKind::Gemini, "AIza-test".to_owned()),
        ]
    );
}

#[test]
fn init_overwrites_existing_config_when_confirmed() {
    let dir = tempdir().unwrap();
    let model_catalog =
        MockModelCatalog::default().with_models(ProviderKind::Openai, &["gpt-5-mini"]);
    let mut init_ui = MockInitUi::new(vec![ProviderKind::Openai])
        .with_overwrite_confirmation(true)
        .with_api_key(ProviderKind::Openai, "sk-new")
        .with_model(
            ProviderKind::Openai,
            ModelSelection::Preset("gpt-5-mini".to_owned()),
        );
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
        &mut init_ui,
        &model_catalog,
    )
    .unwrap();

    assert!(output.contains("Created config file"));
    assert_eq!(factory.build_log().len(), 0);

    let config: Config =
        serde_json::from_str(&fs::read_to_string(dir.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(
        config.providers.get(&ProviderKind::Openai).unwrap().api_key,
        "sk-new"
    );
}

#[test]
fn init_aborts_when_overwrite_is_rejected() {
    let dir = tempdir().unwrap();
    let model_catalog = NoopModelCatalog;
    let mut init_ui = MockInitUi::new(vec![]).with_overwrite_confirmation(false);
    let original = r#"{
      "default_providers": ["openai"],
      "providers": {
        "openai": { "api_key": "openai-key" }
      }
    }"#;
    write_config(dir.path(), original);
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
        &mut init_ui,
        &model_catalog,
    )
    .unwrap_err();

    assert!(error.to_string().contains("aborted"));
    assert_eq!(factory.build_log().len(), 0);
    assert_eq!(
        fs::read_to_string(dir.path().join("config.json")).unwrap(),
        original
    );
}

#[test]
fn init_accepts_custom_model_selection() {
    let dir = tempdir().unwrap();
    let factory = MockFactory::default();
    let model_catalog =
        MockModelCatalog::default().with_models(ProviderKind::Openai, &["gpt-5.2", "gpt-5-mini"]);
    let mut init_ui = MockInitUi::new(vec![ProviderKind::Openai])
        .with_api_key(ProviderKind::Openai, "sk-test")
        .with_model(ProviderKind::Openai, ModelSelection::Custom)
        .with_custom_model(ProviderKind::Openai, "gpt-5.2-codex");

    run(
        Cli {
            command: Some(Command::Init),
            question: None,
            providers: vec![],
            last: false,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
        &mut init_ui,
        &model_catalog,
    )
    .unwrap();

    let config: Config =
        serde_json::from_str(&fs::read_to_string(dir.path().join("config.json")).unwrap()).unwrap();
    assert_eq!(config.default_providers, vec![ProviderKind::Openai]);
    assert_eq!(
        config
            .providers
            .get(&ProviderKind::Openai)
            .unwrap()
            .model
            .as_deref(),
        Some("gpt-5.2-codex")
    );
}

#[test]
fn init_falls_back_to_static_models_when_fetch_fails() {
    let dir = tempdir().unwrap();
    let factory = MockFactory::default();
    let model_catalog =
        MockModelCatalog::default().with_error(ProviderKind::Openai, "unauthorized");
    let mut init_ui = MockInitUi::new(vec![ProviderKind::Openai])
        .with_api_key(ProviderKind::Openai, "sk-test")
        .with_model(
            ProviderKind::Openai,
            ModelSelection::Preset("gpt-5-mini".to_owned()),
        );

    run(
        Cli {
            command: Some(Command::Init),
            question: None,
            providers: vec![],
            last: false,
        },
        &AppPaths::from_base_dir(dir.path()),
        &factory,
        &FixedClock,
        &mut init_ui,
        &model_catalog,
    )
    .unwrap();

    let fallback_models = ProviderKind::Openai
        .init_models()
        .iter()
        .map(|model| (*model).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        init_ui.offered_models(ProviderKind::Openai),
        Some(&fallback_models)
    );
}
