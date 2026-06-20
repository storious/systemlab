use std::path::Path;

use crate::doctable::DocTable;
use crate::filecrawler;
use crate::fileparser;
use crate::memindex::InvertedIndex;
use crate::query::{QueryProcessor, SearchResult};

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
        for path in filecrawler::crawl(root)? {
            let tokens = match fileparser::parse_file(&path) {
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
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
