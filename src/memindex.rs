use crate::doctable::DocId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) type Term = String;
pub(crate) type Position = u64;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InvertedIndex {
    terms: HashMap<Term, HashMap<DocId, Vec<Position>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStats {
    pub terms: usize,
    pub postings: usize,
    pub total_positions: usize,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            terms: HashMap::new(),
        }
    }

    pub fn add_document_tokens(&mut self, doc_id: DocId, tokens: Vec<(String, Position)>) {
        for (word, pos) in tokens {
            self.terms
                .entry(word)
                .or_default()
                .entry(doc_id)
                .or_default()
                .push(pos);
        }
    }

    pub fn lookup(&self, word: &str) -> Option<&HashMap<DocId, Vec<Position>>> {
        self.terms.get(word)
    }

    pub fn contains_doc(&self, word: &str, doc_id: DocId) -> bool {
        self.lookup(word)
            .is_some_and(|postings| postings.contains_key(&doc_id))
    }

    pub fn document_frequency(&self, term: &str) -> usize {
        self.lookup(term)
            .map(|position| position.len())
            .unwrap_or(0)
    }

    pub fn stats(&self) -> IndexStats {
        let terms = self.terms.len();

        let postings = self.terms.values().map(|docs| docs.len()).sum();

        let total_positions = self
            .terms
            .values()
            .flat_map(|docs| docs.values())
            .map(|positions| positions.len())
            .sum();

        IndexStats {
            terms,
            postings,
            total_positions,
        }
    }

    pub fn terms(&self) -> impl Iterator<Item = &String> {
        self.terms.keys()
    }

    pub fn postings_iter(&self) -> impl Iterator<Item = (&String, &HashMap<DocId, Vec<Position>>)> {
        self.terms.iter()
    }

    pub fn insert_postings(&mut self, term: String, postings: HashMap<DocId, Vec<Position>>) {
        self.terms.insert(term, postings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_index_has_no_terms() {
        let index = InvertedIndex::new();

        assert!(index.lookup("rust").is_none());
    }

    #[test]
    fn add_single_document_token() {
        let mut index = InvertedIndex::new();

        index.add_document_tokens(1, vec![("rust".to_string(), 0)]);

        let postings = index.lookup("rust").unwrap();

        assert_eq!(postings.get(&1), Some(&vec![0]));
    }

    #[test]
    fn add_multiple_positions_for_same_word() {
        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            1,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 3),
                ("rust".to_string(), 7),
            ],
        );

        let postings = index.lookup("rust").unwrap();

        assert_eq!(postings.get(&1), Some(&vec![0, 3, 7]));
    }

    #[test]
    fn same_word_in_multiple_documents() {
        let mut index = InvertedIndex::new();

        index.add_document_tokens(1, vec![("rust".to_string(), 0)]);

        index.add_document_tokens(2, vec![("rust".to_string(), 4)]);

        let postings = index.lookup("rust").unwrap();

        assert_eq!(postings.get(&1), Some(&vec![0]));
        assert_eq!(postings.get(&2), Some(&vec![4]));
    }

    #[test]
    fn different_words_are_indexed_separately() {
        let mut index = InvertedIndex::new();

        index.add_document_tokens(1, vec![("rust".to_string(), 0), ("memory".to_string(), 1)]);

        assert!(index.lookup("rust").is_some());
        assert!(index.lookup("memory").is_some());
        assert!(index.lookup("python").is_none());
    }
    #[test]
    fn stats_counts_terms_postings_and_positions() {
        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            1,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 1),
                ("memory".to_string(), 2),
            ],
        );

        index.add_document_tokens(2, vec![("rust".to_string(), 0), ("system".to_string(), 1)]);

        let stats = index.stats();

        assert_eq!(stats.terms, 3);
        assert_eq!(stats.postings, 4);
        assert_eq!(stats.total_positions, 5);
    }
}
