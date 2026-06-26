use std::cell::RefCell;
use std::collections::HashMap;
use std::io;

use crate::storage::Storage;

#[derive(Default)]
pub struct MemoryStorage {
    files: RefCell<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemoryStorage {
    fn create_dir_all(&self, _path: &str) -> io::Result<()> {
        Ok(())
    }

    fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_string(), data.to_vec());

        Ok(())
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.files.borrow().get(path).cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("memory storage path not found: {path}"),
            )
        })
    }

    fn remove_dir_all(&self, path: &str) -> io::Result<()> {
        let prefix = format!("{path}/");

        self.files
            .borrow_mut()
            .retain(|key, _| key != path && !key.starts_with(&prefix));

        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        let prefix = format!("{path}/");

        self.files
            .borrow()
            .keys()
            .any(|key| key == path || key.starts_with(&prefix))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_storage_writes_and_reads_file() {
        let storage = MemoryStorage::new();

        storage
            .write("segments/seg_000001/docs.bin", b"hello")
            .unwrap();

        assert_eq!(
            storage.read("segments/seg_000001/docs.bin").unwrap(),
            b"hello"
        );
    }

    #[test]
    fn memory_storage_reports_directory_exists() {
        let storage = MemoryStorage::new();

        storage
            .write("segments/seg_000001/docs.bin", b"hello")
            .unwrap();

        assert!(storage.exists("segments"));
        assert!(storage.exists("segments/seg_000001"));
        assert!(storage.exists("segments/seg_000001/docs.bin"));
        assert!(!storage.exists("segments/seg_000002"));
    }

    #[test]
    fn memory_storage_removes_directory_tree() {
        let storage = MemoryStorage::new();

        storage.write("segments/seg_000001/docs.bin", b"a").unwrap();
        storage
            .write("segments/seg_000001/terms.bin", b"b")
            .unwrap();
        storage.write("segments/seg_000002/docs.bin", b"c").unwrap();

        storage.remove_dir_all("segments/seg_000001").unwrap();

        assert!(!storage.exists("segments/seg_000001"));
        assert!(storage.exists("segments/seg_000002"));
    }
}
