use std::io;
use std::path::Path;
use std::time::Instant;

use crate::engine::SearchEngine;
use crate::query::QueryMode;
use crate::segment::format::{Manifest, next_segment_id};
use crate::segment::reader::SegmentReaderCache;
use crate::segment::search::SegmentSearcher;
use crate::segment::store::SegmentStore;
use crate::snapshot;

pub(crate) fn run_build(docs: &str, index_path: &str) -> io::Result<()> {
    let mut engine = SearchEngine::new();

    let start = Instant::now();
    engine.index_dir(Path::new(docs))?;
    let elapsed = start.elapsed();

    let stats = engine.stats();

    eprintln!(
        "indexed docs={} terms={} postings={} positions={} index_time={:.2?}",
        engine.doc_count(),
        stats.terms,
        stats.postings,
        stats.total_positions,
        elapsed,
    );

    let snapshot = engine.into_snapshot();
    snapshot::save(Path::new(index_path), &snapshot)?;

    eprintln!("saved index={index_path}");

    Ok(())
}

pub(crate) fn run_search(
    index_path: &str,
    query: &str,
    limit: usize,
    mode_arg: QueryMode,
) -> io::Result<()> {
    let load_start = Instant::now();
    let snapshot = snapshot::load(Path::new(index_path))?;
    let engine = SearchEngine::from_snapshot(snapshot);
    let load_elapsed = load_start.elapsed();

    let mode = if query.starts_with('"') && query.ends_with('"') {
        QueryMode::Phrase
    } else {
        mode_arg
    };

    let query = query.trim_matches('"');

    let search_start = Instant::now();
    let results = engine.search(query, mode);
    let search_elapsed = search_start.elapsed();

    eprintln!("load_time={:.2?}", load_elapsed);
    eprintln!("search_time={:.2?}", search_elapsed);

    for result in results.into_iter().take(limit) {
        println!("{} score={}", result.path, result.score);
    }

    Ok(())
}

pub(crate) fn run_update(index_path: &str, docs: &str) -> io::Result<()> {
    let snapshot = snapshot::load(Path::new(index_path))?;
    let mut engine = SearchEngine::from_snapshot(snapshot);

    let start = Instant::now();
    let added = engine.index_dir_incremental(Path::new(docs))?;
    let elapsed = start.elapsed();

    let snapshot = engine.into_snapshot();
    snapshot::save(Path::new(index_path), &snapshot)?;

    eprintln!("added_docs={} update_time={:.2?}", added, elapsed);

    Ok(())
}

pub(crate) fn run_build_segment(docs: &str, index_dir: &str) -> io::Result<()> {
    let mut engine = SearchEngine::new();

    let start = Instant::now();
    engine.index_dir(Path::new(docs))?;
    let elapsed = start.elapsed();

    let stats = engine.stats();

    eprintln!(
        "indexed docs={} terms={} postings={} positions={} index_time={:.2?}",
        engine.doc_count(),
        stats.terms,
        stats.postings,
        stats.total_positions,
        elapsed,
    );

    let store = SegmentStore::new(index_dir);
    let segment_id = "seg_000001";

    let segment = engine.into_segment(segment_id);
    store.save_segment(&segment)?;

    let manifest = Manifest {
        segments: vec![segment_id.to_string()],
    };

    store.save_manifest(&manifest)?;

    eprintln!("saved segment_index={index_dir}");

    Ok(())
}

pub(crate) fn run_search_segments(
    index_dir: &str,
    query: &str,
    limit: usize,
    mode_arg: QueryMode,
) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let mode = if query.starts_with('"') && query.ends_with('"') {
        QueryMode::Phrase
    } else {
        mode_arg
    };

    let terms: Vec<String> = crate::fileparser::tokenize(query)
        .into_iter()
        .map(|(term, _)| term)
        .collect();

    let mut all_results = Vec::new();

    let start = Instant::now();

    let cache = SegmentReaderCache::open(&store)?;

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

    let elapsed = start.elapsed();

    all_results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    eprintln!("fast_search_time={elapsed:.2?}");

    for result in all_results.into_iter().take(limit) {
        println!("{} score={}", result.path, result.score);
    }

    Ok(())
}

pub(crate) fn run_update_segment(index_dir: &str, docs: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let mut manifest = store.load_manifest()?;
    let segment_id = next_segment_id(&manifest);

    let mut engine = SearchEngine::new();

    let start = Instant::now();
    engine.index_dir(Path::new(docs))?;
    let elapsed = start.elapsed();

    let stats = engine.stats();

    eprintln!(
        "indexed docs={} terms={} postings={} positions={} index_time={:.2?}",
        engine.doc_count(),
        stats.terms,
        stats.postings,
        stats.total_positions,
        elapsed,
    );

    let segment = engine.into_segment(segment_id.clone());
    store.save_segment(&segment)?;

    manifest.segments.push(segment_id.clone());
    store.save_manifest(&manifest)?;

    eprintln!("added segment={segment_id}");

    Ok(())
}
