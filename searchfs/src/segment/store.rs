use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use crate::engine::SearchEngine;
use crate::index::doctable::DocId;
use crate::index::memindex::{InvertedIndex, Position};
use crate::segment::codec::{CompressedPostingCodec, PostingCodec};
use crate::segment::format::{
    DocMetaEntry, MANIFEST_VERSION, Manifest, SEGMENT_DOC_META_VERSION, SEGMENT_META_VERSION,
    SEGMENT_TERMS_VERSION, Segment, SegmentData, SegmentDocMeta, SegmentDocs, SegmentMeta,
    SegmentTerms, TermEntry, next_segment_id,
};
use crate::segment::reader::{SegmentReader, SegmentReaderCache};
use crate::storage::Storage;
use crate::storage::local::LocalStorage;

pub struct SegmentStore<S: Storage> {
    storage: S,
    pub codec: Box<dyn PostingCodec>,
}

impl SegmentStore<LocalStorage> {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_storage(LocalStorage::new(root))
    }
}

impl<S: Storage> SegmentStore<S> {
    pub fn with_storage(storage: S) -> Self {
        Self {
            storage,
            codec: Box::new(CompressedPostingCodec),
        }
    }

    pub fn open_reader_cache(&self) -> io::Result<SegmentReaderCache> {
        let manifest = self.load_manifest()?;

        let mut readers = Vec::new();

        for id in manifest.segments {
            readers.push(self.open_reader(&id)?);
        }

        Ok(SegmentReaderCache::new(readers))
    }

