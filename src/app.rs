use anyhow::{Result, anyhow};

use crate::cli::{Cli, Command};
use crate::config::AppPaths;
use crate::config::Config;
use crate::history::{HistoryEntry, load_history, save_history};
use crate::provider::{ProviderFactory, ask_providers};

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub fn run(
    cli: Cli,
    paths: &AppPaths,
    factory: &dyn ProviderFactory,
    clock: &dyn Clock,
) -> Result<String> {
    if cli.command == Some(Command::Init) {
        Config::init_file(&paths.config_path)?;
        return Ok(format!(
            "Created config file: {}",
            paths.config_path.display()
        ));
    }

    if cli.last {
        return load_history(&paths.history_path)?.answer.render();
    }

    let question = cli
        .question
        .filter(|question| !question.trim().is_empty())
        .ok_or_else(|| anyhow!("question is required unless --last is used"))?;
    let config = Config::load_from_path(&paths.config_path)?;
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

    answer.render()
}
