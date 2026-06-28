use serde::{Deserialize, Serialize};

use crate::index::doctable::{DocId, DocTable};
use crate::index::memindex::{InvertedIndex, Position};

use std::collections::HashMap;
use std::fmt;

pub const MANIFEST_VERSION: u32 = 1;
pub const SEGMENT_META_VERSION: u32 = 1;
pub const SEGMENT_TERMS_VERSION: u32 = 1;
pub const SEGMENT_DOC_META_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    pub doctable: DocTable,
    pub index: InvertedIndex,
}

/// Segment layout in memeroy
pub struct SegmentData {
    pub id: String,
    pub docs: DocTable,
    pub terms: HashMap<String, TermEntry>,
    pub postings: Vec<u8>,
    pub meta: SegmentMeta,
    pub doc_lens: HashMap<DocId, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
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
    pub doc_freq: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentTerms {
    pub version: u32,
    pub terms: Vec<TermEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TermPostings {
    pub docs: Vec<(DocId, Vec<Position>)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentMeta {
    pub version: u32,
    pub id: String,
    pub doc_count: usize,
    pub term_count: usize,
    pub posting_count: usize,
    pub position_count: usize,
    pub postings_size: u64,
}

impl fmt::Display for SegmentMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  docs={}", self.doc_count)?;
        writeln!(f, "  terms={}", self.term_count)?;
        writeln!(f, "  postings={}", self.posting_count)?;
        writeln!(f, "  positions={}", self.position_count)?;

        writeln!(f, "  avg_doc_len={:.2}", self.avg_doc_len())?;

        writeln!(f, "  postings_size={:.2} MB", self.postings_size_mb())?;

        Ok(())
    }
}

impl SegmentMeta {
    pub fn avg_doc_len(&self) -> f64 {
        if self.doc_count == 0 {
            return 0.0;
        }

        self.position_count as f64 / self.doc_count as f64
    }

    pub fn postings_size_mb(&self) -> f64 {
        self.postings_size as f64 / 1_000_000.0
    }

    pub fn accumulate(&mut self, other: &SegmentMeta) {
        self.doc_count += other.doc_count;
        self.term_count += other.term_count;
        self.posting_count += other.posting_count;
        self.position_count += other.position_count;
        self.postings_size += other.postings_size;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocMetaEntry {
    pub doc_id: DocId,
    pub doc_len: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentDocMeta {
    pub version: u32,
    pub docs: Vec<DocMetaEntry>,
}

pub fn next_segment_id(manifest: &Manifest) -> String {
    let next = manifest
        .segments
        .iter()
        .filter_map(|id| id.strip_prefix("seg_"))
        .filter_map(|suffix| suffix.parse::<usize>().ok())
        .max()
        .unwrap_or(0)
        + 1;

    format!("seg_{next:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_segment_id_uses_max_existing_segment_id() {
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            segments: vec!["seg_000010".to_string(), "seg_000002".to_string()],
        };

        assert_eq!(next_segment_id(&manifest), "seg_000011");
    }
}
