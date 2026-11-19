//! Output: final HTML and compiled ops for SSR (SPEC §3 "Compiled format").
use crate::directive::{self, Dir};
use crate::html::{Element, Node, VOID, open_tag, push_attr};
use crate::validate::BLOCKS;
use serde_json::{Value, json};

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

pub fn write_element(e: &Element, out: &mut String, compact: bool) {
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

/// Roots the server knows; the others (state, shared) are left to the client.
fn server_path(p: &str, vars: &[String]) -> bool {
    let r = directive::root(p);
    matches!(r, "data" | "params" | "query" | "page") || vars.iter().any(|v| v == r)
}

struct Ops {
    ops: Vec<Value>,
    /// Ops up to this index cannot receive concatenated html.
    sealed: usize,
}

impl Ops {
    fn new() -> Self {
        Ops { ops: Vec::new(), sealed: 0 }
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

pub fn ops(nodes: &[Node]) -> Value {
    let mut o = Ops::new();
    let mut vars = Vec::new();
    nodes_ops(nodes, &mut o, &mut vars, false);
    Value::Array(o.ops)
}

fn block(e: &Element, vars: &mut Vec<String>) -> Value {
    let mut o = Ops::new();
    if let Some(root) = e.element_children().next() {
        element_ops(root, &mut o, vars, true);
    }
    Value::Array(o.ops)
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
                if let Some(v) = e.value("data-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = server_path(p, vars).then(|| tid.clone());
                    if chain.is_some() {
                        o.op(json!(["if", tid, p, neg, block(e, vars)]));
                    }
                } else if let Some(v) = e.value("data-else-if") {
                    let (neg, p) = directive::split_neg(v);
                    chain = chain.take().filter(|_| server_path(p, vars));
                    if let Some(id) = &chain {
                        o.op(json!(["elif", id, p, neg, block(e, vars)]));
                    }
                } else if let Some(v) = e.value("data-for") {
                    chain = None;
                    if let Some((var, path)) = directive::parse_for(v)
                        && server_path(path, vars)
                    {
                        let key = e.value("data-key").unwrap_or("").to_string();
                        vars.push(var.to_string());
                        let body = block(e, vars);
                        vars.pop();
                        o.op(json!(["for", tid, var, path, key, body]));
                        chain = Some(tid);
                    }
                } else if let Some(id) = chain.take() {
                    let kind = if e.has("data-else") { "else" } else { "empty" };
                    o.op(json!([kind, id, block(e, vars)]));
                }
            }
            Node::Element(e) => {
                chain = None;
                element_ops(e, o, vars, false);
            }
        }
    }
}

fn element_ops(e: &Element, o: &mut Ops, vars: &mut Vec<String>, seal: bool) {
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
        if !server_path(p, vars) {
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
        let src = r#"<ul class="a" data-class-on="data.on"><template data-for="x in data.xs" data-key="x.id" data-tid="t1"><li data-text="x.name">-</li></template><template data-empty data-tid="t2"><li>empty</li></template></ul><p data-show="!state.ok" data-attr-title="data.t" title="y">z</p>"#;
        let (nodes, _) = parse(src, 0);
        let v = ops(&nodes);
        let expected = json!([
            ["html", "<ul data-class-on=\"data.on\""],
            ["class", "a", [["on", "data.on", false]]],
            ["html", "><template data-for=\"x in data.xs\" data-key=\"x.id\" data-tid=\"t1\"><li data-text=\"x.name\">-</li></template>"],
            ["for", "t1", "x", "data.xs", "x.id", [["html", "<li"], ["html", " data-text=\"x.name\">"], ["text", "x.name"], ["html", "</li>"]]],
            ["html", "<template data-empty data-tid=\"t2\"><li>empty</li></template>"],
            ["empty", "t1", [["html", "<li"], ["html", ">empty</li>"]]],
            ["html", "</ul><p data-show=\"!state.ok\" data-attr-title=\"data.t\""],
            ["attr", "title", "data.t"],
            ["html", ">z</p>"]
        ]);
        assert_eq!(v, expected);
    }
}
