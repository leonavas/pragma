//! Saída: HTML final e ops compiladas para SSR (SPEC §3 "Formato compilado").
use crate::directive::{self, Dir};
use crate::html::{Element, Node, VOID, is_blank, open_tag, push_attr};
use serde_json::{Value, json};

/// Serializa nós; `<template>` sai só com o elemento raiz. `compact` reduz espaços entre tags.
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

/// Raízes que o servidor conhece; as outras (state, shared) ficam para o cliente.
fn server_path(p: &str, vars: &[String]) -> bool {
    let r = directive::root(p);
    matches!(r, "data" | "params" | "query" | "page") || vars.iter().any(|v| v == r)
}

struct Ops {
    ops: Vec<Value>,
    /// Ops até este índice não podem receber html concatenado.
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

fn block(e: Option<&Element>, vars: &mut Vec<String>) -> Value {
    let mut o = Ops::new();
    if let Some(root) = e.and_then(|t| t.element_children().next()) {
        element_ops(root, &mut o, vars, true);
    }
    Value::Array(o.ops)
}

fn nodes_ops(nodes: &[Node], o: &mut Ops, vars: &mut Vec<String>, compact: bool) {
    let mut i = 0;
    while i < nodes.len() {
        match &nodes[i] {
            Node::Text(t, _) => {
                if compact && t.trim().is_empty() {
                    o.html(" ");
                } else {
                    o.html(t);
                }
            }
            Node::Comment => {}
            Node::Element(e) if e.is("template") && (e.has("data-if") || e.has("data-for")) => {
                let mut s = String::new();
                write_element(e, &mut s, false);
                o.html(&s);
                let mut j = i + 1;
                while j < nodes.len() && is_blank(&nodes[j]) {
                    j += 1;
                }
                let alt_name = if e.has("data-if") { "data-else" } else { "data-empty" };
                let alt = match nodes.get(j) {
                    Some(Node::Element(x)) if x.is("template") && x.has(alt_name) => Some(x),
                    _ => None,
                };
                if let Some(a) = alt {
                    let mut s = String::new();
                    write_element(a, &mut s, false);
                    o.html(&s);
                    i = j;
                }
                let tid = e.value("data-tid").unwrap_or("").to_string();
                if let Some(v) = e.value("data-if") {
                    let (neg, p) = directive::split_neg(v);
                    if server_path(p, vars) {
                        let yes = block(Some(e), vars);
                        let no = block(alt, vars);
                        o.op(json!(["if", tid, p, neg, yes, no]));
                    }
                } else if let Some((var, path)) = e.value("data-for").and_then(directive::parse_for)
                    && server_path(path, vars) {
                        let key = e.value("data-key").unwrap_or("").to_string();
                        vars.push(var.to_string());
                        let body = block(Some(e), vars);
                        vars.pop();
                        let empty = block(alt, vars);
                        o.op(json!(["for", tid, var, path, key, body, empty]));
                    }
            }
            Node::Element(e) => element_ops(e, o, vars, false),
        }
        i += 1;
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
        let src = r#"<ul class="a" data-class-on="data.on"><template data-for="x in data.xs" data-key="x.id" data-tid="t1"><li data-text="x.nome">-</li></template><template data-empty data-tid="t2"><li>vazio</li></template></ul><p data-show="!state.ok" data-attr-title="data.t" title="y">z</p>"#;
        let (nodes, _) = parse(src, 0);
        let v = ops(&nodes);
        let expected = json!([
            ["html", "<ul data-class-on=\"data.on\""],
            ["class", "a", [["on", "data.on", false]]],
            ["html", "><template data-for=\"x in data.xs\" data-key=\"x.id\" data-tid=\"t1\"><li data-text=\"x.nome\">-</li></template><template data-empty data-tid=\"t2\"><li>vazio</li></template>"],
            ["for", "t1", "x", "data.xs", "x.id", [["html", "<li"], ["html", " data-text=\"x.nome\">"], ["text", "x.nome"], ["html", "</li>"]], [["html", "<li"], ["html", ">vazio</li>"]]],
            ["html", "</ul><p data-show=\"!state.ok\" data-attr-title=\"data.t\""],
            ["attr", "title", "data.t"],
            ["html", ">z</p>"]
        ]);
        assert_eq!(v, expected);
    }
}
