//! Output: final HTML and compiled ops for SSR (SPEC §3 "Compiled format").
use crate::directive::{self, Dir};
use crate::html::{Attr, Element, Node, VOID, open_tag, push_attr};
use serde_json::{Value, json};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Levels of a component that renders itself that the server renders; deeper ones render on the client.
pub const SSR_SELF_DEPTH: usize = 12;
/// Self calls the server renders per page at most (a component that calls itself twice doubles per level).
pub const SSR_SELF_CALLS: usize = 64;

pub fn write_nodes(nodes: &[Node], out: &mut String, compact: bool) {
    for n in nodes {
        match n {
            Node::Text(t, _) => {
                if compact && t.trim().is_empty() {
                    out.push(if t.contains('\n') { '\n' } else { ' ' });
                } else {
                    out.push_str(t);
                }
            }
            Node::Comment => {}
            Node::Element(e) => write_element(e, out, compact),
        }
    }
}

/// `#interp` → `<!--{path}-->` (`<!--n{path}-->` for slot content n components up); `<!--}-->` closes it.
fn interp_open(e: &Element) -> String {
    format!("<!--{}{}-->", e.value("data-slot").unwrap_or(""), e.value("data-text").unwrap_or(""))
}

const INTERP_CLOSE: &str = "<!--}-->";

/// A value inside an anchor comment: bytes outside `[A-Za-z0-9_$.!]` are percent-encoded, so no
/// value can end the comment (`-->`, `--!>`) or split its fields.
fn anchor_value(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$' | b'.' | b'!') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Block anchor (SPEC §3 "Rendered block DOM"): `<!--p:<kind>[slot] <tid>[ <value>…]-->` with kind
/// `if`/`elif` (`[!]path`), `else`, `for` (`var path key`) or `empty`.
pub fn anchor(e: &Element) -> Option<String> {
    let (kind, values): (&str, Vec<&str>) = if let Some(v) = e.value("data-if") {
        ("if", vec![v])
    } else if let Some(v) = e.value("data-else-if") {
        ("elif", vec![v])
    } else if e.has("data-else") {
        ("else", Vec::new())
    } else if let Some(v) = e.value("data-for") {
        let mut it = v.split_whitespace();
        let var = it.next().unwrap_or("");
        ("for", vec![var, it.nth(1).unwrap_or(""), e.value("data-key").unwrap_or("")])
    } else if e.has("data-empty") {
        ("empty", Vec::new())
    } else {
        return None;
    };
    let mut out = format!("<!--p:{kind}{} {}", e.value("data-slot").unwrap_or(""), anchor_value(e.value("data-tid").unwrap_or("")));
    for v in values {
        out.push(' ');
        out.push_str(&anchor_value(v));
    }
    out.push_str("-->");
    Some(out)
}

pub fn write_element(e: &Element, out: &mut String, compact: bool) {
    if e.name == directive::INTERP {
        out.push_str(&interp_open(e));
        out.push_str(INTERP_CLOSE);
        return;
    }
    let lname = e.name.to_ascii_lowercase();
    if lname == "template"
        && let Some(a) = anchor(e)
    {
        out.push_str(&a);
        return;
    }
    open_tag(out, e);
    if VOID.contains(&lname.as_str()) {
        return;
    }
    if lname == "template" {
        for c in e.element_children() {
            write_element(c, out, compact);
        }
    } else {
        let raw = matches!(lname.as_str(), "pre" | "textarea" | "script" | "style");
        write_nodes(&e.children, out, compact && !raw);
    }
    out.push_str("</");
    out.push_str(&e.name);
    out.push('>');
}

/// Moves every block body out of the page: the block stays as an empty anchor whose `data-tid` is a
/// short id (`<prefix><base36>`, equal bodies share one): `l` in the layout, `t` inside `<main
/// data-page>`. Returns one `<template data-tpl="<tid>">` per body and the `<template data-def>` of
/// self calls, which leave the page for its template module.
pub fn hoist(body: &mut Vec<Node>) -> Vec<Element> {
    hoist_with(body, 'l')
}

fn hoist_with(nodes: &mut Vec<Node>, prefix: char) -> Vec<Element> {
    let mut h = Hoisted { prefix, ids: HashMap::new(), out: Vec::new() };
    hoist_nodes(nodes, &mut h, false);
    let mut out = h.out;
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            Node::Element(e) if e.is("template") && e.has("data-def") => {
                let Node::Element(def) = nodes.remove(i) else { unreachable!() };
                out.push(def);
            }
            _ => i += 1,
        }
    }
    out
}

