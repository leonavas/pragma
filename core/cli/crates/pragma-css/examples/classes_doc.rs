//! Escreve core/docs/classes.md com a paleta padrão.

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../docs/classes.md");
    let doc = pragma_css::classes_doc(&pragma_css::Theme::default());
    std::fs::write(path, doc).unwrap_or_else(|e| panic!("não consegui escrever {path}: {e}"));
    let path = std::fs::canonicalize(path).unwrap();
    println!("escrito: {}", path.display());
}
