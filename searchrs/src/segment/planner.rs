use crate::segment::reader::SegmentReader;

pub struct QueryPlan<'a> {
    terms: Vec<&'a str>,
}

impl<'a> QueryPlan<'a> {
    pub fn for_all_terms(reader: &SegmentReader, terms: &'a [String]) -> Self {
        let mut terms: Vec<&str> = terms.iter().map(|term| term.as_str()).collect();

        terms.sort_by_key(|term| reader.term_df(term).unwrap_or(usize::MAX));

        Self { terms }
    }

    pub fn terms(&self) -> &[&'a str] {
        &self.terms
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    pub fn first_term(&self) -> Option<&'a str> {
        self.terms.first().copied()
    }

    pub fn remaining_terms(&self) -> &[&'a str] {
        if self.terms.len() <= 1 {
            &[]
        } else {
            &self.terms[1..]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::segment::format::Segment;
    use crate::segment::store::SegmentStore;
    use tempfile::tempdir;

    #[test]
    fn query_plan_orders_terms_by_document_frequency() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());

        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            doc1,
            vec![("common".to_string(), 0), ("rare".to_string(), 1)],
        );

        index.add_document_tokens(doc2, vec![("common".to_string(), 0)]);

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = store.open_reader("seg_000001").unwrap();

        let terms = vec!["common".to_string(), "rare".to_string()];
        let plan = QueryPlan::for_all_terms(&reader, &terms);

        assert_eq!(plan.terms(), &["rare", "common"]);
    }
}