    pub fn open_reader(&self, id: &str) -> io::Result<SegmentReader> {
        let docs_bytes = self.read_segment_docs_bytes(id)?;
        let terms_bytes = self.read_segment_terms_bytes(id)?;
        let postings_bytes = self.read_segment_postings_bytes(id)?;

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment docs: {err}"),
            )
        })?;

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment terms: {err}"),
            )
        })?;

        let terms = terms
            .terms
            .into_iter()
            .map(|entry| (entry.term.clone(), entry))
            .collect();

        let meta = self.load_segment_meta(id)?;
        let docmeta = self.load_segment_docmeta(id)?;

        let doc_lens = docmeta
            .docs
            .into_iter()
            .map(|entry| (entry.doc_id, entry.doc_len))
            .collect();
        let data = SegmentData {
            id: id.to_string(),
            docs: docs.doctable,
            terms,
            postings: postings_bytes,
            meta,
            doc_lens,
        };

        Ok(SegmentReader::new(data, self.codec.clone_box()))
    }

    pub fn init(&self) -> std::io::Result<()> {
        self.storage.create_dir_all(&self.segments_dir())
    }

    pub fn save_segment(&self, segment: &Segment) -> std::io::Result<()> {
        self.init()?;
        self.storage
            .create_dir_all(&self.segment_dir(&segment.id))?;

        let docs = SegmentDocs {
            doctable: segment.doctable.clone(),
        };

        self.storage.write(
            &self.segment_docs_path(&segment.id),
            &bincode::serialize(&docs).expect("serialize docs"),
        )?;
        let mut doc_lens: HashMap<DocId, usize> = HashMap::new();

        let mut entries = Vec::new();
        let mut postings_bytes = Vec::new();
        let mut offset = 0u64;

        let mut term_count = 0usize;
        let mut posting_count = 0usize;
        let mut position_count = 0usize;

        for (term, postings) in segment.index.postings_iter() {
            term_count += 1;
            posting_count += postings.len();
            position_count += postings
                .values()
                .map(|positions| positions.len())
                .sum::<usize>();

            for (&doc_id, positions) in postings {
                *doc_lens.entry(doc_id).or_insert(0) += positions.len();
            }

            let docs: Vec<_> = postings
                .iter()
                .map(|(&doc_id, positions)| (doc_id, positions.clone()))
                .collect();

            let doc_freq = docs.len();

            let postings: HashMap<DocId, Vec<Position>> = postings
                .iter()
                .map(|(&doc_id, positions)| (doc_id, positions.clone()))
                .collect();

            let bytes = self.codec.encode(&postings)?;

            postings_bytes.extend_from_slice(&bytes);

            entries.push(TermEntry {
                term: term.clone(),
                offset,
                len: bytes.len() as u64,
                doc_freq,
            });

            offset += bytes.len() as u64;
        }

        self.storage
            .write(&self.segment_postings_path(&segment.id), &postings_bytes)?;

        let terms = SegmentTerms {
            version: SEGMENT_TERMS_VERSION,
            terms: entries,
        };

        let postings_size = offset;

        let meta = SegmentMeta {
            version: SEGMENT_META_VERSION,
            id: segment.id.clone(),
            doc_count: segment.doctable.len(),
            term_count,
            posting_count,
            position_count,
            postings_size,
        };

        let mut docmeta_docs: Vec<_> = doc_lens
            .into_iter()
            .map(|(doc_id, doc_len)| DocMetaEntry { doc_id, doc_len })
            .collect();

        docmeta_docs.sort_by_key(|entry| entry.doc_id);

        let docmeta = SegmentDocMeta {
            version: SEGMENT_DOC_META_VERSION,
            docs: docmeta_docs,
        };

        self.storage.write(
            &self.segment_docmeta_path(&segment.id),
            &bincode::serialize(&docmeta).expect("serialize segment doc meta"),
        )?;

        self.storage.write(
            &self.segment_terms_path(&segment.id),
            &bincode::serialize(&terms).expect("serialize terms"),
        )?;

        self.storage.write(
            &self.segment_meta_path(&segment.id),
            &bincode::serialize(&meta).expect("serialize segment meta"),
        )?;

        Ok(())
    }

    pub fn load_segment(&self, id: &str) -> std::io::Result<Segment> {
        let docs_bytes = self.storage.read(&self.segment_docs_path(id))?;
        let terms_bytes = self.storage.read(&self.segment_terms_path(id))?;

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment docs: {err}"),
            )
        })?;

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment terms: {err}"),
            )
        })?;

        let postings_bytes = self.storage.read(&self.segment_postings_path(id))?;
        let mut index = InvertedIndex::new();

        for entry in terms.terms {
            let start = entry.offset as usize;
            let end = start + entry.len as usize;

            let Some(buf) = postings_bytes.get(start..end) else {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("posting slice out of bounds for term {}", entry.term),
                ));
            };

            let map = self.codec.decode(buf)?;

            index.insert_postings(entry.term, map);
        }

        Ok(Segment {
            id: id.to_string(),
            doctable: docs.doctable,
            index,
        })
    }

    pub fn load_segment_meta(&self, id: &str) -> io::Result<SegmentMeta> {
        let bytes = self.storage.read(&self.segment_meta_path(id))?;
        let meta: SegmentMeta = bincode::deserialize(&bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment meta: {err}"),
            )
        })?;

        if meta.version != SEGMENT_META_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported segment meta version: {}", meta.version),
            ));
        }

        Ok(meta)
    }

    pub fn save_manifest(&self, manifest: &Manifest) -> std::io::Result<()> {
        self.init()?;

        self.storage.write(
            &self.manifest_path(),
            &bincode::serialize(manifest).expect("serialize manifest"),
        )
    }

    pub fn load_manifest(&self) -> std::io::Result<Manifest> {
        let bytes = self.storage.read(&self.manifest_path())?;
        let manifest = bincode::deserialize(&bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize manifest: {err}"),
            )
        })?;

        Ok(manifest)
    }

    pub fn load_segment_docmeta(&self, id: &str) -> io::Result<SegmentDocMeta> {
        let bytes = self.storage.read(&self.segment_docmeta_path(id))?;

        let docmeta: SegmentDocMeta = bincode::deserialize(&bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize segment doc meta: {err}"),
            )
        })?;

        if docmeta.version != SEGMENT_DOC_META_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported segment doc meta version: {}", docmeta.version),
            ));
        }

        Ok(docmeta)
    }

    pub(crate) fn remove_segment(&self, id: &str) -> io::Result<()> {
        if self.segment_exists(id) {
            self.storage.remove_dir_all(&self.segment_dir(id))?;
        }

        Ok(())
    }

    pub(crate) fn segments_dir(&self) -> String {
        "segments".to_string()
    }

    pub(crate) fn segment_dir(&self, id: &str) -> String {
        format!("segments/{id}")
    }

    pub(crate) fn segment_docs_path(&self, id: &str) -> String {
        format!("segments/{id}/docs.bin")
    }

    pub(crate) fn segment_terms_path(&self, id: &str) -> String {
        format!("segments/{id}/terms.bin")
    }

    pub(crate) fn segment_postings_path(&self, id: &str) -> String {
        format!("segments/{id}/postings.bin")
    }

    pub(crate) fn manifest_path(&self) -> String {
        "manifest.bin".to_string()
    }

    pub(crate) fn segment_exists(&self, id: &str) -> bool {
        self.storage.exists(&self.segment_dir(id))
    }

    pub fn merge_all_segments(&self) -> io::Result<String> {
        let manifest = self.load_manifest()?;
        let old_segments = manifest.segments.clone();

        if manifest.segments.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cannot merge empty manifest",
            ));
        }

        if manifest.segments.len() == 1 {
            return Ok(manifest.segments[0].clone());
        }

        let mut merged = SearchEngine::new();

        for segment_id in &manifest.segments {
            let segment = self.load_segment(segment_id)?;
            merged.merge_segment(segment);
        }

        let new_id = next_segment_id(&manifest);
        let segment = merged.into_segment(new_id.clone());

        self.save_segment(&segment)?;

        let new_manifest = Manifest {
            version: MANIFEST_VERSION,
            segments: vec![new_id.clone()],
        };

        self.save_manifest(&new_manifest)?;

        for old_id in old_segments {
            if old_id != new_id {
                self.remove_segment(&old_id)?;
            }
        }
        Ok(new_id)
    }

    fn segment_meta_path(&self, id: &str) -> String {
        format!("segments/{id}/meta.bin")
    }

    fn segment_docmeta_path(&self, id: &str) -> String {
        format!("segments/{id}/docmeta.bin")
    }

    pub(crate) fn read_segment_docs_bytes(&self, id: &str) -> io::Result<Vec<u8>> {
        self.storage.read(&self.segment_docs_path(id))
    }

    pub(crate) fn read_segment_terms_bytes(&self, id: &str) -> io::Result<Vec<u8>> {
        self.storage.read(&self.segment_terms_path(id))
    }

    pub(crate) fn read_segment_postings_bytes(&self, id: &str) -> io::Result<Vec<u8>> {
        self.storage.read(&self.segment_postings_path(id))
    }
}

