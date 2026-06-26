use std::fs;
use std::path::{Path, PathBuf};

pub fn crawl(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    crawl_inner(root, &mut files)?;
    Ok(files)
}

fn crawl_inner(path: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if path.is_file() {
        if is_indexable_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            crawl_inner(&entry.path(), files)?;
        }
    }

    Ok(())
}

fn is_indexable_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("txt" | "html" | "htm" | "rs" | "c" | "cc" | "cpp" | "h" | "hpp" | "md")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn crawl_finds_file_in_root_dir() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("a.txt");

        fs::write(&file_path, "hello").unwrap();

        let files = crawl(dir.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&file_path));
    }

    #[test]
    fn crawl_finds_nested_files() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");

        fs::create_dir(&nested).unwrap();

        let file1 = dir.path().join("a.txt");
        let file2 = nested.join("b.txt");

        fs::write(&file1, "hello").unwrap();
        fs::write(&file2, "world").unwrap();

        let mut files = crawl(dir.path()).unwrap();
        files.sort();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
    }

    #[test]
    fn crawl_empty_dir_returns_empty_vec() {
        let dir = tempdir().unwrap();

        let files = crawl(dir.path()).unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn indexable_file_extensions_are_accepted() {
        assert!(is_indexable_file(Path::new("a.txt")));
        assert!(is_indexable_file(Path::new("a.html")));
        assert!(is_indexable_file(Path::new("main.rs")));
        assert!(is_indexable_file(Path::new("README.md")));
    }

    #[test]
    fn binary_file_extensions_are_rejected() {
        assert!(!is_indexable_file(Path::new("a.o")));
        assert!(!is_indexable_file(Path::new("a.a")));
        assert!(!is_indexable_file(Path::new("a.pdf")));
        assert!(!is_indexable_file(Path::new("a.gz")));
    }
}
