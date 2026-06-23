use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::doctable::DocTable;
use crate::memindex::InvertedIndex;

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

        let path = self.segment_path(&segment.id);
        let bytes = bincode::serialize(segment).expect("serialize segment");

        fs::write(path, bytes)
    }

    pub fn load_segment(&self, id: &str) -> std::io::Result<Segment> {
        let path = self.segment_path(id);
        let bytes = fs::read(path)?;

        let segment = bincode::deserialize(&bytes).expect("deserialize segment");

        Ok(segment)
    }

    pub fn save_manifest(&self, manifest: &Manifest) -> std::io::Result<()> {
        self.init()?;

        let path = self.manifest_path();
        let bytes = bincode::serialize(manifest).expect("serialize manifest");

        fs::write(path, bytes)
    }

    pub fn load_manifest(&self) -> std::io::Result<Manifest> {
        let path = self.manifest_path();
        let bytes = fs::read(path)?;

        let manifest = bincode::deserialize(&bytes).expect("deserialize manifest");

        Ok(manifest)
    }

    fn segments_dir(&self) -> PathBuf {
        self.root.join("segments")
    }

    fn segment_path(&self, id: &str) -> PathBuf {
        self.segments_dir().join(format!("{id}.bin"))
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
}
