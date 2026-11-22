use crate::layout::{DEV_OUT, SKIPPED_NAMES};
use crate::new::{core_files, write};
use pragma_jslint::legacy;
use pragma_diag::Diagnostic;
use std::collections::HashSet;
use std::path::Path;

/// How the app's `.pragma/` differs from the one embedded in this binary (paths relative to the app).
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

    pub fn removes_legacy_files(&self) -> bool {
        self.removed.iter().any(|rel| is_legacy(rel))
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
    collect(&root.join(".pragma"), ".pragma", &mut present);
    present.retain(|rel| !known.contains(rel.as_str()));
    present.extend(legacy_files(root));
    present.sort();
    out.removed = present;
    out
}

/// The files an app in the old `core/` layout still has from that layout; its own files in `core/`
/// are not listed.
fn legacy_files(root: &Path) -> Vec<String> {
    if !legacy::MARKERS.iter().any(|marker| root.join(marker).is_file()) {
        return Vec::new();
    }
    legacy::FILES.iter().filter(|rel| root.join(rel).is_file()).map(|rel| rel.to_string()).collect()
}

/// Makes `.pragma/` equal to the embedded version and removes the files of the old `core/` layout,
/// its markers last: a run that stops halfway still finds the layout the next time.
pub fn upgrade(root: &Path) -> Result<CoreDiff, String> {
    let changes = diff(root);
    for (rel, content) in core_files() {
        if changes.added.contains(&rel) || changes.updated.contains(&rel) {
            write(&root.join(&rel), content)?;
        }
    }
    let (markers, rest): (Vec<&String>, Vec<&String>) = changes.removed.iter().partition(|rel| legacy::MARKERS.contains(&rel.as_str()));
    for rel in rest.into_iter().chain(markers) {
        let path = root.join(rel);
        std::fs::remove_file(&path).map_err(|e| format!("cannot remove '{}': {e}", path.display()))?;
        let base = root.join(if is_legacy(rel) { legacy::DIR } else { ".pragma" });
        remove_empty_dirs(&base, &path);
        if is_legacy(rel) && is_empty_dir(&base) {
            std::fs::remove_dir(&base).map_err(|e| format!("cannot remove '{}': {e}", base.display()))?;
        }
    }
    Ok(changes)
}

pub fn installed_version(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(".pragma/version")).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// `app/core-outdated` warning for `pragma check`, when `.pragma/` is not this binary's version.
pub fn outdated(root: &Path) -> Option<Diagnostic> {
    let changes = diff(root);
    if changes.is_empty() {
        return None;
    }
    let shown: Vec<&str> = changes.paths().take(3).map(String::as_str).collect();
    let more = changes.len().saturating_sub(shown.len());
    let more = if more > 0 { format!(" and {more} more") } else { String::new() };
    let current = env!("CARGO_PKG_VERSION");
    let message = match installed_version(root) {
        Some(from) if from != current => format!(
            ".pragma/ is at {from}, installed pragma is {current} ({}{more}); run `pragma upgrade`",
            shown.join(", ")
        ),
        _ => format!(
            ".pragma/ differs from the version in pragma {current} ({}{more}); run `pragma upgrade`",
            shown.join(", ")
        ),
    };
    Some(Diagnostic::warning(".pragma", 1, 1, "app/core-outdated", message))
}

/// Before `pragma build`: an outdated `.pragma/` stops the build (the compiler may emit ops the old
/// runtime cannot render), unless `allow`. Ok: the warning to print, if any.
pub fn build_gate(root: &Path, allow: bool) -> Result<Option<Diagnostic>, (Diagnostic, String)> {
    match outdated(root) {
        None => Ok(None),
        Some(warning) if allow => Ok(Some(warning)),
        Some(mut error) => {
            error.severity = pragma_diag::Severity::Error;
            Err((error, ".pragma/ is older than this pragma: run `pragma upgrade` first (or `pragma build --allow-outdated-core` to build with it anyway)".into()))
        }
    }
}

fn is_legacy(rel: &str) -> bool {
    Path::new(rel).starts_with(legacy::DIR)
}

fn collect(dir: &Path, rel: &str, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = format!("{rel}/{name}");
        if SKIPPED_NAMES.contains(&name.as_str()) || child == DEV_OUT {
            continue;
        }
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

fn is_empty_dir(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
}
