use anyhow::{Result, anyhow};

use crate::cli::{Cli, Command};
use crate::config::AppPaths;
use crate::history::{HistoryEntry, load_history, render_answer, save_history};
use crate::init::{InitUi, ModelCatalog, run_init};
use crate::provider::{ProviderFactory, ask_providers};

pub struct AppDeps<'a> {
    pub factory: &'a dyn ProviderFactory,
    pub clock: &'a dyn Clock,
    pub editor: &'a dyn QuestionEditor,
    pub stdin: &'a dyn QuestionStdin,
    pub init_ui: &'a mut dyn InitUi,
    pub model_catalog: &'a dyn ModelCatalog,
}

pub trait Clock {
    fn now_rfc3339(&self) -> String;
}

pub trait QuestionEditor {
    fn edit(&self, initial: &str) -> Result<Option<String>>;
}

pub trait QuestionStdin {
    fn read_to_string(&self) -> Result<String>;
}

fn format_provider_failure(name: &str, err: &anyhow::Error) -> String {
    format!("{name} failed: {err}")
}

fn all_providers_failed_error(errors: &[(String, anyhow::Error)]) -> anyhow::Error {
    let details = errors
        .iter()
        .map(|(name, err)| format_provider_failure(name, err))
        .collect::<Vec<_>>()
        .join("; ");
    anyhow!("all providers failed: {details}")
}

fn normalize_question(text: String) -> String {
    text.trim_end_matches(&['\n', '\r'][..]).to_owned()
}

fn read_question_from_stdin(stdin: &dyn QuestionStdin) -> Result<String> {
    let question = normalize_question(stdin.read_to_string()?);
    if question.trim().is_empty() {
        return Err(anyhow!("stdin did not contain a question"));
    }
    Ok(question)
}

fn resolve_question(
    cli: &Cli,
    editor: &dyn QuestionEditor,
    stdin: &dyn QuestionStdin,
) -> Result<String> {
    if cli.editor {
        let initial = cli.question.as_deref().unwrap_or_default();
        let question = editor
            .edit(initial)?
            .ok_or_else(|| anyhow!("editor did not return a question"))?;
        if question.trim().is_empty() {
            return Err(anyhow!("question cannot be empty"));
        }
        return Ok(question);
    }

    if cli.stdin || cli.question.as_deref() == Some("-") {
        return read_question_from_stdin(stdin);
    }

    cli.question
        .clone()
        .filter(|question| !question.trim().is_empty())
        .ok_or_else(|| anyhow!("question is required unless --last is used"))
}

pub fn run(cli: Cli, paths: &AppPaths, deps: AppDeps<'_>) -> Result<String> {
    let AppDeps {
        factory,
        clock,
        editor,
        stdin,
        init_ui,
        model_catalog,
    } = deps;
    if cli.command == Some(Command::Init) {
        return run_init(&paths.config_path, init_ui, model_catalog);
    }

    if cli.last {
        return render_answer(&load_history(&paths.history_path)?.answer);
    }

    let question = resolve_question(&cli, editor, stdin)?;
    let config = crate::config::Config::load_from_path(&paths.config_path)?;
    let providers_to_use = config.providers_to_use(&cli.providers)?;

    let mut providers = Vec::new();
    for kind in &providers_to_use {
        let provider_config = config.resolved_provider_config(*kind)?;
        providers.push((*kind, factory.build(*kind, &provider_config)?));
    }

    let result = ask_providers(&question, providers)?;
    for (name, err) in &result.errors {
        eprintln!("Error: {}", format_provider_failure(name, err));
    }
    if result.answers.is_empty() {
        return Err(all_providers_failed_error(&result.errors));
    }
    let succeeded_providers = providers_to_use
        .into_iter()
        .filter(|kind| result.answers.contains_key(kind.as_str()))
        .collect();
    save_history(
        &paths.history_path,
        &HistoryEntry {
            question,
            answer: result.answers.clone(),
            providers: succeeded_providers,
            timestamp: clock.now_rfc3339(),
        },
    )?;

    render_answer(&result.answers)
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::{all_providers_failed_error, format_provider_failure};

    #[test]
    fn formats_provider_failure_with_provider_name() {
        let error = anyhow!("rate limited");

        assert_eq!(
            format_provider_failure("claude", &error),
            "claude failed: rate limited"
        );
    }

    #[test]
    fn all_failed_error_includes_each_provider_failure() {
        let error = all_providers_failed_error(&[
            ("openai".to_owned(), anyhow!("timeout")),
            ("claude".to_owned(), anyhow!("529 overloaded")),
        ]);

        let message = error.to_string();
        assert!(message.contains("all providers failed"));
        assert!(message.contains("openai failed: timeout"));
        assert!(message.contains("claude failed: 529 overloaded"));
    }
}
