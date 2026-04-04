use anyhow::Result;
use clap::Parser;
use dialoguer::Editor;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use qql::app::{Clock, QuestionEditor, run};
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let mut init_ui = DialoguerInitUi;
    let question_editor = DialoguerQuestionEditor;
    let output = run(
        cli,
        &paths,
        &RealProviderFactory,
        &SystemClock,
        &question_editor,
        &mut init_ui,
        &RealModelCatalog,
    )?;
    println!("{output}");
    Ok(())
}
