//! Build, analysis and directives. Contract: core/SPEC.md §2, §3, §5, §9, §10.
mod a11y;
mod bundle;
mod chunks;
mod compile;
mod deps;
mod directive;
mod expand;
mod hash;
mod html;
mod js;
mod kernel;
pub mod layout;
mod owners;
mod prerender;
mod project;
mod sugar;
mod validate;
mod webmcp;

use pragma_diag::{Diagnostic, Severity};
use pragma_jslint::script::{self, ScriptInfo, ScriptKind};
use pragma_css::scope::owner_selector;
use hash::{Fnv, css_owner, hash8, short_hash};
use project::{Page, Project, rel_str};
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
    /// First-load JS (route entry + its modulepreload list); 0 for a static route.
    pub js_raw_bytes: u64,
    pub js_gzip_bytes: u64,
    pub js_brotli_bytes: u64,
    /// Shared + route CSS, gzip: the number that decides inline (<= 14 kB) or a render-blocking stylesheet.
    pub css_gzip_bytes: u64,
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

fn url_safe(path: &str) -> String {
    rolldown_utils::sanitize_filename::default_sanitize_relative_file_name(path).into_owned()
}

fn gzip_len(bytes: &[u8]) -> u64 {
    miniz_oxide::deflate::compress_to_vec(bytes, 6).len() as u64 + 18
}

fn is_compressible(rel: &str) -> bool {
    matches!(
        Path::new(rel).extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("html" | "css" | "js" | "json" | "svg" | "md" | "xml" | "txt")
    )
}

fn brotli_compress(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut out = Vec::new();
    {
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, 6, 22);
        w.write_all(bytes).ok()?;
    }
    Some(out)
}

fn gzip_compress(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::io::Write;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(bytes).ok()?;
    enc.finish().ok()
}

fn spellings(attr: &str, value: &str) -> Vec<String> {
    vec![format!("[{attr}=\"{value}\"]"), format!("[{attr}='{value}']"), format!("[{attr}={value}]")]
}

/// A component's CSS applies only to the markup it wrote: every selector that does not already name
/// `[data-c="<name>"]` is scoped to its `data-s` token (`:global(…)` opts out). A
/// `routes/**/_components` one gets a per-folder id (`Component::id`), so its own selector is
/// rewritten to that id (BUG-S-11).
fn scope_component_css(css: &str, name: &str, id: &str) -> String {
    let to = format!("[data-c=\"{id}\"]");
    let css = if name == id { css.to_string() } else { spellings("data-c", name).iter().fold(css.to_string(), |out, from| out.replace(from.as_str(), &to)) };
    pragma_css::scope::scope(&css, &owner_selector(&css_owner("component", id)), &spellings("data-c", id))
}

fn scope_page_css(css: &str, page: &str) -> String {
    pragma_css::scope::scope(css, &owner_selector(&css_owner("page", page)), &spellings("data-page", page))
}

fn sort_diags(mut d: Vec<Diagnostic>) -> Vec<Diagnostic> {
    d.sort_by(|a, b| (&a.file, a.line, a.column, a.rule, &a.message).cmp(&(&b.file, b.line, b.column, b.rule, &b.message)));
    d.dedup();
    d
}

struct Writer {
    root: PathBuf,
    written: HashSet<PathBuf>,
    failed: Vec<Diagnostic>,
}

impl Writer {
    fn write(&mut self, rel: &str, bytes: &[u8]) {
        let path = self.root.join(rel.trim_start_matches('/'));
        if fs::read(&path).is_ok_and(|cur| cur == bytes) {
            self.written.insert(path);
            return;
        }
        let written = path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(&path, bytes));
        if let Err(e) = written {
            self.failed.push(Diagnostic::error(&path, 0, 0, "app/output", format!("cannot write: {e}")));
        }
        self.written.insert(path);
    }

    fn write_client(&mut self, rel: &str, bytes: &[u8], prod: bool) {
        self.write(rel, bytes);
        if !prod || bytes.len() < 512 || !is_compressible(rel) {
            return;
        }
        if let Some(br) = brotli_compress(bytes) && br.len() < bytes.len() {
            self.write(&format!("{rel}.br"), &br);
        }
        if let Some(gz) = gzip_compress(bytes) && gz.len() < bytes.len() {
            self.write(&format!("{rel}.gz"), &gz);
        }
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

/// Files in `public/` that production CSS names with `url(/path)`: each one gets a hashed copy under
/// `/assets/` (immutable cache) and the CSS points to it. `data:`, external, relative and unknown
/// URLs stay as written; the file stays in `public/` too.
struct PublicAssets {
    dir: PathBuf,
    files: HashSet<String>,
    hashed: HashMap<String, Option<String>>,
}

impl PublicAssets {
    fn new(root: &Path, files: &[PathBuf]) -> Self {
        let files = files.iter().filter_map(|f| f.strip_prefix("public").ok()).map(rel_str).collect();
        PublicAssets { dir: root.join("public"), files, hashed: HashMap::new() }
    }

    fn rewrite(&mut self, css: &str, w: &mut Writer) -> String {
        pragma_css::urls::map(css, |url| self.url(url, w))
    }

    fn url(&mut self, url: &str, w: &mut Writer) -> Option<String> {
        let path = url.strip_prefix('/').filter(|p| !p.starts_with('/'))?;
        let (path, suffix) = path.split_at(path.find(['?', '#']).unwrap_or(path.len()));
        let rel = percent_decode(path);
        if !self.hashed.contains_key(&rel) {
            let copy = self.files.contains(&rel).then(|| fs::read(self.dir.join(&rel)).ok()).flatten().map(|bytes| {
                let file = Path::new(&rel);
                let stem = file.with_extension("");
                let ext = file.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                let url = format!("/assets/{}-{}{ext}", url_safe(&rel_str(&stem)), hash8(&bytes));
                w.write_client(&format!("client{url}"), &bytes, true);
                url
            });
            self.hashed.insert(rel.clone(), copy);
        }
        self.hashed[&rel].as_ref().map(|u| format!("{u}{suffix}"))
    }
}

fn percent_decode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%').then(|| path.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())).flatten();
        match hex {
            Some(b) => {
                out.push(b);
                i += 3;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| path.to_string())
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
    p: &Project,
    entries: &[PathBuf],
    client: bool,
    sources: &HashMap<PathBuf, String>,
    deps: &deps::Deps,
    requests: &mut BTreeMap<String, bundle::Request>,
    diags: &mut Vec<Diagnostic>,
) -> BTreeMap<PathBuf, Module> {
    let root = p.root.as_path();
    let mut mods = BTreeMap::new();
    let mut unreadable = HashSet::new();
    let mut queue: VecDeque<PathBuf> = entries.iter().cloned().collect();
    while let Some(path) = queue.pop_front() {
        if mods.contains_key(&path) {
            continue;
        }
        let src = match sources.get(&path) {
            Some(src) => src.clone(),
            None => match fs::read(root.join(&path)).map(String::from_utf8) {
                Ok(Ok(src)) => src,
                Ok(Err(e)) => {
                    let valid = String::from_utf8_lossy(&e.as_bytes()[..e.utf8_error().valid_up_to()]).into_owned();
                    let (line, col) = html::line_col(&valid, valid.len());
                    diags.push(Diagnostic::error(&path, line, col, "js/syntax", "the file is not UTF-8 text; save it as UTF-8"));
                    unreadable.insert(path);
                    continue;
                }
                Err(_) => {
                    unreadable.insert(path);
                    continue;
                }
            },
        };
        let lines = html::LineIndex::new(&src);
        let mut bindings: Option<Vec<(String, pragma_jslint::Imported)>> = None;
        let mut imports = Vec::new();
        for imp in js::imports(&src) {
            let is_rel = imp.spec.starts_with("./") || imp.spec.starts_with("../") || imp.spec.starts_with('/');
            let dir = path.parent().unwrap_or(Path::new(""));
            let pos = lines.pos(&src, imp.start, 0);
            let mut ok = None;
            if imp.spec == "pragma/server" {
                let runtime = Some(p.framework.join("server/http.js")).filter(|f| p.root.join(f).is_file());
                match runtime {
                    _ if client => diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, "'pragma/server' is only for .server.js files; client code gets data from load and calls actions with call()")),
                    Some(t) => {
                        queue.push_back(t.clone());
                        imports.push((imp, Some(t)));
                        continue;
                    }
                    None => diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, "'pragma/server' needs .pragma/server/http.js; run `pragma upgrade`")),
                }
                imports.push((imp, None));
                continue;
            }
            if imp.spec == "pragma" {
                // page, component and store scripts lose this import when they become factories
                diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, "'pragma' is only for page, component and store scripts; pass what a helper needs as arguments"));
                imports.push((imp, None));
                continue;
            }
            if pragma_jslint::is_package(&imp.spec) {
                let resolved = match deps.check_listed(&imp.spec, client) {
                    Err(e) => Err(e),
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
                    let store = rel_str(&t).strip_prefix("lib/state/").and_then(|n| n.strip_suffix(".js")).map(str::to_string);
                    if let Some(name) = store {
                        diags.push(Diagnostic::error(&path, pos.line, pos.col, "js/store-import", format!("stores are reached through `shared`: `import {{ shared }} from 'pragma'`, then `shared.{name}`")));
                    } else if client && rel_str(&t).ends_with(".server.js") {
                        diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, format!("client module cannot import {} (server code)", imp.spec)));
                    } else if is_wasm(&t) {
                        if client {
                            diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, format!("{} is WebAssembly; import .wasm only in server code (.server.js)", imp.spec)));
                        } else {
                            ok = Some(t);
                        }
                    } else {
                        queue.push_back(t.clone());
                        ok = Some(t);
                    }
                }
                None if is_rel => diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, format!("import \"{}\" points outside the app folder; remove the extra `../`", imp.spec))),
                _ if is_rel => diags.push(Diagnostic::error(&path, pos.line, pos.col, deps::RULE_IMPORT, format!("import \"{}\" not found", imp.spec))),
                _ => {}
            }
            imports.push((imp, ok));
        }
        mods.insert(path, Module { src, imports });
    }
    for m in mods.values_mut() {
        for (_, target) in &mut m.imports {
            if target.as_ref().is_some_and(|t| unreadable.contains(t)) {
                *target = None;
            }
        }
    }
    mods
}

