//! Kernel per route (core/proposals/kernel-per-route.md): what a route uses (§4), its descriptors
//! (§5), the build `disabled` of JS-only controls (§6) and its entry module (§3.1).
use crate::compile::{BUILD_DISABLED, DESC, moves_to_descriptor};
use crate::directive::{self, Arg, Dir, INTERP};
use crate::html::{Attr, Element, Node};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Inline in every page's head: `html[data-view-transition]` while a cross-document transition runs (§3.4).
pub const VT: &str = "<script>addEventListener('pageswap',e=>e.viewTransition&&document.documentElement.setAttribute('data-view-transition',''));addEventListener('pagereveal',e=>{const v=e.viewTransition,h=document.documentElement;if(v)h.setAttribute('data-view-transition',''),v.finished.finally(()=>h.removeAttribute('data-view-transition'))});addEventListener('pageshow',e=>e.persisted&&document.documentElement.removeAttribute('data-view-transition'))</script>";

/// The kernel file the route entry imports `start` from.
pub const KERNEL: &str = "page.js";

/// Constructs of a route's closure (body + hoisted bodies) that decide its modules.
#[derive(Debug, Default, Clone, Copy)]
pub struct Scan {
    /// Any runtime directive or `#interp` (§3.3).
    pub directive: bool,
    pub branch: bool,
    pub list: bool,
    pub model: bool,
    pub selfs: bool,
    pub vars: bool,
    /// `data-open` / `data-close`: the route loads `overlay.js` at start (`R.overlays`).
    pub opens: bool,
}

impl Scan {
    pub fn blocks(&self) -> bool {
        self.branch || self.list
    }
}

/// Per-use modules of a route, in the order of §1.
#[derive(Debug, Default, Clone, Copy)]
pub struct Uses {
    pub branch: bool,
    pub list: bool,
    pub model: bool,
    pub motion: bool,
    pub selfs: bool,
    pub vars: bool,
    pub persist: bool,
    pub net: bool,
}

impl Uses {
    /// (import name, file in the framework folder).
    pub fn modules(&self) -> Vec<(&'static str, &'static str)> {
        let all = [
            (self.branch, "branch", "branch.js"),
            (self.list, "list", "list.js"),
            (self.model, "model", "model.js"),
            (self.motion, "motion", "motion.js"),
            (self.selfs, "recur", "self.js"),
            (self.vars, "vars", "vars.js"),
            (self.persist, "persist", "persist.js"),
            (self.net, "net", "net.js"),
        ];
        all.into_iter().filter(|(on, ..)| *on).map(|(_, name, file)| (name, file)).collect()
    }
}

fn runtime_dir(d: Dir) -> bool {
    matches!(d, Dir::Text | Dir::Attr(_) | Dir::Var(_) | Dir::Class(_) | Dir::Show | Dir::If | Dir::ElseIf | Dir::Else | Dir::For | Dir::Empty | Dir::On(_) | Dir::Model | Dir::Open | Dir::Close)
}

pub fn scan(body: &[Node], bodies: &[Element]) -> Scan {
    let mut s = Scan::default();
    scan_nodes(body, &mut s);
    for b in bodies {
        scan_nodes(&b.children, &mut s);
    }
    s
}

fn scan_nodes(nodes: &[Node], s: &mut Scan) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.name == INTERP {
            s.directive = true;
            continue;
        }
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            if lname == "data-self" {
                s.directive = true;
                s.selfs = true;
                continue;
            }
            let Some(Some(d)) = directive::parse(&lname) else { continue };
            if !runtime_dir(d) {
                continue;
            }
            s.directive = true;
            match d {
                Dir::If | Dir::ElseIf | Dir::Else | Dir::Empty if e.is("template") => s.branch = true,
                Dir::For if e.is("template") => s.list = true,
                Dir::Model => s.model = true,
                Dir::Var(_) => s.vars = true,
                Dir::Open | Dir::Close => s.opens = true,
                _ => {}
            }
        }
        scan_nodes(&e.children, s);
    }
}

