use crate::index::doctable::DocId;
use crate::segment::reader::SegmentReader;

pub struct Bm25Scorer<'a> {
    reader: &'a SegmentReader,
    k1: f64,
    b: f64,
}

impl<'a> Bm25Scorer<'a> {
    pub fn new(reader: &'a SegmentReader) -> Self {
        Self {
            reader,
            k1: 1.2,
            b: 0.75,
        }
    }

    pub fn score(&self, term: &str, doc_id: DocId, tf: usize) -> f64 {
        let n = self.reader.doc_count() as f64;
        let df = self.reader.document_frequency(term) as f64;

        if n == 0.0 || df == 0.0 || tf == 0 {
            return 0.0;
        }

        let tf = tf as f64;
        let doc_len = self.reader.doc_len(doc_id) as f64;
        let avg_doc_len = self.reader.avg_doc_len();

        if avg_doc_len == 0.0 {
            return 0.0;
        }

        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

        let denom = tf + self.k1 * (1.0 - self.b + self.b * doc_len / avg_doc_len);

        idf * (tf * (self.k1 + 1.0)) / denom
    }
}
