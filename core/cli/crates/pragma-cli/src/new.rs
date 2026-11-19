use std::path::Path;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// Creates the app in `dir`; returns how many files were written.
pub fn create(dir: &Path) -> Result<usize, String> {
    if dir.exists() {
        let mut entries = std::fs::read_dir(dir).map_err(|_| format!("'{}' exists and is not a directory", dir.display()))?;
        if entries.next().is_some() {
            return Err(format!("directory '{}' is not empty", dir.display()));
        }
    }
    let name = std::path::absolute(dir)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "app".into());
    for (rel, bytes) in embedded::STARTER {
        let content = if *rel == "pragma.json" { rename(bytes, &name) } else { bytes.to_vec() };
        write(&dir.join(rel), &content)?;
    }
    let core = core_files();
    for (rel, content) in &core {
        write(&dir.join(rel), content)?;
    }
    Ok(embedded::STARTER.len() + core.len())
}

/// Everything this binary writes under `core/`: (path relative to the app, content).
pub fn core_files() -> Vec<(String, &'static [u8])> {
    let runtime = embedded::RUNTIME.iter().map(|(rel, bytes)| (format!("core/{rel}"), *bytes));
    let docs = embedded::DOCS.iter().map(|(rel, bytes)| (format!("core/docs/{rel}"), *bytes));
    let base = std::iter::once(("core/base.css".to_owned(), pragma_css::base_css().as_bytes()));
    runtime.chain(docs).chain(base).collect()
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
