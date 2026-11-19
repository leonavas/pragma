//! Writes core/docs/classes.md with the default palette.

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../docs/classes.md");
    let doc = pragma_css::classes_doc(&pragma_css::Theme::default());
    std::fs::write(path, doc).unwrap_or_else(|e| panic!("cannot write {path}: {e}"));
    let path = std::fs::canonicalize(path).unwrap();
    println!("escrito: {}", path.display());
}
