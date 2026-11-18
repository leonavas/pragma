use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut out = String::new();
    for (name, dir, skip_top) in [("STARTER", "starter", &[][..]), ("RUNTIME", "runtime", &["test"][..]), ("DOCS", "docs", &[][..])] {
        let dir = core.join(dir).canonicalize().unwrap_or_else(|_| core.join(dir));
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files = Vec::new();
        collect(&dir, &dir, skip_top, &mut files);
        files.sort();
        writeln!(out, "pub static {name}: &[(&str, &[u8])] = &[").unwrap();
        for (rel, abs) in files {
            writeln!(out, "    ({rel:?}, include_bytes!({:?})),", abs.display().to_string()).unwrap();
        }
        writeln!(out, "];").unwrap();
    }
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(dest, out).unwrap();
}

fn collect(base: &Path, dir: &Path, skip_top: &[&str], files: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(name.as_str(), "node_modules" | ".git" | ".DS_Store") {
            continue;
        }
        if dir == base && skip_top.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            collect(base, &path, skip_top, files);
        } else {
            let rel = path.strip_prefix(base).unwrap().components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((rel, path));
        }
    }
}
