use pragma_diag::Diagnostic;
use pragma_jslint::Imported;
use rolldown::{
    Bundler, BundlerOptions, ChunkFilenamesOutputOption, InputItem, LegalComments, LogLevel, OutputFormat, Platform, RawMinifyOptions, ResolveOptions,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const LOCKFILES: &[&str] = &[
    "package.json",
    "package-lock.json",
    "bun.lock",
    "bun.lockb",
    "pnpm-lock.yaml",
    "yarn.lock",
    "node_modules/.package-lock.json",
    "node_modules/.modules.yaml",
];

pub struct Request {
    pub resolved: PathBuf,
    pub names: BTreeSet<String>,
    pub all: bool,
    pub bare: bool,
    pub at: (PathBuf, u32, u32),
}

impl Request {
    pub fn new(resolved: PathBuf, at: (PathBuf, u32, u32)) -> Request {
        Request { resolved, names: BTreeSet::new(), all: false, bare: false, at }
    }

    pub fn add(&mut self, what: &Imported) {
        match what {
            Imported::Name(n) => {
                self.names.insert(n.clone());
            }
            Imported::All => self.all = true,
            Imported::Bare => self.bare = true,
        }
    }
}

pub struct Bundle {
    pub files: Vec<(String, String)>,
    pub entries: BTreeMap<String, String>,
    pub aliases: BTreeMap<String, u32>,
}

struct Group {
    name: String,
    specs: Vec<(String, Option<u32>)>,
}

pub fn file_stem(package: &str) -> String {
    package.trim_start_matches('@').replace('/', "__")
}

fn groups(requests: &BTreeMap<String, Request>) -> Vec<Group> {
    let mut by_pkg: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
    for spec in requests.keys() {
        by_pkg.entry(pragma_jslint::package_name(spec)).or_default().push(spec);
    }
    let mut out = Vec::new();
    for (pkg, mut specs) in by_pkg {
        specs.sort_by_key(|s| (s.as_str() != pkg, s.as_str()));
        let mut names: BTreeSet<&str> = BTreeSet::new();
        let mut star = false;
        let mut g = Group { name: file_stem(pkg), specs: Vec::new() };
        for (i, spec) in specs.into_iter().enumerate() {
            let r = &requests[spec];
            let plain = !star && (i == 0 || (!r.all && r.names.iter().all(|n| !names.contains(n.as_str()))));
            if plain {
                star |= r.all;
                names.extend(r.names.iter().map(String::as_str));
            }
            g.specs.push((spec.clone(), if plain { None } else { Some(i as u32) }));
        }
        out.push(g);
    }
    out
}

fn has_default(path: &Path) -> bool {
    let Ok(src) = fs::read_to_string(path) else { return false };
    if !crate::js::has_module_syntax(&src) {
        return true;
    }
    pragma_jslint::exported_names(&src).iter().any(|n| n == "default")
}

fn entry_source(g: &Group, requests: &BTreeMap<String, Request>) -> String {
    let mut src = String::new();
    for (spec, alias) in &g.specs {
        let r = &requests[spec];
        let path = serde_json::to_string(&r.resolved.to_string_lossy()).unwrap();
        let mut names: Vec<String> = r.names.iter().cloned().collect();
        match alias {
            None => {
                if r.all {
                    src.push_str(&format!("export * from {path};\n"));
                    if !r.names.contains("default") && has_default(&r.resolved) {
                        names.push("default".into());
                    }
                }
            }
            Some(k) => {
                if r.all {
                    src.push_str(&format!("export * as {} from {path};\n", pragma_jslint::package_alias(*k, None)));
                }
                names = names.iter().map(|n| format!("{n} as {}", pragma_jslint::package_alias(*k, Some(n)))).collect();
            }
        }
        if !names.is_empty() {
            src.push_str(&format!("export {{ {} }} from {path};\n", names.join(", ")));
        }
        if r.bare && names.is_empty() && !r.all {
            src.push_str(&format!("import {path};\n"));
        }
    }
    src
}

fn mtime(path: &Path) -> String {
    match fs::metadata(path) {
        Ok(m) => format!("{}:{:?}", m.len(), m.modified().ok()),
        Err(_) => "-".into(),
    }
}

fn cache_key(root: &Path, prod: bool, entries: &[(String, String)], requests: &BTreeMap<String, Request>) -> String {
    let mut h = crate::Fnv::new();
    h.write(env!("CARGO_PKG_VERSION").as_bytes());
    h.write(if prod { b"prod" } else { b"dev" });
    for (name, src) in entries {
        h.write(name.as_bytes());
        h.write(src.as_bytes());
    }
    for f in LOCKFILES {
        h.write(mtime(&root.join(f)).as_bytes());
    }
    let pkgs: BTreeSet<&str> = requests.keys().map(|s| pragma_jslint::package_name(s)).collect();
    for p in pkgs {
        h.write(mtime(&root.join("node_modules").join(p).join("package.json")).as_bytes());
    }
    crate::short_hash(h.finish())
}

fn load_cache(dir: &Path, key: &str) -> Option<Bundle> {
    if fs::read_to_string(dir.join("key")).ok()? != key {
        return None;
    }
    let meta: serde_json::Value = serde_json::from_str(&fs::read_to_string(dir.join("bundle.json")).ok()?).ok()?;
    let entries = meta.get("entries")?.as_object()?.iter().map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect::<Option<BTreeMap<_, _>>>()?;
    let mut files = Vec::new();
    for f in meta.get("files")?.as_array()? {
        let name = f.as_str()?;
        files.push((name.to_string(), fs::read_to_string(dir.join("out").join(name)).ok()?));
    }
    Some(Bundle { files, entries, aliases: BTreeMap::new() })
}

fn save_cache(dir: &Path, key: &str, b: &Bundle) {
    let out = dir.join("out");
    let _ = fs::remove_dir_all(&out);
    let _ = fs::create_dir_all(&out);
    for (name, code) in &b.files {
        let _ = fs::write(out.join(name), code);
    }
    let meta = serde_json::json!({ "entries": b.entries, "files": b.files.iter().map(|f| &f.0).collect::<Vec<_>>() });
    let _ = fs::write(dir.join("bundle.json"), meta.to_string());
    let _ = fs::write(dir.join("key"), key);
}

fn package_of(text: &str) -> Option<String> {
    let rest = &text[text.rfind("node_modules/")? + "node_modules/".len()..];
    let mut parts = rest.split('/');
    let first = parts.next()?;
    let name = if first.starts_with('@') { format!("{first}/{}", parts.next()?) } else { first.to_string() };
    Some(name.trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')).to_string())
}

fn locate<'a>(text: &str, requests: &'a BTreeMap<String, Request>) -> &'a (PathBuf, u32, u32) {
    let pkg = package_of(text);
    requests
        .iter()
        .find(|(spec, _)| pkg.as_deref() == Some(pragma_jslint::package_name(spec)))
        .or_else(|| requests.iter().find(|(spec, _)| text.contains(&format!("\"{spec}\"")) || text.contains(&format!("'{spec}'"))))
        .or_else(|| requests.iter().next())
        .map(|(_, r)| &r.at)
        .unwrap()
}

