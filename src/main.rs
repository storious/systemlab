use std::env;
use std::io;
use std::path::Path;
use std::time::Instant;

use searchfs::engine::SearchEngine;
use searchfs::query::QueryMode;
use searchfs::segment::format::{Manifest, next_segment_id};
use searchfs::segment::reader::SegmentReaderCache;
use searchfs::segment::search::SegmentSearcher;
use searchfs::segment::store::SegmentStore;
use searchfs::snapshot;

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    match args.next().as_deref() {
        Some("build") => {
            let Some(docs) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing docs path",
                ));
            };

            let Some(index) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index path",
                ));
            };

            run_build(&docs, &index)
        }

        Some("update") => {
            let Some(docs) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing docs path",
                ));
            };

            let Some(index) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index path",
                ));
            };
            run_update(&docs, &index)
        }

        Some("search") => {
            let Some(index) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index path",
                ));
            };

            let Some(query) = args.next() else {
                print_usage();
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing query"));
            };

            let limit = args
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10);

            let mode = args.next().unwrap_or_else(|| "and".to_string());

            run_search(&index, &query, limit, &mode)
        }

        Some("build-segment") => {
            let Some(docs) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing docs path",
                ));
            };

            let Some(index_dir) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index dir",
                ));
            };

            run_build_segment(&docs, &index_dir)
        }

        Some("search-segments") => {
            let Some(index_dir) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index dir",
                ));
            };

            let Some(query) = args.next() else {
                print_usage();
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing query"));
            };

            let limit = args
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10);

            let mode = args.next().unwrap_or_else(|| "and".to_string());

            run_search_segments(&index_dir, &query, limit, &mode)
        }

        Some("update-segment") => {
            let Some(index_dir) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing index dir",
                ));
            };

            let Some(docs) = args.next() else {
                print_usage();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing docs path",
                ));
            };

            run_update_segment(&index_dir, &docs)
        }

        _ => {
            print_usage();
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown or missing command",
            ))
        }
    }
}

fn run_build(docs: &str, index_path: &str) -> io::Result<()> {
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

fn run_search(index_path: &str, query: &str, limit: usize, mode_arg: &str) -> io::Result<()> {
    let load_start = Instant::now();
    let snapshot = snapshot::load(Path::new(index_path))?;
    let engine = SearchEngine::from_snapshot(snapshot);
    let load_elapsed = load_start.elapsed();

    let mode = if query.starts_with('"') && query.ends_with('"') {
        QueryMode::Phrase
    } else {
        QueryMode::try_from(mode_arg)
            .map_err(|msg| io::Error::new(io::ErrorKind::InvalidInput, msg))?
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

fn run_update(index_path: &str, docs: &str) -> io::Result<()> {
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

fn run_build_segment(docs: &str, index_dir: &str) -> io::Result<()> {
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

fn run_search_segments(
    index_dir: &str,
    query: &str,
    limit: usize,
    mode_arg: &str,
) -> io::Result<()> {
    let store = SegmentStore::new(index_dir);

    let mode = if query.starts_with('"') && query.ends_with('"') {
        QueryMode::Phrase
    } else {
        QueryMode::try_from(mode_arg)
            .map_err(|msg| io::Error::new(io::ErrorKind::InvalidInput, msg))?
    };

    let terms: Vec<String> = searchfs::fileparser::tokenize(query)
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

fn run_update_segment(index_dir: &str, docs: &str) -> io::Result<()> {
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

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  searchfs build <docs> <index>");
    eprintln!("  searchfs search <index> <query> [limit] [and|or|phrase]");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  searchfs build docs searchfs.idx");
    eprintln!("  searchfs search searchfs.idx \"rust memory\" 10 and");
    eprintln!("  searchfs search searchfs.idx '\"white whale\"' 5");
    eprintln!("  searchfs build-segment <docs> <index-dir>");
    eprintln!("  searchfs search-segments <index-dir> <query> [limit] [and|or|phrase]");
    eprintln!("  searchfs update-segment <index-dir> <docs>");
}
