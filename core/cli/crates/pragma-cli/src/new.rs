use std::path::Path;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

pub struct Created {
    pub written: usize,
    pub skipped: usize,
    pub non_empty: bool,
}

/// Creates the app in `dir`; keeps existing files when `dir` is not empty.
pub fn create(dir: &Path) -> Result<Created, String> {
    let mut non_empty = false;
    if dir.exists() {
        let mut entries = std::fs::read_dir(dir).map_err(|_| format!("'{}' exists and is not a directory", dir.display()))?;
        non_empty = entries.next().is_some();
    }
    let name = std::path::absolute(dir)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "app".into());
    let (mut written, mut skipped) = (0, 0);
    for (rel, bytes) in embedded::STARTER {
        let target = dir.join(rel);
        if target.exists() {
            skipped += 1;
            continue;
        }
        let content = if *rel == "pragma.json" { rename(bytes, &name) } else { bytes.to_vec() };
        write(&target, &content)?;
        written += 1;
    }
    let core = core_files();
    for (rel, content) in &core {
        let target = dir.join(rel);
        if target.exists() {
            skipped += 1;
            continue;
        }
        write(&target, content)?;
        written += 1;
    }
    Ok(Created { written, skipped, non_empty })
}

/// Everything this binary writes under `.pragma/`: (path relative to the app, content).
pub fn core_files() -> Vec<(String, &'static [u8])> {
    let runtime = embedded::RUNTIME.iter().map(|(rel, bytes)| (format!(".pragma/{rel}"), *bytes));
    let docs = embedded::DOCS.iter().map(|(rel, bytes)| (format!(".pragma/docs/{rel}"), *bytes));
    let base = std::iter::once((".pragma/base.css".to_owned(), pragma_css::base_css().as_bytes()));
    let version = std::iter::once((".pragma/version".to_owned(), concat!(env!("CARGO_PKG_VERSION"), "\n").as_bytes()));
    runtime.chain(docs).chain(base).chain(version).collect()
}

pub(crate) fn write(path: &Path, content: &[u8]) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("cannot write '{}': {e}", path.display());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(fail)?;
    }
    std::fs::write(path, content).map_err(fail)
}

/// Replaces only the "name" value, keeping the starter pragma.json formatting.
fn rename(json: &[u8], name: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(json);
    let Some(key) = text.find("\"name\"") else { return json.to_vec() };
    let after = &text[key + 6..];
    let Some(open) = after.find('"') else { return json.to_vec() };
    if after[..open].trim() != ":" {
        return json.to_vec();
    }
    let start = key + 6 + open + 1;
    let Some(len) = text[start..].find('"') else { return json.to_vec() };
    let value = serde_json::Value::String(name.to_owned()).to_string();
    format!("{}{}{}", &text[..start - 1], value, &text[start + len + 1..]).into_bytes()
}

#[cfg(test)]
mod tests {
    #[test]
    fn renames_only_name_value() {
        let out = super::rename(b"{\n  \"name\": \"starter\",\n  \"budgets\": {}\n}", "my \"shop\"");
        assert_eq!(String::from_utf8(out).unwrap(), "{\n  \"name\": \"my \\\"shop\\\"\",\n  \"budgets\": {}\n}");
    }
}
