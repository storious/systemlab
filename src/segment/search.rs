use crate::index::doctable::DocId;
use crate::query::SearchResult;
use crate::segment::reader::SegmentReader;

use std::collections::HashMap;
use std::io;

pub struct SegmentSearcher<'a> {
    reader: &'a SegmentReader,
}

impl<'a> SegmentSearcher<'a> {
    pub fn new(reader: &'a SegmentReader) -> Self {
        Self { reader }
    }

    pub fn search_all(&self, terms: &[String]) -> io::Result<Vec<SearchResult>> {
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let mut ordered_terms: Vec<&String> = terms.iter().collect();

        ordered_terms.sort_by_key(|term| self.reader.term_df(term).unwrap_or(usize::MAX));

        let first_term = ordered_terms[0];

        let Some(first_postings) = self.reader.lookup(first_term)? else {
            return Ok(Vec::new());
        };

        let mut other_postings = Vec::new();

        for term in ordered_terms.iter().skip(1) {
            let Some(postings) = self.reader.lookup(term)? else {
                return Ok(Vec::new());
            };

            other_postings.push((*term, postings));
        }

        let mut results = Vec::new();

        for (&doc_id, first_positions) in &first_postings {
            let mut score = self.bm25(first_term, first_positions.len());
            let mut matched = true;

            for (term, postings) in &other_postings {
                let Some(positions) = postings.get(&doc_id) else {
                    matched = false;
                    break;
                };

                score += self.bm25(term, positions.len());
            }

            if !matched {
                continue;
            }

            let Some(path) = self.reader.doc_path(doc_id) else {
                continue;
            };

            results.push(SearchResult {
                doc_id,
                path: path.to_string(),
                score,
            });
        }

        self.sort_results(&mut results);
        Ok(results)
    }

    pub fn search_any(&self, terms: &[String]) -> io::Result<Vec<SearchResult>> {
        let mut merged: HashMap<DocId, SearchResult> = HashMap::new();

        for term in terms {
            let Some(postings) = self.reader.lookup(term)? else {
                continue;
            };

            for (&doc_id, positions) in &postings {
                let Some(path) = self.reader.doc_path(doc_id) else {
                    continue;
                };

                let score = self.bm25(term, positions.len());

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
        self.sort_results(&mut results);
        Ok(results)
    }

    pub fn search_phrase(&self, terms: &[String]) -> io::Result<Vec<SearchResult>> {
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let mut postings_by_term = Vec::new();

        for term in terms {
            let Some(postings) = self.reader.lookup(term)? else {
                return Ok(Vec::new());
            };

            postings_by_term.push(postings);
        }

        let first_postings = &postings_by_term[0];
        let mut results = Vec::new();

        for (&doc_id, first_positions) in first_postings {
            let mut phrase_count = 0;

            for &start_pos in first_positions {
                let matched =
                    postings_by_term
                        .iter()
                        .enumerate()
                        .skip(1)
                        .all(|(offset, postings)| {
                            postings.get(&doc_id).is_some_and(|positions| {
                                positions.contains(&(start_pos + offset as u64))
                            })
                        });

                if matched {
                    phrase_count += 1;
                }
            }

            if phrase_count == 0 {
                continue;
            }

            let Some(path) = self.reader.doc_path(doc_id) else {
                continue;
            };

            results.push(SearchResult {
                doc_id,
                path: path.to_string(),
                score: phrase_count as f64,
            });
        }

        self.sort_results(&mut results);
        Ok(results)
    }

    fn sort_results(&self, results: &mut [SearchResult]) {
        results.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.path.cmp(&b.path))
        });
    }

    fn bm25(&self, term: &str, tf: usize) -> f64 {
        let n = self.reader.doc_count() as f64;
        let df = self.reader.document_frequency(term) as f64;

        if n == 0.0 || df == 0.0 || tf == 0 {
            return 0.0;
        }

        let k1 = 1.2;

        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
        let tf = tf as f64;

        idf * ((tf * (k1 + 1.0)) / (tf + k1))
    }
}

#[cfg(test)]
mod tests {

    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::segment::format::Segment;
    use crate::segment::reader::SegmentReader;
    use crate::segment::search::SegmentSearcher;
    use crate::segment::store::SegmentStore;

    use tempfile::tempdir;

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
        let results = SegmentSearcher::new(&reader)
            .search_all(&["rust".to_string(), "memory".to_string()])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
    }

    #[test]
    fn search_reader_any_returns_docs_with_any_term() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());
        let doc3 = doctable.add_document("c.txt".to_string());

        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            doc1,
            vec![("rust".to_string(), 0), ("memory".to_string(), 1)],
        );

        index.add_document_tokens(doc2, vec![("rust".to_string(), 0), ("rust".to_string(), 1)]);

        index.add_document_tokens(doc3, vec![("python".to_string(), 0)]);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        let terms = vec!["memory".to_string(), "python".to_string()];
        let results = SegmentSearcher::new(&reader).search_any(&terms).unwrap();

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["a.txt", "c.txt"]);
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
        let results = SegmentSearcher::new(&reader).search_phrase(&terms).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
        assert_eq!(results[0].score, 2.0);
    }

    #[test]
    fn bm25_saturates_term_frequency() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();

        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());

        let mut index = InvertedIndex::new();

        index.add_document_tokens(doc1, vec![("rust".to_string(), 0)]);

        let mut tokens = Vec::new();

        for pos in 0..100 {
            tokens.push(("rust".to_string(), pos));
        }

        index.add_document_tokens(doc2, tokens);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        let searcher = SegmentSearcher::new(&reader);

        let score1 = searcher.bm25("rust", 1);
        let score100 = searcher.bm25("rust", 100);

        assert!(score100 > score1);

        assert!(score100 < score1 * 20.0);
    }
}
