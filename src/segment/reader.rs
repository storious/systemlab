use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::index::doctable::{DocId, DocTable};
use crate::index::memindex::Position;
use crate::segment::format::{SegmentDocs, SegmentMeta, SegmentTerms, TermEntry, TermPostings};
use crate::segment::store::SegmentStore;

pub struct SegmentReader {
    id: String,
    docs: DocTable,
    terms: HashMap<String, TermEntry>,
    postings_path: PathBuf,
    meta: SegmentMeta,
    doc_lens: HashMap<DocId, usize>,
}

pub struct SegmentReaderCache {
    readers: Vec<SegmentReader>,
}

impl SegmentReaderCache {
    pub fn open(store: &SegmentStore) -> std::io::Result<Self> {
        let manifest = store.load_manifest()?;

        let mut readers = Vec::new();

        for segment_id in manifest.segments {
            readers.push(SegmentReader::open(store, &segment_id)?);
        }

        Ok(Self { readers })
    }

    pub fn readers(&self) -> &[SegmentReader] {
        &self.readers
    }
}

impl SegmentReader {
    pub fn open(store: &SegmentStore, id: &str) -> std::io::Result<Self> {
        let docs_bytes = fs::read(store.segment_docs_path(id))?;
        let terms_bytes = fs::read(store.segment_terms_path(id))?;

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("deserialize segment docs: {err}"),
            )
        })?;

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("deserialize segment terms: {err}"),
            )
        })?;

        let terms = terms
            .terms
            .into_iter()
            .map(|entry| (entry.term.clone(), entry))
            .collect();

        let meta = store.load_segment_meta(id)?;

        let docmeta = store.load_segment_docmeta(id)?;

        let doc_lens = docmeta
            .docs
            .into_iter()
            .map(|entry| (entry.doc_id, entry.doc_len))
            .collect();

        Ok(Self {
            id: id.to_string(),
            docs: docs.doctable,
            terms,
            postings_path: store.segment_postings_path(id),
            meta,
            doc_lens,
        })
    }

    pub fn lookup(&self, term: &str) -> std::io::Result<Option<HashMap<DocId, Vec<Position>>>> {
        let Some(entry) = self.terms.get(term) else {
            return Ok(None);
        };

        let mut file = fs::File::open(&self.postings_path)?;

        file.seek(SeekFrom::Start(entry.offset))?;

        let mut buf = vec![0u8; entry.len as usize];
        file.read_exact(&mut buf)?;

        let postings: TermPostings = bincode::deserialize(&buf).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("deserialize term postings: {err}"),
            )
        })?;

        Ok(Some(postings.docs.into_iter().collect()))
    }

    pub fn doc_path(&self, doc_id: DocId) -> Option<&str> {
        self.docs.get_path(doc_id)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    pub fn document_frequency(&self, term: &str) -> usize {
        self.term_df(term).unwrap_or(0)
    }

    pub fn term_df(&self, term: &str) -> Option<usize> {
        self.terms.get(term).map(|entry| entry.doc_freq)
    }

    pub fn position_count(&self) -> usize {
        self.meta.position_count
    }

    pub fn doc_len(&self, doc_id: DocId) -> usize {
        self.doc_lens.get(&doc_id).copied().unwrap_or(0)
    }

    pub fn avg_doc_len(&self) -> f64 {
        if self.meta.doc_count == 0 {
            return 0.0;
        }

        self.meta.position_count as f64 / self.meta.doc_count as f64
    }

    pub fn term_count(&self) -> usize {
        self.meta.term_count
    }

    pub fn posting_count(&self) -> usize {
        self.meta.posting_count
    }
}

#[cfg(test)]
mod tests {

    use crate::index::doctable::DocTable;
    use crate::index::memindex::InvertedIndex;
    use crate::segment::format::{MANIFEST_VERSION, Manifest, Segment};
    use crate::segment::reader::{SegmentReader, SegmentReaderCache};
    use crate::segment::store::SegmentStore;

    use tempfile::tempdir;

    #[test]
    fn segment_reader_lookup_reads_single_term_postings() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());
        let mut index = InvertedIndex::new();
        index.add_document_tokens(
            1,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 2),
                ("memory".to_string(), 1),
            ],
        );
        let mut doctable = DocTable::new();
        doctable.add_document("a.txt".to_string());
        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };
        store.save_segment(&segment).unwrap();
        let reader = SegmentReader::open(&store, "seg_000001").unwrap();
        let postings = reader.lookup("rust").unwrap().unwrap();
        assert_eq!(postings.get(&1), Some(&vec![0, 2]));
        assert_eq!(reader.lookup("missing").unwrap(), None);
    }

    #[test]
    fn segment_reader_cache_opens_all_segments() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());
        let mut doctable = DocTable::new();
        let doc_id = doctable.add_document("a.txt".to_string());
        let mut index = InvertedIndex::new();
        index.add_document_tokens(doc_id, vec![("rust".to_string(), 0)]);
        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };
        store.save_segment(&segment).unwrap();
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            segments: vec!["seg_000001".to_string()],
        };
        store.save_manifest(&manifest).unwrap();
        let cache = SegmentReaderCache::open(&store).unwrap();
        assert_eq!(cache.readers().len(), 1);
        assert_eq!(cache.readers()[0].id(), "seg_000001");
    }

    #[test]
    fn segment_reader_loads_document_lengths() {
        let dir = tempdir().unwrap();
        let store = SegmentStore::new(dir.path());

        let mut doctable = DocTable::new();
        let doc1 = doctable.add_document("a.txt".to_string());
        let doc2 = doctable.add_document("b.txt".to_string());

        let mut index = InvertedIndex::new();

        index.add_document_tokens(
            doc1,
            vec![
                ("rust".to_string(), 0),
                ("memory".to_string(), 1),
                ("safety".to_string(), 2),
            ],
        );

        index.add_document_tokens(
            doc2,
            vec![("rust".to_string(), 0), ("system".to_string(), 1)],
        );

        let segment = Segment {
            id: "seg_000001".to_string(),
            doctable,
            index,
        };

        store.save_segment(&segment).unwrap();

        let reader = SegmentReader::open(&store, "seg_000001").unwrap();

        assert_eq!(reader.doc_len(doc1), 3);
        assert_eq!(reader.doc_len(doc2), 2);
        assert_eq!(reader.position_count(), 5);
        assert_eq!(reader.avg_doc_len(), 2.5);
    }
}
