use crate::doctable::DocTable;
use crate::memindex::InvertedIndex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct IndexSnapshot {
    pub doctable: DocTable,
    pub index: InvertedIndex,
}

pub fn save(path: &Path, snapshot: &IndexSnapshot) -> std::io::Result<()> {
    let bytes = bincode::serialize(snapshot).expect("serialize");

    std::fs::write(path, bytes)
}

pub fn load(path: &Path) -> std::io::Result<IndexSnapshot> {
    let bytes = std::fs::read(path)?;

    let snapshot = bincode::deserialize(&bytes).expect("deserialize");

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SearchEngine;
    use crate::query::QueryMode;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn snapshot_roundtrip_preserves_search_results() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("a.txt"), "rust memory safety").unwrap();
        fs::write(dir.path().join("b.txt"), "rust distributed system").unwrap();

        let mut engine = SearchEngine::new();
        engine.index_dir(dir.path()).unwrap();

        let snapshot = engine.into_snapshot();
        let bytes = bincode::serialize(&snapshot).unwrap();
        let restored: IndexSnapshot = bincode::deserialize(&bytes).unwrap();

        let engine = SearchEngine::from_snapshot(restored);
        let results = engine.search("rust memory", QueryMode::All);

        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("a.txt"));
    }

    #[test]
    fn snapshot_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("a.txt"), "rust memory safety").unwrap();

        let mut engine = SearchEngine::new();
        engine.index_dir(dir.path()).unwrap();

        let snapshot_path = dir.path().join("searchfs.idx");

        let snapshot = engine.into_snapshot();
        save(&snapshot_path, &snapshot).unwrap();

        let restored = load(&snapshot_path).unwrap();
        let engine = SearchEngine::from_snapshot(restored);

        let results = engine.search("rust memory", QueryMode::All);

        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("a.txt"));
    }
}
