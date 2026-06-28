use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

use crate::cmd::utils::{
    ReplCommandResult, ReplState, handle_repl_command, print_results, search_with_cache,
};
use crate::engine::SearchEngine;
use crate::query::{QueryMode, parse_query_mode};
use crate::segment::format::{
    MANIFEST_VERSION, Manifest, SEGMENT_META_VERSION, SegmentMeta, next_segment_id,
};
use crate::segment::merge_scheduler::MergeScheduler;
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

    let index_path = Path::new(index_dir);

    if index_path.exists() {
        std::fs::remove_dir_all(index_path)?;
    }

    let store = SegmentStore::new(index_dir);
    let segment_id = "seg_000001";

    let segment = engine.into_segment(segment_id);
    store.save_segment(&segment)?;

    let manifest = Manifest {
        version: MANIFEST_VERSION,
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

    let cache = store.open_reader_cache()?;

    let results = search_with_cache(&cache, query, mode, limit)?;

    let elapsed = start.elapsed();

    eprintln!("search_time={elapsed:.2?}");
    print_results(results, limit);

    Ok(())
}

pub(crate) fn run_update_segment(index_dir: &str, docs: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let manifest = store.load_manifest()?;
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

    let mut segments = manifest.segments;
    segments.push(segment_id.clone());

    store.save_manifest(&Manifest {
        version: MANIFEST_VERSION,
        segments,
    })?;

    eprintln!("added segment={segment_id}");

    let scheduler = MergeScheduler::default();
    let manifest = store.load_manifest()?;

    if scheduler.should_merge(manifest.segments.len()) {
        let merged_id = store.merge_all_segments()?;
        eprintln!(
            "auto_merged segment={merged_id} reason=segment_count>{}",
            scheduler.max_segments()
        );
    }

    Ok(())
}

pub(crate) fn run_repl(index_dir: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let load_start = Instant::now();
    let cache = store.open_reader_cache()?;
    let load_elapsed = load_start.elapsed();

    eprintln!(
        "loaded segments={} load_time={:.2?}",
        cache.readers().len(),
        load_elapsed,
    );
    eprintln!("type a query, or :quit to exit");

    let stdin = io::stdin();
    let mut line = String::new();
    let mut state = ReplState::default();

    loop {
        print!("searchrs> ");
        io::stdout().flush()?;

        line.clear();

        if stdin.read_line(&mut line)? == 0 {
            break;
        }

        let normalized = line.trim().replace('：', ":");
        let input = normalized.trim();

        if input.is_empty() {
            continue;
        }

        match handle_repl_command(input, &cache, &mut state) {
            ReplCommandResult::Continue => continue,
            ReplCommandResult::Exit => break,
            ReplCommandResult::Search => {}
        }

        let (query, query_mode) = parse_query_mode(input, state.mode);

        let start = Instant::now();
        let results = search_with_cache(&cache, query, query_mode, state.limit)?;
        let elapsed = start.elapsed();

        eprintln!("search_time={elapsed:.2?}");
        print_results(results, state.limit);
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

pub(crate) fn run_inspect_segments(index_dir: &str) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);
    let manifest = store.load_manifest()?;

    let mut total = SegmentMeta {
        version: SEGMENT_META_VERSION,
        id: "total".to_string(),
        doc_count: 0,
        term_count: 0,
        posting_count: 0,
        position_count: 0,
        postings_size: 0,
    };

    println!("segments={}", manifest.segments.len());

    for segment_id in &manifest.segments {
        let start = Instant::now();

        let meta = store.load_segment_meta(segment_id).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("load meta for segment {segment_id}: {err}"),
            )
        })?;

        let elapsed = start.elapsed();

        total.accumulate(&meta);

        println!();
        println!("{segment_id}");
        println!("  load_time={elapsed:.2?}");
        print!("{meta}");
    }

    println!();
    println!("total");
    print!("{total}");

    Ok(())
}
