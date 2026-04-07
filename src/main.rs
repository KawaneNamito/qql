use std::io::Read;

use anyhow::Result;
use clap::Parser;
use dialoguer::Editor;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use qql::app::{AppDeps, Clock, QuestionEditor, QuestionStdin, run};
use qql::cli::Cli;
use qql::config::AppPaths;
use qql::init::{DialoguerInitUi, RealModelCatalog};
use qql::provider::RealProviderFactory;

struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
    }
}

struct DialoguerQuestionEditor;

impl QuestionEditor for DialoguerQuestionEditor {
    fn edit(&self, initial: &str) -> Result<Option<String>> {
        let mut editor = Editor::new();
        editor.extension(".md");
        editor.edit(initial).map_err(Into::into)
    }
}

struct RealQuestionStdin;

impl QuestionStdin for RealQuestionStdin {
    fn read_to_string(&self) -> Result<String> {
        let mut question = String::new();
        std::io::stdin().read_to_string(&mut question)?;
        Ok(question)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let mut init_ui = DialoguerInitUi;
    let question_editor = DialoguerQuestionEditor;
    let question_stdin = RealQuestionStdin;
    let output = run(
        cli,
        &paths,
        AppDeps {
            factory: &RealProviderFactory,
            clock: &SystemClock,
            editor: &question_editor,
            stdin: &question_stdin,
            init_ui: &mut init_ui,
            model_catalog: &RealModelCatalog,
        },
    )?;
    println!("{output}");
    Ok(())
}
