use crate::index::doctable::DocId;
use crate::query::{SearchResult, TopKCollector};
use crate::segment::reader::SegmentReader;
use crate::segment::scorer::Bm25Scorer;

use std::collections::HashMap;
use std::io;

pub struct SegmentSearcher<'a> {
    reader: &'a SegmentReader,
    scorer: Bm25Scorer<'a>,
}

impl<'a> SegmentSearcher<'a> {
    pub fn new(reader: &'a SegmentReader) -> Self {
        Self {
            reader,
            scorer: Bm25Scorer::new(reader),
        }
    }

    pub fn search_all_topk(&self, terms: &[String], limit: usize) -> io::Result<Vec<SearchResult>> {
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

        let mut collector = TopKCollector::new(limit);

        for (&doc_id, first_positions) in &first_postings {
            let mut score = self.scorer.score(first_term, doc_id, first_positions.len());
            let mut matched = true;

            for (term, postings) in &other_postings {
                let Some(positions) = postings.get(&doc_id) else {
                    matched = false;
                    break;
                };

                score += self.scorer.score(term, doc_id, positions.len());
            }

            if !matched {
                continue;
            }

            let Some(path) = self.reader.doc_path(doc_id) else {
                continue;
            };

            collector.collect(SearchResult {
                doc_id,
                path: path.to_string(),
                score,
            });
        }

        Ok(collector.into_sorted_vec())
    }

    pub fn search_any_topk(&self, terms: &[String], limit: usize) -> io::Result<Vec<SearchResult>> {
        let mut merged: HashMap<DocId, SearchResult> = HashMap::new();

        for term in terms {
            let Some(postings) = self.reader.lookup(term)? else {
                continue;
            };

            for (&doc_id, positions) in &postings {
                let Some(path) = self.reader.doc_path(doc_id) else {
                    continue;
                };

                let score = self.scorer.score(term, doc_id, positions.len());

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

        let mut collector = TopKCollector::new(limit);

        for result in merged.into_values() {
            collector.collect(result);
        }

        Ok(collector.into_sorted_vec())
    }

    pub fn search_phrase_topk(
        &self,
        terms: &[String],
        limit: usize,
    ) -> io::Result<Vec<SearchResult>> {
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
        let mut collector = TopKCollector::new(limit);

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

            collector.collect(SearchResult {
                doc_id,
                path: path.to_string(),
                score: phrase_count as f64,
            });
        }

        Ok(collector.into_sorted_vec())
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
            let mut score = self.scorer.score(first_term, doc_id, first_positions.len());
            let mut matched = true;

            for (term, postings) in &other_postings {
                let Some(positions) = postings.get(&doc_id) else {
                    matched = false;
                    break;
                };

                score += self.scorer.score(term, doc_id, positions.len());
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

                let score = self.scorer.score(term, doc_id, positions.len());

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

        let results: Vec<_> = merged.into_values().collect();
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

        Ok(results)
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

        let mut paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();
        paths.sort();

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

        let score1 = searcher.scorer.score("rust", doc1, 1);
        let score100 = searcher.scorer.score("rust", doc2, 100);

        assert!(score100 > score1);

        assert!(score100 < score1 * 20.0);
    }
}
