use rayon::prelude::*;

use crate::query::QueryMode;
use crate::query::{SearchResult, TopKCollector};
use crate::segment::reader::{SegmentReader, SegmentReaderCache};
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

#[derive(Debug, Default)]
struct ReplStats {
    segments: usize,
    docs: usize,
    terms: usize,
    postings: usize,
    positions: usize,
}

impl ReplStats {
    fn from_cache(cache: &SegmentReaderCache) -> Self {
        let mut stats = Self {
            segments: cache.readers().len(),
            ..Self::default()
        };

        for reader in cache.readers() {
            stats.docs += reader.doc_count();
            stats.terms += reader.term_count();
            stats.postings += reader.posting_count();
            stats.positions += reader.position_count();
        }

        stats
    }

    fn avg_doc_len(&self) -> f64 {
        if self.docs == 0 {
            0.0
        } else {
            self.positions as f64 / self.docs as f64
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

    search_reader_cache(cache, &terms, mode, limit)
}

fn search_reader_cache(
    cache: &SegmentReaderCache,
    terms: &[String],
    mode: QueryMode,
    limit: usize,
) -> io::Result<Vec<SearchResult>> {
    let partial_results: Vec<io::Result<Vec<SearchResult>>> = cache
        .readers()
        .par_iter()
        .map(|reader| search_segment(reader, terms, mode, limit))
        .collect();

    let partial_results: Vec<Vec<SearchResult>> =
        partial_results.into_iter().collect::<io::Result<_>>()?;

    Ok(merge_topk(partial_results, limit))
}

fn search_segment(
    reader: &SegmentReader,
    terms: &[String],
    mode: QueryMode,
    limit: usize,
) -> io::Result<Vec<SearchResult>> {
    let searcher = SegmentSearcher::new(reader);

    match mode {
        QueryMode::All => searcher.search_all(terms, limit),
        QueryMode::Any => searcher.search_any(terms, limit),
        QueryMode::Phrase => searcher.search_phrase(terms, limit),
    }
}

fn merge_topk(partial_results: Vec<Vec<SearchResult>>, limit: usize) -> Vec<SearchResult> {
    let mut collector = TopKCollector::new(limit);

    for results in partial_results {
        collector.extend(results);
    }

    collector.into_sorted_vec()
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
    let stats = ReplStats::from_cache(cache);

    eprintln!("segments={}", stats.segments);
    eprintln!("docs={}", stats.docs);
    eprintln!("terms={}", stats.terms);
    eprintln!("postings={}", stats.postings);
    eprintln!("positions={}", stats.positions);
    eprintln!("avg_doc_len={:.2}", stats.avg_doc_len());
    eprintln!("mode={}", mode.as_str());
    eprintln!("limit={limit}");
}

#[cfg(test)]
mod tests {
    use crate::cmd::utils::search_reader_cache;
    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::query::QueryMode;
    use crate::segment::format::{MANIFEST_VERSION, Manifest, Segment};
    use crate::segment::store::SegmentStore;
    use tempfile::tempdir;

    #[test]
    fn search_reader_cache_merges_topk_across_segments() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable1 = DocTable::new();
        let doc1 = doctable1.add_document("a.txt".to_string());

        let mut index1 = InvertedIndex::new();
        index1.add_document_tokens(doc1, vec![("rust".to_string(), 0)]);

        let segment1 = Segment {
            id: "seg_000001".to_string(),
            doctable: doctable1,
            index: index1,
        };

        let mut doctable2 = DocTable::new();
        let doc2 = doctable2.add_document("b.txt".to_string());

        let mut index2 = InvertedIndex::new();
        index2.add_document_tokens(doc2, vec![("rust".to_string(), 0)]);

        let segment2 = Segment {
            id: "seg_000002".to_string(),
            doctable: doctable2,
            index: index2,
        };

        store.save_segment(&segment1).unwrap();
        store.save_segment(&segment2).unwrap();

        store
            .save_manifest(&Manifest {
                version: MANIFEST_VERSION,
                segments: vec!["seg_000001".to_string(), "seg_000002".to_string()],
            })
            .unwrap();

        let cache = store.open_reader_cache().unwrap();

        let terms = vec!["rust".to_string()];

        let results = search_reader_cache(&cache, &terms, QueryMode::Any, 2).unwrap();

        let mut paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();
        paths.sort();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn search_reader_cache_respects_global_limit() {
        use crate::index::doctable::DocTable;
        use crate::index::memindex::InvertedIndex;
        use crate::query::QueryMode;
        use crate::segment::format::{MANIFEST_VERSION, Manifest, Segment};
        use crate::segment::store::SegmentStore;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        for (id, path) in [("seg_000001", "a.txt"), ("seg_000002", "b.txt")] {
            let mut doctable = DocTable::new();
            let doc = doctable.add_document(path.to_string());

            let mut index = InvertedIndex::new();
            index.add_document_tokens(doc, vec![("rust".to_string(), 0)]);

            let segment = Segment {
                id: id.to_string(),
                doctable,
                index,
            };

            store.save_segment(&segment).unwrap();
        }

        store
            .save_manifest(&Manifest {
                version: MANIFEST_VERSION,
                segments: vec!["seg_000001".to_string(), "seg_000002".to_string()],
            })
            .unwrap();

        let cache = store.open_reader_cache().unwrap();

        let results =
            search_reader_cache(&cache, &["rust".to_string()], QueryMode::Any, 1).unwrap();

        assert_eq!(results.len(), 1);
    }
}
