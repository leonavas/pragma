use crate::new::{core_files, write};
use pragma_diag::Diagnostic;
use std::collections::HashSet;
use std::path::Path;

/// How the app's `core/` differs from the one embedded in this binary (paths relative to the app).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CoreDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
}

impl CoreDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }

    pub fn len(&self) -> usize {
        self.added.len() + self.updated.len() + self.removed.len()
    }

    fn paths(&self) -> impl Iterator<Item = &String> {
        self.updated.iter().chain(&self.added).chain(&self.removed)
    }
}

pub fn diff(root: &Path) -> CoreDiff {
    let files = core_files();
    let mut out = CoreDiff::default();
    for (rel, content) in &files {
        match std::fs::read(root.join(rel)) {
            Ok(current) if current == *content => {}
            Ok(_) => out.updated.push(rel.clone()),
            Err(_) => out.added.push(rel.clone()),
        }
    }
    let known: HashSet<&str> = files.iter().map(|(rel, _)| rel.as_str()).collect();
    let mut present = Vec::new();
    collect(&root.join("core"), "core", &mut present);
    present.sort();
    out.removed = present.into_iter().filter(|rel| !known.contains(rel.as_str())).collect();
    out
}

/// Makes `core/` equal to the embedded version. Writes and removes only inside `core/`.
pub fn upgrade(root: &Path) -> Result<CoreDiff, String> {
    let changes = diff(root);
    for (rel, content) in core_files() {
        if changes.added.contains(&rel) || changes.updated.contains(&rel) {
            write(&root.join(&rel), content)?;
        }
    }
    for rel in &changes.removed {
        let path = root.join(rel);
        std::fs::remove_file(&path).map_err(|e| format!("cannot remove '{}': {e}", path.display()))?;
        remove_empty_dirs(&root.join("core"), &path);
    }
    Ok(changes)
}

/// `app/core-outdated` warning for `pragma check`, when `core/` is not this binary's version.
pub fn outdated(root: &Path) -> Option<Diagnostic> {
    let changes = diff(root);
    if changes.is_empty() {
        return None;
    }
    let shown: Vec<&str> = changes.paths().take(3).map(String::as_str).collect();
    let more = changes.len().saturating_sub(shown.len());
    let more = if more > 0 { format!(" and {more} more") } else { String::new() };
    let message = format!(
        "core/ differs from the version in pragma {} ({}{more}); run `pragma upgrade`",
        env!("CARGO_PKG_VERSION"),
        shown.join(", ")
    );
    Some(Diagnostic::warning("core", 1, 1, "app/core-outdated", message))
}

/// Before `pragma build`: an outdated `core/` stops the build (the compiler may emit ops the old
/// runtime cannot render), unless `allow`. Ok: the warning to print, if any.
pub fn build_gate(root: &Path, allow: bool) -> Result<Option<Diagnostic>, (Diagnostic, String)> {
    match outdated(root) {
        None => Ok(None),
        Some(warning) if allow => Ok(Some(warning)),
        Some(mut error) => {
            error.severity = pragma_diag::Severity::Error;
            Err((error, "core/ is older than this pragma: run `pragma upgrade` first (or `pragma build --allow-outdated-core` to build with it anyway)".into()))
        }
    }
}

fn collect(dir: &Path, rel: &str, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(name.as_str(), "node_modules" | ".git" | ".DS_Store") {
            continue;
        }
        let child = format!("{rel}/{name}");
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect(&entry.path(), &child, files);
        } else {
            files.push(child);
        }
    }
}

fn remove_empty_dirs(core: &Path, file: &Path) {
    let mut dir = file.parent();
    while let Some(current) = dir {
        if current == core || !current.starts_with(core) || std::fs::remove_dir(current).is_err() {
            return;
        }
        dir = current.parent();
    }
}
