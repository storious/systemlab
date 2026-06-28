pub fn clean_project_gutenberg(text: &str) -> &str {
    let start = find_body_start(text).unwrap_or(0);
    let end = find_body_end(text).unwrap_or(text.len());

    if start < end { &text[start..end] } else { text }
}

fn find_body_start(text: &str) -> Option<usize> {
    let markers = [
        "*** START OF THE PROJECT GUTENBERG EBOOK",
        "*** START OF THIS PROJECT GUTENBERG EBOOK",
        "***START OF THE PROJECT GUTENBERG EBOOK",
        "START OF THE PROJECT GUTENBERG",
    ];

    markers
        .iter()
        .filter_map(|m| text.find(m).map(|idx| idx + m.len()))
        .min()
}

fn find_body_end(text: &str) -> Option<usize> {
    let markers = [
        "*** END OF THE PROJECT GUTENBERG EBOOK",
        "*** END OF THIS PROJECT GUTENBERG EBOOK",
        "***END OF THE PROJECT GUTENBERG EBOOK",
        "END OF THE PROJECT GUTENBERG",
    ];

    markers.iter().filter_map(|m| text.find(m)).min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_gutenberg_header_and_footer() {
        let text = "\
header stuff
*** START OF THE PROJECT GUTENBERG EBOOK TEST ***
real body
*** END OF THE PROJECT GUTENBERG EBOOK TEST ***
footer stuff";

        let cleaned = clean_project_gutenberg(text);

        assert!(cleaned.contains("real body"));
        assert!(!cleaned.contains("header stuff"));
        assert!(!cleaned.contains("footer stuff"));
    }

    #[test]
    fn leaves_normal_text_unchanged() {
        let text = "just normal text";

        assert_eq!(clean_project_gutenberg(text), text);
    }
}
