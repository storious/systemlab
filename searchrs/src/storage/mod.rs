pub mod local;
pub mod memory;
pub mod mmap;

use std::io;

pub trait Storage {
    fn create_dir_all(&self, path: &str) -> io::Result<()>;

    fn write(&self, path: &str, data: &[u8]) -> io::Result<()>;

    fn read(&self, path: &str) -> io::Result<Vec<u8>>;

    fn remove_dir_all(&self, path: &str) -> io::Result<()>;

    fn exists(&self, path: &str) -> bool;
}
