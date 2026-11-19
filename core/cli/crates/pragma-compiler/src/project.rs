//! Descoberta do app (SPEC §2): rotas, layout, 404, componentes, módulos, stores e config.
use crate::html::{self, Node, Pos};
use pragma_diag::Diagnostic;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct Budgets {
    pub initial_js_kb: Option<f64>,
    pub route_js_kb: Option<f64>,
    pub css_kb: Option<f64>,
}

#[derive(Debug, Default)]
pub struct Config {
    pub name: String,
    pub budgets: Budgets,
    pub budget_lines: HashMap<&'static str, u32>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub html: u32,
    pub pattern: String,
    /// Nome do template SSR e do HTML de saída, sem extensão: `index`, `loja/[id]`.
    pub out_name: String,
    /// Pasta (relativa à raiz) onde a resolução de componentes começa.
    pub dir: PathBuf,
    pub module: Option<PathBuf>,
    pub server: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub html: u32,
    pub dir: PathBuf,
    pub module: Option<PathBuf>,
    pub css: Option<PathBuf>,
    /// Pasta de onde partem as buscas dos componentes que este usa.
    pub scope: PathBuf,
}

pub struct Project {
    pub root: PathBuf,
    /// Arquivos com id (usado em `Pos.file`), relativos à raiz.
    pub files: Vec<PathBuf>,
    pub sources: Vec<String>,
    pub trees: Vec<Vec<Node>>,
    pub config: Config,
    pub layout: Option<u32>,
    pub not_found: Option<Page>,
    pub pages: Vec<Page>,
    /// Pasta de componentes (`routes/x/_components` ou `lib/components`) → nome → componente.
    pub components: HashMap<PathBuf, BTreeMap<String, Component>>,
    pub stores: BTreeMap<String, PathBuf>,
    pub theme_css: Option<String>,
    /// Todos os .js/.mjs do app (inclui core/), relativos.
    pub js_files: Vec<PathBuf>,
    pub public_files: Vec<PathBuf>,
    pub diags: Vec<Diagnostic>,
}

pub fn rel_str(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn walk(root: &Path, rel: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(root.join(rel)) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name();
        let path = rel.join(&name);
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(root, &path, out),
            Ok(_) => out.push(path),
            Err(_) => {}
        }
    }
}

pub fn component_tag(tag: &str) -> Option<(bool, &str)> {
    tag.strip_prefix("c-").map(|n| (false, n)).or_else(|| tag.strip_prefix("p-").map(|n| (true, n)))
}

pub fn is_component_tag(tag: &str) -> bool {
    component_tag(&tag.to_ascii_lowercase()).is_some()
}

pub fn comment_positions(src: &str, open: &str) -> Vec<(u32, u32)> {
    src.match_indices(open)
        .map(|(i, _)| {
            let head = &src[..i];
            let line = head.matches('\n').count() as u32 + 1;
            let col = (i - head.rfind('\n').map_or(0, |n| n + 1)) as u32 + 1;
            (line, col)
        })
        .collect()
}

