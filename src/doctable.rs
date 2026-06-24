use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) type DocId = u64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocTable {
    next_id: DocId,
    id_to_path: HashMap<DocId, String>,
    path_to_id: HashMap<String, DocId>,
}

impl DocTable {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            id_to_path: HashMap::new(),
            path_to_id: HashMap::new(),
        }
    }

    pub fn add_document(&mut self, path: String) -> DocId {
        if let Some(&id) = self.path_to_id.get(&path) {
            return id;
        }

        let id = self.next_id;
        self.next_id += 1;

        self.id_to_path.insert(id, path.clone());
        self.path_to_id.insert(path, id);

        id
    }

    pub fn get_path(&self, id: DocId) -> Option<&str> {
        self.id_to_path.get(&id).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.id_to_path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id_to_path.is_empty()
    }

    pub fn contains_path(&self, path: &str) -> bool {
        self.path_to_id.contains_key(path)
    }
}

impl Default for DocTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_document_returns_new_id() {
        let mut table = DocTable::new();

        let id = table.add_document("a.txt".to_string());

        assert_eq!(id, 1);
        assert_eq!(table.get_path(id), Some("a.txt"));
    }

    #[test]
    fn add_same_document_returns_same_id() {
        let mut table = DocTable::new();

        let id1 = table.add_document("a.txt".to_string());
        let id2 = table.add_document("a.txt".to_string());

        assert_eq!(id1, id2);
    }

    #[test]
    fn different_documents_get_different_ids() {
        let mut table = DocTable::new();

        let id1 = table.add_document("a.txt".to_string());
        let id2 = table.add_document("b.txt".to_string());

        assert_ne!(id1, id2);
        assert_eq!(table.get_path(id1), Some("a.txt"));
        assert_eq!(table.get_path(id2), Some("b.txt"));
    }

    #[test]
    fn unknown_doc_id_returns_none() {
        let table = DocTable::new();

        assert_eq!(table.get_path(999), None);
    }
}