struct Hoisted {
    prefix: char,
    ids: HashMap<String, String>,
    out: Vec<Element>,
}

fn base36(mut n: usize) -> String {
    let mut digits = Vec::new();
    loop {
        digits.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[n % 36]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    digits.iter().rev().map(|d| *d as char).collect()
}

impl Hoisted {
    fn id(&mut self, key: String) -> (String, bool) {
        if let Some(id) = self.ids.get(&key) {
            return (id.clone(), false);
        }
        let id = format!("{}{}", self.prefix, base36(self.ids.len()));
        self.ids.insert(key, id.clone());
        (id, true)
    }
}

/// `foreign`: inside `<svg>`/`<math>` (outside `<foreignObject>`), where the runtime parses a body
/// in that namespace; its bodies never share an id with HTML ones.
fn hoist_nodes(nodes: &mut [Node], h: &mut Hoisted, foreign: bool) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.is("main") && e.has("data-page") {
            let page = hoist_with(&mut e.children, 't');
            h.out.extend(page);
            continue;
        }
        let foreign = !e.is("foreignObject") && (foreign || e.is("svg") || e.is("math"));
        hoist_nodes(&mut e.children, h, foreign);
        if !directive::is_block(e) {
            continue;
        }
        let mut key = String::from(if foreign { "f" } else { "h" });
        for c in e.element_children() {
            write_element(c, &mut key, true);
        }
        let (id, new) = h.id(key);
        let children = std::mem::take(&mut e.children);
        e.remove("data-tid");
        e.attrs.push(Attr { name: "data-tid".into(), value: Some(id.clone()), pos: e.pos });
        if new {
            h.out.push(Element { name: "template".into(), attrs: vec![Attr { name: "data-tpl".into(), value: Some(id), pos: e.pos }], children, pos: e.pos });
        }
    }
}

fn js_string(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('\'');
    for c in v.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

fn js_object(entries: &[(String, String)]) -> String {
    let fields: Vec<String> = entries
        .iter()
        .map(|(k, v)| {
            let bare = k.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$') && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
            format!("{}:{}", if bare { k.clone() } else { js_string(k) }, js_string(v))
        })
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// The page's template module: `export default { <tid>: '<body>' }` and, for self calls,
/// `export const d = { '<data-c id>': '<root>' }`.
pub fn template_module(bodies: &[Element], compact: bool) -> String {
    let (mut tpls, mut defs) = (Vec::new(), Vec::new());
    for t in bodies {
        let mut html = String::new();
        for c in t.element_children() {
            write_element(c, &mut html, compact);
        }
        if let Some(id) = t.value("data-tpl") {
            tpls.push((id.to_string(), html));
        } else if let Some(id) = t.value("data-def") {
            defs.push((id.to_string(), html));
        }
    }
    let mut out = format!("export default {}\n", js_object(&tpls));
    if !defs.is_empty() {
        out.push_str(&format!("export const d = {}\n", js_object(&defs)));
    }
    out
}

pub fn page_content(nodes: &[Node]) -> &[Node] {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.is("main") && e.has("data-page") {
            return &e.children;
        }
        let inner = page_content(&e.children);
        if !inner.is_empty() {
            return inner;
        }
    }
    &[]
}

fn server_path(p: &str, vars: &[String]) -> bool {
    let r = directive::root(p);
    directive::SSR_ROOTS.contains(&r) || vars.iter().any(|v| v == r)
}

struct Ctx<'s> {
    scripted: &'s BTreeSet<String>,
    defs: BTreeMap<String, Element>,
    tpls: HashMap<String, Element>,
    depth: Cell<usize>,
    calls: Cell<usize>,
}

