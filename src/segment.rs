use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::doctable::{DocId, DocTable};
use crate::memindex::{InvertedIndex, Position};
use crate::query::SearchResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub doctable: DocTable,
    pub index: InvertedIndex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub segments: Vec<String>,
}

pub struct SegmentStore {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentDocs {
    pub doctable: DocTable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TermEntry {
    pub term: String,
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentTerms {
    pub terms: Vec<TermEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TermPostings {
    pub docs: Vec<(DocId, Vec<Position>)>,
}

pub struct SegmentReader {
    id: String,
    docs: DocTable,
    terms: HashMap<String, TermEntry>,
    postings_path: PathBuf,
}

pub struct SegmentReaderCache {
    readers: Vec<SegmentReader>,
}

impl SegmentReaderCache {
    pub fn open(store: &SegmentStore) -> std::io::Result<Self> {
        let manifest = store.load_manifest()?;

        let mut readers = Vec::new();

        for segment_id in manifest.segments {
            readers.push(SegmentReader::open(store, &segment_id)?);
        }

        Ok(Self { readers })
    }

    pub fn readers(&self) -> &[SegmentReader] {
        &self.readers
    }
}

pub fn search_reader_all(
    reader: &SegmentReader,
    terms: &[String],
) -> std::io::Result<Vec<SearchResult>> {
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let Some(first_postings) = reader.lookup(&terms[0])? else {
        return Ok(Vec::new());
    };

    let mut results = Vec::new();

    for (&doc_id, positions) in &first_postings {
        let mut score = tf_idf(reader, &terms[0], positions.len());

        let mut matched = true;

        for term in &terms[1..] {
            let Some(postings) = reader.lookup(term)? else {
                matched = false;
                break;
            };

            let Some(term_positions) = postings.get(&doc_id) else {
                matched = false;
                break;
            };
            score += tf_idf(reader, term, term_positions.len());
        }

        if !matched {
            continue;
        }

        let Some(path) = reader.doc_path(doc_id) else {
            continue;
        };

        results.push(SearchResult {
            doc_id,
            path: path.to_string(),
            score,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(results)
}

pub fn search_reader_any(
    reader: &SegmentReader,
    terms: &[String],
) -> std::io::Result<Vec<SearchResult>> {
    let mut merged: HashMap<DocId, SearchResult> = HashMap::new();

    for term in terms {
        let Some(postings) = reader.lookup(term)? else {
            continue;
        };

        for (&doc_id, positions) in &postings {
            let Some(path) = reader.doc_path(doc_id) else {
                continue;
            };

            let score = tf_idf(reader, term, positions.len());

            merged
                .entry(doc_id)
                .and_modify(|result| {
                    result.score += score;
                })
                .or_insert_with(|| SearchResult {
                    doc_id,
                    path: path.to_string(),
                    score,
                });
        }
    }

    let mut results: Vec<_> = merged.into_values().collect();

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(results)
}

pub fn search_reader_phrase(
    reader: &SegmentReader,
    terms: &[String],
) -> std::io::Result<Vec<SearchResult>> {
    if terms.is_empty() {
        return Ok(Vec::new());
    }

    let mut postings_by_term = Vec::new();

    for term in terms {
        let Some(postings) = reader.lookup(term)? else {
            return Ok(Vec::new());
        };

        postings_by_term.push(postings);
    }

    let first_postings = &postings_by_term[0];
    let mut results = Vec::new();

    for (&doc_id, first_positions) in first_postings {
        let mut phrase_count = 0;

        for &start_pos in first_positions {
            let matched = postings_by_term
                .iter()
                .enumerate()
                .skip(1)
                .all(|(offset, postings)| {
                    postings
                        .get(&doc_id)
                        .is_some_and(|positions| positions.contains(&(start_pos + offset as u64)))
                });

            if matched {
                phrase_count += 1;
            }
        }

        if phrase_count == 0 {
            continue;
        }

        let Some(path) = reader.doc_path(doc_id) else {
            continue;
        };

        results.push(SearchResult {
            doc_id,
            path: path.to_string(),
            score: phrase_count as f64,
        });
    }

    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(results)
}

fn tf_idf(reader: &SegmentReader, term: &str, tf: usize) -> f64 {
    let n = reader.doc_count() as f64;

    let df = reader
        .lookup(term)
        .ok()
        .flatten()
        .map(|postings| postings.len() as f64)
        .unwrap_or(0.0);

    if n == 0.0 || df == 0.0 {
        return 0.0;
    }

    tf as f64 * (n / df).ln()
}

impl SegmentReader {
    pub fn open(store: &SegmentStore, id: &str) -> std::io::Result<Self> {
        let docs_bytes = fs::read(store.segment_docs_path(id))?;
        let terms_bytes = fs::read(store.segment_terms_path(id))?;

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).expect("deserialize docs");

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).expect("deserialize terms");

        let terms = terms
            .terms
            .into_iter()
            .map(|entry| (entry.term.clone(), entry))
            .collect();

        Ok(Self {
            id: id.to_string(),
            docs: docs.doctable,
            terms,
            postings_path: store.segment_postings_path(id),
        })
    }

    pub fn lookup(&self, term: &str) -> std::io::Result<Option<HashMap<DocId, Vec<Position>>>> {
        let Some(entry) = self.terms.get(term) else {
            return Ok(None);
        };

        let mut file = fs::File::open(&self.postings_path)?;

        file.seek(SeekFrom::Start(entry.offset))?;

        let mut buf = vec![0u8; entry.len as usize];
        file.read_exact(&mut buf)?;

        let postings: TermPostings = bincode::deserialize(&buf).expect("deserialize term postings");

        Ok(Some(postings.docs.into_iter().collect()))
    }

    pub fn doc_path(&self, doc_id: DocId) -> Option<&str> {
        self.docs.get_path(doc_id)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    pub fn document_frequency(&self, term: &str) -> usize {
        self.terms
            .get(term)
            .map(|entry| {
                let postings = self.lookup(&entry.term).ok().flatten();
                postings.map(|p| p.len()).unwrap_or(0)
            })
            .unwrap_or(0)
    }
}

pub fn next_segment_id(manifest: &Manifest) -> String {
    format!("seg_{:06}", manifest.segments.len() + 1)
}

impl SegmentStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn init(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.segments_dir())
    }

    pub fn save_segment(&self, segment: &Segment) -> std::io::Result<()> {
        self.init()?;
        fs::create_dir_all(self.segment_dir(&segment.id))?;

        let docs = SegmentDocs {
            doctable: segment.doctable.clone(),
        };

        fs::write(
            self.segment_docs_path(&segment.id),
            bincode::serialize(&docs).expect("serialize docs"),
        )?;

        let mut entries = Vec::new();
        let mut postings_file = fs::File::create(self.segment_postings_path(&segment.id))?;
        let mut offset = 0u64;

        for (term, postings) in segment.index.postings_iter() {
            let docs: Vec<_> = postings
                .iter()
                .map(|(&doc_id, positions)| (doc_id, positions.clone()))
                .collect();

            let bytes =
                bincode::serialize(&TermPostings { docs }).expect("serialize term postings");

            postings_file.write_all(&bytes)?;

            entries.push(TermEntry {
                term: term.clone(),
                offset,
                len: bytes.len() as u64,
            });

            offset += bytes.len() as u64;
        }

        let terms = SegmentTerms { terms: entries };

        fs::write(
            self.segment_terms_path(&segment.id),
            bincode::serialize(&terms).expect("serialize terms"),
        )?;

        Ok(())
    }

    pub fn load_segment(&self, id: &str) -> std::io::Result<Segment> {
        let docs_bytes = fs::read(self.segment_docs_path(id))?;
        let terms_bytes = fs::read(self.segment_terms_path(id))?;

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).expect("deserialize docs");

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).expect("deserialize terms");

        let mut postings_file = fs::File::open(self.segment_postings_path(id))?;
        let mut index = InvertedIndex::new();

        for entry in terms.terms {
            postings_file.seek(SeekFrom::Start(entry.offset))?;

            let mut buf = vec![0u8; entry.len as usize];
            postings_file.read_exact(&mut buf)?;

            let postings: TermPostings =
                bincode::deserialize(&buf).expect("deserialize term postings");

            let map = postings.docs.into_iter().collect();

            index.insert_postings(entry.term, map);
        }

        Ok(Segment {
            id: id.to_string(),
            doctable: docs.doctable,
            index,
        })
    }

    pub fn save_manifest(&self, manifest: &Manifest) -> std::io::Result<()> {
        self.init()?;

        fs::write(
            self.manifest_path(),
            bincode::serialize(manifest).expect("serialize manifest"),
        )
    }

    pub fn load_manifest(&self) -> std::io::Result<Manifest> {
        let bytes = fs::read(self.manifest_path())?;
        let manifest = bincode::deserialize(&bytes).expect("deserialize manifest");

        Ok(manifest)
    }

    fn segments_dir(&self) -> PathBuf {
        self.root.join("segments")
    }

    fn segment_dir(&self, id: &str) -> PathBuf {
        self.segments_dir().join(id)
    }

    fn segment_docs_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("docs.bin")
    }

