use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::doctable::{DocId, DocTable};
use crate::memindex::Position;
use crate::segment::format::{SegmentDocs, SegmentTerms, TermEntry, TermPostings};
use crate::segment::store::SegmentStore;

pub struct SegmentReader {
    id: String,
    docs: DocTable,
    terms: HashMap<String, TermEntry>,
    postings_path: PathBuf,
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

        let docs: SegmentDocs = bincode::deserialize(&docs_bytes).expect("deserialize docs");

        let terms: SegmentTerms = bincode::deserialize(&terms_bytes).expect("deserialize terms");

        let terms = terms
            .terms
            .into_iter()
            .map(|entry| (entry.term.clone(), entry))
            .collect();

        Ok(Self {
            id: id.to_string(),
            docs: docs.doctable,
            terms,
            postings_path: store.segment_postings_path(id),
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

        let postings: TermPostings = bincode::deserialize(&buf).expect("deserialize term postings");

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
        self.terms
            .get(term)
            .map(|entry| {
                let postings = self.lookup(&entry.term).ok().flatten();
                postings.map(|p| p.len()).unwrap_or(0)
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {

    use crate::doctable::DocTable;
    use crate::memindex::InvertedIndex;
    use crate::segment::format::{Manifest, Segment};
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
            segments: vec!["seg_000001".to_string()],
        };
        store.save_manifest(&manifest).unwrap();
        let cache = SegmentReaderCache::open(&store).unwrap();
        assert_eq!(cache.readers().len(), 1);
        assert_eq!(cache.readers()[0].id(), "seg_000001");
    }
}