impl Ctx<'_> {
    fn self_call(&self, e: &Element) -> Option<Element> {
        if !e.has("data-self") || !e.children.is_empty() || self.depth.get() >= SSR_SELF_DEPTH || self.calls.get() >= SSR_SELF_CALLS {
            return None;
        }
        let def = self.defs.get(e.value("data-c")?)?;
        self.calls.set(self.calls.get() + 1);
        Some(Element { children: def.children.clone(), ..e.clone() })
    }
}

struct Ops<'s> {
    ops: Vec<Value>,
    /// Ops up to this index cannot receive concatenated html.
    sealed: usize,
    ctx: &'s Ctx<'s>,
}

impl<'s> Ops<'s> {
    fn new(ctx: &'s Ctx<'s>) -> Self {
        Ops { ops: Vec::new(), sealed: 0, ctx }
    }

    fn html(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.ops.len() > self.sealed
            && let Some(Value::Array(last)) = self.ops.last_mut()
                && last[0] == "html" {
                    let cur = last[1].as_str().unwrap_or("").to_string();
                    last[1] = Value::String(cur + s);
                    return;
                }
        self.ops.push(json!(["html", s]));
    }

    fn op(&mut self, v: Value) {
        self.ops.push(v);
    }
}

/// SSR ops of the page content. `scripted`: `data-c` ids of components with a script, whose root
/// becomes a `comp` op (the server renders it in its own instance scope when it can run it).
/// `bodies`: what `hoist` moved out (block bodies by tid, self-call bodies by `data-c` id).
pub fn ops(nodes: &[Node], scripted: &BTreeSet<String>, bodies: &[Element]) -> Value {
    let root = |t: &Element, attr: &str| Some((t.value(attr)?.to_string(), t.element_children().next()?.clone()));
    let defs = bodies.iter().filter_map(|t| root(t, "data-def")).collect();
    let tpls = bodies.iter().filter_map(|t| root(t, "data-tpl")).collect();
    let ctx = Ctx { scripted, defs, tpls, depth: Cell::new(0), calls: Cell::new(0) };
    let mut o = Ops::new(&ctx);
    let mut vars = Vec::new();
    nodes_ops(nodes, &mut o, &mut vars, false);
    Value::Array(o.ops)
}

fn block(e: &Element, vars: &mut Vec<String>, ctx: &Ctx) -> Value {
    let mut o = Ops::new(ctx);
    if let Some(root) = e.value("data-tid").and_then(|id| ctx.tpls.get(id)) {
        element_ops(root, &mut o, vars, true);
    }
    Value::Array(o.ops)
}

fn slot_of(e: &Element) -> Option<u32> {
    e.value("data-slot").and_then(|v| v.parse().ok())
}

fn push_slotted(o: &mut Ops, slot: Option<u32>, op: Value) {
    match slot {
        Some(n) => o.op(json!(["slot", n, [op]])),
        None => o.op(op),
    }
}

