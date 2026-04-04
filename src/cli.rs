use clap::{Parser, Subcommand};

use crate::config::ProviderKind;

#[derive(Debug, Parser)]
#[command(name = "qql", args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    pub question: Option<String>,

    #[arg(short, long = "provider")]
    pub providers: Vec<ProviderKind>,

    #[arg(short = 'e', long, conflicts_with = "last")]
    pub editor: bool,

    #[arg(long, conflicts_with = "editor")]
    pub last: bool,
}

#[derive(Debug, Clone, Subcommand, PartialEq, Eq)]
pub enum Command {
    Init,
}
