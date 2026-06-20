use crate::doctable::DocId;
use std::collections::HashMap;

pub(crate) type Term = String;
pub(crate) type Position = u64;

#[derive(Debug, Default)]
pub struct InvertedIndex {
    terms: HashMap<Term, HashMap<DocId, Vec<Position>>>,
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
}
