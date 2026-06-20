use std::path::Path;

use searchfs::doctable::DocTable;
use searchfs::filecrawler::crawl;
use searchfs::fileparser::parse_file;
use searchfs::memindex::InvertedIndex;

fn main() -> std::io::Result<()> {
    let root = Path::new("./docs");

    let mut doctable = DocTable::new();
    let mut index = InvertedIndex::new();

    for path in crawl(root)? {
        let path_str = path.to_string_lossy().to_string();
        let doc_id = doctable.add_document(path_str);

        let tokens = parse_file(&path)?;
        index.add_document_tokens(doc_id, tokens);
    }

    println!("index built");

    Ok(())
}