pub fn is_component_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase()) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Project {
    pub fn err(&mut self, file: impl Into<PathBuf>, line: u32, col: u32, rule: &'static str, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(file, line, col, rule, msg));
    }

    pub fn err_at(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        let file = self.files[pos.file as usize].clone();
        self.diags.push(Diagnostic::error(file, pos.line, pos.col, rule, msg));
    }

    pub fn read(&self, rel: &Path) -> Option<String> {
        fs::read_to_string(self.root.join(rel)).ok()
    }

    fn add_html(&mut self, rel: PathBuf) -> u32 {
        let id = self.files.len() as u32;
        let src = self.read(&rel).unwrap_or_default();
        let (mut tree, errors) = html::parse(&src, id);
        let mut sugar_errors = Vec::new();
        crate::sugar::desugar(&mut tree, &mut sugar_errors);
        self.files.push(rel);
        self.sources.push(src);
        self.trees.push(tree);
        for e in errors {
            self.err_at(e.pos, "html/sintaxe", e.message);
        }
        for (pos, message) in sugar_errors {
            self.err_at(pos, "html/sintaxe-antiga", message);
        }
        for (line, col) in comment_positions(&self.sources[id as usize], "<!--") {
            self.err_at(Pos { file: id, line, col }, "html/comentario", "comentário é proibido; o código é a documentação");
        }
        id
    }

    pub fn load(root: &Path) -> Project {
        let mut p = Project {
            root: root.to_path_buf(),
            files: Vec::new(),
            sources: Vec::new(),
            trees: Vec::new(),
            config: Config::default(),
            layout: None,
            not_found: None,
            pages: Vec::new(),
            components: HashMap::new(),
            stores: BTreeMap::new(),
            theme_css: None,
            js_files: Vec::new(),
            public_files: Vec::new(),
            diags: Vec::new(),
        };
        p.load_config();
        let mut all = Vec::new();
        for dir in ["routes", "lib", "utils", "core"] {
            walk(root, Path::new(dir), &mut all);
        }
        walk(root, Path::new("public"), &mut p.public_files);
        if !root.join("routes").is_dir() {
            p.err("routes", 1, 1, "app/estrutura", "pasta routes/ não encontrada");
        }
        for f in &all {
            let s = rel_str(f);
            let skip = s.starts_with("core/docs/") || s.starts_with("core/test/") || s.contains("/node_modules/");
            if !skip && (s.ends_with(".js") || s.ends_with(".mjs")) {
                p.js_files.push(f.clone());
            }
        }
        p.theme_css = p.read(Path::new("lib/theme.css"));
        p.discover_components(&all);
        p.discover_routes(&all);
        p.discover_stores(&all);
        p
    }

    fn load_config(&mut self) {
        let Some(src) = self.read(Path::new("pragma.json")) else {
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json não encontrado");
            return;
        };
        let v: serde_json::Value = match serde_json::from_str(&src) {
            Ok(v) => v,
            Err(e) => {
                self.err("pragma.json", e.line() as u32, e.column() as u32, "app/pragma-json", format!("JSON inválido: {e}"));
                return;
            }
        };
        let line_of = |key: &str| -> u32 {
            let pat = format!("\"{key}\"");
            src.find(&pat).map_or(1, |i| src[..i].matches('\n').count() as u32 + 1)
        };
        let Some(obj) = v.as_object() else {
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json deve ser um objeto");
            return;
        };
        for (k, val) in obj {
            match k.as_str() {
                "name" => match val.as_str() {
                    Some(s) => self.config.name = s.to_string(),
                    None => self.err("pragma.json", line_of("name"), 1, "app/pragma-json", "\"name\" deve ser texto"),
                },
                "budgets" => {
                    let Some(b) = val.as_object() else {
                        self.err("pragma.json", line_of("budgets"), 1, "app/pragma-json", "\"budgets\" deve ser um objeto");
                        continue;
                    };
                    for (bk, bv) in b {
                        let key: &'static str = match bk.as_str() {
                            "initialJsKb" => "initialJsKb",
                            "routeJsKb" => "routeJsKb",
                            "cssKb" => "cssKb",
                            _ => {
                                let l = line_of(bk);
                                self.err("pragma.json", l, 1, "app/pragma-json", format!("orçamento desconhecido \"{bk}\" (use initialJsKb, routeJsKb, cssKb)"));
                                continue;
                            }
                        };
                        let l = line_of(key);
                        let Some(n) = bv.as_f64() else {
                            self.err("pragma.json", l, 1, "app/pragma-json", format!("\"{key}\" deve ser número"));
                            continue;
                        };
                        self.config.budget_lines.insert(key, l);
                        match key {
                            "initialJsKb" => self.config.budgets.initial_js_kb = Some(n),
                            "routeJsKb" => self.config.budgets.route_js_kb = Some(n),
                            _ => self.config.budgets.css_kb = Some(n),
                        }
                    }
                }
                _ => {
                    let l = line_of(k);
                    self.err("pragma.json", l, 1, "app/pragma-json", format!("chave desconhecida \"{k}\" (use name, budgets)"));
                }
            }
        }
        if self.config.name.is_empty() {
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json precisa de \"name\"");
        }
    }

    fn discover_components(&mut self, all: &[PathBuf]) {
        // Pastas imediatamente dentro de um diretório de componentes.
        let mut dirs: BTreeMap<PathBuf, (PathBuf, String)> = BTreeMap::new();
        for f in all {
            let comps: Vec<_> = f.ancestors().filter(|a| a.file_name().is_some_and(|n| n == "_components") || *a == Path::new("lib/components")).collect();
            let Some(base) = comps.last() else { continue };
            let Ok(rest) = f.strip_prefix(base) else { continue };
            let Some(first) = rest.components().next() else { continue };
            let name = first.as_os_str().to_string_lossy().to_string();
            if rest.components().count() == 1 {
                self.err(f, 1, 1, "app/estrutura", format!("arquivo solto em {}; use {}/<nome>/<nome>.html", rel_str(base), rel_str(base)));
                continue;
            }
            dirs.entry(base.join(&name)).or_insert((base.to_path_buf(), name));
        }
        for (dir, (base, name)) in dirs {
            if !is_component_name(&name) {
                self.err(&dir, 1, 1, "app/estrutura", format!("nome de componente inválido \"{name}\": use minúsculas, dígitos e hífen"));
                continue;
            }
            let html = dir.join(format!("{name}.html"));
            if !all.contains(&html) {
                self.err(&dir, 1, 1, "app/estrutura", format!("componente sem {name}.html"));
                continue;
            }
            let id = self.add_html(html);
            let module = Some(dir.join(format!("{name}.js"))).filter(|m| all.contains(m));
            let css = Some(dir.join(format!("{name}.css"))).filter(|m| all.contains(m));
            if all.contains(&dir.join(format!("{name}.server.js"))) {
                self.err(dir.join(format!("{name}.server.js")), 1, 1, "app/estrutura", "componente não tem .server.js; dados ficam no .server.js da página");
            }
            let scope = if base == Path::new("lib/components") { base.clone() } else { base.parent().unwrap().to_path_buf() };
            let c = Component { name: name.clone(), html: id, dir: dir.clone(), module, css, scope };
            self.components.entry(base).or_default().insert(name, c);
        }
    }

    fn discover_routes(&mut self, all: &[PathBuf]) {
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        for f in all {
            let s = rel_str(f);
            if !s.starts_with("routes/") {
                continue;
            }
            let in_private = f.components().skip(1).any(|c| c.as_os_str().to_string_lossy().starts_with('_'));
            let file_name = f.file_name().unwrap().to_string_lossy().to_string();
            if s == "routes/_layout.html" {
                self.layout = Some(self.add_html(f.clone()));
                continue;
            }
            if s == "routes/_404.html" {
                let id = self.add_html(f.clone());
                let module = Some(PathBuf::from("routes/_404.js")).filter(|m| all.contains(m));
                if all.contains(&PathBuf::from("routes/_404.server.js")) {
                    self.err("routes/_404.server.js", 1, 1, "app/estrutura", "_404 não tem loader na v0");
                }
                self.not_found = Some(Page { html: id, pattern: "*".into(), out_name: "_404".into(), dir: "routes".into(), module, server: None });
                continue;
            }
            if s == "routes/_404.js" || s == "routes/_404.server.js" {
                continue;
            }
            if in_private {
                if f.ancestors().any(|a| a.file_name().is_some_and(|n| n == "_components")) {
                    continue;
                }
                if file_name == "_layout.js" || file_name == "_layout.server.js" {
                    self.err(f, 1, 1, "app/estrutura", "o layout não tem módulo na v0");
                } else if file_name.starts_with('_') && file_name.ends_with(".html") {
                    self.err(f, 1, 1, "app/estrutura", "só routes/_layout.html e routes/_404.html são especiais; layouts aninhados estão fora da v0");
                }
                continue;
            }
            if let Some(stem) = file_name.strip_suffix(".server.js").or_else(|| file_name.strip_suffix(".js")) {
                if !all.contains(&f.with_file_name(format!("{stem}.html"))) {
                    self.err(f, 1, 1, "app/estrutura", format!("módulo sem página: falta {stem}.html"));
                }
                continue;
            }
            let Some(stem) = file_name.strip_suffix(".html") else {
                self.err(f, 1, 1, "app/estrutura", "arquivo não suportado em routes/ (estáticos vão em public/, CSS em lib/theme.css ou no componente)");
                continue;
            };
            let rel = f.strip_prefix("routes").unwrap();
            let mut segs: Vec<String> = rel.parent().into_iter().flat_map(|p| p.components()).map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
            if stem != "index" {
                segs.push(stem.to_string());
            }
            let mut pattern = String::new();
            let mut bad = false;
            for seg in &segs {
                pattern.push('/');
                if let Some(inner) = seg.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                    if !is_ident(inner) {
                        bad = true;
                    }
                    pattern.push(':');
                    pattern.push_str(inner);
                } else {
                    if seg.contains('[') || seg.contains(']') {
                        bad = true;
                    }
                    pattern.push_str(seg);
                }
            }
            if bad {
                self.err(f, 1, 1, "app/estrutura", "parâmetro de rota inválido: use [nome] com letras, dígitos e _");
                continue;
            }
            if pattern.is_empty() {
                pattern.push('/');
            }
            if stem != "index" && all.contains(&f.with_extension("").join("index.html")) {
                let other = rel_str(&f.with_extension("").join("index.html"));
                self.err(f, 1, 1, "app/rota-conflito", format!("{s} e {other} respondem pela mesma rota"));
                continue;
            }
            let shape: String = pattern.split('/').map(|p| if p.starts_with(':') { ":" } else { p }).collect::<Vec<_>>().join("/");
            if let Some(prev) = seen.get(&shape) {
                let prev = rel_str(prev);
                self.err(f, 1, 1, "app/rota-conflito", format!("rota {pattern} conflita com {prev}"));
                continue;
            }
            seen.insert(shape, f.clone());
            let id = self.add_html(f.clone());
            let module = Some(f.with_file_name(format!("{stem}.js"))).filter(|m| all.contains(m));
            let server = Some(f.with_file_name(format!("{stem}.server.js"))).filter(|m| all.contains(m));
            let out_name = rel_str(&rel.with_extension(""));
            self.pages.push(Page { html: id, pattern, out_name, dir: f.parent().unwrap().to_path_buf(), module, server });
        }
        if self.layout.is_none() && self.root.join("routes").is_dir() {
            self.err("routes/_layout.html", 1, 1, "app/estrutura", "falta routes/_layout.html com <main data-page><slot></slot></main>");
        }
        self.pages.sort_by(|a, b| route_order(&a.pattern, &b.pattern));
    }

    fn discover_stores(&mut self, all: &[PathBuf]) {
        for f in all {
            let s = rel_str(f);
            if let Some(name) = s.strip_prefix("lib/state/").and_then(|n| n.strip_suffix(".js")) {
                if name.contains('/') || !is_ident(name) {
                    self.err(f, 1, 1, "app/estrutura", "store deve ser lib/state/<nome>.js com nome identificador");
                    continue;
                }
                self.stores.insert(name.to_string(), f.clone());
            }
        }
    }

    pub fn component_search(&self, start: &Path, local: bool) -> Vec<PathBuf> {
        if !local {
            return vec![PathBuf::from("lib/components")];
        }
        let mut out = Vec::new();
        if start.starts_with("routes") {
            for a in start.ancestors() {
                out.push(a.join("_components"));
                if a == Path::new("routes") {
                    break;
                }
            }
        }
        out
    }

    pub fn find_component(&self, start: &Path, local: bool, name: &str) -> Option<&Component> {
        self.component_search(start, local).iter().find_map(|d| self.components.get(d).and_then(|m| m.get(name)))
    }

    pub fn all_components(&self) -> impl Iterator<Item = &Component> {
        self.components.values().flat_map(|m| m.values())
    }
}

/// Estático vence param, segmento a segmento.
pub fn route_order(a: &str, b: &str) -> std::cmp::Ordering {
    let sa: Vec<&str> = a.split('/').collect();
    let sb: Vec<&str> = b.split('/').collect();
    for (x, y) in sa.iter().zip(&sb) {
        let o = x.starts_with(':').cmp(&y.starts_with(':')).then_with(|| x.cmp(y));
        if o.is_ne() {
            return o;
        }
    }
    sa.len().cmp(&sb.len())
}
