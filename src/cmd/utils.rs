use crate::query::QueryMode;
use crate::query::{SearchResult, TopKCollector};
use crate::segment::reader::SegmentReaderCache;
use crate::segment::search::SegmentSearcher;

use std::io;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReplState {
    pub(crate) limit: usize,
    pub(crate) mode: QueryMode,
}

impl Default for ReplState {
    fn default() -> Self {
        Self {
            limit: 10,
            mode: QueryMode::All,
        }
    }
}

pub(crate) enum ReplCommandResult {
    Continue,
    Exit,
    Search,
}

pub(crate) fn handle_repl_command(
    input: &str,
    cache: &SegmentReaderCache,
    state: &mut ReplState,
) -> ReplCommandResult {
    match input {
        ":q" | ":quit" => return ReplCommandResult::Exit,
        ":help" => {
            print_repl_help();
            return ReplCommandResult::Continue;
        }
        ":stats" => {
            print_repl_stats(cache, state.mode, state.limit);
            return ReplCommandResult::Continue;
        }
        ":mode" => {
            eprintln!("mode={}", state.mode.as_str());
            return ReplCommandResult::Continue;
        }
        ":limit" => {
            eprintln!("limit={}", state.limit);
            return ReplCommandResult::Continue;
        }
        _ => {}
    }

    if let Some(value) = input.strip_prefix(":limit ") {
        set_repl_limit(value, state);
        return ReplCommandResult::Continue;
    }

    if let Some(value) = input.strip_prefix(":mode ") {
        set_repl_mode(value, state);
        return ReplCommandResult::Continue;
    }

    ReplCommandResult::Search
}

fn set_repl_limit(value: &str, state: &mut ReplState) {
    match value.parse::<usize>() {
        Ok(n) if n > 0 => {
            state.limit = n;
            eprintln!("limit={}", state.limit);
        }
        _ => {
            eprintln!("invalid limit: {value}");
            eprintln!("limit={}", state.limit);
        }
    }
}

fn set_repl_mode(value: &str, state: &mut ReplState) {
    match value {
        "and" | "all" => {
            state.mode = QueryMode::All;
            eprintln!("mode=and");
        }
        "or" | "any" => {
            state.mode = QueryMode::Any;
            eprintln!("mode=or");
        }
        "phrase" => {
            state.mode = QueryMode::Phrase;
            eprintln!("mode=phrase");
        }
        _ => {
            eprintln!("invalid mode: {value}");
            eprintln!("mode={}", state.mode.as_str());
        }
    }
}

pub(crate) fn search_with_cache(
    cache: &SegmentReaderCache,
    query: &str,
    mode: QueryMode,
    limit: usize,
) -> io::Result<Vec<SearchResult>> {
    let terms: Vec<String> = crate::index::parser::tokenize(query)
        .into_iter()
        .map(|(term, _)| term)
        .collect();

    let mut collector = TopKCollector::new(limit);

    for reader in cache.readers() {
        let searcher = SegmentSearcher::new(reader);

        let results = match mode {
            QueryMode::All => searcher.search_all_topk(&terms, limit)?,
            QueryMode::Any => searcher.search_any_topk(&terms, limit)?,
            QueryMode::Phrase => searcher.search_phrase_topk(&terms, limit)?,
        };

        for result in results {
            collector.collect(result);
        }
    }

    Ok(collector.into_sorted_vec())
}

pub(crate) fn print_results(results: Vec<SearchResult>, limit: usize) {
    for result in results.into_iter().take(limit) {
        println!("{} score={}", result.path, result.score);
    }
}

pub(crate) fn print_repl_help() {
    eprintln!("commands:");
    eprintln!("  :help             show this help");
    eprintln!("  :limit <n>        set result limit");
    eprintln!("  :mode and         use AND search");
    eprintln!("  :mode or          use OR search");
    eprintln!("  :mode phrase      use phrase search");
    eprintln!("  :stats            show index and REPL stats");
    eprintln!("  :q, :quit         exit");
    eprintln!();
    eprintln!("queries:");
    eprintln!("  rust memory       search with current mode");
    eprintln!("  \"white whale\"     force phrase search");
}

pub(crate) fn print_repl_stats(cache: &SegmentReaderCache, mode: QueryMode, limit: usize) {
    let mut total_docs = 0usize;
    let mut total_terms = 0usize;
    let mut total_postings = 0usize;
    let mut total_positions = 0usize;

    for reader in cache.readers() {
        total_docs += reader.doc_count();
        total_terms += reader.term_count();
        total_postings += reader.posting_count();
        total_positions += reader.position_count();
    }

    let avg_doc_len = if total_docs == 0 {
        0.0
    } else {
        total_positions as f64 / total_docs as f64
    };

    eprintln!("segments={}", cache.readers().len());
    eprintln!("docs={total_docs}");
    eprintln!("terms={total_terms}");
    eprintln!("postings={total_postings}");
    eprintln!("positions={total_positions}");
    eprintln!("avg_doc_len={avg_doc_len:.2}");
    eprintln!("mode={}", mode.as_str());
    eprintln!("limit={limit}");
}
