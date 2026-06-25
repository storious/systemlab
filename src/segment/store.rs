use crate::segment::format::{
    DocMetaEntry, MANIFEST_VERSION, Manifest, SEGMENT_DOC_META_VERSION, SEGMENT_META_VERSION,
    SEGMENT_TERMS_VERSION, Segment, SegmentDocMeta, SegmentDocs, SegmentMeta, SegmentTerms,
    TermEntry, TermPostings, next_segment_id,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::engine::SearchEngine;
use crate::index::doctable::DocId;
use crate::index::memindex::InvertedIndex;

pub struct SegmentStore {
    root: PathBuf,
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
        let mut doc_lens: HashMap<DocId, usize> = HashMap::new();

        let mut entries = Vec::new();
        let mut postings_file = fs::File::create(self.segment_postings_path(&segment.id))?;
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

            let bytes =
                bincode::serialize(&TermPostings { docs }).expect("serialize term postings");

            postings_file.write_all(&bytes)?;

            entries.push(TermEntry {
                term: term.clone(),
                offset,
                len: bytes.len() as u64,
                doc_freq,
            });

            offset += bytes.len() as u64;
        }

        let terms = SegmentTerms {
            version: SEGMENT_TERMS_VERSION,
            terms: entries,
        };
        let meta = SegmentMeta {
            version: SEGMENT_META_VERSION,
            id: segment.id.clone(),
            doc_count: segment.doctable.len(),
            term_count,
            posting_count,
            position_count,
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

        fs::write(
            self.segment_docmeta_path(&segment.id),
            bincode::serialize(&docmeta).expect("serialize segment doc meta"),
        )?;

        fs::write(
            self.segment_terms_path(&segment.id),
            bincode::serialize(&terms).expect("serialize terms"),
        )?;

        fs::write(
            self.segment_meta_path(&segment.id),
            bincode::serialize(&meta).expect("serialize segment meta"),
        )?;

        Ok(())
    }

    pub fn load_segment(&self, id: &str) -> std::io::Result<Segment> {
        let docs_bytes = fs::read(self.segment_docs_path(id))?;
        let terms_bytes = fs::read(self.segment_terms_path(id))?;

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

        let mut postings_file = fs::File::open(self.segment_postings_path(id))?;
        let mut index = InvertedIndex::new();

        for entry in terms.terms {
            postings_file.seek(SeekFrom::Start(entry.offset))?;

            let mut buf = vec![0u8; entry.len as usize];
            postings_file.read_exact(&mut buf)?;

            let postings: TermPostings = bincode::deserialize(&buf).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("deserialize terms postings: {err}"),
                )
            })?;

            let map = postings.docs.into_iter().collect();

            index.insert_postings(entry.term, map);
        }

        Ok(Segment {
            id: id.to_string(),
            doctable: docs.doctable,
            index,
        })
    }

    pub fn load_segment_meta(&self, id: &str) -> io::Result<SegmentMeta> {
        let bytes = fs::read(self.segment_meta_path(id))?;
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

        fs::write(
            self.manifest_path(),
            bincode::serialize(manifest).expect("serialize manifest"),
        )
    }

    pub fn load_manifest(&self) -> std::io::Result<Manifest> {
        let bytes = fs::read(self.manifest_path())?;
        let manifest = bincode::deserialize(&bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialize manifest: {err}"),
            )
        })?;

        Ok(manifest)
    }

    pub fn load_segment_docmeta(&self, id: &str) -> io::Result<SegmentDocMeta> {
        let bytes = fs::read(self.segment_docmeta_path(id))?;

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

    pub(crate) fn segments_dir(&self) -> PathBuf {
        self.root.join("segments")
    }

    pub(crate) fn segment_dir(&self, id: &str) -> PathBuf {
        self.segments_dir().join(id)
    }

    pub(crate) fn segment_docs_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("docs.bin")
    }

    pub(crate) fn segment_terms_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("terms.bin")
    }

    pub(crate) fn segment_postings_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("postings.bin")
    }

    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.bin")
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
                let old_dir = self.segment_dir(&old_id);

                if old_dir.exists() {
                    fs::remove_dir_all(old_dir)?;
                }
            }
        }

        Ok(new_id)
    }

    fn segment_meta_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("meta.bin")
    }

    fn segment_docmeta_path(&self, id: &str) -> PathBuf {
        self.segment_dir(id).join("docmeta.bin")
    }
}

#[cfg(test)]
mod tests {
    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::segment::format::{MANIFEST_VERSION, Manifest, Segment};
    use crate::segment::reader::SegmentReaderCache;
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

        let cache = SegmentReaderCache::open(&store).unwrap();
        assert_eq!(cache.readers().len(), 1);

        let searcher = SegmentSearcher::new(&cache.readers()[0]);
        let results = searcher.search_any(&["rust".to_string()]).unwrap();

        let mut paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();
        paths.sort();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);
        assert!(!store.segment_dir("seg_000001").exists());
        assert!(!store.segment_dir("seg_000002").exists());
        assert!(store.segment_dir("seg_000003").exists());
    }
}