const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];

fn check_endpoints(p: &Project, sources: &HashMap<PathBuf, String>) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for e in &p.endpoints {
        let Some(src) = sources.get(&e.file) else { continue };
        let names = pragma_jslint::exported_names(src);
        let stem = e.file.file_name().map_or(String::new(), |n| n.to_string_lossy().trim_end_matches(".server.js").to_string());
        for name in &names {
            if ["load", "actions", "ssr"].contains(&name.as_str()) {
                out.push(Diagnostic::error(&e.file, 1, 1, "app/endpoint", format!("`{name}` needs a page: add {stem}.html next to this file; an endpoint without a page exports only GET, POST, PUT, PATCH or DELETE")));
            } else if !METHODS.contains(&name.as_str()) {
                out.push(Diagnostic::error(&e.file, 1, 1, "app/endpoint", format!("an endpoint exports only HTTP method handlers; move `{name}` to a private helper such as _{stem}.server.js")));
            }
        }
        if !names.iter().any(|n| METHODS.contains(&n.as_str())) {
            out.push(Diagnostic::error(&e.file, 1, 1, "app/endpoint", format!("no page {stem}.html and no handler: export GET, POST, PUT, PATCH or DELETE returning a Response, or add {stem}.html")));
        }
        if let Some(i) = src.find("Response.json(") {
            let (line, col) = html::line_col(src, i);
            out.push(Diagnostic::warning(&e.file, line, col, "app/endpoint-json", "endpoint answers JSON: page data comes from load and writes from actions (.pragma/docs/data.md); keep endpoints for files, streams, webhooks and WebSockets"));
        }
    }
    if let Some(l) = &p.layout_server
        && let Some(src) = sources.get(l)
    {
        let names = pragma_jslint::exported_names(src);
        if !names.iter().any(|n| n == "load") {
            out.push(Diagnostic::error(l, 1, 1, "app/layout-load", "export async function load({ request, url, cookies, locals }) that returns what the layout shows (read it as `layout`)"));
        }
        if let Some(n) = names.iter().find(|n| *n != "load") {
            out.push(Diagnostic::error(l, 1, 1, "app/layout-load", format!("routes/_layout.server.js exports only load; move `{n}` to a page's .server.js")));
        }
    }
    if let Some(m) = &p.middleware
        && let Some(src) = sources.get(m)
        && !pragma_jslint::exported_names(src).iter().any(|n| n == "handle")
    {
        out.push(Diagnostic::error(m, 1, 1, "app/middleware", "export async function handle({ request, url, cookies, locals, next }) that returns next() or a Response"));
    }
    out
}

fn is_app_code(path: &Path) -> bool {
    ["routes/", "lib/", "utils/"].iter().any(|d| rel_str(path).starts_with(d))
}

fn is_npm(path: &Path) -> bool {
    path.starts_with("node_modules")
}

/// The server and the build run a script when everything it imports is app code or an npm
/// package; a package that throws (DOM, `window`) leaves the script to the browser at run time.
fn server_runnable(mods: &BTreeMap<PathBuf, Module>, m: &Path) -> bool {
    closure(mods, &[m.to_path_buf()], true).iter().all(|f| is_app_code(f) || is_npm(f))
}

fn reaches_npm(mods: &BTreeMap<PathBuf, Module>, m: &Path) -> bool {
    closure(mods, &[m.to_path_buf()], true).iter().any(|f| is_npm(f))
}

fn is_wasm(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "wasm")
}

fn relative_spec(from: &str, to: &str) -> String {
    let depth = from.matches('/').count();
    if depth == 0 { format!("./{to}") } else { format!("{}{to}", "../".repeat(depth)) }
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

#[derive(Default)]
struct SsrScripts {
    page: Option<PathBuf>,
    components: Vec<(String, PathBuf)>,
    /// scripts that reach an npm package: imported on first use, so a package that throws on
    /// import leaves the script to the browser instead of breaking the whole server
    lazy: BTreeSet<PathBuf>,
}

struct PageOut<'a> {
    page: &'a Page,
    exp: expand::Expanded,
    loader: bool,
    ssr: bool,
    stores: BTreeSet<String>,
    page_classes: BTreeSet<String>,
    is_404: bool,
    overlays: bool,
    /// Block and self-call bodies `compile::hoist` moved out of the page (`R.t`, `R.d`).
    bodies: Vec<html::Element>,
    /// What the closure of the route uses (kernel-per-route §4).
    scan: kernel::Scan,
    /// Descriptors of a reactive route; `None`: a static route (no JS).
    described: Option<kernel::Described>,
}

