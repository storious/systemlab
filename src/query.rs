use crate::index::doctable::{DocId, DocTable};
use crate::index::memindex::InvertedIndex;
use crate::index::parser::tokenize;

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, PartialEq)]
pub struct SearchResult {
    pub doc_id: DocId,
    pub path: String,
    pub score: f64,
}

impl SearchResult {
    pub fn sort(results: &mut [Self]) {
        results.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.path.cmp(&b.path))
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryMode {
    All,
    Any,
    Phrase,
}

impl QueryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryMode::All => "and",
            QueryMode::Any => "or",
            QueryMode::Phrase => "phrase",
        }
    }
}

impl TryFrom<&str> for QueryMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "and" | "all" => Ok(QueryMode::All),
            "or" | "any" => Ok(QueryMode::Any),
            "phrase" => Ok(QueryMode::Phrase),
            other => Err(format!("unknown query mode: {other}")),
        }
    }
}

#[derive(Debug)]
struct ScoredDoc(SearchResult);

impl Eq for ScoredDoc {}

impl PartialEq for ScoredDoc {
    fn eq(&self, other: &Self) -> bool {
        self.0.score == other.0.score && self.0.path == other.0.path
    }
}

impl Ord for ScoredDoc {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .score
            .total_cmp(&self.0.score)
            .then_with(|| self.0.path.cmp(&other.0.path))
    }
}

impl PartialOrd for ScoredDoc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TopKCollector {
    limit: usize,
    heap: BinaryHeap<ScoredDoc>,
}

impl TopKCollector {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            heap: BinaryHeap::new(),
        }
    }

    pub fn collect(&mut self, result: SearchResult) {
        if self.limit == 0 {
            return;
        }

        self.heap.push(ScoredDoc(result));

        if self.heap.len() > self.limit {
            self.heap.pop();
        }
    }

    pub fn into_sorted_vec(self) -> Vec<SearchResult> {
        let mut results: Vec<_> = self.heap.into_iter().map(|item| item.0).collect();
        SearchResult::sort(&mut results);
        results
    }
}

/// Legacy in-memory query processor used by snapshot search.
pub struct QueryProcessor<'a> {
    index: &'a InvertedIndex,
    doctable: &'a DocTable,
}

impl<'a> QueryProcessor<'a> {
    pub fn new(index: &'a InvertedIndex, doctable: &'a DocTable) -> Self {
        Self { index, doctable }
    }

    pub fn search(&self, query: &str, mode: QueryMode) -> Vec<SearchResult> {
        let terms: Vec<String> = tokenize(query).into_iter().map(|(term, _)| term).collect();
        match mode {
            QueryMode::Any => self.search_any(&terms),
            QueryMode::All => self.search_all(&terms),
            QueryMode::Phrase => self.search_phrase(&terms),
        }
    }

    fn search_all(&self, terms: &[String]) -> Vec<SearchResult> {
        if terms.is_empty() {
            return Vec::new();
        }

        let Some(first_postings) = self.index.lookup(&terms[0]) else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for (&doc_id, positions) in first_postings {
            let matched = terms[1..].iter().all(|term| {
                self.index
                    .lookup(term)
                    .is_some_and(|p| p.contains_key(&doc_id))
            });

            if !matched {
                continue;
            }

            let mut score = self.tf_idf(&terms[0], positions.len());

            for term in &terms[1..] {
                if let Some(postings) = self.index.lookup(term) {
                    score += postings
                        .get(&doc_id)
                        .map(|p| self.tf_idf(term, p.len()))
                        .unwrap_or(0.0);
                }
            }

            let Some(path) = self.doctable.get_path(doc_id) else {
                continue;
            };

            results.push(SearchResult {
                doc_id,
                path: path.to_string(),
                score,
            });
        }

        SearchResult::sort(&mut results);
        results
    }