fn nodes_ops(nodes: &[Node], o: &mut Ops, vars: &mut Vec<String>, compact: bool) {
    // tid of the if/for that opens the chain; None when the server does not decide the chain (left to the client).
    let mut chain: Option<String> = None;
    for n in nodes {
        match n {
            Node::Text(t, _) => {
                if compact && t.trim().is_empty() {
                    o.html(" ");
                } else {
                    o.html(t);
                }
            }
            Node::Comment => {}
            Node::Element(e) if directive::is_block(e) => {
                let mut s = String::new();
                write_element(e, &mut s, false);
                o.html(&s);
                let tid = e.value("data-tid").unwrap_or("").to_string();
                let (slot, ctx) = (slot_of(e), o.ctx);
                if let Some(v) = e.value("data-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = server_path(p, vars).then(|| tid.clone());
                    if chain.is_some() {
                        push_slotted(o, slot, json!(["if", tid, p, neg, block(e, vars, ctx)]));
                    }
                } else if let Some(v) = e.value("data-else-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = chain.take().filter(|_| server_path(p, vars));
                    if let Some(id) = &chain {
                        push_slotted(o, slot, json!(["elif", id, p, neg, block(e, vars, ctx)]));
                    }
                } else if let Some(v) = e.value("data-for") {
                    chain = None;
                    if let Some((var, path)) = directive::parse_for(v)
                        && server_path(path, vars)
                    {
                        let key = e.value("data-key").unwrap_or("").to_string();
                        vars.push(var.to_string());
                        let body = block(e, vars, ctx);
                        vars.pop();
                        push_slotted(o, slot, json!(["for", tid, var, path, key, body]));
                        chain = Some(tid);
                    }
                } else if let Some(id) = chain.take() {
                    let kind = if e.has("data-else") { "else" } else { "empty" };
                    push_slotted(o, slot, json!([kind, id, block(e, vars, ctx)]));
                }
            }
            Node::Element(e) => {
                chain = None;
                element_ops(e, o, vars, false);
            }
        }
    }
}

/// Slot content (`slot` op) is evaluated outside; inside it, the root of a component with a
/// script opens its instance scope (`comp` op, with its props: text or a path outside).
fn element_ops(e: &Element, o: &mut Ops, vars: &mut Vec<String>, seal: bool) {
    if let Some(call) = o.ctx.self_call(e) {
        o.ctx.depth.set(o.ctx.depth.get() + 1);
        element_ops(&call, o, vars, seal);
        o.ctx.depth.set(o.ctx.depth.get() - 1);
        return;
    }
    if e.has("data-self") && e.children.is_empty() {
        let mut s = String::new();
        write_element(e, &mut s, false);
        return o.html(&s);
    }
    let Some(n) = slot_of(e) else { return component_ops(e, o, vars, seal) };
    let mut inner = Ops::new(o.ctx);
    component_ops(e, &mut inner, vars, true);
    o.op(json!(["slot", n, inner.ops]));
}

fn component_ops(e: &Element, o: &mut Ops, vars: &mut Vec<String>, seal: bool) {
    let Some(id) = e.value("data-c").filter(|id| o.ctx.scripted.contains(*id)) else { return element_body(e, o, vars, seal) };
    let props: Vec<Value> = e
        .attrs
        .iter()
        .filter_map(|a| {
            let name = a.name.to_ascii_lowercase();
            let value = a.value.clone().unwrap_or_default();
            if let Some(k) = name.strip_prefix("data-prop-") {
                Some(json!([k, "text", value]))
            } else if let Some(k) = name.strip_prefix("data-with-")
                && let Ok(literal) = serde_json::from_str::<Value>(&value)
            {
                Some(json!([k, "text", literal]))
            } else {
                name.strip_prefix("data-with-").map(|k| json!([k, "path", value]))
            }
        })
        .collect();
    let mut inner = Ops::new(o.ctx);
    element_body(e, &mut inner, vars, true);
    o.op(json!(["comp", id, props, inner.ops]));
}