#[cfg(test)]
mod tests {
    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::segment::format::{MANIFEST_VERSION, Manifest, Segment};
    use crate::segment::search::SegmentSearcher;
    use crate::segment::store::SegmentStore;
    use tempfile::tempdir;

    #[test]
    fn segment_store_saves_and_loads_manifest() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());
        let manifest = Manifest {
            version: MANIFEST_VERSION,
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
    fn merge_all_segments_compacts_manifest() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let segment1 = Segment {
            id: "seg_000001".to_string(),
            doctable: DocTable::new(),
            index: InvertedIndex::new(),
        };

        let segment2 = Segment {
            id: "seg_000002".to_string(),
            doctable: DocTable::new(),
            index: InvertedIndex::new(),
        };

        store.save_segment(&segment1).unwrap();
        store.save_segment(&segment2).unwrap();

        store
            .save_manifest(&Manifest {
                version: MANIFEST_VERSION,
                segments: vec!["seg_000001".to_string(), "seg_000002".to_string()],
            })
            .unwrap();

        let merged_id = store.merge_all_segments().unwrap();

        assert_eq!(merged_id, "seg_000003");

        let manifest = store.load_manifest().unwrap();

        assert_eq!(manifest.segments, vec!["seg_000003"]);
    }

    #[test]
    fn merge_all_segments_preserves_search_results() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable1 = DocTable::new();
        let doc1 = doctable1.add_document("a.txt".to_string());

        let mut index1 = InvertedIndex::new();
        index1.add_document_tokens(
            doc1,
            vec![("rust".to_string(), 0), ("memory".to_string(), 1)],
        );

        let segment1 = Segment {
            id: "seg_000001".to_string(),
            doctable: doctable1,
            index: index1,
        };

        let mut doctable2 = DocTable::new();
        let doc2 = doctable2.add_document("b.txt".to_string());

        let mut index2 = InvertedIndex::new();
        index2.add_document_tokens(
            doc2,
            vec![("rust".to_string(), 0), ("system".to_string(), 1)],
        );

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

        store.merge_all_segments().unwrap();

        let cache = store.open_reader_cache().unwrap();
        assert_eq!(cache.readers().len(), 1);

        let searcher = SegmentSearcher::new(&cache.readers()[0]);
        let results = searcher.search_any(&["rust".to_string()], 2).unwrap();

        let mut paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();
        paths.sort();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);
        assert!(!store.segment_exists("seg_000001"));
        assert!(!store.segment_exists("seg_000002"));
        assert!(store.segment_exists("seg_000003"));
    }

    #[test]
    fn segment_meta_contains_postings_size() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc = doctable.add_document("a.txt".into());

        let mut index = InvertedIndex::new();
        index.add_document_tokens(doc, vec![("rust".into(), 0)]);

        let segment = Segment {
            id: "seg_000001".into(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let meta = store.load_segment_meta("seg_000001").unwrap();

        assert!(meta.postings_size > 0);
    }

    #[test]
    fn segment_store_works_with_memory_storage() {
        use crate::storage::memory::MemoryStorage;

        let storage = MemoryStorage::new();
        let store = SegmentStore::with_storage(storage);

        let mut doctable = DocTable::new();
        let doc = doctable.add_document("a.txt".to_string());

        let mut index = InvertedIndex::new();
        index.add_document_tokens(doc, vec![("rust".to_string(), 0)]);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let restored = store.load_segment("seg_000001").unwrap();

        assert_eq!(restored.id, "seg_000001");
    }

    #[test]
    fn merge_scheduler_can_trigger_segment_merge() {
        use crate::segment::merge_scheduler::MergeScheduler;

        let scheduler = MergeScheduler::new(2);

        assert!(!scheduler.should_merge(2));
        assert!(scheduler.should_merge(3));
    }
}