/// §6: a JS-only control gets `BUILD_DISABLED`. Runs on the whole tree before `compile::hoist`, so
/// a control inside a block body still sees its `<form>`.
pub fn mark_disabled(nodes: &mut [Node]) {
    let mut forms = HashMap::new();
    collect_forms(nodes, &mut forms);
    mark(nodes, &forms, None);
}

/// (has `action`, has `data-on-submit`) of a form.
type Form = (bool, bool);

fn form_of(e: &Element) -> Form {
    (e.has("action"), e.has("data-on-submit"))
}

fn collect_forms(nodes: &[Node], out: &mut HashMap<String, Form>) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.is("form")
            && let Some(id) = e.value("id")
        {
            out.entry(id.to_string()).or_insert_with(|| form_of(e));
        }
        collect_forms(&e.children, out);
    }
}

fn mark(nodes: &mut [Node], forms: &HashMap<String, Form>, form: Option<Form>) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        let owner = match e.value("form") {
            Some(id) => forms.get(id).copied(),
            None => form,
        };
        if js_only(e, owner) {
            e.attrs.push(Attr { name: BUILD_DISABLED.into(), value: None, pos: e.pos });
        }
        let inner = if e.is("form") { Some(form_of(e)) } else { form };
        mark(&mut e.children, forms, inner);
    }
}

fn js_only(e: &Element, owner: Option<Form>) -> bool {
    if e.has("disabled") {
        return false;
    }
    let ty = e.value("type").map(|t| t.trim().to_ascii_lowercase());
    let submit = if e.is("button") {
        ty.as_deref().is_none_or(|t| t == "submit")
    } else if e.is("input") {
        let Some(t) = ty.as_deref() else { return false };
        if !matches!(t, "button" | "submit" | "reset" | "image" | "checkbox" | "radio" | "range" | "color" | "file") {
            return false;
        }
        matches!(t, "submit" | "image")
    } else if e.is("select") {
        false
    } else {
        return false;
    };
    let js = e.attrs.iter().any(|a| {
        let n = a.name.to_ascii_lowercase();
        n.starts_with("data-on-") || n == "data-open" || n == "data-close"
    });
    if js {
        !(submit && owner == Some((true, false)))
    } else {
        submit && owner == Some((false, true))
    }
}

/// A static route: the build writes no `BUILD_DISABLED` and none of the attributes a descriptor takes.
pub fn strip(body: &mut [Node], bodies: &mut [Element]) {
    fn walk(nodes: &mut [Node]) {
        for n in nodes {
            let Node::Element(e) = n else { continue };
            e.attrs.retain(|a| a.name != BUILD_DISABLED && !moves_to_descriptor(&a.name));
            walk(&mut e.children);
        }
    }
    walk(body);
    for b in bodies {
        walk(&mut b.children);
    }
}

struct Table {
    prefix: char,
    ids: HashMap<String, String>,
    list: Vec<(String, Value)>,
}

impl Table {
    fn new(prefix: char) -> Self {
        Table { prefix, ids: HashMap::new(), list: Vec::new() }
    }

    fn id(&mut self, d: Value) -> String {
        let key = d.to_string();
        if let Some(id) = self.ids.get(&key) {
            return id.clone();
        }
        let id = format!("{}{}", self.prefix, crate::compile::base36(self.list.len()));
        self.ids.insert(key, id.clone());
        self.list.push((id.clone(), d));
        id
    }
}

/// Descriptor tables of a reactive route: `b` (layout ids first) and, when the layout writes
/// `{path}` beside elements at its top level, the id `<body>` carries.
pub struct Described {
    pub b: Vec<(String, Value)>,
    pub body: Option<String>,
}

