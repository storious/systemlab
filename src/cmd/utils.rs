use crate::query::QueryMode;
use crate::query::SearchResult;
use crate::segment::reader::SegmentReaderCache;
use crate::segment::search::SegmentSearcher;

use std::io;

pub(crate) fn search_with_cache(
    cache: &SegmentReaderCache,
    query: &str,
    mode: QueryMode,
) -> io::Result<Vec<SearchResult>> {
    let terms: Vec<String> = crate::index::parser::tokenize(query)
        .into_iter()
        .map(|(term, _)| term)
        .collect();

    let mut all_results = Vec::new();

    for reader in cache.readers() {
        let searcher = SegmentSearcher::new(reader);

        match mode {
            QueryMode::All => {
                all_results.extend(searcher.search_all(&terms)?);
            }
            QueryMode::Any => {
                all_results.extend(searcher.search_any(&terms)?);
            }
            QueryMode::Phrase => {
                all_results.extend(searcher.search_phrase(&terms)?);
            }
        }
    }

    all_results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(all_results)
}

pub(crate) fn print_results(results: Vec<SearchResult>, limit: usize) {
    for result in results.into_iter().take(limit) {
        println!("{} score={}", result.path, result.score);
    }
}
