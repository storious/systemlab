use crate::segment::format::{
    Manifest, Segment, SegmentDocs, SegmentTerms, TermEntry, TermPostings,
};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::memindex::InvertedIndex;

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
}

#[cfg(test)]
mod tests {
    use crate::doctable::DocTable;
    use crate::memindex::InvertedIndex;
    use crate::segment::format::{Manifest, Segment};
    use crate::segment::store::SegmentStore;
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
}