/// §5: gives every element and anchor with something for the runtime a `data-b` id, in the body
/// and in the hoisted bodies (prefix of the region each came from).
pub fn describe(body: &mut [Node], bodies: &mut [Element]) -> Described {
    let (mut l, mut p) = (Table::new('l'), Table::new('p'));
    walk(body, false, &mut l, &mut p);
    let marks = markers(body);
    let body_id = (!marks.is_empty()).then(|| l.id(json!({ "b": marks })));
    for b in bodies.iter_mut() {
        let page = b.value("data-tpl").is_some_and(|id| id.starts_with('t')) || b.value("data-r") == Some("t");
        walk(&mut b.children, page, &mut l, &mut p);
    }
    let mut out = l.list;
    out.extend(p.list);
    Described { b: out, body: body_id }
}

fn walk(nodes: &mut [Node], page: bool, l: &mut Table, p: &mut Table) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.name == INTERP {
            continue;
        }
        let table = if page { &mut *p } else { &mut *l };
        if directive::is_block(e) {
            let id = table.id(anchor(e));
            e.attrs.push(Attr { name: DESC.into(), value: Some(id), pos: e.pos });
            continue;
        }
        let d = element(e);
        if !d.is_empty() {
            let id = table.id(Value::Object(d));
            e.attrs.push(Attr { name: DESC.into(), value: Some(id), pos: e.pos });
        }
        let inner = page || (e.is("main") && e.has("data-page"));
        walk(&mut e.children, inner, l, p);
    }
}

fn path(v: &str) -> Value {
    Value::Array(v.split('.').map(|k| Value::String(k.to_string())).collect())
}

/// `"{t.text} left"` → `["", ["t","text"], " left"]`.
fn tpl(v: &str) -> Value {
    let Ok(parts) = directive::template(v) else { return json!([v]) };
    let mut out = vec![Value::String(String::new())];
    for part in parts {
        match part {
            directive::Part::Text(t) => {
                let Some(Value::String(last)) = out.last_mut() else { unreachable!() };
                last.push_str(&t);
            }
            directive::Part::Path(p) => {
                out.push(path(&p));
                out.push(Value::String(String::new()));
            }
        }
    }
    Value::Array(out)
}

fn neg(v: &str) -> (Value, u8) {
    let (n, p) = directive::split_neg(v);
    (path(p), u8::from(n))
}

fn literal(s: &str) -> Value {
    if let Some(q @ ('\'' | '"')) = s.chars().next() {
        return Value::String(s[1..].strip_suffix(q).unwrap_or(&s[1..]).to_string());
    }
    if let Ok(v) = serde_json::from_str::<Value>(s) {
        return v;
    }
    s.parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::Null, Value::Number)
}

/// `fn(args)` → `[fn path, args]`; arg: `0` event, `1` el, `{v}` literal, path.
fn call(v: &str) -> Value {
    let Ok(c) = directive::parse_call(v) else { return json!([path(v.trim()), [0]]) };
    let args: Vec<Value> = match &c.args {
        None => vec![json!(0)],
        Some(args) => args
            .iter()
            .map(|a| match a {
                Arg::Event => json!(0),
                Arg::El => json!(1),
                Arg::Literal(s) => json!({ "v": literal(s) }),
                Arg::Path(p) => path(p),
            })
            .collect(),
    };
    json!([path(c.name), args])
}

fn operand(v: &str) -> Value {
    match serde_json::from_str::<Value>(v) {
        Ok(x) if !x.is_object() && !x.is_array() => json!({ "v": x }),
        _ => path(v),
    }
}

/// MARK bindings of `{path}` markers among direct children.
fn markers(children: &[Node]) -> Vec<Value> {
    let mut out = Vec::new();
    for n in children {
        let Node::Element(c) = n else { continue };
        if c.name != INTERP {
            continue;
        }
        let v = c.value("data-text").unwrap_or("");
        let p = directive::template_paths(v).into_iter().next().unwrap_or_default();
        let slot: u64 = c.value("data-slot").and_then(|s| s.parse().ok()).unwrap_or(0);
        out.push(json!([5, path(&p), out.len(), slot]));
    }
    out
}

