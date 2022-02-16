use clap::{AppSettings, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Hash, Debug, Eq, PartialEq, Subcommand, Serialize, Deserialize)]
pub enum Command {
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Set {
        key: String,
        value: String,
    },
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Get {
        key: String,
    },
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Rm {
        key: String,
    },
    Open {
        path: PathBuf,
    },
    Version,
}

#[derive(Debug)]
pub struct CommandPosition {
    pub start: u64,
    pub length: u64,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}
