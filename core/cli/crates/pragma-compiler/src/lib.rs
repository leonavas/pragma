//! Build, analysis and directives. Contract: core/SPEC.md §2, §3, §5, §9, §10.
mod a11y;
mod bundle;
mod compile;
mod deps;
mod directive;
mod expand;
mod html;
mod js;
mod project;
mod sugar;
mod validate;

use pragma_diag::{Diagnostic, Severity};
use project::{Component, Page, Project, rel_str};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Component as PathPart, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dev,
    Prod,
}

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub root: PathBuf,
    pub out_dir: PathBuf,
    pub mode: Mode,
}

#[derive(Debug, Clone, Default)]
pub struct PageReport {
    pub pattern: String,
    pub js_gzip_bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct BuildReport {
    pub pages: Vec<PageReport>,
    pub css_gzip_bytes: u64,
    pub has_server: bool,
    pub warnings: Vec<Diagnostic>,
}

pub fn build(opts: &BuildOptions) -> Result<BuildReport, Vec<Diagnostic>> {
    let (diags, mut report) = run(opts);
    if diags.iter().any(|d| d.severity == Severity::Error) {
        return Err(diags);
    }
    report.warnings = diags;
    Ok(report)
}

pub fn check(root: &Path) -> Vec<Diagnostic> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let out_dir = std::env::temp_dir().join(format!("pragma-check-{}-{nanos}", std::process::id()));
    let (diags, _) = run(&BuildOptions { root: root.to_path_buf(), out_dir: out_dir.clone(), mode: Mode::Prod });
    let _ = fs::remove_dir_all(&out_dir);
    diags
}

pub(crate) struct Fnv(u64);

impl Fnv {
    pub fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) fn short_hash(h: u64) -> String {
    format!("{h:016x}")
}

fn hash8(bytes: &[u8]) -> String {
    let mut h = Fnv::new();
    h.write(bytes);
    short_hash(h.finish())[..8].to_string()
}

fn gzip_len(bytes: &[u8]) -> u64 {
    miniz_oxide::deflate::compress_to_vec(bytes, 6).len() as u64 + 18
}

fn sort_diags(mut d: Vec<Diagnostic>) -> Vec<Diagnostic> {
    d.sort_by(|a, b| (&a.file, a.line, a.column, a.rule, &a.message).cmp(&(&b.file, b.line, b.column, b.rule, &b.message)));
    d.dedup();
    d
}

/// Writes only what changed and remembers what was written, to delete the rest at the end.
struct Writer {
    root: PathBuf,
    written: HashSet<PathBuf>,
}

impl Writer {
    fn write(&mut self, rel: &str, bytes: &[u8]) {
        let path = self.root.join(rel.trim_start_matches('/'));
        if fs::read(&path).is_ok_and(|cur| cur == bytes) {
            self.written.insert(path);
            return;
        }
        if let Some(dir) = path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(&path, bytes);
        self.written.insert(path);
    }

    fn has(&self, rel: &str) -> bool {
        self.written.contains(&self.root.join(rel.trim_start_matches('/')))
    }

