use serde::{Deserialize, Serialize};

use crate::doctable::{DocId, DocTable};
use crate::memindex::{InvertedIndex, Position};

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

pub fn next_segment_id(manifest: &Manifest) -> String {
    format!("seg_{:06}", manifest.segments.len() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_segment_id_uses_manifest_len() {
        let manifest = Manifest {
            segments: vec!["seg_000001".to_string(), "seg_000002".to_string()],
        };

        assert_eq!(next_segment_id(&manifest), "seg_000003");
    }
}