fn element(e: &Element) -> Map<String, Value> {
    let mut d = Map::new();
    let mut b: Vec<Value> = Vec::new();
    let mut events = Map::new();
    let mut with: Vec<Value> = Vec::new();
    let mut props = Map::new();
    for a in &e.attrs {
        let lname = a.name.to_ascii_lowercase();
        let v = a.value.as_deref().unwrap_or("");
        let Some(Some(dir)) = directive::parse(&lname) else { continue };
        match dir {
            Dir::Text => b.push(json!([0, tpl(v)])),
            Dir::Show => {
                let (p, n) = neg(v);
                b.push(json!([1, p, n]));
            }
            Dir::Attr(n) if directive::has_braces(v) => b.push(json!([4, tpl(v), 0, n])),
            Dir::Attr(n) => b.push(json!([2, path(v), 0, n])),
            Dir::Class(c) => {
                let (p, n) = neg(v);
                b.push(json!([3, p, n, c]));
            }
            Dir::Var(n) => b.push(json!([6, path(v), 0, n])),
            Dir::On(t) => {
                events.insert(t.to_string(), call(v));
            }
            Dir::Model => {
                d.insert("m".into(), path(v));
            }
            Dir::With(n) => with.push(json!([n, operand(v)])),
            Dir::Open | Dir::Close => {
                d.insert("o".into(), json!(1));
            }
            Dir::Marker if lname == "data-page" && e.is("main") => {
                d.insert("x".into(), json!(2));
            }
            Dir::Generated => match lname.as_str() {
                "data-c" => {
                    d.insert("x".into(), json!(1));
                    d.insert("c".into(), json!(v));
                }
                "data-self" => {
                    d.insert("self".into(), json!(v));
                }
                "data-slot" => {
                    d.insert("u".into(), json!(v.parse::<u64>().unwrap_or(0)));
                }
                _ => {
                    if let Some(n) = lname.strip_prefix("data-prop-") {
                        props.insert(n.to_string(), json!(v));
                    }
                }
            },
            _ => {}
        }
    }
    b.extend(markers(&e.children));
    if !b.is_empty() {
        d.insert("b".into(), Value::Array(b));
    }
    if !events.is_empty() {
        d.insert("e".into(), Value::Object(events));
    }
    if !with.is_empty() {
        d.insert("w".into(), Value::Array(with));
    }
    if !props.is_empty() {
        d.insert("p".into(), Value::Object(props));
    }
    if e.has(BUILD_DISABLED) {
        d.insert("d".into(), json!(1));
    }
    d
}

fn anchor(e: &Element) -> Value {
    let mut d = Map::new();
    let tid = e.value("data-tid").unwrap_or("");
    if let Some(v) = e.value("data-if").or_else(|| e.value("data-else-if")) {
        let (p, n) = neg(v);
        d.insert("t".into(), json!(if e.has("data-if") { 1 } else { 4 }));
        d.insert("s".into(), json!(tid));
        d.insert("p".into(), p);
        d.insert("n".into(), json!(n));
    } else if let Some(v) = e.value("data-for") {
        let (var, p) = directive::parse_for(v).unwrap_or(("", ""));
        d.insert("t".into(), json!(2));
        d.insert("s".into(), json!(tid));
        d.insert("v".into(), json!(var));
        d.insert("p".into(), path(p));
        d.insert("k".into(), e.value("data-key").map_or(json!([]), path));
    } else {
        d.insert("t".into(), json!(3));
        d.insert("s".into(), json!(tid));
    }
    if let Some(u) = e.value("data-slot").and_then(|s| s.parse::<u64>().ok()) {
        d.insert("u".into(), json!(u));
    }
    Value::Object(d)
}

