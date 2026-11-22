use oxc_resolver::{ResolveError, ResolveOptions, Resolver};
use pragma_diag::Diagnostic;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const UI_LIBS: &[&str] = &[
    "react",
    "react-dom",
    "preact",
    "vue",
    "svelte",
    "solid-js",
    "lit",
    "lit-html",
    "lit-element",
    "@angular/core",
    "jquery",
    "htm",
    "alpinejs",
    "@hotwired/stimulus",
    "@hotwired/turbo",
    "mithril",
    "inferno",
    "hyperapp",
];

pub use pragma_jslint::{RULE_IMPORT, RULE_PACKAGE};

pub struct Deps {
    root: PathBuf,
    listed: BTreeSet<String>,
    has_manifest: bool,
    resolver: Resolver,
    server_resolver: Resolver,
}

impl Deps {
    pub fn new(root: &Path, diags: &mut Vec<Diagnostic>) -> Deps {
        let mut listed = BTreeSet::new();
        let manifest = std::fs::read_to_string(root.join("package.json")).ok();
        if let Some(src) = &manifest {
            match serde_json::from_str::<serde_json::Value>(src) {
                Ok(v) => {
                    if let Some(deps) = v.get("dependencies").and_then(|d| d.as_object()) {
                        listed.extend(deps.keys().cloned());
                    }
                }
                Err(e) => diags.push(Diagnostic::error("package.json", e.line() as u32, e.column() as u32, "app/package", format!("invalid package.json: {e}"))),
            }
        }
        let resolver = Resolver::new(ResolveOptions {
            condition_names: ["browser", "import", "module", "default"].map(String::from).to_vec(),
            main_fields: ["browser", "module", "main"].map(String::from).to_vec(),
            alias_fields: vec![vec!["browser".into()]],
            extensions: [".js", ".mjs", ".cjs"].map(String::from).to_vec(),
            symlinks: false,
            node_path: false,
            ..ResolveOptions::default()
        });
        let server_resolver = Resolver::new(ResolveOptions {
            condition_names: ["workerd", "worker", "import", "module", "default"].map(String::from).to_vec(),
            main_fields: ["module", "main"].map(String::from).to_vec(),
            extensions: [".js", ".mjs", ".cjs", ".wasm"].map(String::from).to_vec(),
            node_path: false,
            ..ResolveOptions::default()
        });
        Deps { root: root.to_path_buf(), listed, has_manifest: manifest.is_some(), resolver, server_resolver }
    }

    pub fn check_listed(&self, spec: &str, client: bool) -> Result<(), (&'static str, String)> {
        if pragma_jslint::is_workers_module(spec) {
            return if client { Err((RULE_PACKAGE, pragma_jslint::workers_module_message(spec))) } else { Ok(()) };
        }
        let name = pragma_jslint::package_name(spec);
        if client && UI_LIBS.contains(&name) {
            return Err((RULE_IMPORT, format!("package \"{name}\" is a UI framework; in pragma markup lives in .html and the runtime owns the DOM")));
        }
        if self.listed.contains(name) {
            return Ok(());
        }
        if pragma_jslint::is_node_builtin(spec) {
            return if client { Err((RULE_PACKAGE, pragma_jslint::node_builtin_message(spec))) } else { Ok(()) };
        }
        if !self.has_manifest {
            return Err((RULE_IMPORT, format!("package \"{name}\" needs a package.json; create one with `npm init -y` and install with `npm i {name}` or `bun add {name}`")));
        }
        Err((RULE_IMPORT, format!("package \"{name}\" is not in package.json dependencies; install with `npm i {name}` or `bun add {name}`")))
    }

    pub fn resolve(&self, from: &Path, spec: &str) -> Result<PathBuf, (&'static str, String)> {
        let full = match self.resolver.resolve(self.root.join(from), spec) {
            Ok(r) => r.into_path_buf(),
            Err(ResolveError::NotFound(_)) => {
                return Err((RULE_IMPORT, format!("\"{spec}\" not found in node_modules; run `npm install` or `bun install`")));
            }
            Err(ResolveError::Ignored(_)) => return Err((RULE_PACKAGE, format!("\"{spec}\" is not available in the browser (package browser field)"))),
            Err(ResolveError::Json(e)) => {
                let pkg = pragma_jslint::package_name(spec);
                return Err((RULE_PACKAGE, format!("invalid package.json in {pkg}: {}", e.message)));
            }
            Err(e) => return Err((RULE_PACKAGE, format!("cannot resolve \"{spec}\": {e}"))),
        };
        let real = std::fs::canonicalize(&full).unwrap_or_else(|_| full.clone());
        let inside = std::fs::canonicalize(&self.root).map_or(true, |root| real.starts_with(root));
        let Some(rel) = full.strip_prefix(&self.root).ok().filter(|_| inside) else {
            return Err((RULE_PACKAGE, format!("\"{spec}\" resolves outside the project ({}); install the package in the app node_modules", real.display())));
        };
        let rel_s = crate::project::rel_str(rel);
        if ![".js", ".mjs", ".cjs"].iter().any(|e| rel_s.ends_with(e)) {
            return Err((RULE_PACKAGE, format!("\"{spec}\" resolves to {rel_s}, which is not plain JS; only JavaScript packages (.js, .mjs, .cjs) work on the client")));
        }
        Ok(full)
    }
}

impl Deps {
    /// A server import of a package file that is WebAssembly (`highs/runtime` -> `highs.wasm`).
    pub fn resolve_server_wasm(&self, from: &Path, spec: &str) -> Option<PathBuf> {
        let full = self.server_resolver.resolve(self.root.join(from), spec).ok()?.into_path_buf();
        let rel = full.strip_prefix(&self.root).ok()?;
        rel.extension().is_some_and(|e| e == "wasm").then(|| rel.to_path_buf())
    }
}

pub fn asset_path(rel: &str) -> String {
    if let Some(rest) = rel.strip_prefix("node_modules/") {
        format!("npm/{rest}")
    } else if let Some(rest) = rel.strip_prefix(".pragma/") {
        format!("core/{rest}")
    } else {
        rel.to_string()
    }
}