    fn search_any(&self, terms: &[String]) -> Vec<SearchResult> {
        let mut merged: HashMap<DocId, SearchResult> = HashMap::new();

        for term in terms {
            let Some(postings) = self.index.lookup(term) else {
                continue;
            };

            for (&doc_id, positions) in postings {
                let Some(path) = self.doctable.get_path(doc_id) else {
                    continue;
                };
                let score = self.tf_idf(term, positions.len());

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
        SearchResult::sort(&mut results);

        results
    }

    fn search_phrase(&self, terms: &[String]) -> Vec<SearchResult> {
        if terms.is_empty() {
            return Vec::new();
        }

        let Some(first_postings) = self.index.lookup(&terms[0]) else {
            return Vec::new();
        };

        let mut results = Vec::new();

        for (&doc_id, first_positions) in first_postings {
            let mut phrase_count = 0;

            for &start_pos in first_positions {
                let matched = terms.iter().enumerate().skip(1).all(|(offset, term)| {
                    self.index
                        .lookup(term)
                        .and_then(|postings| postings.get(&doc_id))
                        .is_some_and(|positions| positions.contains(&(start_pos + offset as u64)))
                });

                if matched {
                    phrase_count += 1;
                }
            }

            if phrase_count == 0 {
                continue;
            }

            let Some(path) = self.doctable.get_path(doc_id) else {
                continue;
            };

            results.push(SearchResult {
                doc_id,
                path: path.to_string(),
                score: phrase_count as f64,
            });
        }

        SearchResult::sort(&mut results);
        results
    }

    fn tf_idf(&self, term: &str, tf: usize) -> f64 {
        let total_docs = self.doctable.len() as f64;
        let df = self.index.document_frequency(term) as f64;
        if total_docs == 0.0 || df == 0.0 {
            return 0.0;
        }

        let idf = (total_docs / df).ln();

        tf as f64 * idf
    }
}

pub fn parse_query_mode(query: &str, default: QueryMode) -> (&str, QueryMode) {
    if query.starts_with('"') && query.ends_with('"') {
        (query.trim_matches('"'), QueryMode::Phrase)
    } else {
        (query, default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;

    fn build_test_engine() -> (DocTable, InvertedIndex) {
        let mut doctable = DocTable::new();
        let mut index = InvertedIndex::new();

        let doc1 = doctable.add_document("a.txt".to_string());
        index.add_document_tokens(
            doc1,
            vec![
                ("rust".to_string(), 0),
                ("memory".to_string(), 1),
                ("safety".to_string(), 2),
            ],
        );

        let doc2 = doctable.add_document("b.txt".to_string());
        index.add_document_tokens(
            doc2,
            vec![
                ("rust".to_string(), 0),
                ("distributed".to_string(), 1),
                ("system".to_string(), 2),
            ],
        );

        let doc3 = doctable.add_document("c.txt".to_string());
        index.add_document_tokens(
            doc3,
            vec![("python".to_string(), 0), ("memory".to_string(), 1)],
        );

        (doctable, index)
    }

    #[test]
    fn single_term_query_returns_matching_docs() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("rust", QueryMode::All);

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn unknown_term_returns_empty_results() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("golang", QueryMode::All);

        assert!(results.is_empty());
    }

    #[test]
    fn empty_query_returns_empty_results() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("!!!", QueryMode::All);

        assert!(results.is_empty());
    }

    #[test]
    fn query_is_normalized_for_any_mode() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("Rust!!! MEMORY", QueryMode::Any);

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn multi_term_query_returns_union_of_matching_docs() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("rust memory", QueryMode::Any);

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn rare_terms_get_positive_tfidf_score() {
        let mut doctable = DocTable::new();
        let mut index = InvertedIndex::new();

        let doc1 = doctable.add_document("a.txt".to_string());
        index.add_document_tokens(
            doc1,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 1),
                ("memory".to_string(), 2),
            ],
        );

        let doc2 = doctable.add_document("b.txt".to_string());
        index.add_document_tokens(doc2, vec![("rust".to_string(), 0)]);

        let qp = QueryProcessor::new(&index, &doctable);
        let results = qp.search("memory", QueryMode::Any);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
        assert!(results[0].score > 0.0);
    }
    #[test]
    fn phrase_query_matches_adjacent_terms() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("rust memory safety", QueryMode::Phrase);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "a.txt");
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn phrase_query_rejects_non_adjacent_terms() {
        let (doctable, index) = build_test_engine();
        let qp = QueryProcessor::new(&index, &doctable);

        let results = qp.search("rust safety", QueryMode::Phrase);

        assert!(results.is_empty());
    }

    #[test]
    fn topk_collector_keeps_highest_scoring_results() {
        let mut collector = TopKCollector::new(2);

        collector.collect(SearchResult {
            doc_id: 1,
            path: "a.txt".to_string(),
            score: 1.0,
        });

        collector.collect(SearchResult {
            doc_id: 2,
            path: "b.txt".to_string(),
            score: 3.0,
        });

        collector.collect(SearchResult {
            doc_id: 3,
            path: "c.txt".to_string(),
            score: 2.0,
        });

        let results = collector.into_sorted_vec();

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["b.txt", "c.txt"]);
    }

    #[test]
    fn topk_collector_breaks_ties_by_path() {
        let mut collector = TopKCollector::new(2);

        collector.collect(SearchResult {
            doc_id: 1,
            path: "b.txt".to_string(),
            score: 1.0,
        });

        collector.collect(SearchResult {
            doc_id: 2,
            path: "a.txt".to_string(),
            score: 1.0,
        });

        collector.collect(SearchResult {
            doc_id: 3,
            path: "c.txt".to_string(),
            score: 1.0,
        });

        let results = collector.into_sorted_vec();

        let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();

        assert_eq!(paths, vec!["a.txt", "b.txt"]);
    }
}
