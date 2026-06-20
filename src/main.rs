use std::env;
use std::path::Path;

use searchfs::engine::SearchEngine;
use searchfs::query::QueryMode;

fn main() -> std::io::Result<()> {
    let root = env::args().nth(1).unwrap_or_else(|| "./docs".to_string());
    let query = env::args().nth(2).unwrap_or_else(|| "rust".to_string());

    let limit = env::args()
        .nth(3)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let mode_arg = env::args().nth(4).unwrap_or_else(|| "and".to_string());

    let mode = if query.starts_with('"') && query.ends_with('"') {
        QueryMode::Phrase
    } else {
        QueryMode::try_from(mode_arg.as_str())
            .map_err(|msg| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))?
    };

    let query = query.trim_matches('"');

    let mut engine = SearchEngine::new();
    engine.index_dir(Path::new(&root))?;

    let results = engine.search(query, mode);

    for result in results.into_iter().take(limit) {
        println!("{} score={}", result.path, result.score);
    }

    Ok(())
}
