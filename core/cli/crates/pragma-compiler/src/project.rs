//! App discovery (SPEC §2): routes, layout, 404, components, modules, stores and config.
use crate::html::{self, Node, Pos};
use pragma_diag::Diagnostic;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Config {
    pub name: String,
    pub lang: String,
    pub webmcp: bool,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub html: u32,
    pub pattern: String,
    /// Output name, without extension: `index`, `shop/[id]`.
    pub out_name: String,
    /// Folder (relative to the root) where component resolution starts.
    pub dir: PathBuf,
    /// `x.js` or the `<script>` of `x.html` (see `Project::embedded`).
    pub module: Option<PathBuf>,
    pub css: Option<PathBuf>,
    pub server: Option<PathBuf>,
}

/// A `.server.js` without a sibling `.html`: uppercase method handlers that return a `Response`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub pattern: String,
    pub file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    /// `data-c` value: the name for `lib/components` (global), `<name>-<folder hash>` for a
    /// `routes/**/_components` one, so same-named private components never share CSS or state.
    pub id: String,
    pub html: u32,
    pub dir: PathBuf,
    pub module: Option<PathBuf>,
    pub css: Option<PathBuf>,
    /// Folder where lookups for the components this one uses start.
    pub scope: PathBuf,
}

pub struct Project {
    pub root: PathBuf,
    /// Files with an id (used in `Pos.file`), relative to the root.
    pub files: Vec<PathBuf>,
    pub sources: Vec<String>,
    pub trees: Vec<Vec<Node>>,
    pub config: Config,
    /// `routes/_layout.html`; a bare `<main data-page><slot></slot></main>` when it is missing (an error).
    pub layout: u32,
    pub nested_layouts: HashMap<PathBuf, u32>,
    pub not_found: Option<Page>,
    pub pages: Vec<Page>,
    pub endpoints: Vec<Endpoint>,
    /// `routes/_middleware.server.js`: `handle` runs before every request.
    pub middleware: Option<PathBuf>,
    /// Component folder (`routes/x/_components` or `lib/components`) -> name -> component.
    pub components: HashMap<PathBuf, BTreeMap<String, Component>>,
    pub stores: BTreeMap<String, PathBuf>,
    pub theme_css: Option<String>,
    /// All .js/.mjs files, the framework's included, relative.
    pub js_files: Vec<PathBuf>,
    /// `<script>` / `<style>` of a page or component `.html`, under the path the separate file would
    /// have (`routes/x.js`, `lib/components/x/x.css`) -> (code padded with blank lines so lines and
    /// columns match the `.html`, the `.html`).
    pub embedded: HashMap<PathBuf, (String, PathBuf)>,
    pub public_files: Vec<PathBuf>,
    /// Folder of the framework runtime: `.pragma`, or `core` in apps made before it moved.
    pub framework: PathBuf,
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
        if rel == Path::new(".pragma") && matches!(name.to_str(), Some("dev" | "docs" | "test")) {
            continue;
        }
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
    let lines = html::LineIndex::new(src);
    src.match_indices(open).map(|(i, _)| lines.pos(src, i, 0)).map(|p| (p.line, p.col)).collect()
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
        if let Some((code, _)) = self.embedded.get(rel) {
            return Some(code.clone());
        }
        fs::read_to_string(self.root.join(rel)).ok()
    }

    /// The file a diagnostic about `rel` belongs to: the `.html` for an embedded script or style.
    pub fn origin<'a>(&'a self, rel: &'a Path) -> &'a Path {
        self.embedded.get(rel).map_or(rel, |(_, html)| html.as_path())
    }

    /// Takes the top-level `<script>` and `<style>` out of a page or component `.html`. Returns the
    /// module and CSS paths (the separate files or the embedded code).
    fn extract(&mut self, id: u32, js: PathBuf, css: PathBuf, all: &[PathBuf]) -> (Option<PathBuf>, Option<PathBuf>) {
        let html = self.files[id as usize].clone();
        let page = html.starts_with("routes") && !html.components().any(|c| c.as_os_str() == "_components");
        let (mut scripts, mut styles, mut kept) = (Vec::new(), Vec::new(), Vec::new());
        for n in std::mem::take(&mut self.trees[id as usize]) {
            match n {
                Node::Element(e) if crate::directive::is_json_ld(&e) && page => kept.push(Node::Element(e)),
                Node::Element(e) if crate::directive::is_json_ld(&e) => {
                    self.err_at(e.pos, "html/script", "JSON-LD <script type=\"application/ld+json\"> goes at the top level of a page, not in a component");
                }
                Node::Element(e) if e.is("script") => scripts.push(e),
                Node::Element(e) if e.is("style") => styles.push(e),
                n => kept.push(n),
            }
        }
        self.trees[id as usize] = kept;
        (self.embed(&html, scripts, js, all, "script"), self.embed(&html, styles, css, all, "style"))
    }

    fn embed(&mut self, html: &Path, found: Vec<html::Element>, path: PathBuf, all: &[PathBuf], tag: &str) -> Option<PathBuf> {
        let rule = if tag == "script" { "html/script" } else { "html/style-inline" };
        let on_disk = all.contains(&path);
        let mut found = found.into_iter();
        let Some(first) = found.next() else { return on_disk.then_some(path) };
        for extra in found {
            self.err_at(extra.pos, rule, format!("one <{tag}> per file"));
        }
        for a in &first.attrs {
            let module = tag == "script" && a.name.eq_ignore_ascii_case("type") && a.value.as_deref() == Some("module");
            if module {
                continue;
            }
            let msg = if tag == "script" && a.name.eq_ignore_ascii_case("src") {
                format!("no src: write the code inside <script> or in {}", rel_str(&path))
            } else if tag == "script" {
                "<script> takes no attributes: it is always an ES module".to_string()
            } else {
                "<style> takes no attributes".to_string()
            };
            self.err_at(a.pos, rule, msg);
        }
        if on_disk {
            let msg = format!("{} and the <{tag}> in {} both define it; keep one", rel_str(&path), rel_str(html));
            self.err_at(first.pos, "html/duplicate-module", msg);
            return Some(path);
        }
        let (text, pos) = match first.children.first() {
            Some(Node::Text(t, p)) => (t.clone(), *p),
            _ => (String::new(), first.pos),
        };
        let padded = format!("{}{}{text}", "\n".repeat(pos.line.saturating_sub(1) as usize), " ".repeat(pos.col.saturating_sub(1) as usize));
        self.embedded.insert(path.clone(), (padded, html.to_path_buf()));
        Some(path)
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
            self.err_at(e.pos, "html/syntax", e.message);
        }
        for (pos, rule, message) in sugar_errors {
            self.err_at(pos, rule, message);
        }
        for (line, col) in comment_positions(&self.sources[id as usize], "<!--") {
            self.err_at(Pos { file: id, line, col }, "html/comment", "comments are not allowed; the code is the documentation");
        }
        id
    }

    pub fn load(root: &Path) -> Project {
        let mut p = Project {
            root: root.to_path_buf(),
            files: Vec::new(),
            sources: Vec::new(),
            trees: Vec::new(),
            config: Config { webmcp: true, ..Config::default() },
            layout: 0,
            nested_layouts: HashMap::new(),
            not_found: None,
            pages: Vec::new(),
            endpoints: Vec::new(),
            middleware: None,
            components: HashMap::new(),
            stores: BTreeMap::new(),
            theme_css: None,
            js_files: Vec::new(),
            embedded: HashMap::new(),
            public_files: Vec::new(),
            framework: PathBuf::from(if !root.join(".pragma/boot.js").is_file() && root.join("core/boot.js").is_file() { "core" } else { ".pragma" }),
            diags: Vec::new(),
        };
        p.load_config();
        let mut all = Vec::new();
        for dir in ["routes", "lib", "utils", ".pragma", "core"] {
            walk(root, Path::new(dir), &mut all);
        }
        walk(root, Path::new("public"), &mut p.public_files);
        if !root.join("routes").is_dir() {
            p.err("routes", 1, 1, "app/structure", "routes/ folder not found");
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
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json not found");
            return;
        };
        let v: serde_json::Value = match serde_json::from_str(&src) {
            Ok(v) => v,
            Err(e) => {
                self.err("pragma.json", e.line() as u32, e.column() as u32, "app/pragma-json", format!("invalid JSON: {e}"));
                return;
            }
        };
        let line_of = |key: &str| -> u32 {
            let pat = format!("\"{key}\"");
            src.find(&pat).map_or(1, |i| src[..i].matches('\n').count() as u32 + 1)
        };
        let Some(obj) = v.as_object() else {
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json must be an object");
            return;
        };
        for (k, val) in obj {
            match k.as_str() {
                "name" => match val.as_str() {
                    Some(s) => self.config.name = s.to_string(),
                    None => self.err("pragma.json", line_of("name"), 1, "app/pragma-json", "\"name\" must be a string"),
                },
                "lang" => match val.as_str() {
                    Some(s) if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') => self.config.lang = s.to_string(),
                    _ => self.err("pragma.json", line_of("lang"), 1, "app/pragma-json", "\"lang\" must be a language tag such as \"en\" or \"pt-BR\""),
                },
                "webmcp" => match val.as_bool() {
                    Some(b) => self.config.webmcp = b,
                    None => self.err("pragma.json", line_of("webmcp"), 1, "app/pragma-json", "\"webmcp\" must be true or false"),
                },
                "budgets" => {}
                _ => {
                    let l = line_of(k);
                    self.err("pragma.json", l, 1, "app/pragma-json", format!("unknown key \"{k}\" (use name, lang, webmcp)"));
                }
            }
        }
        if self.config.lang.is_empty() {
            self.config.lang = "en".into();
        }
        if self.config.name.is_empty() {
            self.err("pragma.json", 1, 1, "app/pragma-json", "pragma.json needs \"name\"");
        }
    }

    fn discover_components(&mut self, all: &[PathBuf]) {
        let mut dirs: BTreeMap<PathBuf, (PathBuf, String)> = BTreeMap::new();
        for f in all {
            let comps: Vec<_> = f.ancestors().filter(|a| a.file_name().is_some_and(|n| n == "_components") || *a == Path::new("lib/components")).collect();
            let Some(base) = comps.last() else { continue };
            let Ok(rest) = f.strip_prefix(base) else { continue };
            let Some(first) = rest.components().next() else { continue };
            let name = first.as_os_str().to_string_lossy().to_string();
            if rest.components().count() == 1 {
                self.err(f, 1, 1, "app/structure", format!("loose file in {}; use {}/<name>/<name>.html", rel_str(base), rel_str(base)));
                continue;
            }
            dirs.entry(base.join(&name)).or_insert((base.to_path_buf(), name));
        }
        for (dir, (base, name)) in dirs {
            if !is_component_name(&name) {
                self.err(&dir, 1, 1, "app/structure", format!("invalid component name \"{name}\": use lowercase letters, digits and hyphens"));
                continue;
            }
            let html = dir.join(format!("{name}.html"));
            if !all.contains(&html) {
                self.err(&dir, 1, 1, "app/structure", format!("component has no {name}.html"));
                continue;
            }
            let id = self.add_html(html);
            let (module, css) = self.extract(id, dir.join(format!("{name}.js")), dir.join(format!("{name}.css")), all);
            if all.contains(&dir.join(format!("{name}.server.js"))) {
                self.err(dir.join(format!("{name}.server.js")), 1, 1, "app/structure", "components cannot have .server.js; data lives in the page .server.js");
            }
            let global = base == Path::new("lib/components");
            let scope = if global { base.clone() } else { base.parent().unwrap().to_path_buf() };
            let cid = if global {
                name.clone()
            } else {
                let mut h = crate::Fnv::new();
                h.write(rel_str(&dir).as_bytes());
                format!("{name}-{}", &crate::short_hash(h.finish())[..6])
            };
            let c = Component { name: name.clone(), id: cid, html: id, dir: dir.clone(), module, css, scope };
            self.components.entry(base).or_default().insert(name, c);
        }
    }

    fn discover_routes(&mut self, all: &[PathBuf]) {
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        let mut layout = None;
        for f in all {
            let s = rel_str(f);
            if !s.starts_with("routes/") {
                continue;
            }
            let in_private = f.components().skip(1).any(|c| c.as_os_str().to_string_lossy().starts_with('_'));
            let file_name = f.file_name().unwrap().to_string_lossy().to_string();
            if s == "routes/_layout.html" {
                layout = Some(self.add_html(f.clone()));
                continue;
            }
            if s == "routes/_404.html" {
                let id = self.add_html(f.clone());
                let (module, css) = self.extract(id, PathBuf::from("routes/_404.js"), PathBuf::from("routes/_404.css"), all);
                if all.contains(&PathBuf::from("routes/_404.server.js")) {
                    self.err("routes/_404.server.js", 1, 1, "app/structure", "_404 cannot have a loader in v0");
                }
                self.not_found = Some(Page { html: id, pattern: "*".into(), out_name: "_404".into(), dir: "routes".into(), module, css, server: None });
                continue;
            }
            if s == "routes/_404.js" || s == "routes/_404.server.js" {
                continue;
            }
            if in_private {
                if f.ancestors().any(|a| a.file_name().is_some_and(|n| n == "_components")) {
                    continue;
                }
                let parent_private = f.parent().is_some_and(|p| p.components().skip(1).any(|c| c.as_os_str().to_string_lossy().starts_with('_')));
                if file_name == "_layout.html" && !parent_private {
                    let id = self.add_html(f.clone());
                    self.nested_layouts.insert(f.parent().unwrap().to_path_buf(), id);
                } else if file_name == "_middleware.server.js" {
                    if s == "routes/_middleware.server.js" {
                        self.middleware = Some(f.clone());
                    } else {
                        self.err(f, 1, 1, "app/middleware", "middleware lives only in routes/_middleware.server.js; it runs for every request, so check url.pathname there");
                    }
                } else if file_name == "_layout.js" || file_name == "_layout.server.js" {
                    self.err(f, 1, 1, "app/structure", "the layout cannot have a module or .server.js");
                } else if file_name.starts_with('_') && file_name.ends_with(".html") {
                    self.err(f, 1, 1, "app/structure", "only _layout.html and routes/_404.html are special in routes/");
                }
                continue;
            }
            if let Some(stem) = file_name.strip_suffix(".server.js")
                && !all.contains(&f.with_file_name(format!("{stem}.html")))
            {
                if let Some(pattern) = self.claim_route(f, stem, &mut seen) {
                    self.endpoints.push(Endpoint { pattern, file: f.clone() });
                }
                continue;
            }
            if let Some(stem) = file_name.strip_suffix(".server.js").or_else(|| file_name.strip_suffix(".js")) {
                if !all.contains(&f.with_file_name(format!("{stem}.html"))) {
                    self.err(f, 1, 1, "app/structure", format!("module without a page: missing {stem}.html"));
                }
                continue;
            }
            let Some(stem) = file_name.strip_suffix(".html") else {
                self.err(f, 1, 1, "app/structure", "unsupported file in routes/ (static files go in public/, CSS in lib/theme.css or the component)");
                continue;
            };
            let rel = f.strip_prefix("routes").unwrap();
            if stem != "index" && all.contains(&f.with_extension("").join("index.html")) {
                let other = rel_str(&f.with_extension("").join("index.html"));
                self.err(f, 1, 1, "app/route-conflict", format!("{s} and {other} serve the same route"));
                continue;
            }
            let Some(pattern) = self.claim_route(f, stem, &mut seen) else { continue };
            let id = self.add_html(f.clone());
            let (module, css) = self.extract(id, f.with_file_name(format!("{stem}.js")), f.with_file_name(format!("{stem}.css")), all);
            let server = Some(f.with_file_name(format!("{stem}.server.js"))).filter(|m| all.contains(m));
            let out_name = rel_str(&rel.with_extension(""));
            self.pages.push(Page { html: id, pattern, out_name, dir: f.parent().unwrap().to_path_buf(), module, css, server });
        }
        self.layout = layout.unwrap_or_else(|| {
            if self.root.join("routes").is_dir() {
                self.err("routes/_layout.html", 1, 1, "app/structure", "missing routes/_layout.html with <main data-page><slot></slot></main>");
            }
            self.files.push(PathBuf::from("routes/_layout.html"));
            self.sources.push(String::new());
            self.trees.push(html::parse("<main data-page><slot></slot></main>", self.files.len() as u32 - 1).0);
            self.files.len() as u32 - 1
        });
        self.pages.sort_by(|a, b| route_order(&a.pattern, &b.pattern));
        self.endpoints.sort_by(|a, b| route_order(&a.pattern, &b.pattern));
    }

    /// URL pattern of a page or endpoint file (`routes/shop/[id].html` -> `/shop/:id`), or `None`
    /// after reporting a bad param or a route another file already serves.
    fn claim_route(&mut self, f: &Path, stem: &str, seen: &mut BTreeMap<String, PathBuf>) -> Option<String> {
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
            self.err(f, 1, 1, "app/structure", "invalid route param: use [name] with letters, digits and _");
            return None;
        }
        if pattern.is_empty() {
            pattern.push('/');
        }
        let shape: String = pattern.split('/').map(|p| if p.starts_with(':') { ":" } else { p }).collect::<Vec<_>>().join("/");
        if let Some(prev) = seen.get(&shape) {
            let prev = rel_str(prev);
            self.err(f, 1, 1, "app/route-conflict", format!("route {pattern} conflicts with {prev}"));
            return None;
        }
        seen.insert(shape, f.to_path_buf());
        Some(pattern)
    }

    fn discover_stores(&mut self, all: &[PathBuf]) {
        for f in all {
            let s = rel_str(f);
            if let Some(name) = s.strip_prefix("lib/state/").and_then(|n| n.strip_suffix(".js")) {
                if name.contains('/') || !is_ident(name) {
                    self.err(f, 1, 1, "app/structure", "a store must be lib/state/<name>.js with an identifier name");
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

/// Static beats param, segment by segment.
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