    fn segment_terms_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("terms.bin")
    }

    fn segment_postings_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("postings.bin")
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.bin")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn segment_store_saves_and_loads_manifest() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let manifest = Manifest {
            segments: vec!["seg_000001".to_string()],
        };

        store.save_manifest(&manifest).unwrap();

        let restored = store.load_manifest().unwrap();

        assert_eq!(restored.segments, vec!["seg_000001"]);
    }

    #[test]
    fn segment_store_saves_and_loads_segment() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable: DocTable::new(),
            index: InvertedIndex::new(),
        };

        store.save_segment(&segment).unwrap();

        let restored = store.load_segment("seg_000001").unwrap();

        assert_eq!(restored.id, "seg_000001");
    }

    #[test]
    fn segment_roundtrip_preserves_search_results() {
        use crate::engine::SearchEngine;
        use crate::query::QueryMode;
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        fs::write(dir.path().join("a.txt"), "rust memory safety").unwrap();
        fs::write(dir.path().join("b.txt"), "rust distributed system").unwrap();

        let mut engine = SearchEngine::new();
        engine.index_dir(dir.path()).unwrap();

        let segment = engine.into_segment("seg_000001");
        let store = SegmentStore::new(dir.path().join("index"));

        store.save_segment(&segment).unwrap();

        let restored = store.load_segment("seg_000001").unwrap();
        let engine = SearchEngine::from_segment(restored);

        let results = engine.search("rust memory", QueryMode::All);

        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("a.txt"));
    }

    #[test]
    fn next_segment_id_uses_manifest_len() {
        let manifest = Manifest {
            segments: vec!["seg_000001".to_string(), "seg_000002".to_string()],
        };

        assert_eq!(next_segment_id(&manifest), "seg_000003");
    }

    #[test]
    fn segment_reader_lookup_reads_single_term_postings() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut index = InvertedIndex::new();
        index.add_document_tokens(
            1,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 2),
                ("memory".to_string(), 1),
            ],
        );

        let mut doctable = DocTable::new();
        doctable.add_document("a.txt".to_string());

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        let postings = reader.lookup("rust").unwrap().unwrap();

        assert_eq!(postings.get(&1), Some(&vec![0, 2]));
        assert_eq!(reader.lookup("missing").unwrap(), None);
    }

    #[test]
    fn search_reader_all_matches_docs_with_all_terms() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());

        let mut index = InvertedIndex::new();
        index.add_document_tokens(
            doc1,
            vec![("rust".to_string(), 0), ("memory".to_string(), 1)],
        );
        index.add_document_tokens(doc2, vec![("rust".to_string(), 0)]);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        let results =
            search_reader_all(&reader, &["rust".to_string(), "memory".to_string()]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
    }

    #[test]
    fn search_reader_phrase_matches_adjacent_terms() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());

        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            doc1,
            vec![
                ("white".to_string(), 0),
                ("whale".to_string(), 1),
                ("white".to_string(), 4),
                ("whale".to_string(), 5),
            ],
        );

        index.add_document_tokens(
            doc2,
            vec![
                ("white".to_string(), 0),
                ("rust".to_string(), 1),
                ("whale".to_string(), 2),
            ],
        );

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        let terms = vec!["white".to_string(), "whale".to_string()];
        let results = search_reader_phrase(&reader, &terms).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
        assert_eq!(results[0].score, 2.0);
    }

    #[test]
    fn segment_reader_cache_opens_all_segments() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc_id = doctable.add_document("a.txt".to_string());

        let mut index = InvertedIndex::new();
        index.add_document_tokens(doc_id, vec![("rust".to_string(), 0)]);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let manifest = Manifest {
            segments: vec!["seg_000001".to_string()],
        };

        store.save_manifest(&manifest).unwrap();

        let cache = SegmentReaderCache::open(&store).unwrap();

        assert_eq!(cache.readers().len(), 1);
        assert_eq!(cache.readers()[0].id(), "seg_000001");
    }
}
