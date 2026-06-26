use std::path::Path;

use crate::index::DocTable;
use crate::index::crawler;
use crate::index::memindex::IndexStats;
use crate::index::memindex::InvertedIndex;
use crate::index::parser;
use crate::query::{QueryProcessor, SearchResult};
use crate::segment::format::Segment;
use crate::snapshot::IndexSnapshot;

pub struct SearchEngine {
    doctable: DocTable,
    index: InvertedIndex,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            doctable: DocTable::new(),
            index: InvertedIndex::new(),
        }
    }

    pub fn index_dir(&mut self, root: &Path) -> std::io::Result<()> {
        for path in crawler::crawl(root)? {
            let tokens = match parser::parse_file(&path) {
                Ok(tokens) => tokens,
                Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
                    eprintln!("skip non-utf8 file: {}", path.display());
                    continue;
                }
                Err(err) => return Err(err),
            };

            let path_str = path.to_string_lossy().to_string();
            let doc_id = self.doctable.add_document(path_str);

            self.index.add_document_tokens(doc_id, tokens);
        }

        Ok(())
    }

    pub fn search(&self, query: &str, mode: crate::query::QueryMode) -> Vec<SearchResult> {
        let processor = QueryProcessor::new(&self.index, &self.doctable);
        processor.search(query, mode)
    }

    pub fn stats(&self) -> IndexStats {
        self.index.stats()
    }

    pub fn doc_count(&self) -> usize {
        self.doctable.len()
    }

    pub fn into_snapshot(self) -> IndexSnapshot {
        IndexSnapshot {
            doctable: self.doctable,
            index: self.index,
        }
    }

    pub fn from_snapshot(snapshot: IndexSnapshot) -> Self {
        Self {
            doctable: snapshot.doctable,
            index: snapshot.index,
        }
    }

    pub fn index_dir_incremental(&mut self, root: &Path) -> std::io::Result<usize> {
        let mut added = 0;

        for path in crawler::crawl(root)? {
            let path_str = path.to_string_lossy().to_string();

            if self.doctable.contains_path(&path_str) {
                continue;
            }

            let tokens = parser::parse_file(&path)?;
            let doc_id = self.doctable.add_document(path_str);

            self.index.add_document_tokens(doc_id, tokens);
            added += 1;
        }

        Ok(added)
    }

    pub fn into_segment(self, id: impl Into<String>) -> Segment {
        Segment {
            id: id.into(),
            doctable: self.doctable,
            index: self.index,
        }
    }

    pub fn from_segment(segment: Segment) -> Self {
        Self {
            doctable: segment.doctable,
            index: segment.index,
        }
    }

    pub fn merge_segment(&mut self, segment: Segment) {
        let doc_id_offset = self.doctable.max_doc_id();

        for (_doc_id, path) in segment.doctable.iter() {
            self.doctable.add_document(path.to_string());
        }

        self.index
            .merge_with_doc_id_offset(&segment.index, doc_id_offset);
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