    fn prune(&self, dir: &Path) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let path = e.path();
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                self.prune(&path);
                let _ = fs::remove_dir(&path);
            } else if !self.written.contains(&path) {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

fn normalize(base: &Path, spec: &str) -> Option<PathBuf> {
    let joined = if let Some(abs) = spec.strip_prefix('/') { PathBuf::from(abs) } else { base.join(spec) };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            PathPart::Normal(x) => out.push(x),
            PathPart::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            PathPart::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

struct Module {
    src: String,
    imports: Vec<(js::Import, Option<PathBuf>)>,
}

fn module_graph(
    root: &Path,
    entries: &[PathBuf],
    client: bool,
    sources: &HashMap<PathBuf, String>,
    deps: &deps::Deps,
    requests: &mut BTreeMap<String, bundle::Request>,
    diags: &mut Vec<Diagnostic>,
) -> BTreeMap<PathBuf, Module> {
    let mut mods = BTreeMap::new();
    let mut queue: VecDeque<PathBuf> = entries.iter().cloned().collect();
    while let Some(path) = queue.pop_front() {
        if mods.contains_key(&path) {
            continue;
        }
        let Some(src) = sources.get(&path).cloned().or_else(|| fs::read_to_string(root.join(&path)).ok()) else { continue };
        let lines = html::LineIndex::new(&src);
        let mut bindings: Option<Vec<(String, pragma_jslint::Imported)>> = None;
        let mut imports = Vec::new();
        for imp in js::imports(&src) {
            let is_rel = imp.spec.starts_with("./") || imp.spec.starts_with("../") || imp.spec.starts_with('/');
            let dir = path.parent().unwrap_or(Path::new(""));
            let pos = lines.pos(&src, imp.start, 0);
            let mut ok = None;
            if pragma_jslint::is_package(&imp.spec) {
                let resolved = match deps.check_listed(&imp.spec, client) {
                    Err(e) => Err((deps::RULE_IMPORT, e)),
                    Ok(()) if client => deps.resolve(dir, &imp.spec).map(Some),
                    Ok(()) => Ok(None),
                };
                match resolved {
                    Ok(Some(full)) => {
                        let all = bindings.get_or_insert_with(|| pragma_jslint::imported_bindings(&src));
                        let req = requests.entry(imp.spec.clone()).or_insert_with(|| bundle::Request::new(full, (path.clone(), pos.line, pos.col)));
                        let mut seen = false;
                        for (_, what) in all.iter().filter(|(s, _)| *s == imp.spec) {
                            req.add(what);
                            seen = true;
                        }
                        if !seen {
                            req.add(if imp.dynamic { &pragma_jslint::Imported::All } else { &pragma_jslint::Imported::Bare });
                        }
                    }
                    Ok(None) => {}
                    Err((rule, e)) => diags.push(Diagnostic::error(&path, pos.line, pos.col, rule, e)),
                }
                imports.push((imp, None));
                continue;
            }
            let target = if is_rel { normalize(dir, &imp.spec) } else { None };
            match target {
                Some(t) if root.join(&t).is_file() => {
                    if client && rel_str(&t).ends_with(".server.js") {
                        diags.push(Diagnostic::error(&path, pos.line, pos.col, "js/import", format!("client module cannot import {} (server code)", imp.spec)));
                    } else {
                        queue.push_back(t.clone());
                        ok = Some(t);
                    }
                }
                _ if is_rel => diags.push(Diagnostic::error(&path, pos.line, pos.col, "js/import", format!("import \"{}\" not found", imp.spec))),
                _ => {}
            }
            imports.push((imp, ok));
        }
        mods.insert(path, Module { src, imports });
    }
    mods
}

fn npm_path(file: &str) -> PathBuf {
    PathBuf::from("node_modules").join(file)
}

fn link_packages(mods: &mut BTreeMap<PathBuf, Module>, b: &bundle::Bundle, diags: &mut Vec<Diagnostic>) {
    for (path, m) in mods.iter_mut() {
        if m.imports.iter().any(|(imp, _)| b.aliases.contains_key(&imp.spec)) {
            match pragma_jslint::alias_package_imports(&m.src, |s| b.aliases.get(s).copied()) {
                Ok(src) => {
                    let old: HashMap<String, Option<PathBuf>> = m.imports.drain(..).map(|(i, t)| (i.spec, t)).collect();
                    m.imports = js::imports(&src).into_iter().map(|i| (old.get(&i.spec).cloned().flatten(), i)).map(|(t, i)| (i, t)).collect();
                    m.src = src;
                }
                Err(e) => diags.push(Diagnostic::error(path, 1, 1, deps::RULE_IMPORT, e)),
            }
        }
        for (imp, t) in m.imports.iter_mut() {
            if let Some(f) = b.entries.get(&imp.spec) {
                *t = Some(npm_path(f));
            }
        }
    }
    let names: HashSet<&str> = b.files.iter().map(|f| f.0.as_str()).collect();
    for (file, code) in &b.files {
        let imports = js::imports(code)
            .into_iter()
            .map(|imp| {
                let t = imp.spec.strip_prefix("./").filter(|n| names.contains(n)).map(npm_path);
                (imp, t)
            })
            .collect();
        mods.insert(npm_path(file), Module { src: code.clone(), imports });
    }
}

/// Reachable modules; `dynamic` also follows `import()`.
fn closure(mods: &BTreeMap<PathBuf, Module>, entries: &[PathBuf], dynamic: bool) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut order = Vec::new();
    let mut queue: VecDeque<&PathBuf> = entries.iter().collect();
    while let Some(p) = queue.pop_front() {
        if !seen.insert(p.clone()) {
            continue;
        }
        let Some(m) = mods.get(p) else { continue };
        order.push(p.clone());
        for (imp, t) in &m.imports {
            if let Some(t) = t.as_ref().filter(|_| dynamic || !imp.dynamic) {
                queue.push_back(t);
            }
        }
    }
    order
}

struct PageOut<'a> {
    page: &'a Page,
    exp: expand::Expanded,
    loader: bool,
    ssr: bool,
    stores: BTreeSet<String>,
    is_404: bool,
    overlays: bool,
}

