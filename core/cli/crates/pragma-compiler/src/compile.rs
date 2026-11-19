//! Output: final HTML and compiled ops for SSR (SPEC §3 "Compiled format").
use crate::directive::{self, Dir};
use crate::html::{Element, Node, VOID, open_tag, push_attr};
use crate::validate::BLOCKS;
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Serializes nodes; `<template>` is emitted with only its root element. `compact` collapses whitespace between tags.
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

pub fn write_element(e: &Element, out: &mut String, compact: bool) {
    if e.name == directive::INTERP {
        out.push_str(&interp_open(e));
        out.push_str(INTERP_CLOSE);
        return;
    }
    open_tag(out, e);
    let lname = e.name.to_ascii_lowercase();
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

/// Roots the server can render; `shared` is left to the client. `state` gets ops too: the server
/// renders them when it runs the page or component script (`comp` op), else leaves them.
fn server_path(p: &str, vars: &[String]) -> bool {
    let r = directive::root(p);
    matches!(r, "data" | "params" | "query" | "page" | "state") || vars.iter().any(|v| v == r)
}

struct Ops<'s> {
    ops: Vec<Value>,
    /// Ops up to this index cannot receive concatenated html.
    sealed: usize,
    /// `data-c` ids of components with a script.
    scripted: &'s BTreeSet<String>,
}

impl<'s> Ops<'s> {
    fn new(scripted: &'s BTreeSet<String>) -> Self {
        Ops { ops: Vec::new(), sealed: 0, scripted }
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
pub fn ops(nodes: &[Node], scripted: &BTreeSet<String>) -> Value {
    let mut o = Ops::new(scripted);
    let mut vars = Vec::new();
    nodes_ops(nodes, &mut o, &mut vars, false);
    Value::Array(o.ops)
}

fn block(e: &Element, vars: &mut Vec<String>, scripted: &BTreeSet<String>) -> Value {
    let mut o = Ops::new(scripted);
    if let Some(root) = e.element_children().next() {
        element_ops(root, &mut o, vars, true);
    }
    Value::Array(o.ops)
}

/// `data-slot="n"`: slot content evaluated in the scope n components up (`slot` op).
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
            Node::Element(e) if e.is("template") && BLOCKS.iter().any(|b| e.has(b)) => {
                let mut s = String::new();
                write_element(e, &mut s, false);
                o.html(&s);
                let tid = e.value("data-tid").unwrap_or("").to_string();
                let (slot, scripted) = (slot_of(e), o.scripted);
                if let Some(v) = e.value("data-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = server_path(p, vars).then(|| tid.clone());
                    if chain.is_some() {
                        push_slotted(o, slot, json!(["if", tid, p, neg, block(e, vars, scripted)]));
                    }
                } else if let Some(v) = e.value("data-else-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = chain.take().filter(|_| server_path(p, vars));
                    if let Some(id) = &chain {
                        push_slotted(o, slot, json!(["elif", id, p, neg, block(e, vars, scripted)]));
                    }
                } else if let Some(v) = e.value("data-for") {
                    chain = None;
                    if let Some((var, path)) = directive::parse_for(v)
                        && server_path(path, vars)
                    {
                        let key = e.value("data-key").unwrap_or("").to_string();
                        vars.push(var.to_string());
                        let body = block(e, vars, scripted);
                        vars.pop();
                        push_slotted(o, slot, json!(["for", tid, var, path, key, body]));
                        chain = Some(tid);
                    }
                } else if let Some(id) = chain.take() {
                    let kind = if e.has("data-else") { "else" } else { "empty" };
                    push_slotted(o, slot, json!([kind, id, block(e, vars, scripted)]));
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
    let Some(n) = slot_of(e) else { return component_ops(e, o, vars, seal) };
    let mut inner = Ops::new(o.scripted);
    component_ops(e, &mut inner, vars, true);
    o.op(json!(["slot", n, inner.ops]));
}

fn component_ops(e: &Element, o: &mut Ops, vars: &mut Vec<String>, seal: bool) {
    let Some(id) = e.value("data-c").filter(|id| o.scripted.contains(*id)) else { return element_body(e, o, vars, seal) };
    let props: Vec<Value> = e
        .attrs
        .iter()
        .filter_map(|a| {
            let name = a.name.to_ascii_lowercase();
            let value = a.value.clone().unwrap_or_default();
            if let Some(k) = name.strip_prefix("data-prop-") {
                Some(json!([k, "text", value]))
            } else {
                name.strip_prefix("data-with-").map(|k| json!([k, "path", value]))
            }
        })
        .collect();
    let mut inner = Ops::new(o.scripted);
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
    for a in &e.attrs {
        let lname = a.name.to_ascii_lowercase();
        let Some(Some(d)) = directive::parse(&lname) else { continue };
        let v = a.value.as_deref().unwrap_or("");
        let (neg, p) = directive::split_neg(v);
        if !crate::validate::values(d, p).iter().all(|p| server_path(p, vars)) {
            continue;
        }
        match d {
            Dir::Text => text = Some(p.to_string()),
            Dir::Attr(n) => attrs.push((n.to_string(), p.to_string())),
            Dir::Class(n) => classes.push(json!([n, p, neg])),
            Dir::Show => show = Some((p.to_string(), neg)),
            _ => {}
        }
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
        let (nodes, _) = parse(src, 0);
        let v = ops(&nodes, &BTreeSet::new());
        let expected = json!([
            ["html", "<ul data-class-on=\"data.on\""],
            ["class", "a", [["on", "data.on", false]]],
            ["html", "><template data-for=\"x in data.xs\" data-key=\"x.id\" data-tid=\"t1\"><li data-text=\"{x.name}\"></li></template>"],
            ["for", "t1", "x", "data.xs", "x.id", [["html", "<li"], ["html", " data-text=\"{x.name}\">"], ["text", "{x.name}"], ["html", "</li>"]]],
            ["html", "<template data-empty data-tid=\"t2\"><li>empty</li></template>"],
            ["empty", "t1", [["html", "<li"], ["html", ">empty</li>"]]],
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
        let v = ops(&[Node::Element(p)], &BTreeSet::new());
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
    fn script_component_root_and_slot_content() {
        let src = r#"<section data-c="box" data-prop-step="2" data-with-items="data.items" data-text="{state.n}"></section><div data-c="plain"><p data-slot="1" data-text="{state.title}"></p></div><div data-c="box"><b data-slot="1" data-text="{state.open}"></b><template data-slot="1" data-if="state.open" data-tid="t9"><i>on</i></template></div>"#;
        let (nodes, _) = parse(src, 0);
        let v = ops(&nodes, &BTreeSet::from(["box".to_string()]));
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
                ["html", "<template data-slot=\"1\" data-if=\"state.open\" data-tid=\"t9\"><i>on</i></template>"],
                ["slot", 1, [["if", "t9", "state.open", false, [["html", "<i"], ["html", ">on</i>"]]]]],
                ["html", "</div>"]
            ]]
        ]);
        assert_eq!(v, expected);
    }
}
