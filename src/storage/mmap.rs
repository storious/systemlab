use memmap2::Mmap;

pub struct MmapBytes {
    pub(crate) mmap: Mmap,
}

impl MmapBytes {
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap
    }

    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }
}
