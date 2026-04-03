use anyhow::{Result, anyhow};

use crate::cli::{Cli, Command};
use crate::config::AppPaths;
use crate::history::{HistoryEntry, load_history, render_answer, save_history};
use crate::init::{InitUi, ModelCatalog, run_init};
use crate::provider::{ProviderFactory, ask_providers};

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub fn run(
    cli: Cli,
    paths: &AppPaths,
    factory: &dyn ProviderFactory,
    clock: &dyn Clock,
    init_ui: &mut dyn InitUi,
    model_catalog: &dyn ModelCatalog,
) -> Result<String> {
    if cli.command == Some(Command::Init) {
        return run_init(&paths.config_path, init_ui, model_catalog);
    }

    if cli.last {
        return render_answer(&load_history(&paths.history_path)?.answer);
    }

    let question = cli
        .question
        .filter(|question| !question.trim().is_empty())
        .ok_or_else(|| anyhow!("question is required unless --last is used"))?;
    let config = crate::config::Config::load_from_path(&paths.config_path)?;
    let providers_to_use = config.providers_to_use(&cli.providers)?;

    let mut providers = Vec::new();
    for kind in &providers_to_use {
        let provider_config = config.resolved_provider_config(*kind)?;
        providers.push((*kind, factory.build(*kind, &provider_config)?));
    }

    let answer = ask_providers(&question, providers)?;
    save_history(
        &paths.history_path,
        &HistoryEntry {
            question,
            answer: answer.clone(),
            providers: providers_to_use,
            timestamp: clock.now_rfc3339(),
        },
    )?;

    render_answer(&answer)
}
