use std::fs;
use std::path::Path;

use crate::index::cleaner::clean_project_gutenberg;
use crate::index::memindex::Position;

pub fn parse_file(path: &Path) -> std::io::Result<Vec<(String, Position)>> {
    let content = fs::read_to_string(path)?;
    let cleaned = clean_project_gutenberg(&content);

    Ok(tokenize(cleaned))
}

pub fn tokenize(content: &str) -> Vec<(String, Position)> {
    let mut tokens = Vec::new();

    for (pos, raw) in content.split_whitespace().enumerate() {
        let word = normalize(raw);

        if !word.is_empty() {
            tokens.push((word, pos as Position));
        }
    }

    tokens
}

fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_words() {
        let tokens = tokenize("hello world");

        assert_eq!(
            tokens,
            vec![("hello".to_string(), 0), ("world".to_string(), 1),]
        );
    }

    #[test]
    fn tokenize_lowercases_words() {
        let tokens = tokenize("Rust RUST rust");

        assert_eq!(
            tokens,
            vec![
                ("rust".to_string(), 0),
                ("rust".to_string(), 1),
                ("rust".to_string(), 2),
            ]
        );
    }

    #[test]
    fn tokenize_removes_punctuation() {
        let tokens = tokenize("hello, world!");

        assert_eq!(
            tokens,
            vec![("hello".to_string(), 0), ("world".to_string(), 1),]
        );
    }

    #[test]
    fn tokenize_skips_empty_tokens() {
        let tokens = tokenize("!!! hello ???");

        assert_eq!(tokens, vec![("hello".to_string(), 1),]);
    }
}