fn element_body(e: &Element, o: &mut Ops, vars: &mut Vec<String>, seal: bool) {
    if e.name == directive::INTERP {
        o.html(&interp_open(e));
        let v = e.value("data-text").unwrap_or("");
        if directive::template_paths(v).iter().all(|p| server_path(p, vars)) {
            o.op(json!(["text", v]));
        }
        o.html(INTERP_CLOSE);
        return;
    }
    o.html(&format!("<{}", e.name));
    if seal {
        o.sealed = o.ops.len();
    }
    let mut attrs: Vec<(String, String)> = Vec::new();
    let mut classes: Vec<Value> = Vec::new();
    let mut show: Option<(String, bool)> = None;
    let mut text: Option<String> = None;
    let mut css_vars: Vec<String> = Vec::new();
    for a in &e.attrs {
        let lname = a.name.to_ascii_lowercase();
        let Some(Some(d)) = directive::parse(&lname) else { continue };
        let v = a.value.as_deref().unwrap_or("");
        let (neg, p) = directive::split_neg(v);
        if !directive::values(d, p).iter().all(|p| server_path(p, vars)) {
            continue;
        }
        match d {
            Dir::Text => text = Some(p.to_string()),
            Dir::Attr(n) => attrs.push((n.to_string(), p.to_string())),
            Dir::Class(n) => classes.push(json!([n, p, neg])),
            Dir::Show => show = Some((p.to_string(), neg)),
            Dir::Var(n) => css_vars.push(format!("--{n}:{{{p}}}")),
            _ => {}
        }
    }
    if !css_vars.is_empty() {
        let fixed = e.value("style").map(|v| v.replace('{', "\\{")).into_iter();
        attrs.push(("style".into(), fixed.chain(css_vars).collect::<Vec<_>>().join(";")));
    }
    for a in &e.attrs {
        let lname = a.name.to_ascii_lowercase();
        if (lname == "class" && !classes.is_empty()) || attrs.iter().any(|(n, _)| n.eq_ignore_ascii_case(&lname)) || (lname == "hidden" && show.is_some()) {
            continue;
        }
        let mut s = String::from(" ");
        push_attr(&mut s, a);
        o.html(&s);
    }
    for (n, p) in attrs {
        o.op(json!(["attr", n, p]));
    }
    if !classes.is_empty() {
        let statics = e.value("class").unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
        o.op(json!(["class", statics, classes]));
    }
    if let Some((p, neg)) = show {
        o.op(json!(["show", p, neg]));
    }
    o.html(">");
    let lname = e.name.to_ascii_lowercase();
    if VOID.contains(&lname.as_str()) {
        return;
    }
    match text {
        Some(p) => o.op(json!(["text", p])),
        None if lname == "template" => {
            for c in e.element_children() {
                let mut s = String::new();
                write_element(c, &mut s, false);
                o.html(&s);
            }
        }
        None => nodes_ops(&e.children, o, vars, false),
    }
    o.html(&format!("</{}>", e.name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::parse;

    #[test]
    fn compiles_ops() {
        let src = r#"<ul class="a" data-class-on="data.on"><template data-for="x in data.xs" data-key="x.id" data-tid="t1"><li data-text="{x.name}"></li></template><template data-empty data-tid="t2"><li>empty</li></template></ul><p data-show="!state.ok" data-attr-title="data.t" title="y">z</p>"#;
        let (mut nodes, _) = parse(src, 0);
        let bodies = hoist_with(&mut nodes, 't');
        let v = ops(&nodes, &BTreeSet::new(), &bodies);
        let expected = json!([
            ["html", "<ul data-class-on=\"data.on\""],
            ["class", "a", [["on", "data.on", false]]],
            ["html", "><!--p:for t0 x data.xs x.id-->"],
            ["for", "t0", "x", "data.xs", "x.id", [["html", "<li"], ["html", " data-text=\"{x.name}\">"], ["text", "{x.name}"], ["html", "</li>"]]],
            ["html", "<!--p:empty t1-->"],
            ["empty", "t0", [["html", "<li"], ["html", ">empty</li>"]]],
            ["html", "</ul><p data-show=\"!state.ok\" data-attr-title=\"data.t\""],
            ["attr", "title", "data.t"],
            ["show", "state.ok", true],
            ["html", ">z</p>"]
        ]);
        assert_eq!(v, expected);
    }

    #[test]
    fn templates_and_markers() {
        let mut p = Element { name: "p".into(), attrs: Vec::new(), children: Vec::new(), pos: Default::default() };
        let interp = |v: &str| Element { name: directive::INTERP.into(), attrs: vec![crate::html::Attr { name: "data-text".into(), value: Some(v.into()), pos: Default::default() }], children: Vec::new(), pos: Default::default() };
        p.children = vec![Node::Text("Hi ".into(), Default::default()), Node::Element(interp("{data.name}")), Node::Text(", ".into(), Default::default()), Node::Element(interp("{state.n}"))];
        p.attrs.push(crate::html::Attr { name: "data-attr-href".into(), value: Some("/p/{data.id}".into()), pos: Default::default() });
        p.attrs.push(crate::html::Attr { name: "data-attr-title".into(), value: Some("{state.t}".into()), pos: Default::default() });
        let mut html = String::new();
        write_element(&p, &mut html, false);
        assert_eq!(html, "<p data-attr-href=\"/p/{data.id}\" data-attr-title=\"{state.t}\">Hi <!--{data.name}--><!--}-->, <!--{state.n}--><!--}--></p>");
        let v = ops(&[Node::Element(p)], &BTreeSet::new(), &[]);
        let expected = json!([
            ["html", "<p data-attr-href=\"/p/{data.id}\" data-attr-title=\"{state.t}\""],
            ["attr", "href", "/p/{data.id}"],
            ["attr", "title", "{state.t}"],
            ["html", ">Hi <!--{data.name}-->"],
            ["text", "{data.name}"],
            ["html", "<!--}-->, <!--{state.n}-->"],
            ["text", "{state.n}"],
            ["html", "<!--}--></p>"]
        ]);
        assert_eq!(v, expected);
    }

    #[test]
    fn css_variables_render_on_the_server() {
        let (nodes, _) = parse(r#"<i class="bar" style="--y:1" data-var-x="data.x" data-var-z="shared.ui.z"></i>"#, 0);
        let expected = json!([
            ["html", "<i class=\"bar\" data-var-x=\"data.x\" data-var-z=\"shared.ui.z\""],
            ["attr", "style", "--y:1;--x:{data.x}"],
            ["html", "></i>"]
        ]);
        assert_eq!(ops(&nodes, &BTreeSet::new(), &[]), expected);
    }

    #[test]
    fn self_calls_render_on_the_server_up_to_a_depth() {
        let src = r#"<ul data-c="tree"><template data-for="c in state.node.kids" data-key="c.id" data-tid="t1"><ul data-c="tree" data-self="c-tree" data-with-node="c"></ul></template></ul><template data-def="tree"><ul data-c="tree"><template data-for="c in state.node.kids" data-key="c.id" data-tid="t1"><ul data-c="tree" data-self="c-tree" data-with-node="c"></ul></template></ul></template>"#;
        let (mut nodes, _) = parse(src, 0);
        let bodies = hoist_with(&mut nodes, 't');
        let v = ops(&nodes, &BTreeSet::from(["tree".to_string()]), &bodies).to_string();
        assert_eq!(v.matches(r#"["comp","tree""#).count(), 1 + SSR_SELF_DEPTH, "{v}");
        assert_eq!(v.matches(r#"["for","t0""#).count(), 1 + SSR_SELF_DEPTH, "{v}");
        assert!(!v.contains("data-def"), "the def body leaves the page: {v}");
    }

    #[test]
    fn script_component_root_and_slot_content() {
        let src = r#"<section data-c="box" data-prop-step="2" data-with-items="data.items" data-text="{state.n}"></section><div data-c="plain"><p data-slot="1" data-text="{state.title}"></p></div><div data-c="box"><b data-slot="1" data-text="{state.open}"></b><template data-slot="1" data-if="state.open" data-tid="t9"><i>on</i></template></div>"#;
        let (mut nodes, _) = parse(src, 0);
        let bodies = hoist_with(&mut nodes, 't');
        let v = ops(&nodes, &BTreeSet::from(["box".to_string()]), &bodies);
        let expected = json!([
            ["comp", "box", [["step", "text", "2"], ["items", "path", "data.items"]], [
                ["html", "<section"],
                ["html", " data-c=\"box\" data-prop-step=\"2\" data-with-items=\"data.items\" data-text=\"{state.n}\">"],
                ["text", "{state.n}"],
                ["html", "</section>"]
            ]],
            ["html", "<div data-c=\"plain\">"],
            ["slot", 1, [["html", "<p"], ["html", " data-slot=\"1\" data-text=\"{state.title}\">"], ["text", "{state.title}"], ["html", "</p>"]]],
            ["html", "</div>"],
            ["comp", "box", [], [
                ["html", "<div"],
                ["html", " data-c=\"box\">"],
                ["slot", 1, [["html", "<b"], ["html", " data-slot=\"1\" data-text=\"{state.open}\">"], ["text", "{state.open}"], ["html", "</b>"]]],
                ["html", "<!--p:if1 t0 state.open-->"],
                ["slot", 1, [["if", "t0", "state.open", false, [["html", "<i"], ["html", ">on</i>"]]]]],
                ["html", "</div>"]
            ]]
        ]);
        assert_eq!(v, expected);
    }

    #[test]
    fn literal_props_are_typed_values() {
        let (nodes, _) = parse(r#"<p data-c="box" data-with-open="false" data-with-n="-2.5" data-with-label='"a b"' data-with-items="data.items"></p>"#, 0);
        let v = ops(&nodes, &BTreeSet::from(["box".to_string()]), &[]);
        assert_eq!(v[0][2], json!([["open", "text", false], ["n", "text", -2.5], ["label", "text", "a b"], ["items", "path", "data.items"]]));
    }

    #[test]
    fn hoists_block_bodies_once() {
        let src = r#"<main data-page><ul><template data-for="x in data.xs" data-key="x.id" data-tid="a"><li><template data-if="x.on" data-tid="b"><b>on</b></template></li></template></ul><template data-if="data.on" data-tid="c"><b>on</b></template></main><svg><template data-if="data.on" data-tid="d"><b>on</b></template></svg><template data-if="data.on" data-tid="e"><b>on</b></template><template data-def="x"><i data-c="x"></i></template>"#;
        let (mut nodes, _) = parse(src, 0);
        let bodies = hoist(&mut nodes);
        let mut out = String::new();
        write_nodes(&nodes, &mut out, true);
        assert_eq!(out, "<main data-page><ul><!--p:for t1 x data.xs x.id--></ul><!--p:if t0 data.on--></main><svg><!--p:if l0 data.on--></svg><!--p:if l1 data.on-->");
        let ids: Vec<&str> = bodies.iter().filter_map(|t| t.value("data-tpl").or(t.value("data-def"))).collect();
        assert_eq!(ids, ["t0", "t1", "l0", "l1", "x"], "equal bodies share one id; an <svg> body never shares an HTML one");
        let module = template_module(&bodies, true);
        assert_eq!(module, "export default {t0:'<b>on</b>',t1:'<li><!--p:if t0 x.on--></li>',l0:'<b>on</b>',l1:'<b>on</b>'}\nexport const d = {x:'<i data-c=\"x\"></i>'}\n");
        assert_eq!(js_object(&[("md-node".into(), "it's \\ a\nb".into())]), r"{'md-node':'it\'s \\ a\nb'}");
        let v = ops(&nodes, &BTreeSet::new(), &bodies);
        assert!(v.to_string().contains(r#"["if","t0","x.on",false,[["html","<b"],["html",">on</b>"]]]"#), "the server renders hoisted bodies: {v}");
    }

    #[test]
    fn anchor_values_cannot_end_the_comment() {
        let (nodes, _) = parse(r#"<template data-if="!a-->b" data-slot="2" data-tid="t1"></template>"#, 0);
        let mut out = String::new();
        write_nodes(&nodes, &mut out, true);
        assert_eq!(out, "<!--p:if2 t1 !a%2D%2D%3Eb-->");
    }
}