const PD: &str = "<script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}})</script>";
const THEME: &str = "<script>try{document.documentElement.dataset.theme=localStorage.getItem('pragma:theme')||(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light')}catch{}</script>";

fn any_element(nodes: &[html::Node], f: &impl Fn(&html::Element) -> bool) -> bool {
    nodes.iter().any(|n| matches!(n, html::Node::Element(e) if f(e) || any_element(&e.children, f)))
}

/// `aria-current="page"` on links `href="/path"` to the page itself (the router keeps it updated).
fn mark_current(nodes: &mut [html::Node], path: &str) {
    for n in nodes {
        if let html::Node::Element(e) = n {
            let routed = !["target", "download", "data-reload", "aria-current"].iter().any(|a| e.has(a));
            if e.is("a") && routed && e.value("href") == Some(path) {
                e.attrs.push(html::Attr { name: "aria-current".into(), value: Some("page".into()), pos: e.pos });
            }
            mark_current(&mut e.children, path);
        }
    }
}

fn run(opts: &BuildOptions) -> (Vec<Diagnostic>, BuildReport) {
    let prod = opts.mode == Mode::Prod;
    let mut p = Project::load(&opts.root);
    let mut diags = std::mem::take(&mut p.diags);
    let mut report = BuildReport::default();

    let mut sources: HashMap<PathBuf, String> = HashMap::new();
    for f in &p.js_files {
        if let Some(s) = p.read(f) {
            let app = ["routes/", "lib/", "utils/"].iter().any(|d| rel_str(f).starts_with(d));
            if app {
                diags.extend(pragma_jslint::lint(f, &s, &pragma_jslint::LintContext { is_core: false }));
            }
            sources.insert(f.clone(), s);
        }
    }

    let theme_src = p.theme_css.clone().unwrap_or_default();
    let css_comment = |path: &Path, src: &str, diags: &mut Vec<Diagnostic>| {
        for (line, col) in project::comment_positions(src, "/*") {
            diags.push(Diagnostic::error(path, line, col, "css/comment", "comments are not allowed; the code is the documentation"));
        }
    };
    css_comment(Path::new("lib/theme.css"), &theme_src, &mut diags);
    let theme = pragma_css::parse_theme(&theme_src);
    let mut project_classes = HashSet::new();
    validate::css_classes(&theme_src, &mut project_classes);
    let mut comp_css: HashMap<PathBuf, String> = HashMap::new();
    for c in p.all_components() {
        if let Some(css) = c.css.as_ref().and_then(|f| p.read(f)) {
            css_comment(c.css.as_ref().unwrap(), &css, &mut diags);
            validate::css_classes(&css, &mut project_classes);
            comp_css.insert(c.dir.clone(), css);
        }
    }
    let mut classes = validate::ClassCheck::new(project_classes, theme);

    let comp_ids: HashSet<u32> = p.all_components().map(|c| c.html).collect();
    for id in 0..p.files.len() as u32 {
        let kind = if Some(id) == p.layout {
            validate::Kind::Layout
        } else if comp_ids.contains(&id) {
            validate::Kind::Component
        } else {
            validate::Kind::Page
        };
        let mut fc = validate::FileCheck { files: &p.files, kind, classes: &mut classes, diags: Vec::new() };
        fc.run(&p.trees[id as usize], html::Pos { file: id, line: 1, col: 1 });
        diags.extend(fc.diags);
    }

    let mut actions_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let mut actions_of = |f: &Path| -> Vec<String> {
        actions_cache.entry(f.to_path_buf()).or_insert_with(|| sources.get(f).map(|s| pragma_jslint::exported_actions(s)).unwrap_or_default()).clone()
    };

    let mut outs: Vec<PageOut> = Vec::new();
    let mut used_classes: BTreeSet<String> = BTreeSet::new();
    let mut used_components: BTreeMap<PathBuf, Component> = BTreeMap::new();
    let all_pages: Vec<(&Page, bool)> = p.pages.iter().map(|pg| (pg, false)).chain(p.not_found.iter().map(|pg| (pg, true))).collect();
    for (page, is_404) in all_pages {
        let (mut exp, d) = expand::Expander::new(&p).page(page);
        diags.extend(d);
        if !is_404 && !page.pattern.contains(':') {
            mark_current(&mut exp.body, &page.pattern);
            mark_current(&mut exp.page, &page.pattern);
        }
        diags.extend(a11y::check(&p.files, if p.layout.is_some() { &exp.body } else { &exp.page }));
        let overlays = any_element(if p.layout.is_some() { &exp.body } else { &exp.page }, &|e| e.is("dialog") || e.has("popover"));
        let (loader, ssr) = match page.server.as_ref().and_then(|s| sources.get(s)) {
            Some(src) => {
                let names = pragma_jslint::exported_names(src);
                let ssr = js::exports(src).iter().any(|(n, v)| n == "ssr" && v.as_deref() == Some("true"));
                (names.iter().any(|n| n == "load"), ssr)
            }
            None => (false, false),
        };
        let page_actions = page.module.as_ref().map(|m| actions_of(m));
        let comp_actions_owned: Vec<(String, Vec<String>)> = exp.components.values().filter_map(|c| c.module.as_ref().map(|m| (c.name.clone(), actions_of(m)))).collect();
        let ctx = expand::PageCtx {
            loader,
            params: page.pattern.split('/').filter_map(|s| s.strip_prefix(':')).map(str::to_string).collect(),
            page_actions: page_actions.as_deref(),
            page_file: &p.files[page.html as usize],
            comp_actions: comp_actions_owned.iter().map(|(n, a)| (n.clone(), a.as_slice())).collect(),
        };
        let mut sc = expand::Scoped { p: &p, ctx: &ctx, diags: Vec::new(), stores: BTreeSet::new(), classes: BTreeSet::new(), outer: Vec::new() };
        sc.run(&exp.body);
        if p.layout.is_none() {
            sc.run(&exp.page);
        }
        diags.extend(sc.diags);
        used_classes.extend(sc.classes);
        for c in exp.components.values() {
            used_components.insert(c.dir.clone(), c.clone());
        }
        outs.push(PageOut { page, exp, loader, ssr, stores: sc.stores, is_404, overlays });
    }

    // client JS
    let boot = PathBuf::from("core/boot.js");
    let overlay = PathBuf::from("core/overlay.js");
    if !opts.root.join(&boot).is_file() {
        diags.push(Diagnostic::error("core/boot.js", 1, 1, "app/structure", "core/boot.js not found; the framework lives in core/ (created by pragma new)"));
    }
    let page_entries = |o: &PageOut| -> Vec<PathBuf> {
        let mut e = vec![boot.clone()];
        e.extend(o.page.module.clone());
        e.extend(o.exp.components.values().filter_map(|c| c.module.clone()));
        e.extend(o.stores.iter().filter_map(|s| p.stores.get(s).cloned()));
        if o.overlays && opts.root.join(&overlay).is_file() {
            e.push(overlay.clone());
        }
        e
    };
    let all_entries: Vec<PathBuf> = outs.iter().flat_map(page_entries).collect();
    let deps = deps::Deps::new(&opts.root, &mut diags);
    let mut requests = BTreeMap::new();
    let mut mods = module_graph(&opts.root, &all_entries, true, &sources, &deps, &mut requests, &mut diags);
    let out_abs = std::path::absolute(&opts.out_dir).unwrap_or_else(|_| opts.out_dir.clone());
    if let Some(b) = bundle::bundle(&opts.root, &out_abs.join(".cache/npm"), prod, &requests, &mut diags) {
        link_packages(&mut mods, &b, &mut diags);
    }
    let content_hash: HashMap<&PathBuf, u64> = mods
        .iter()
        .map(|(k, m)| {
            let mut h = Fnv::new();
            h.write(m.src.as_bytes());
            (k, h.finish())
        })
        .collect();
    let mut urls: HashMap<PathBuf, String> = HashMap::new();
    for path in mods.keys() {
        let rel = deps::asset_path(&rel_str(path));
        let url = if prod {
            let mut deps = closure(&mods, std::slice::from_ref(path), true);
            deps.sort();
            let mut h = Fnv::new();
            h.write(rel.as_bytes());
            for d in &deps {
                h.write(rel_str(d).as_bytes());
                h.write(&content_hash[d].to_le_bytes());
            }
            let stem = rel.strip_suffix(".js").or_else(|| rel.strip_suffix(".mjs")).unwrap_or(&rel);
            let ext = if rel.ends_with(".mjs") { "mjs" } else { "js" };
            format!("/assets/{stem}-{}.{ext}", &short_hash(h.finish())[..8])
        } else {
            format!("/assets/{rel}")
        };
        urls.insert(path.clone(), url);
    }

    let mut w = Writer { root: opts.out_dir.clone(), written: HashSet::new() };
    let mut gz: HashMap<PathBuf, u64> = HashMap::new();
    for (path, m) in &mods {
        let map: HashMap<&str, &str> = m.imports.iter().filter_map(|(imp, t)| t.as_ref().map(|t| (imp.spec.as_str(), urls[t].as_str()))).collect();
        let code = if prod && !path.starts_with("node_modules") {
            match pragma_jslint::minify_with_imports(&m.src, |s| map.get(s).map(|u| u.to_string())) {
                Ok(c) => c,
                Err(e) => {
                    diags.push(Diagnostic::error(path, 1, 1, "js/syntax", format!("cannot minify: {e}")));
                    m.src.clone()
                }
            }
        } else {
            js::rewrite(&m.src, &m.imports.iter().map(|(i, _)| i.clone()).collect::<Vec<_>>(), |i| map.get(i.spec.as_str()).map(|u| u.to_string()))
        };
        gz.insert(path.clone(), gzip_len(code.as_bytes()));
        let code = if !prod && urls[path] == "/assets/core/log.js" { with_ignore_map(code) } else { code };
        w.write(&format!("client{}", urls[path]), code.as_bytes());
    }

    // CSS global
    let names: Vec<String> = used_classes.iter().filter(|c| !classes.project.contains(*c)).cloned().collect();
    let generated = pragma_css::generate(&names, &classes.theme);
    let mut css = String::from(pragma_css::base_css());
    css.push('\n');
    css.push_str(&theme_src);
    css.push('\n');
    for c in used_components.values() {
        if let Some(s) = comp_css.get(&c.dir) {
            css.push_str(&format!("/* {} */\n@layer components {{\n{s}\n}}\n", rel_str(c.css.as_ref().unwrap())));
        }
    }
    css.push_str(&generated.css);
    if prod {
        match pragma_css::minify(&css) {
            Ok(m) => css = m,
            Err(e) => diags.push(Diagnostic::error("lib/theme.css", 1, 1, "css/syntax", e)),
        }
    }
    let css_url = if prod { format!("/assets/app-{}.css", hash8(css.as_bytes())) } else { "/assets/app.css".to_string() };
    w.write(&format!("client{css_url}"), css.as_bytes());
    report.css_gzip_bytes = gzip_len(css.as_bytes());

    // pages
    let theme_js = mods.contains_key(Path::new("core/theme.js"));
    let mut manifest = Vec::new();
    let mut server_routes = Vec::new();
    let boot_url = urls.get(&boot).cloned().unwrap_or_else(|| "/assets/core/boot.js".into());
    for o in &outs {
        let entries = page_entries(o);
        let modules = closure(&mods, &entries, false);
        let module_urls: Vec<&str> = modules.iter().filter(|m| **m != boot).map(|m| urls[m].as_str()).collect();
        let url_of = |f: &Option<PathBuf>| f.as_ref().and_then(|f| urls.get(f)).map(|u| Value::String(u.clone())).unwrap_or(Value::Null);
        let comps: serde_json::Map<String, Value> = o.exp.components.values().filter(|c| c.module.is_some()).map(|c| (c.name.clone(), url_of(&c.module))).collect();
        let stores: serde_json::Map<String, Value> = p.stores.iter().filter(|(name, f)| o.stores.contains(*name) || modules.contains(f)).map(|(name, f)| (name.clone(), url_of(&Some(f.clone())))).collect();
        let route = json!({
            "pattern": o.page.pattern,
            "page": url_of(&o.page.module),
            "components": comps,
            "stores": stores,
            "loader": o.loader,
            "ssr": o.ssr,
        });
        let title = o.exp.title.clone().or_else(|| {
            o.exp.head.iter().find_map(|n| match n {
                html::Node::Element(e) if e.is("title") => {
                    let mut s = String::new();
                    html::serialize(&e.children, &mut s);
                    Some(s.trim().to_string())
                }
                _ => None,
            })
        });
        let mut doc = String::with_capacity(4096);
        doc.push_str(&format!("<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n", html::escape_attr(&p.config.lang)));
        doc.push_str(&format!("<title>{}</title>\n", title.unwrap_or_else(|| html::escape_text(&p.config.name))));
        let extras: Vec<html::Node> = o.exp.head.iter().filter(|n| !matches!(n, html::Node::Element(e) if e.is("title"))).cloned().collect();
        let mut ex = String::new();
        compile::write_nodes(&extras, &mut ex, true);
        let ex = ex.trim();
        if !ex.is_empty() {
            doc.push_str(ex);
            doc.push('\n');
        }
        if theme_js {
            doc.push_str(THEME);
            doc.push('\n');
        }
        doc.push_str(&format!("<link rel=\"stylesheet\" href=\"{css_url}\">\n"));
        if !prod {
            doc.push_str("<script>self.__pragmaDev=true</script>\n");
        }
        if o.loader && !o.ssr {
            doc.push_str(PD);
            doc.push('\n');
        }
        for u in &module_urls {
            doc.push_str(&format!("<link rel=\"modulepreload\" href=\"{u}\">\n"));
        }
        let route_json = serde_json::to_string(&route).unwrap().replace('<', "\\u003c");
        doc.push_str(&format!("<script type=\"application/json\" id=\"pragma-route\">{route_json}</script>\n"));
        doc.push_str(&format!("<script type=\"module\" src=\"{boot_url}\"></script>\n"));
        if !prod {
            doc.push_str("<script type=\"module\" src=\"/_pragma/dev.js\"></script>\n");
        }
        doc.push_str("</head>\n<body>\n");
        let mut body = String::new();
        compile::write_nodes(if p.layout.is_some() { &o.exp.body } else { &o.exp.page }, &mut body, prod);
        doc.push_str(body.trim());
        doc.push_str("\n</body>\n</html>\n");
        let html_url = format!("/{}.html", o.page.out_name);
        w.write(&format!("client{html_url}"), doc.as_bytes());

        let initial: u64 = modules.iter().map(|m| gz.get(m).copied().unwrap_or(0)).sum();
        let route_only: u64 = modules.iter().filter(|m| !rel_str(m).starts_with("core/")).map(|m| gz.get(m).copied().unwrap_or(0)).sum();
        if !o.is_404 {
            manifest.push(json!({ "pattern": o.page.pattern, "html": html_url, "modules": module_urls, "loader": o.loader, "ssr": o.ssr }));
            report.pages.push(PageReport { pattern: o.page.pattern.clone(), js_gzip_bytes: initial });
            budget(&p, "initialJsKb", p.config.budgets.initial_js_kb, initial, &o.page.pattern, &mut diags);
            budget(&p, "routeJsKb", p.config.budgets.route_js_kb, route_only, &o.page.pattern, &mut diags);
        }
        if let Some(server) = &o.page.server {
            let (template, shell) = if o.ssr {
                let ops = compile::ops(&o.exp.page);
                w.write(&format!("templates/{}.json", o.page.out_name), serde_json::to_string(&ops).unwrap().as_bytes());
                (ops, Value::String(doc.clone()))
            } else {
                (Value::Null, Value::Null)
            };
            server_routes.push((o.page.pattern.clone(), server.clone(), o.ssr, template, shell));
        }
    }
    budget(&p, "cssKb", p.config.budgets.css_kb, report.css_gzip_bytes, "global CSS", &mut diags);
    w.write("client/_pragma/manifest.json", serde_json::to_string_pretty(&Value::Array(manifest)).unwrap().as_bytes());

    for f in &p.public_files {
        let rel = rel_str(f.strip_prefix("public").unwrap());
        let target = format!("client/{rel}");
        if w.has(&target) {
            diags.push(Diagnostic::error(f, 1, 1, "app/structure", format!("public/{rel} conflicts with a file generated by the build")));
            continue;
        }
        if let Ok(bytes) = fs::read(opts.root.join(f)) {
            w.write(&target, &bytes);
        }
    }

    report.has_server = !server_routes.is_empty();
    let handler = PathBuf::from("core/server/handler.js");
    if opts.root.join(&handler).is_file() {
        let mut entries = vec![handler.clone(), PathBuf::from("core/server/node.mjs")];
        entries.extend(server_routes.iter().map(|r| r.1.clone()));
        entries.retain(|e| opts.root.join(e).is_file());
        let smods = module_graph(&opts.root, &entries, false, &sources, &deps, &mut BTreeMap::new(), &mut diags);
        for (path, m) in &smods {
            w.write(&format!("server/{}", rel_str(path)), m.src.as_bytes());
        }
        let mut worker = String::from("import { createHandler } from './core/server/handler.js'\n");
        for (i, r) in server_routes.iter().enumerate() {
            worker.push_str(&format!("import * as s{i} from {}\n", json!(format!("./{}", rel_str(&r.1)))));
        }
        worker.push_str("\nexport default createHandler({\n");
        worker.push_str(&format!("  service: {},\n  routes: [\n", json!(p.config.name)));
        for (i, (pattern, _, ssr, template, shell)) in server_routes.iter().enumerate() {
            worker.push_str(&format!("    {{ pattern: {}, server: s{i}, ssr: {ssr}, template: {template}, shell: {shell} }},\n", json!(pattern)));
        }
        worker.push_str("  ],\n})\n");
        w.write("server/worker.js", worker.as_bytes());
        w.write("server/package.json", b"{ \"type\": \"module\" }\n");
        w.write(
            "server/node.mjs",
            b"import h from './worker.js'\nimport { serve } from './core/server/node.mjs'\n\nserve(h, { root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000 })\n",
        );
        let wrangler = json!({
            "name": p.config.name,
            "main": "worker.js",
            "compatibility_date": "2025-11-01",
            "assets": { "directory": "../client", "binding": "ASSETS", "not_found_handling": "single-page-application", "run_worker_first": true },
        });
        w.write("server/wrangler.jsonc", serde_json::to_string_pretty(&wrangler).unwrap().as_bytes());
    } else if report.has_server {
        diags.push(Diagnostic::error("core/server/handler.js", 1, 1, "app/structure", "core/server/handler.js not found; required by *.server.js"));
    }

    for d in ["client", "server", "templates"] {
        w.prune(&opts.out_dir.join(d));
    }
    if !prod {
        for d in diags.iter_mut().filter(|d| d.rule == "app/budget") {
            d.severity = Severity::Warning;
        }
    }
    (sort_diags(diags), report)
}

fn with_ignore_map(code: String) -> String {
    let lines = code.lines().count().max(1);
    let map = serde_json::json!({
        "version": 3,
        "sources": ["log.js"],
        "sourcesContent": [code],
        "mappings": format!("AAAA{}", ";AACA".repeat(lines - 1)),
        "ignoreList": [0],
        "x_google_ignoreList": [0],
    });
    format!("{code}\n//# sourceMappingURL=data:application/json;base64,{}\n", base64(map.to_string().as_bytes()))
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn budget(p: &Project, key: &'static str, limit: Option<f64>, bytes: u64, what: &str, diags: &mut Vec<Diagnostic>) {
    let Some(limit) = limit else { return };
    let kb = bytes as f64 / 1024.0;
    if kb > limit {
        let line = p.config.budget_lines.get(key).copied().unwrap_or(1);
        diags.push(Diagnostic::error("pragma.json", line, 1, "app/budget", format!("{key} exceeded in {what}: {kb:.1} kB gzip > {limit} kB")));
    }
}
