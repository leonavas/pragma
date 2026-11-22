use pragma_cli::self_update::TARGETS;
use pragma_jslint::SHORT_OK;
use pragma_jslint::script::PRAGMA_EXPORTS;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn set<'a>(names: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
    names.into_iter().map(str::to_string).collect()
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let from = text.find(start).unwrap_or_else(|| panic!("no {start:?}")) + start.len();
    &text[from..from + text[from..].find(end).unwrap_or_else(|| panic!("no {end:?} after {start:?}"))]
}

fn backticked(text: &str) -> BTreeSet<String> {
    text.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

#[test]
fn pragma_exports_match_the_runtime_services_and_the_docs() {
    let page = read("core/runtime/page.js");
    let services = between(&page, "const services = {", "\n}");
    let keys = services.lines().map(str::trim).flat_map(|line| match line.strip_prefix("get ") {
        Some(getter) => vec![getter.split('(').next().unwrap()],
        None if line.contains(':') => vec![line.split(':').next().unwrap()],
        None => line.split(',').map(str::trim).filter(|key| !key.is_empty()).collect(),
    });
    assert_eq!(set(keys), set(PRAGMA_EXPORTS.iter().copied()), "core/runtime/page.js services");
    let import_list = |text: &str, label: &str| set(between(text, &format!("{label}`import {{ "), " } from 'pragma'").split(", "));
    assert_eq!(import_list(&read("core/starter/AGENTS.md"), "Import services from `'pragma'`: "), set(PRAGMA_EXPORTS.iter().copied()), "core/starter/AGENTS.md");
    assert_eq!(import_list(&read("core/SPEC.md"), "Services: "), set(PRAGMA_EXPORTS.iter().copied()), "core/SPEC.md");
}

#[test]
fn short_names_match_the_docs() {
    let cli = read("core/docs/cli.md");
    let listed = between(&cli, "names under 3 chars except `", "`");
    assert_eq!(set(listed.split(' ')), set(SHORT_OK.iter().copied()), "core/docs/cli.md");
    let agents = read("core/starter/AGENTS.md");
    let listed = backticked(between(&agents, "names, except ", ", and parameters"));
    assert_eq!(listed, set(SHORT_OK.iter().copied()), "core/starter/AGENTS.md");
}

#[test]
fn every_rule_the_tools_report_is_in_the_docs() {
    let cli = read("core/docs/cli.md");
    let documented = backticked(&cli);
    let accessibility = backticked(between(&cli, "## Accessibility (`a11y/*`", "\n## "));
    let crates = repo().join("core/cli/crates");
    let mut sources = String::new();
    for dir in ["pragma-jslint/src", "pragma-css/src", "pragma-compiler/src", "pragma-cli/src"] {
        for entry in std::fs::read_dir(crates.join(dir)).unwrap().flatten() {
            if entry.path().extension().is_some_and(|e| e == "rs") {
                sources.push_str(&std::fs::read_to_string(entry.path()).unwrap());
            }
        }
    }
    let mut missing = BTreeSet::new();
    for group in ["js", "html", "css", "app", "a11y", "webmcp"] {
        for (at, _) in sources.match_indices(&format!("\"{group}/")) {
            let rest = &sources[at + 1..];
            let len = group.len() + 1 + rest[group.len() + 1..].find(|c: char| !(c.is_ascii_lowercase() || c == '-')).unwrap_or(0);
            let rule = &rest[..len];
            let documented = documented.contains(rule)
                || documented.iter().any(|d| d.starts_with(&format!("{rule} ")))
                || rule.strip_prefix("a11y/").is_some_and(|name| accessibility.contains(name));
            if rest[len..].starts_with('"') && len > group.len() + 1 && !documented {
                missing.insert(rule.to_string());
            }
        }
    }
    assert!(documented.contains("internal"));
    assert_eq!(missing, BTreeSet::new(), "rules missing in core/docs/cli.md");
}

#[test]
fn the_theme_key_is_the_same_everywhere() {
    let contract = read("core/runtime/contract.js");
    let key = between(&contract, "export const THEME_KEY = '", "'");
    assert!(read("core/runtime/theme.js").contains(&format!("localStorage.setItem('{key}', ")), "core/runtime/theme.js (kept out of contract.js: importing it would split contract.js into its own chunk)");
    assert!(read("core/cli/crates/pragma-compiler/src/lib.rs").contains(&format!("localStorage.getItem('{key}')")), "the theme script of pragma-compiler");
}

#[test]
fn release_targets_agree() {
    let targets = set(TARGETS.iter().map(|t| t.2));
    let workflow = read(".github/workflows/release.yml");
    let built = workflow.lines().filter_map(|l| l.trim().strip_prefix("- target: ")).map(str::trim);
    assert_eq!(set(built), targets, "release.yml matrix");
    for (os, _, target) in TARGETS {
        let ext = if *os == "windows" { "zip" } else { "tar.gz" };
        assert!(workflow.contains(&format!("archive: pragma-{target}.{ext}")), "release.yml archive of {target}");
    }
    let install = read("install.sh");
    let installed = install.lines().filter_map(|l| l.split("TARGET=\"").nth(1)).map(|rest| rest.split('"').next().unwrap());
    assert_eq!(set(installed), set(TARGETS.iter().filter(|t| t.0 != "windows").map(|t| t.2)), "install.sh");
    let stage = read("npm/scripts/stage.mjs");
    let staged = stage.lines().filter_map(|l| l.trim().strip_prefix("target: \"")).map(|rest| rest.trim_end_matches(['"', ',']));
    assert_eq!(set(staged), targets, "npm/scripts/stage.mjs");
}
