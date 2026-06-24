use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "searchfs")]
#[command(version, about = "A toy search engine written in Rust")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Build a snapshot index.
    Build { docs: String, index: String },

    /// Update a snapshot index.
    Update { index: String, docs: String },

    /// Search a snapshot index.
    Search {
        index: String,
        query: String,

        #[arg(default_value_t = 10)]
        limit: usize,

        #[arg(value_enum, default_value_t = CliQueryMode::And)]
        mode: CliQueryMode,
    },

    /// Start an interactive REPL over a segment index.
    Repl { index_dir: String },

    /// Build a segment index.
    BuildSegment { docs: String, index_dir: String },

    /// Update a segment index.
    UpdateSegment { index_dir: String, docs: String },

    /// Search a segment index.
    SearchSegments {
        index_dir: String,
        query: String,

        #[arg(default_value_t = 10)]
        limit: usize,

        #[arg(value_enum, default_value_t = CliQueryMode::And)]
        mode: CliQueryMode,
    },

    /// Merge all segments into one compacted segment.
    MergeSegments { index_dir: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliQueryMode {
    And,
    Or,
    Phrase,
}

impl From<CliQueryMode> for crate::query::QueryMode {
    fn from(mode: CliQueryMode) -> Self {
        match mode {
            CliQueryMode::And => Self::All,
            CliQueryMode::Or => Self::Any,
            CliQueryMode::Phrase => Self::Phrase,
        }
    }
}