const PD: &str = "<script>self.__pd=fetch('/_data'+location.pathname+location.search,{headers:{accept:'application/json'}});self.__pd.catch(()=>{})</script>";
const PL: &str = "<script>self.__pl=fetch('/_layout'+location.pathname+location.search,{headers:{accept:'application/json'}});self.__pl.catch(()=>{})</script>";
const THEME: &str = "<script>try{document.documentElement.dataset.theme=localStorage.getItem('pragma:theme')||(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light')}catch{}</script>";
const SPECULATION_RULES: &str = r#"<script type="speculationrules">{"prefetch":[{"where":{"and":[{"href_matches":"/*"},{"not":{"href_matches":"/_*"}},{"not":{"selector_matches":"[data-reload]"}}]},"eagerness":"moderate"}]}</script>"#;

fn any_element(nodes: &[html::Node], f: &impl Fn(&html::Element) -> bool) -> bool {
    nodes.iter().any(|n| matches!(n, html::Node::Element(e) if f(e) || any_element(&e.children, f)))
}

/// `aria-current="page"` on links `href="/path"` to the page itself (the router keeps it updated).
/// Layout block and self-call bodies are left alone: the runtime renders them from the module of
/// the page the app booted on, on every later page too.
fn mark_current(nodes: &mut [html::Node], path: &str, in_page: bool) {
    for n in nodes {
        let html::Node::Element(e) = n else { continue };
        if !in_page && e.is("template") {
            continue;
        }
        let routed = !["target", "download", "data-reload", "aria-current"].iter().any(|a| e.has(a));
        if e.is("a") && routed && e.value("href") == Some(path) {
            e.attrs.push(html::Attr { name: "aria-current".into(), value: Some("page".into()), pos: e.pos });
        }
        let in_page = in_page || (e.is("main") && e.has("data-page"));
        mark_current(&mut e.children, path, in_page);
    }
}

fn run(opts: &BuildOptions) -> (Vec<Diagnostic>, BuildReport) {
    let mut p = Project::load(&opts.root);
    let mut diags = std::mem::take(&mut p.diags);
    let mut report = BuildReport::default();
    let mut w = Writer { root: opts.out_dir.clone(), written: HashSet::new(), failed: Vec::new() };
    let a = analyze(&p, &mut diags);
    let client = emit_client(&p, &a, opts, &mut w, &mut report, &mut diags);
    emit_server(&p, &a, &client, opts, &mut w, &mut report, &mut diags);
    for d in [layout::CLIENT, layout::SERVER] {
        w.prune(&opts.out_dir.join(d));
    }
    diags.append(&mut w.failed);
    for d in &mut diags {
        d.file = p.origin(&d.file).to_path_buf();
    }
    (sort_diags(diags), report)
}

struct Analysis<'p> {
    sources: HashMap<PathBuf, String>,
    theme_src: String,
    comp_css: HashMap<PathBuf, String>,
    page_css: HashMap<PathBuf, String>,
    classes: validate::ClassCheck,
    outs: Vec<PageOut<'p>>,
    shared_classes: BTreeSet<String>,
}

