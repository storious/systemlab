pub fn print_usage() {
    eprintln!("usage:");
    eprintln!("  searchfs build <docs> <index>");
    eprintln!("  searchfs search <index> <query> [limit] [and|or|phrase]");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  searchfs build docs searchfs.idx");
    eprintln!("  searchfs search searchfs.idx \"rust memory\" 10 and");
    eprintln!("  searchfs search searchfs.idx '\"white whale\"' 5");
    eprintln!("  searchfs build-segment <docs> <index-dir>");
    eprintln!("  searchfs search-segments <index-dir> <query> [limit] [and|or|phrase]");
    eprintln!("  searchfs update-segment <index-dir> <docs>");
}