/// What the entry module of a reactive route holds (§3.1), independent of how imports are named
/// (dev URL or bundler path).
pub struct Entry {
    /// (import clause, module), in the order of §3.1.
    pub imports: Vec<(String, PathBuf)>,
    pub body: String,
}

pub struct EntryInput<'a> {
    pub framework: &'a Path,
    pub pattern: &'a str,
    pub loader: bool,
    pub ssr: bool,
    pub layout: bool,
    pub overlays: bool,
    pub uses: Uses,
    pub page: Option<&'a PathBuf>,
    pub comps: Vec<(String, PathBuf)>,
    /// (name, file, exports `persist`).
    pub stores: Vec<(String, PathBuf, bool)>,
    pub t: Vec<(String, String)>,
    pub d: Vec<(String, String)>,
    pub b: &'a [(String, Value)],
}

fn key(k: &str) -> String {
    let bare = k.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$') && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if bare { k.to_string() } else { Value::String(k.to_string()).to_string() }
}

fn object(entries: impl IntoIterator<Item = (String, String)>) -> String {
    let fields: Vec<String> = entries.into_iter().map(|(k, v)| format!("    {}: {v},\n", key(&k))).collect();
    if fields.is_empty() { "{}".into() } else { format!("{{\n{}  }}", fields.concat()) }
}

impl Entry {
    pub fn new(i: EntryInput) -> Entry {
        let mut imports = vec![("{ start }".to_string(), i.framework.join(KERNEL))];
        let modules = i.uses.modules();
        for (name, file) in &modules {
            imports.push((name.to_string(), i.framework.join(file)));
        }
        if let Some(p) = i.page {
            imports.push(("page".into(), p.clone()));
        }
        for (k, (_, m)) in i.comps.iter().enumerate() {
            imports.push((format!("c{k}"), m.clone()));
        }
        for (k, (_, m, persist)) in i.stores.iter().enumerate() {
            imports.push((if *persist { format!("s{k}, {{ persist as s{k}p }}") } else { format!("s{k}") }, m.clone()));
        }
        let mut r = String::from("const R = {\n");
        r.push_str(&format!("  pattern: {},\n", Value::String(i.pattern.to_string())));
        for (on, k) in [(i.loader, "loader"), (i.ssr, "ssr"), (i.layout, "layout"), (i.overlays, "overlays")] {
            if on {
                r.push_str(&format!("  {k}: 1,\n"));
            }
        }
        r.push_str(&format!("  use: [{}],\n", modules.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")));
        if i.page.is_some() {
            r.push_str("  page,\n");
        }
        r.push_str(&format!("  comps: {},\n", object(i.comps.iter().enumerate().map(|(k, (id, _))| (id.clone(), format!("c{k}"))))));
        let stores = i.stores.iter().enumerate().map(|(k, (name, _, persist))| (name.clone(), if *persist { format!("[s{k}, s{k}p]") } else { format!("[s{k}]") }));
        r.push_str(&format!("  stores: {},\n", object(stores)));
        let strings = |list: &[(String, String)]| object(list.iter().map(|(id, html)| (id.clone(), Value::String(html.clone()).to_string())));
        r.push_str(&format!("  t: {},\n", strings(&i.t)));
        if !i.d.is_empty() {
            r.push_str(&format!("  d: {},\n", strings(&i.d)));
        }
        r.push_str(&format!("  b: {},\n", object(i.b.iter().map(|(id, d)| (id.clone(), d.to_string())))));
        r.push_str("}\nexport default R\nstart(R)\n");
        Entry { imports, body: r }
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.imports.iter().map(|(_, m)| m.clone()).collect()
    }

    pub fn source(&self, spec: impl Fn(&Path) -> String) -> String {
        let mut out = String::new();
        for (clause, m) in &self.imports {
            out.push_str(&format!("import {clause} from {}\n", Value::String(spec(m))));
        }
        out.push('\n');
        out.push_str(&self.body);
        out
    }
}
