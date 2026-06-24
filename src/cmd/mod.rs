pub mod cli;
pub mod commands;
pub mod usage;

use crate::cmd::cli::{Cli, Command};
use clap::Parser;

pub fn run() -> std::io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { docs, index } => commands::run_build(&docs, &index),
        Command::Update { index, docs } => commands::run_update(&index, &docs),
        Command::Search {
            index,
            query,
            limit,
            mode,
        } => commands::run_search(&index, &query, limit, mode.into()),
        Command::BuildSegment { docs, index_dir } => commands::run_build_segment(&docs, &index_dir),
        Command::UpdateSegment { index_dir, docs } => {
            commands::run_update_segment(&index_dir, &docs)
        }
        Command::SearchSegments {
            index_dir,
            query,
            limit,
            mode,
        } => commands::run_search_segments(&index_dir, &query, limit, mode.into()),
    }
}
