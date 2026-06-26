use memmap2::Mmap;

use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

use crate::storage::Storage;
use crate::storage::mmap::MmapBytes;

pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    pub fn read_mmap(&self, path: &str) -> io::Result<MmapBytes> {
        let full = self.full_path(path);
        let file = File::open(full)?;

        let mmap = unsafe { Mmap::map(&file)? };

        Ok(MmapBytes { mmap })
    }
}

impl Storage for LocalStorage {
    fn create_dir_all(&self, path: &str) -> io::Result<()> {
        fs::create_dir_all(self.full_path(path))
    }

    fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let full = self.full_path(path);

        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(full, data)
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        fs::read(self.full_path(path))
    }

    fn remove_dir_all(&self, path: &str) -> io::Result<()> {
        fs::remove_dir_all(self.full_path(path))
    }

    fn exists(&self, path: &str) -> bool {
        self.full_path(path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::tempdir;

    #[test]
    fn local_storage_reads_file_with_mmap() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path());

        storage
            .write("segments/seg_000001/postings.bin", b"hello mmap")
            .unwrap();

        let mmap = storage
            .read_mmap("segments/seg_000001/postings.bin")
            .unwrap();

        assert_eq!(mmap.as_slice(), b"hello mmap");
        assert_eq!(mmap.len(), 10);
        assert!(!mmap.is_empty());
    }
}
