use crate::index::doctable::DocId;
use crate::index::memindex::Position;
use std::collections::HashMap;

fn build_skips(len: usize) -> Vec<usize> {
    if len < 4 {
        return vec![];
    }

    let step = (len as f64).sqrt() as usize;
    let step = step.max(2);

    let mut skips = vec![];

    let mut i = step;
    while i < len {
        skips.push(i);
        i += step;
    }

    skips
}

pub struct PostingIterator {
    docs: Vec<(DocId, Vec<Position>)>,
    skips: Vec<usize>,
    cursor: usize,
    advances: usize,
}

impl PostingIterator {
    pub fn from_postings(postings: HashMap<DocId, Vec<Position>>) -> Self {
        let mut docs: Vec<_> = postings.into_iter().collect();
        docs.sort_by_key(|(doc_id, _)| *doc_id);

        let skips = build_skips(docs.len());

        Self {
            docs,
            skips,
            cursor: 0,
            advances: 0,
        }
    }

    pub fn current(&self) -> Option<(DocId, &[Position])> {
        self.docs
            .get(self.cursor)
            .map(|(doc_id, positions)| (*doc_id, positions.as_slice()))
    }

    pub fn advance(&mut self) {
        if self.cursor < self.docs.len() {
            self.cursor += 1;
            self.advances += 1;
        }
    }

    pub fn advance_count(&self) -> usize {
        self.advances
    }

    pub fn advance_to(&mut self, target: DocId) {
        // 1. skip jump
        while let Some(&skip_idx) = self.skips.last() {
            let (skip_doc, _) = &self.docs[skip_idx];

            if *skip_doc <= target {
                self.cursor = skip_idx;
                self.skips.pop();
            } else {
                break;
            }
        }

        // 2. linear finish
        while let Some((doc_id, _)) = self.current() {
            if doc_id >= target {
                break;
            }

            self.advance();
        }
    }

    pub fn is_done(&self) -> bool {
        self.cursor >= self.docs.len()
    }

    pub fn current_doc(&self) -> Option<DocId> {
        self.current().map(|(doc_id, _)| doc_id)
    }

    pub fn current_positions(&self) -> Option<&[Position]> {
        self.current().map(|(_, positions)| positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posting_iterator_iterates_in_doc_id_order() {
        let postings = HashMap::from([(3, vec![0]), (1, vec![2]), (2, vec![4])]);

        let mut iter = PostingIterator::from_postings(postings);

        assert_eq!(iter.current().map(|(doc_id, _)| doc_id), Some(1));
        iter.advance();

        assert_eq!(iter.current().map(|(doc_id, _)| doc_id), Some(2));
        iter.advance();

        assert_eq!(iter.current().map(|(doc_id, _)| doc_id), Some(3));
        iter.advance();

        assert!(iter.is_done());
    }

    #[test]
    fn posting_iterator_advance_to_skips_until_target() {
        let postings = HashMap::from([(1, vec![0]), (3, vec![1]), (7, vec![2])]);

        let mut iter = PostingIterator::from_postings(postings);

        iter.advance_to(4);

        assert_eq!(iter.current().map(|(doc_id, _)| doc_id), Some(7));
    }

    #[test]
    fn posting_iterator_counts_advances() {
        let postings = HashMap::from([(1, vec![0]), (3, vec![1]), (7, vec![2])]);

        let mut iter = PostingIterator::from_postings(postings);

        assert_eq!(iter.advance_count(), 0);

        iter.advance();
        iter.advance();

        assert_eq!(iter.advance_count(), 2);
    }

    #[test]
    fn posting_iterator_advance_to_reaches_target_or_next_doc() {
        let postings = HashMap::from([(1, vec![0]), (3, vec![1]), (7, vec![2]), (10, vec![3])]);

        let mut iter = PostingIterator::from_postings(postings);

        iter.advance_to(8);

        assert_eq!(iter.current_doc(), Some(10));
    }

    #[test]
    fn posting_iterator_builds_skip_pointers() {
        let postings = HashMap::from([
            (1, vec![0]),
            (2, vec![0]),
            (3, vec![0]),
            (4, vec![0]),
            (5, vec![0]),
            (6, vec![0]),
            (7, vec![0]),
            (8, vec![0]),
            (9, vec![0]),
        ]);

        let iter = PostingIterator::from_postings(postings);

        assert!(!iter.skips.is_empty());
    }

    #[test]
    fn posting_iterator_advance_to_uses_skip_pointers() {
        let postings = HashMap::from([
            (1, vec![0]),
            (2, vec![0]),
            (3, vec![0]),
            (4, vec![0]),
            (5, vec![0]),
            (6, vec![0]),
            (7, vec![0]),
            (8, vec![0]),
            (9, vec![0]),
            (10, vec![0]),
            (11, vec![0]),
            (12, vec![0]),
            (13, vec![0]),
            (14, vec![0]),
            (15, vec![0]),
            (16, vec![0]),
        ]);

        let mut iter = PostingIterator::from_postings(postings);

        iter.advance_to(15);

        assert_eq!(iter.current_doc(), Some(15));
        assert!(iter.advance_count() < 14);
    }
}