fn is_pure_js(id: &str) -> bool {
    id.starts_with('\0') || id.starts_with("rolldown:") || [".js", ".mjs", ".cjs", ".json"].iter().any(|e| id.ends_with(e))
}

pub fn bundle(root: &Path, cache_dir: &Path, prod: bool, requests: &BTreeMap<String, Request>, diags: &mut Vec<Diagnostic>) -> Option<Bundle> {
    if requests.is_empty() {
        return Some(Bundle { files: Vec::new(), entries: BTreeMap::new(), aliases: BTreeMap::new() });
    }
    let groups = groups(requests);
    let sources: Vec<(String, String)> = groups.iter().map(|g| (g.name.clone(), entry_source(g, requests))).collect();
    let key = cache_key(root, prod, &sources, requests);
    let aliases: BTreeMap<String, u32> = groups.iter().flat_map(|g| g.specs.iter().filter_map(|(s, a)| a.map(|a| (s.clone(), a)))).collect();
    if let Some(mut b) = load_cache(cache_dir, &key) {
        b.aliases = aliases;
        return Some(b);
    }
    let entry_dir = cache_dir.join("entries");
    let _ = fs::remove_dir_all(&entry_dir);
    if let Err(e) = fs::create_dir_all(&entry_dir) {
        diags.push(Diagnostic::error("package.json", 1, 1, crate::deps::RULE_PACKAGE, format!("cannot prepare packages in {}: {e}", entry_dir.display())));
        return None;
    }
    let mut input = Vec::new();
    for (name, src) in &sources {
        let path = entry_dir.join(format!("{name}.js"));
        let _ = fs::write(&path, src);
        input.push(InputItem { name: Some(name.clone()), import: path.to_string_lossy().into_owned() });
    }
    let env = if prod { "\"production\"" } else { "\"development\"" };
    let options = BundlerOptions {
        input: Some(input),
        cwd: Some(root.to_path_buf()),
        platform: Some(Platform::Browser),
        format: Some(OutputFormat::Esm),
        entry_filenames: Some(ChunkFilenamesOutputOption::String("[name].js".into())),
        chunk_filenames: Some(ChunkFilenamesOutputOption::String("_[name].js".into())),
        minify: Some(if prod { RawMinifyOptions::Bool(true) } else { RawMinifyOptions::DeadCodeEliminationOnly }),
        define: Some([("process.env.NODE_ENV".to_string(), env.to_string())].into_iter().collect()),
        resolve: Some(ResolveOptions {
            condition_names: Some(["browser", "import", "module", "default"].map(String::from).to_vec()),
            main_fields: Some(["browser", "module", "main"].map(String::from).to_vec()),
            ..ResolveOptions::default()
        }),
        legal_comments: Some(LegalComments::None),
        log_level: Some(LogLevel::Silent),
        ..BundlerOptions::default()
    };
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            rt.block_on(async {
                let mut bundler = Bundler::new(options).map_err(|e| e.to_string())?;
                let out = bundler.generate().await;
                let ids: Vec<String> = bundler.module_infos().iter().map(|e| e.key().to_string()).collect();
                let _ = bundler.close().await;
                Ok::<_, String>(out.map(|o| (o, ids)))
            })
        })
        .join()
        .unwrap_or_else(|_| Err("the package bundler crashed".into()))
    });
    let (out, ids) = match result {
        Ok(Ok(out)) => out,
        Ok(Err(errs)) => {
            for e in errs.into_vec() {
                let text = e.to_string();
                let (file, line, col) = locate(&format!("{} {text}", e.id().unwrap_or_default()), requests);
                let rule = if e.kind().to_string() == "MISSING_EXPORT" { crate::deps::RULE_IMPORT } else { crate::deps::RULE_PACKAGE };
                diags.push(Diagnostic::error(file, *line, *col, rule, format!("package: {}", text.trim())));
            }
            return None;
        }
        Err(e) => {
            diags.push(Diagnostic::error("package.json", 1, 1, crate::deps::RULE_PACKAGE, e));
            return None;
        }
    };
    let before = diags.len();
    for id in &ids {
        if !is_pure_js(id) {
            let (file, line, col) = locate(id, requests);
            let rel = id.strip_prefix(&format!("{}/", root.display())).unwrap_or(id);
            diags.push(Diagnostic::error(file, *line, *col, crate::deps::RULE_PACKAGE, format!("the package uses {rel}, which is not plain JS; only JavaScript packages work on the client")));
        }
    }
    for w in &out.warnings {
        let kind = w.kind().to_string();
        if matches!(kind.as_str(), "UNRESOLVED_IMPORT" | "MISSING_EXPORT" | "IMPORT_IS_UNDEFINED" | "MISSING_GLOBAL_NAME") {
            let text = w.to_string();
            let (file, line, col) = locate(&format!("{} {text}", w.id().unwrap_or_default()), requests);
            let rule = if kind == "UNRESOLVED_IMPORT" { crate::deps::RULE_PACKAGE } else { crate::deps::RULE_IMPORT };
            diags.push(Diagnostic::error(file, *line, *col, rule, format!("package: {}", text.trim())));
        }
    }
    let mut files = Vec::new();
    let mut entries = BTreeMap::new();
    let by_name: BTreeMap<&str, &Group> = groups.iter().map(|g| (g.name.as_str(), g)).collect();
    for o in &out.assets {
        match o {
            rolldown_common::Output::Chunk(c) => {
                if c.is_entry
                    && let Some(g) = by_name.get(c.name.as_str())
                {
                    for (spec, _) in &g.specs {
                        entries.insert(spec.clone(), c.filename.to_string());
                    }
                }
                files.push((c.filename.to_string(), c.code.clone()));
            }
            rolldown_common::Output::Asset(a) => {
                let (file, line, col) = locate(&a.filename, requests);
                diags.push(Diagnostic::error(file, *line, *col, crate::deps::RULE_PACKAGE, format!("the package emits {}, which is not JS; package CSS and assets do not work on the client", a.filename)));
            }
        }
    }
    if diags.len() > before {
        return None;
    }
    let b = Bundle { files, entries, aliases };
    save_cache(cache_dir, &key, &b);
    Some(b)
}
