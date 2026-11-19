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

pub const RULE_IMPORT: &str = "js/import";
pub const RULE_PACKAGE: &str = "app/pacote";

pub struct Deps {
    root: PathBuf,
    listed: BTreeSet<String>,
    has_manifest: bool,
    resolver: Resolver,
    env: &'static str,
}

pub fn is_package_file(path: &Path) -> bool {
    path.starts_with("node_modules")
}

impl Deps {
    pub fn new(root: &Path, prod: bool, diags: &mut Vec<Diagnostic>) -> Deps {
        let mut listed = BTreeSet::new();
        let manifest = std::fs::read_to_string(root.join("package.json")).ok();
        if let Some(src) = &manifest {
            match serde_json::from_str::<serde_json::Value>(src) {
                Ok(v) => {
                    if let Some(deps) = v.get("dependencies").and_then(|d| d.as_object()) {
                        listed.extend(deps.keys().cloned());
                    }
                }
                Err(e) => diags.push(Diagnostic::error("package.json", e.line() as u32, e.column() as u32, "app/pacote", format!("package.json inválido: {e}"))),
            }
        }
        let resolver = Resolver::new(ResolveOptions {
            condition_names: ["browser", "import", "module", "default"].map(String::from).to_vec(),
            main_fields: ["module", "main"].map(String::from).to_vec(),
            alias_fields: vec![vec!["browser".into()]],
            extensions: [".js", ".mjs"].map(String::from).to_vec(),
            symlinks: false,
            node_path: false,
            ..ResolveOptions::default()
        });
        Deps { root: root.to_path_buf(), listed, has_manifest: manifest.is_some(), resolver, env: if prod { "production" } else { "development" } }
    }

    pub fn check_listed(&self, spec: &str, client: bool) -> Result<(), String> {
        let name = pragma_jslint::package_name(spec);
        if client && UI_LIBS.contains(&name) {
            return Err(format!("pacote \"{name}\" é framework de UI; no pragma a marcação vive no .html e o runtime cuida do DOM"));
        }
        if self.listed.contains(name) {
            return Ok(());
        }
        if !self.has_manifest {
            return Err(format!("pacote \"{name}\" sem package.json; crie com `npm init -y` e instale com `npm i {name}` ou `bun add {name}`"));
        }
        Err(format!("pacote \"{name}\" não está em dependencies do package.json; instale com `npm i {name}` ou `bun add {name}`"))
    }

    pub fn resolve(&self, from: &Path, spec: &str) -> Result<PathBuf, (&'static str, String)> {
        let full = match self.resolver.resolve(self.root.join(from), spec) {
            Ok(r) => r.into_path_buf(),
            Err(ResolveError::NotFound(_)) => {
                return Err((RULE_IMPORT, format!("\"{spec}\" não encontrado em node_modules; rode `npm install` ou `bun install`")));
            }
            Err(ResolveError::Ignored(_)) => return Err((RULE_PACKAGE, format!("\"{spec}\" não existe no navegador (campo browser do pacote)"))),
            Err(e) => return Err((RULE_PACKAGE, format!("não foi possível resolver \"{spec}\": {e}"))),
        };
        let Ok(rel) = full.strip_prefix(&self.root) else {
            return Err((RULE_PACKAGE, format!("\"{spec}\" resolve para fora do projeto ({}); instale o pacote no node_modules do app", full.display())));
        };
        let rel_s = crate::project::rel_str(rel);
        if !(rel_s.ends_with(".js") || rel_s.ends_with(".mjs")) {
            return Err((RULE_PACKAGE, format!("\"{spec}\" resolve para {rel_s}, que não é JS puro; só pacotes ES module (.js, .mjs) funcionam no pragma")));
        }
        Ok(rel.to_path_buf())
    }

    pub fn prepare(&self, src: String) -> String {
        if src.contains("process.env.NODE_ENV") {
            src.replace("process.env.NODE_ENV", &format!("\"{}\"", self.env))
        } else {
            src
        }
    }
}

pub fn looks_commonjs(src: &str) -> bool {
    !crate::js::has_module_syntax(src) && (src.contains("module.exports") || src.contains("exports.") || src.contains("require("))
}

pub fn asset_path(rel: &str) -> String {
    match rel.strip_prefix("node_modules/") {
        Some(rest) => format!("npm/{rest}"),
        None => rel.to_string(),
    }
}
