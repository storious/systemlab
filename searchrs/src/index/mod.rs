pub mod cleaner;
pub mod crawler;
pub mod doctable;
pub mod memindex;
pub mod parser;

pub use doctable::DocTable;
pub use memindex::{IndexStats, InvertedIndex};
pub use parser::tokenize;