fn analyze<'p>(p: &'p Project, diags: &mut Vec<Diagnostic>) -> Analysis<'p> {
    let mut sources: HashMap<PathBuf, String> = HashMap::new();
    for f in &p.js_files {
        if let Some(s) = p.read(f) {
            if is_app_code(f) {
                diags.extend(pragma_jslint::lint(f, &s));
            }
            sources.insert(f.clone(), s);
        }
    }
    // a <script> in an .html is linted like a file (diagnostics move to the .html at the end)
    for (path, (code, _)) in &p.embedded {
        if rel_str(path).ends_with(".js") {
            diags.extend(pragma_jslint::lint(path, code));
            sources.insert(path.clone(), code.clone());
        }
    }
    let mut scripts: HashMap<PathBuf, ScriptInfo> = HashMap::new();
    let script_kinds: Vec<(PathBuf, ScriptKind)> = p
        .pages
        .iter()
        .chain(p.not_found.iter())
        .filter_map(|pg| pg.module.clone().map(|m| (m, ScriptKind::Page)))
        .chain(p.all_components().filter_map(|c| c.module.clone().map(|m| (m, ScriptKind::Component))))
        .chain(p.stores.values().map(|f| (f.clone(), ScriptKind::Store)))
        .collect();
    for (path, kind) in script_kinds {
        let Some(src) = sources.get(&path).cloned() else { continue };
        let s = script::transform(&path, &src, kind);
        diags.extend(s.diags);
        sources.insert(path.clone(), s.code);
        scripts.insert(path, s.info);
    }

    let theme_src = p.theme_css.clone().unwrap_or_default();
    let css_comment = |path: &Path, src: &str, diags: &mut Vec<Diagnostic>| {
        for (line, col) in project::comment_positions(src, "/*") {
            diags.push(Diagnostic::error(path, line, col, "css/comment", "comments are not allowed; the code is the documentation"));
        }
        for (offset, prefixed, standard) in pragma_css::vendor_prefixes(src) {
            let (line, col) = html::line_col(src, offset);
            diags.push(Diagnostic::error(
                path,
                line,
                col,
                "css/vendor-prefix",
                format!("{prefixed} is written by hand: write `{standard}`; the build adds the prefixes the supported browsers need"),
            ));
        }
    };
    css_comment(Path::new("lib/theme.css"), &theme_src, diags);
    let theme = pragma_css::parse_theme(&theme_src);
    for (class, offset) in pragma_css::prune::utility_redefinitions(&theme_src, &|c| pragma_css::is_utility_with_css(c, &theme)) {
        let (line, col) = html::line_col(&theme_src, offset);
        diags.push(Diagnostic::error(
            "lib/theme.css",
            line,
            col,
            "css/utility-name",
            format!(".{class} has the name of a utility: class=\"{class}\" would get both rules and the utility wins; rename the custom class (e.g. .x-{class}) or use the utility"),
        ));
    }
    let mut project_classes = HashSet::new();
    validate::css_classes(&theme_src, &mut project_classes);
    let mut comp_css: HashMap<PathBuf, String> = HashMap::new();
    for c in p.all_components() {
        if let Some(css) = c.css.as_ref().and_then(|f| p.read(f)) {
            css_comment(c.css.as_ref().unwrap(), &css, diags);
            validate::css_classes(&css, &mut project_classes);
            comp_css.insert(c.dir.clone(), scope_component_css(&css, &c.name, &c.id));
        }
    }
    // a page <style> goes into its per-route CSS, after the layers (beats utilities), scoped to its own markup
    let mut page_css: HashMap<PathBuf, String> = HashMap::new();
    for (f, page) in p.pages.iter().chain(p.not_found.iter()).filter_map(|pg| pg.css.as_ref().map(|f| (f, &pg.out_name))) {
        if let Some(css) = p.read(f) {
            css_comment(f, &css, diags);
            validate::css_classes(&css, &mut project_classes);
            page_css.insert(f.clone(), scope_page_css(&css, page));
        }
    }
    let owned: owners::Owned = p
        .all_components()
        .filter_map(|c| comp_css.get(&c.dir).map(|css| (css_owner("component", &c.id), pragma_css::prune::own_selectors(css))))
        .chain(p.pages.iter().chain(p.not_found.iter()).filter_map(|pg| pg.css.as_ref().and_then(|f| page_css.get(f)).map(|css| (css_owner("page", &pg.out_name), pragma_css::prune::own_selectors(css)))))
        .collect();
    let scoped_owners = p
        .all_components()
        .filter_map(|c| Some(("component", c.css.as_ref()?, comp_css.get(&c.dir)?, c.html)))
        .chain(p.pages.iter().chain(p.not_found.iter()).filter_map(|pg| Some(("page", pg.css.as_ref()?, page_css.get(pg.css.as_ref()?)?, pg.html))));
    for (kind, file, css, html) in scoped_owners {
        let from = &p.files[html as usize];
        let root = |tag: &str| {
            let (local, name) = directive::component_tag(&tag.to_ascii_lowercase()).map(|(l, n)| (l, n.to_string()))?;
            owners::root_of(&p.trees[p.find_component(from, local, &name)?.html as usize])
        };
        for (offset, selector, global) in owners::unmatched(css, &p.trees[html as usize], &root) {
            let (line, col) = html::line_col(css, offset);
            diags.push(Diagnostic::warning(
                file,
                line,
                col,
                "css/unmatched-selector",
                format!("`{selector}` matches no element this {kind} writes; slot content and child components belong to their writer: style them there, or write `{global}`"),
            ));
        }
    }
    let mut classes = validate::ClassCheck::new(project_classes, theme);

    let comp_ids: HashSet<u32> = p.all_components().map(|c| c.html).collect();
    let nested_layout_ids: HashSet<u32> = p.nested_layouts.values().copied().collect();
    for id in 0..p.files.len() as u32 {
        let kind = if id == p.layout {
            validate::Kind::Layout
        } else if nested_layout_ids.contains(&id) {
            validate::Kind::NestedLayout
        } else if comp_ids.contains(&id) {
            validate::Kind::Component
        } else {
            validate::Kind::Page
        };
        let mut fc = validate::FileCheck { files: &p.files, kind, classes: &mut classes, diags: Vec::new() };
        fc.run(&p.trees[id as usize], html::Pos { file: id, line: 1, col: 1 });
        diags.extend(fc.diags);
    }
    if !p.config.webmcp {
        diags.extend(webmcp::disabled(&p.files, &p.trees));
    }

    let mut outs: Vec<PageOut> = Vec::new();
    let mut shared_classes: BTreeSet<String> = BTreeSet::new();
    let all_pages: Vec<(&Page, bool)> = p.pages.iter().map(|pg| (pg, false)).chain(p.not_found.iter().map(|pg| (pg, true))).collect();
    for (page, is_404) in all_pages {
        let (mut exp, d) = expand::Expander::new(p, &scripts, &owned).page(page);
        diags.extend(d);
        if !is_404 && !page.pattern.contains(':') {
            mark_current(&mut exp.body, &page.pattern, false);
        }
        diags.extend(a11y::check(&p.files, &exp.body));
        if p.config.webmcp {
            diags.extend(webmcp::check(&p.files, &exp.body));
        }
        webmcp::strip_notool(&mut exp.body);
        let overlays = any_element(&exp.body, &|e| e.is("dialog") || e.has("popover"));
        kernel::mark_disabled(&mut exp.body);
        let (loader, ssr) = match page.server.as_ref().and_then(|s| sources.get(s)) {
            Some(src) => {
                let names = pragma_jslint::exported_names(src);
                let ssr = js::exports(src).iter().any(|(n, v)| n == "ssr" && v.as_deref() == Some("true"));
                (names.iter().any(|n| n == "load"), ssr)
            }
            None => (false, false),
        };
        let owner = |m: &PathBuf| scripts.get_key_value(m).map(|(k, info)| expand::OwnerScript { file: p.origin(k), info });
        let comp_tags: Vec<(String, String)> = exp.components.values().map(|c| (c.id.clone(), format!("<{}-{}>", if c.id == c.name { "c" } else { "p" }, c.name))).collect();
        let ctx = expand::PageCtx {
            loader,
            ssr,
            params: page.pattern.split('/').filter_map(|s| s.strip_prefix(':')).map(str::to_string).collect(),
            page_script: page.module.as_ref().and_then(owner),
            page_file: &p.files[page.html as usize],
            scripts: &scripts,
            comp_scripts: exp
                .components
                .values()
                .zip(&comp_tags)
                .filter_map(|(c, (id, tag))| c.module.as_ref().and_then(owner).map(|s| (id.clone(), (tag.as_str(), s))))
                .collect(),
        };
        let mut sc = expand::Scoped { p, ctx: &ctx, diags: Vec::new(), stores: BTreeSet::new(), shared_classes: BTreeSet::new(), page_classes: BTreeSet::new(), outer: Vec::new() };
        sc.run(&exp.body);
        sc.head(&exp.head);
        let described = exp.head.iter().any(|n| matches!(n, html::Node::Element(e) if e.is("meta") && e.value("name").is_some_and(|v| v.eq_ignore_ascii_case("description"))));
        if !is_404 && !described {
            diags.push(Diagnostic::warning(
                &p.files[page.html as usize],
                1,
                1,
                "html/meta-description",
                "page without <meta name=\"description\">: search results and link previews show it; write one at the top of this page (what it shows, 1-2 sentences), or one in the <head> of routes/_layout.html for every page",
            ));
        }
        let mut queue: Vec<String> = page
            .module
            .iter()
            .chain(exp.components.values().filter_map(|c| c.module.as_ref()))
            .filter_map(|m| sources.get(m))
            .flat_map(|src| shared_refs(src).map(str::to_string).collect::<Vec<_>>())
            .chain(sc.stores.iter().cloned())
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(s) = queue.pop() {
            let Some(f) = p.stores.get(&s) else { continue };
            if !seen.insert(s.clone()) {
                continue;
            }
            if let Some(src) = sources.get(f) {
                queue.extend(shared_refs(src).map(str::to_string));
            }
            sc.stores.insert(s);
        }
        diags.extend(sc.diags);
        shared_classes.extend(sc.shared_classes);
        let page_classes = if is_404 {
            shared_classes.extend(sc.page_classes);
            BTreeSet::new()
        } else {
            sc.page_classes
        };
        let mut bodies = compile::hoist(&mut exp.body);
        let scan = kernel::scan(&exp.body, &bodies);
        let reactive = scan.directive || page.module.is_some() || exp.components.values().any(|c| c.module.is_some()) || !sc.stores.is_empty() || loader || overlays;
        // a control with @open/@close is enabled once overlay.js is in: the route loads it at start
        let overlays = overlays || scan.opens;
        let described = if reactive {
            Some(kernel::describe(&mut exp.body, &mut bodies))
        } else {
            kernel::strip(&mut exp.body, &mut bodies);
            None
        };
        outs.push(PageOut { page, exp, loader, ssr, stores: sc.stores, page_classes, is_404, overlays, bodies, scan, described });
    }
    Analysis { sources, theme_src, comp_css, page_css, classes, outs, shared_classes }
}

