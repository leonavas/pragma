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
    let mut count = 0;
    for (base, files) in [(dir.to_path_buf(), embedded::STARTER), (dir.join("core"), embedded::RUNTIME), (dir.join("core/docs"), embedded::DOCS)] {
        for (rel, bytes) in files {
            let content = if base == dir && *rel == "pragma.json" { rename(bytes, &name) } else { bytes.to_vec() };
            write(&base.join(rel), &content)?;
            count += 1;
        }
    }
    write(&dir.join("core/base.css"), pragma_css::base_css().as_bytes())?;
    Ok(count + 1)
}

fn write(path: &Path, content: &[u8]) -> Result<(), String> {
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
