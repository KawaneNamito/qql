use anyhow::Result;
use clap::Parser;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use qql::app::{Clock, run};
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;
    let mut init_ui = DialoguerInitUi;
    let output = run(
        cli,
        &paths,
        &RealProviderFactory,
        &SystemClock,
        &mut init_ui,
        &RealModelCatalog,
    )?;
    println!("{output}");
    Ok(())
}