struct Client {
    mods: BTreeMap<PathBuf, Module>,
    deps: deps::Deps,
    docs: Vec<String>,
}

/// Route CSS of one page: the per-route stylesheet (its URL and text) and the CSS its fonts come from.
struct RouteCss {
    url: Option<String>,
    src: Option<String>,
    gz: u64,
    font_css: String,
}

/// First-load JS of a page: raw, gzip and brotli bytes.
#[derive(Default, Clone, Copy)]
struct Size {
    raw: u64,
    gz: u64,
    br: u64,
}

fn emit_client(p: &Project, a: &Analysis, opts: &BuildOptions, w: &mut Writer, report: &mut BuildReport, diags: &mut Vec<Diagnostic>) -> Client {
    let prod = opts.mode == Mode::Prod;
    let Analysis { sources, theme_src, comp_css, page_css, classes, outs, shared_classes } = a;
    let mut docs = Vec::with_capacity(outs.len());
    let kernel_js = p.framework.join(kernel::KERNEL);
    if !opts.root.join(&kernel_js).is_file() {
        diags.push(Diagnostic::error(".pragma/page.js", 1, 1, "app/structure", ".pragma/page.js not found; the framework lives in .pragma/ (created by pragma new, updated by pragma upgrade)"));
    }

    let shared_names: Vec<String> = shared_classes.iter().cloned().collect();
    let generated = pragma_css::generate(&shared_names, &classes.theme);
    let mut css = if prod {
        let mut all_used = String::with_capacity(theme_src.len() + generated.css.len() + 4096);
        all_used.push_str(theme_src);
        all_used.push_str(&generated.css);
        for s in comp_css.values().chain(page_css.values()) {
            all_used.push_str(s);
            all_used.push('\n');
        }
        let all_route_vec: Vec<String> = outs.iter().flat_map(|o| o.page_classes.iter()).filter(|c| !shared_classes.contains(*c)).cloned().collect::<BTreeSet<_>>().into_iter().collect();
        all_used.push_str(&pragma_css::generate(&all_route_vec, &classes.theme).css);
        if let Err(e) = pragma_css::minify(theme_src) {
            diags.push(Diagnostic::error("lib/theme.css", 1, 1, "css/syntax", e));
        }
        pragma_css::base_css_for(&all_used)
    } else {
        String::from(pragma_css::base_css())
    };
    css.push('\n');
    css.push_str(&generated.css);
    let mut public_assets = PublicAssets::new(&opts.root, &p.public_files);
    if prod {
        match pragma_css::minify(&css) {
            Ok(m) => css = m,
            Err(e) => diags.push(Diagnostic::error("lib/theme.css", 1, 1, "css/syntax", e)),
        }
        css = public_assets.rewrite(&css, w);
    }
    let css_url = if prod { format!("/assets/app-{}.css", hash8(css.as_bytes())) } else { "/assets/app.css".to_string() };
    w.write_client(&format!("client{css_url}"), css.as_bytes(), prod);
    let shared_css_gz = gzip_len(css.as_bytes());
    report.css_gzip_bytes = shared_css_gz;

    let mut route_css: Vec<RouteCss> = Vec::with_capacity(outs.len());
    for o in outs {
        let mut markup = pragma_css::prune::Markup::default();
        owners::markup_of(&o.exp.body, &mut markup);
        for b in &o.bodies {
            owners::markup_of(&b.children, &mut markup);
        }
        let mut scoped = String::new();
        let mut seen_dirs = HashSet::new();
        for c in o.exp.components.values() {
            if seen_dirs.insert(&c.dir)
                && let Some(s) = comp_css.get(&c.dir)
            {
                scoped.push_str(&format!("/* {} */\n{}\n", rel_str(p.origin(c.css.as_ref().unwrap())), pragma_css::prune::prune_scoped(s, &markup).trim()));
            }
        }
        if let Some(f) = &o.page.css
            && let Some(s) = page_css.get(f)
        {
            scoped.push_str(&format!("/* {} */\n{}\n", rel_str(p.origin(f)), pragma_css::prune::prune_scoped(s, &markup).trim()));
        }
        let route_names: Vec<String> = o.page_classes.iter().filter(|c| !shared_classes.contains(*c)).cloned().collect();
        scoped.push_str(&pragma_css::generate(&route_names, &classes.theme).css);
        let route_theme = pragma_css::prune::prune_theme(theme_src, &markup, &scoped);
        let mut rcss = if route_theme.trim().is_empty() { scoped } else { format!("/* lib/theme.css */\n{}\n{scoped}", route_theme.trim()) };
        if prod {
            rcss = public_assets.rewrite(&rcss, w);
        }
        let font_css = format!("{css}\n{rcss}");
        let mut out = RouteCss { url: None, src: None, gz: 0, font_css };
        if !rcss.trim().is_empty() {
            if prod {
                match pragma_css::minify(&rcss) {
                    Ok(m) => rcss = m,
                    Err(e) => {
                        let file = o.page.css.as_deref().unwrap_or(&p.files[o.page.html as usize]);
                        diags.push(Diagnostic::error(file, 1, 1, "css/syntax", e));
                    }
                }
            }
            let css_name = url_safe(&o.page.out_name);
            let url = if prod { format!("/assets/css/{css_name}-{}.css", hash8(rcss.as_bytes())) } else { format!("/assets/css/{css_name}.css") };
            w.write_client(&format!("client{url}"), rcss.as_bytes(), prod);
            out.gz = gzip_len(rcss.as_bytes());
            out.url = Some(url);
            out.src = Some(rcss);
        }
        route_css.push(out);
    }

    // the entry of each reactive route (kernel-per-route §3.1, §4)
    let mut missing: BTreeSet<&str> = BTreeSet::new();
    let mut entries: Vec<Option<kernel::Entry>> = Vec::with_capacity(outs.len());
    for (o, rc) in outs.iter().zip(&route_css) {
        let Some(described) = &o.described else {
            entries.push(None);
            continue;
        };
        let persists = |f: &PathBuf| sources.get(f).is_some_and(|src| pragma_jslint::exported_names(src).iter().any(|n| n == "persist"));
        let stores: Vec<(String, PathBuf, bool)> = p.stores.iter().filter(|(name, _)| o.stores.contains(*name)).map(|(name, f)| (name.clone(), f.clone(), persists(f))).collect();
        let motion = [css.as_str(), rc.src.as_deref().unwrap_or("")].iter().any(|c| c.contains("data-entering") || c.contains("data-leaving"));
        let uses = kernel::Uses {
            branch: o.scan.branch,
            list: o.scan.list,
            model: o.scan.model,
            motion: o.scan.blocks() && motion,
            selfs: o.scan.selfs,
            vars: o.scan.vars,
            persist: stores.iter().any(|s| s.2),
            net: o.loader || p.layout_server.is_some(),
        };
        for (_, file) in uses.modules() {
            if !opts.root.join(p.framework.join(file)).is_file() {
                missing.insert(file);
            }
        }
        let (t, d) = compile::body_tables(&o.bodies, prod);
        entries.push(Some(kernel::Entry::new(kernel::EntryInput {
            framework: &p.framework,
            pattern: &o.page.pattern,
            loader: o.loader,
            ssr: o.ssr,
            layout: p.layout_server.is_some(),
            overlays: o.overlays,
            uses,
            page: o.page.module.as_ref(),
            comps: o.exp.components.values().filter_map(|c| c.module.clone().map(|m| (c.id.clone(), m))).collect(),
            stores,
            t,
            d,
            b: &described.b,
        })));
    }
    for file in missing {
        diags.push(Diagnostic::error(format!(".pragma/{file}"), 1, 1, "app/structure", format!(".pragma/{file} not found; the framework is out of date: run pragma upgrade")));
    }

    let all_entries: Vec<PathBuf> = entries.iter().flatten().flat_map(kernel::Entry::paths).collect();
    let deps = deps::Deps::new(&opts.root, diags);
    let mut requests = BTreeMap::new();
    let mut mods = module_graph(p, &all_entries, true, sources, &deps, &mut requests, diags);
    let unused_components: Vec<PathBuf> = p.all_components().filter_map(|c| c.module.clone()).filter(|m| !mods.contains_key(m)).collect();
    module_graph(p, &unused_components, true, sources, &deps, &mut BTreeMap::new(), diags);
    let out_abs = std::path::absolute(&opts.out_dir).unwrap_or_else(|_| opts.out_dir.clone());
    if let Some(b) = bundle::bundle(&opts.root, &out_abs.join(".cache/npm"), prod, &requests, diags) {
        link_packages(&mut mods, &b, diags);
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
        let rel = url_safe(&deps::asset_path(&rel_str(path)));
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

    let routes: Vec<chunks::Route> = outs.iter().zip(entries).map(|(o, entry)| chunks::Route { name: format!("routes/{}", o.page.out_name), entry }).collect();
    let chunked = if prod && !diags.iter().any(|d| d.severity == Severity::Error) {
        chunks::bundle(&out_abs.join(".cache/app"), &mods, &urls, &kernel_js, &routes, diags)
    } else {
        None
    };

    // every JS file a page can load, by URL: the first-load report reads its sizes
    let mut js: HashMap<String, String> = HashMap::new();
    for (path, m) in &mods {
        if chunked.is_some() && !path.starts_with("node_modules") {
            continue;
        }
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
        let code = if !prod && *path == p.framework.join("log.js") { with_ignore_map(code) } else { code };
        w.write_client(&format!("client{}", urls[path]), code.as_bytes(), prod);
        js.insert(urls[path].clone(), code);
    }
    for (file, code) in chunked.iter().flat_map(|c| &c.files) {
        w.write_client(&format!("client/assets/{file}"), code.as_bytes(), prod);
        js.insert(format!("/assets/{file}"), code.clone());
    }
    // dev (or a prod build with errors): the entry imports each module by the URL it is served at,
    // so the kernel the entry imports is the one `page.js` imports its lazy modules next to
    let dev_entry = |o: &PageOut| format!("/assets/entry/{}.js", url_safe(&o.page.out_name));
    if chunked.is_none() {
        for (o, r) in outs.iter().zip(&routes) {
            let Some(entry) = &r.entry else { continue };
            let code = entry.source(|m| urls.get(m).cloned().unwrap_or_else(|| format!("/assets/{}", url_safe(&deps::asset_path(&rel_str(m))))));
            let url = dev_entry(o);
            w.write_client(&format!("client{url}"), code.as_bytes(), prod);
            js.insert(url, code);
        }
    }
    let npm_paths: HashMap<&str, &PathBuf> = urls.iter().filter(|(k, _)| k.starts_with("node_modules")).map(|(k, u)| (u.as_str(), k)).collect();
    let mut sizes: HashMap<String, Size> = HashMap::new();
    let mut size_of = |u: &str| -> Size {
        *sizes.entry(u.to_string()).or_insert_with(|| {
            let Some(code) = js.get(u) else { return Size::default() };
            let b = code.as_bytes();
            Size { raw: b.len() as u64, gz: gzip_len(b), br: brotli_compress(b).map_or(0, |c| c.len() as u64) }
        })
    };

    let theme_js = mods.contains_key(&p.framework.join("theme.js"));
    let mut manifest = Vec::new();
    let packages = mods.iter().flat_map(|(path, m)| {
        m.imports.iter().filter(|(imp, _)| pragma_jslint::is_package(&imp.spec)).filter_map(move |(imp, t)| t.clone().map(|t| ((path.clone(), imp.spec.clone()), t)))
    });
    let prerender = prerender::Prerender::new(&opts.root, &p.framework, mods.iter().map(|(path, m)| (path.clone(), m.src.clone())), packages);
    let on_server = |m: &PathBuf| server_runnable(&mods, m);
    for (i, o) in outs.iter().enumerate() {
        let rc = &route_css[i];
        // the entry and its static-import closure, in load order (lazy chunks are never preloaded)
        let (entry_url, module_urls): (Option<String>, Vec<String>) = match (&routes[i].entry, &chunked) {
            (None, _) => (None, Vec::new()),
            (Some(_), Some(ch)) => match ch.routes[i].clone() {
                Some(entry) => {
                    let mut list: Vec<String> = Vec::new();
                    for u in ch.closure(&[entry.as_str()]) {
                        match npm_paths.get(u.as_str()) {
                            Some(npm) => list.extend(closure(&mods, &[(*npm).clone()], false).iter().map(|m| urls[m].clone())),
                            None => list.push(u),
                        }
                    }
                    let mut seen = HashSet::new();
                    list.retain(|u| *u != entry && seen.insert(u.clone()));
                    (Some(entry), list)
                }
                None => (None, Vec::new()),
            },
            (Some(entry), None) => {
                let mut seen = HashSet::new();
                let list = closure(&mods, &entry.paths(), false).iter().map(|m| urls[m].clone()).filter(|u| seen.insert(u.clone())).collect();
                (Some(dev_entry(o)), list)
            }
        };
        let reactive = o.described.is_some();
        let rendered = prerender.as_ref().and_then(|r| {
            let scripted: BTreeSet<String> = o.exp.components.values().filter(|c| c.module.is_some()).map(|c| c.id.clone()).collect();
            let ops = compile::ops(&o.exp.body, &scripted, &o.bodies);
            let page = o.page.module.as_deref().filter(|m| on_server(&m.to_path_buf()));
            let comps: Vec<(String, PathBuf)> = o.exp.components.values().filter_map(|c| c.module.clone().filter(|m| on_server(m)).map(|m| (c.id.clone(), m))).collect();
            r.body(&ops, page, &comps)
        });
        let body = rendered.unwrap_or_else(|| {
            let mut body = String::new();
            compile::write_nodes(&o.exp.body, &mut body, prod);
            body
        });
        let mut painted = pragma_css::prune::Markup::default();
        owners::painted_markup(&html::parse(&body, 0).0, &mut painted);
        let fonts = pragma_css::prune::font_preloads(&rc.font_css, &painted);
        let mut doc = String::with_capacity(4096);
        doc.push_str(&format!("<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n", html::escape_attr(&p.config.lang)));
        match o.exp.head.iter().find_map(|n| match n {
            html::Node::Element(e) if e.is("title") => Some(e),
            _ => None,
        }) {
            Some(t) => compile::write_element(t, &mut doc, true),
            None => doc.push_str(&format!("<title>{}</title>", html::escape_text(&p.config.name))),
        }
        doc.push('\n');
        if o.loader && !o.ssr {
            doc.push_str(PD);
            doc.push('\n');
        }
        if reactive && p.layout_server.is_some() && !o.ssr {
            doc.push_str(PL);
            doc.push('\n');
        }
        let extras: Vec<html::Node> = o.exp.head.iter().filter(|n| !matches!(n, html::Node::Element(e) if e.is("title"))).cloned().collect();
        let mut ex = String::new();
        compile::write_nodes(&extras, &mut ex, true);
        let ex = ex.trim();
        if !ex.is_empty() {
            doc.push_str(ex);
            doc.push('\n');
        }
        for href in fonts.iter().filter(|h| !ex.contains(&format!("href=\"{h}\""))) {
            doc.push_str(&format!("<link rel=\"preload\" href=\"{}\" as=\"font\" type=\"font/woff2\" crossorigin>\n", html::escape_attr(href)));
        }
        if theme_js {
            doc.push_str(THEME);
            doc.push('\n');
        }
        if prod && shared_css_gz + rc.gz <= 14 * 1024 {
            doc.push_str("<style data-pragma-shared-css>");
            doc.push_str(&css.replace("</style", "<\\/style"));
            doc.push_str("</style>\n");
            if let Some(rcss) = &rc.src {
                doc.push_str("<style data-pragma-route-css>");
                doc.push_str(&rcss.replace("</style", "<\\/style"));
                doc.push_str("</style>\n");
            }
        } else {
            doc.push_str(&format!("<link rel=\"stylesheet\" href=\"{css_url}\">\n"));
            if let Some(rcss_url) = &rc.url {
                doc.push_str(&format!("<link rel=\"stylesheet\" href=\"{rcss_url}\" data-pragma-route-css>\n"));
            }
        }
        if !prod && reactive {
            doc.push_str("<script>self.__pragmaDev=true</script>\n");
        }
        for u in &module_urls {
            doc.push_str(&format!("<link rel=\"modulepreload\" href=\"{u}\">\n"));
        }
        if let Some(u) = &entry_url {
            doc.push_str(&format!("<script type=\"module\" src=\"{u}\"></script>\n"));
        }
        doc.push_str(kernel::VT);
        doc.push('\n');
        if prod {
            doc.push_str(SPECULATION_RULES);
            doc.push('\n');
        } else {
            doc.push_str("<script type=\"module\" src=\"/_pragma/dev.js\"></script>\n");
        }
        match o.described.as_ref().and_then(|d| d.body.as_ref()) {
            Some(id) => doc.push_str(&format!("</head>\n<body data-b=\"{id}\">\n")),
            None => doc.push_str("</head>\n<body>\n"),
        }
        doc.push_str(body.trim());
        doc.push_str("\n</body>\n</html>\n");
        let html_url = format!("/{}.html", o.page.out_name);
        w.write_client(&format!("client{html_url}"), doc.as_bytes(), prod);

        let first = module_urls.iter().chain(entry_url.iter()).fold(Size::default(), |t, u| {
            let s = size_of(u);
            Size { raw: t.raw + s.raw, gz: t.gz + s.gz, br: t.br + s.br }
        });
        let total_css_gz = shared_css_gz + rc.gz;
        report.css_gzip_bytes = report.css_gzip_bytes.max(total_css_gz);
        if !o.is_404 {
            manifest.push(json!({ "pattern": o.page.pattern, "html": html_url, "entry": entry_url, "modules": module_urls, "loader": o.loader, "ssr": o.ssr }));
            report.pages.push(PageReport { pattern: o.page.pattern.clone(), js_raw_bytes: first.raw, js_gzip_bytes: first.gz, js_brotli_bytes: first.br, css_gzip_bytes: total_css_gz });
        }
        docs.push(doc);
    }
    w.write_client(&format!("{}/{}", layout::CLIENT, layout::MANIFEST), serde_json::to_string_pretty(&Value::Array(manifest)).unwrap().as_bytes(), prod);

    for f in &p.public_files {
        let rel = rel_str(f.strip_prefix("public").unwrap());
        let target = format!("client/{rel}");
        if w.has(&target) {
            diags.push(Diagnostic::error(f, 1, 1, "app/structure", format!("public/{rel} conflicts with a file generated by the build")));
            continue;
        }
        if let Ok(bytes) = fs::read(opts.root.join(f)) {
            w.write_client(&target, &bytes, prod);
        }
    }

    Client { mods, deps, docs }
}

struct ServerRoute {
    pattern: String,
    server: PathBuf,
    ssr: bool,
    template: Value,
    shell: Value,
    scripts: SsrScripts,
}

fn emit_server(p: &Project, a: &Analysis, client: &Client, opts: &BuildOptions, w: &mut Writer, report: &mut BuildReport, diags: &mut Vec<Diagnostic>) {
    let (sources, deps, mods) = (&a.sources, &client.deps, &client.mods);
    let mut server_routes = Vec::new();
    for (o, doc) in a.outs.iter().zip(&client.docs) {
        if let Some(server) = &o.page.server {
            let mut ssr_scripts = SsrScripts::default();
            let (template, shell) = if o.ssr {
                let scripted: BTreeSet<String> = o.exp.components.values().filter(|c| c.module.is_some()).map(|c| c.id.clone()).collect();
                let ops = compile::ops(compile::page_content(&o.exp.body), &scripted, &o.bodies);
                // the server runs a script when everything it imports is app code or an npm package
                // (no browser runtime of the framework); the others stay client-rendered
                let runs_on_server = |m: &PathBuf| server_runnable(mods, m);
                ssr_scripts.page = o.page.module.clone().filter(|m| runs_on_server(m));
                ssr_scripts.components = o.exp.components.values().filter_map(|c| c.module.clone().filter(|m| runs_on_server(m)).map(|m| (c.id.clone(), m))).collect();
                ssr_scripts.lazy.extend(ssr_scripts.page.iter().chain(ssr_scripts.components.iter().map(|(_, m)| m)).filter(|m| reaches_npm(mods, m)).cloned());
                (ops, Value::String(doc.clone()))
            } else {
                (Value::Null, Value::Null)
            };
            server_routes.push(ServerRoute { pattern: o.page.pattern.clone(), server: server.clone(), ssr: o.ssr, template, shell, scripts: ssr_scripts });
        }
    }
    let has_public_md = p.public_files.iter().any(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")));
    diags.extend(check_endpoints(p, sources));
    report.has_server = !server_routes.is_empty() || !p.endpoints.is_empty() || p.middleware.is_some() || p.layout_server.is_some();
    let emit_worker = report.has_server || has_public_md;
    let handler = p.framework.join("server/handler.js");
    let node_adapter = p.framework.join("server/node.mjs");
    // no *.server.js: static-only app, no node in dev/serve (BUG-S-1); still emit Cloudflare worker
    // when public/**/*.md exists so Accept: text/markdown negotiation works on Cloudflare Workers.
    if emit_worker && opts.root.join(&handler).is_file() {
        let mut script_ids: BTreeMap<PathBuf, usize> = BTreeMap::new();
        for r in &server_routes {
            for m in r.scripts.page.iter().chain(r.scripts.components.iter().map(|(_, m)| m)) {
                let next = script_ids.len();
                script_ids.entry(m.clone()).or_insert(next);
            }
        }
        let mut entries = vec![handler.clone()];
        if report.has_server {
            entries.push(node_adapter.clone());
        }
        entries.extend(server_routes.iter().map(|r| r.server.clone()));
        entries.extend(p.endpoints.iter().map(|e| e.file.clone()));
        entries.extend(p.middleware.clone());
        entries.extend(p.layout_server.clone());
        entries.extend(script_ids.keys().cloned());
        entries.retain(|e| opts.root.join(e).is_file() || sources.contains_key(e));
        let smods = module_graph(p, &entries, false, sources, deps, &mut BTreeMap::new(), diags);
        let mut wasm: BTreeMap<String, PathBuf> = BTreeMap::new();
        for (path, m) in &smods {
            let out_rel = deps::asset_path(&rel_str(path));
            let dir = path.parent().unwrap_or(Path::new(""));
            let mut moved: HashMap<&str, String> = HashMap::new();
            for (imp, t) in &m.imports {
                if let Some(t) = t.as_ref().filter(|t| is_wasm(t)) {
                    wasm.insert(deps::asset_path(&rel_str(t)), t.clone());
                } else if t.is_none() && pragma_jslint::is_package(&imp.spec) && let Some(file) = deps.resolve_server_wasm(dir, &imp.spec) {
                    let out = deps::asset_path(&rel_str(&file));
                    moved.insert(&imp.spec, relative_spec(&out_rel, &out));
                    wasm.insert(out, file);
                } else if imp.spec == "pragma/server" && let Some(t) = t {
                    moved.insert(&imp.spec, relative_spec(&out_rel, &deps::asset_path(&rel_str(t))));
                }
            }
            if moved.is_empty() {
                w.write(&format!("server/{out_rel}"), m.src.as_bytes());
            } else {
                let specs: Vec<js::Import> = m.imports.iter().map(|(imp, _)| imp.clone()).collect();
                let src = js::rewrite(&m.src, &specs, |imp| moved.get(imp.spec.as_str()).cloned());
                w.write(&format!("server/{out_rel}"), src.as_bytes());
            }
        }
        for (out, file) in &wasm {
            match fs::read(opts.root.join(file)) {
                Ok(bytes) => w.write(&format!("server/{out}"), &bytes),
                Err(e) => diags.push(Diagnostic::error(file, 1, 1, deps::RULE_PACKAGE, format!("cannot read {}: {e}", rel_str(file)))),
            }
        }
        let mut worker = format!("import {{ createHandler }} from './{}'\n", deps::asset_path(&rel_str(&handler)));
        let node_adapter = deps::asset_path(&rel_str(&node_adapter));
        for (i, r) in server_routes.iter().enumerate() {
            worker.push_str(&format!("import * as s{i} from {}\n", json!(format!("./{}", rel_str(&r.server)))));
        }
        let lazy: BTreeSet<&PathBuf> = server_routes.iter().flat_map(|r| &r.scripts.lazy).collect();
        for (m, i) in &script_ids {
            let spec = json!(format!("./{}", rel_str(m)));
            if lazy.contains(m) {
                worker.push_str(&format!("const m{i} = () => import({spec})\n"));
            } else {
                worker.push_str(&format!("import * as m{i} from {spec}\n"));
            }
        }
        for (i, e) in p.endpoints.iter().enumerate() {
            worker.push_str(&format!("import * as e{i} from {}\n", json!(format!("./{}", rel_str(&e.file)))));
        }
        if let Some(m) = &p.middleware {
            worker.push_str(&format!("import {{ handle as middleware }} from {}\n", json!(format!("./{}", rel_str(m)))));
        }
        if let Some(l) = &p.layout_server {
            worker.push_str(&format!("import * as layout from {}\n", json!(format!("./{}", rel_str(l)))));
        }
        worker.push_str("\nexport default createHandler({\n");
        worker.push_str(&format!("  service: {},\n", json!(p.config.name)));
        if p.middleware.is_some() {
            worker.push_str("  middleware,\n");
        }
        if p.layout_server.is_some() {
            worker.push_str("  layout,\n");
        }
        worker.push_str("  routes: [\n");
        for (i, ServerRoute { pattern, ssr, template, shell, scripts, .. }) in server_routes.iter().enumerate() {
            let mut extra = String::new();
            if let Some(m) = &scripts.page {
                extra.push_str(&format!(", page: m{}", script_ids[m]));
            }
            if !scripts.components.is_empty() {
                let list: Vec<String> = scripts.components.iter().map(|(id, m)| format!("{}: m{}", json!(id), script_ids[m])).collect();
                extra.push_str(&format!(", components: {{ {} }}", list.join(", ")));
            }
            worker.push_str(&format!("    {{ pattern: {}, server: s{i}, ssr: {ssr}, template: {template}, shell: {shell}{extra} }},\n", json!(pattern)));
        }
        for (i, e) in p.endpoints.iter().enumerate() {
            worker.push_str(&format!("    {{ pattern: {}, endpoint: e{i}, file: {} }},\n", json!(e.pattern), json!(rel_str(&e.file))));
        }
        worker.push_str("  ],\n})\n");
        w.write("server/worker.js", worker.as_bytes());
        if report.has_server {
            let routing = json!({ "middleware": p.middleware.is_some(), "layout": p.layout_server.is_some(), "endpoints": p.endpoints.iter().map(|e| e.pattern.clone()).collect::<Vec<_>>() });
            w.write(layout::SERVER_ROUTES, serde_json::to_string_pretty(&routing).unwrap().as_bytes());
        }
        w.write("server/package.json", b"{ \"type\": \"module\" }\n");
        if report.has_server {
            let options = format!("{{ root: new URL('../client/', import.meta.url), port: process.env.PORT ?? 3000, service: {} }}", json!(p.config.name));
            // a static import graph is loaded before any code runs: the .wasm and cloudflare:*
            // hooks must be registered before worker.js is imported
            let node = format!("import {{ serve, registerNodeHooks }} from './{node_adapter}'\n\nregisterNodeHooks()\nconst {{ default: h }} = await import('./worker.js')\nserve(h, {options})\n");
            w.write(layout::NODE_ENTRY, node.as_bytes());
        }
        let mut wrangler = json!({
            "name": p.config.name,
            "main": "worker.js",
            "compatibility_date": "2025-11-01",
            "assets": { "directory": "../client", "binding": "ASSETS", "html_handling": "drop-trailing-slash", "run_worker_first": true },
        });
        if !wasm.is_empty() {
            wrangler["rules"] = json!([{ "type": "CompiledWasm", "globs": ["**/*.wasm"], "fallthrough": true }]);
        }
        w.write("server/wrangler.jsonc", serde_json::to_string_pretty(&wrangler).unwrap().as_bytes());
    } else if report.has_server {
        diags.push(Diagnostic::error(".pragma/server/handler.js", 1, 1, "app/structure", ".pragma/server/handler.js not found; required by *.server.js"));
    }

}

/// Store names after `shared.` in JS source (a text scan: strings and comments count too).
fn shared_refs(src: &str) -> impl Iterator<Item = &str> {
    src.match_indices("shared.").filter_map(|(i, m)| {
        let boundary = src[..i].chars().next_back().is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '$'));
        let rest = &src[i + m.len()..];
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')).unwrap_or(rest.len());
        (boundary && end > 0).then(|| &rest[..end])
    })
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
