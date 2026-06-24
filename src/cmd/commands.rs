use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::cmd::utils::{print_results, search_with_cache};
use crate::engine::SearchEngine;
use crate::query::{QueryMode, parse_query_mode};
use crate::segment::format::{Manifest, next_segment_id};
use crate::segment::reader::SegmentReaderCache;
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

    let (query, mode) = parse_query_mode(query, mode_arg);

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

    let (query, mode) = parse_query_mode(query, mode_arg);

    let start = Instant::now();

    let cache = SegmentReaderCache::open(&store)?;

    let results = search_with_cache(&cache, query, mode)?;

    let elapsed = start.elapsed();

    eprintln!("search_time={elapsed:.2?}");
    print_results(results, limit);

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

pub(crate) fn run_repl(index_dir: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let load_start = Instant::now();
    let cache = SegmentReaderCache::open(&store)?;
    let load_elapsed = load_start.elapsed();

    eprintln!(
        "loaded segments={} load_time={:.2?}",
        cache.readers().len(),
        load_elapsed,
    );

    eprintln!("type a query, or :quit to exit");

    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("searchfs> ");
        io::stdout().flush()?;

        line.clear();
        let n = stdin.read_line(&mut line)?;

        if n == 0 {
            break;
        }

        let query = line.trim();

        if query.is_empty() {
            continue;
        }

        if query == ":quit" || query == ":q" {
            break;
        }

        let (query, mode) = parse_query_mode(query, QueryMode::All);

        let start = Instant::now();
        let results = search_with_cache(&cache, query, mode)?;

        let elapsed = start.elapsed();

        eprintln!("search_time={elapsed:.2?}");
        print_results(results, 10);
    }

    Ok(())
}

pub(crate) fn run_merge_segments(index_dir: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let start = Instant::now();
    let merged_id = store.merge_all_segments()?;
    let elapsed = start.elapsed();

    eprintln!("merged segment={merged_id} merge_time={elapsed:.2?}");

    Ok(())
}
