use crate::a11y::{collect_labels, has};
use crate::html::{Element, Node, Pos};
use crate::directive::is_component_tag;
use pragma_diag::Diagnostic;
use std::collections::HashSet;
use std::path::PathBuf;

const TOOL_ATTRS: &[&str] = &["toolname", "tooldescription", "toolparamdescription", "toolautosubmit", "notool"];
const FIELDS: &[&str] = &["input", "select", "textarea"];
const NOT_PARAMS: &[&str] = &["submit", "reset", "button", "image"];
const UNSAFE_PREFIXES: &[&str] = &["delete_", "remove_", "destroy_", "pay_", "buy_", "purchase_", "cancel_", "send_", "transfer_"];

pub fn is_tool_attr(name: &str) -> bool {
    TOOL_ATTRS.contains(&name)
}

pub fn disabled(files: &[PathBuf], trees: &[Vec<Node>]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for tree in trees {
        disabled_nodes(files, tree, &mut out);
    }
    out
}

fn disabled_nodes(files: &[PathBuf], nodes: &[Node], out: &mut Vec<Diagnostic>) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if !is_component_tag(&e.name) {
            for a in &e.attrs {
                let lname = a.name.to_ascii_lowercase();
                let bound = lname.strip_prefix("data-attr-");
                let Some(attr) = TOOL_ATTRS.iter().copied().find(|t| bound.unwrap_or(&lname) == *t) else { continue };
                // a component passing on its caller's prop (`:toolparamdescription="$toolparamdescription"`)
                if bound.is_some() && a.value.as_deref().is_some_and(|v| v.starts_with("$.")) {
                    continue;
                }
                let msg = format!("{attr} needs WebMCP, which is off: set \"webmcp\": true in pragma.json, or remove {attr}");
                out.push(Diagnostic::error(files[a.pos.file as usize].clone(), a.pos.line, a.pos.col, "webmcp/disabled", msg));
            }
        }
        disabled_nodes(files, &e.children, out);
    }
}

fn written(attr: &str) -> String {
    if matches!(attr, "toolautosubmit" | "notool") { attr.to_string() } else { format!("{attr}=\"…\"") }
}

fn snake_case(v: &str) -> bool {
    v.starts_with(|c: char| c.is_ascii_lowercase())
        && v.contains('_')
        && !v.ends_with('_')
        && !v.contains("__")
        && v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn to_snake_case(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 4);
    let mut after_lower = false;
    for c in v.chars() {
        if c.is_ascii_uppercase() && after_lower {
            out.push('_');
        }
        after_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        out.push(if matches!(c, '-' | '.') { '_' } else { c.to_ascii_lowercase() });
    }
    out
}

pub fn strip_notool(nodes: &mut [Node]) {
    for n in nodes {
        if let Node::Element(e) = n {
            e.remove("notool");
            strip_notool(&mut e.children);
        }
    }
}

struct Tools<'a> {
    files: &'a [PathBuf],
    out: Vec<Diagnostic>,
    label_for: HashSet<String>,
    names: HashSet<String>,
}

impl Tools<'_> {
    fn error(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.out.push(Diagnostic::error(self.files[pos.file as usize].clone(), pos.line, pos.col, rule, msg));
    }

    fn warn(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.out.push(Diagnostic::warning(self.files[pos.file as usize].clone(), pos.line, pos.col, rule, msg));
    }

    fn nodes(&mut self, nodes: &[Node], tool: bool, in_label: bool) {
        let mut before: Option<HashSet<String>> = None;
        let mut branches: HashSet<String> = HashSet::new();
        for n in nodes {
            let Node::Element(e) = n else { continue };
            let tpl = e.is("template");
            if tpl && e.has("data-def") {
                continue;
            }
            let other_branch = tpl && (e.has("data-else-if") || e.has("data-else") || e.has("data-empty"));
            if other_branch && let Some(names) = &before {
                branches.extend(std::mem::replace(&mut self.names, names.clone()));
            } else {
                self.names.extend(branches.drain());
                before = None;
            }
            if tpl && (e.has("data-if") || e.has("data-for")) {
                before = Some(self.names.clone());
            }
            self.element(e, tool, in_label);
        }
        self.names.extend(branches);
    }

    fn element(&mut self, e: &Element, tool: bool, in_label: bool) {
        let name = e.name.to_ascii_lowercase();
        let form = name == "form";
        let field = FIELDS.contains(&name.as_str());
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            let bound = lname.strip_prefix("data-attr-");
            let attr = bound.unwrap_or(&lname);
            // still bound after expansion: the value is not known at build time
            if bound.is_some() && is_tool_attr(attr) {
                let msg = format!("{attr} must be static: write {} with fixed text, no : binding or {{}}; a component can pass it on with :{attr}=\"$prop\" when its caller gives the prop fixed text", written(attr));
                self.error(a.pos, "webmcp/dynamic", msg);
            }
            match attr {
                "toolname" | "tooldescription" | "toolautosubmit" if !form => {
                    self.error(a.pos, "webmcp/attribute", format!("{attr} only goes on <form>: move it from <{name}> to the form"))
                }
                "toolparamdescription" if !field => {
                    self.error(a.pos, "webmcp/attribute", format!("toolparamdescription only goes on <input>, <select> or <textarea>: move it from <{name}> to the field"))
                }
                "notool" if !form => self.error(a.pos, "webmcp/notool", format!("notool only goes on <form>: remove it from <{name}>")),
                _ => {}
            }
        }
        let tool = if form { self.form(e) } else { tool };
        if field && tool {
            self.field(e, &name, in_label);
        }
        self.nodes(&e.children, tool, in_label || name == "label");
    }

    fn form(&mut self, e: &Element) -> bool {
        let named = has(e, "toolname");
        let described = has(e, "tooldescription");
        if has(e, "notool") {
            if named || described || has(e, "toolautosubmit") {
                self.error(e.pos, "webmcp/notool", "<form> has both notool and tool attributes: remove notool to keep the tool, or remove the tool attributes to opt out");
            }
            return false;
        }
        match (named, described) {
            (false, false) => {
                self.warn(e.pos, "webmcp/form-tool", "<form> is not a WebMCP tool: add toolname=\"verb_object\" and tooldescription=\"…\", or notool to opt out");
                return false;
            }
            (true, false) => self.error(e.pos, "webmcp/pair", "<form> has toolname but no tooldescription: add tooldescription=\"…\" (the browser needs both)"),
            (false, true) => self.error(e.pos, "webmcp/pair", "<form> has tooldescription but no toolname: add toolname=\"verb_object\" (the browser needs both)"),
            (true, true) => {}
        }
        if let Some(a) = e.attr("toolname") {
            self.tool_name(a.pos, a.value.as_deref().unwrap_or(""), has(e, "toolautosubmit"));
        }
        if let Some(a) = e.attr("tooldescription")
            && a.value.as_deref().unwrap_or("").split_whitespace().count() < 4
        {
            self.warn(a.pos, "webmcp/description", "tooldescription under 4 words: say what the tool does, e.g. \"Search the catalog by name or category\"");
        }
        true
    }

    fn tool_name(&mut self, pos: Pos, v: &str, autosubmit: bool) {
        let valid = v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')) && (1..=128).contains(&v.len());
        if !valid {
            self.error(pos, "webmcp/name", format!("toolname \"{v}\" is invalid: use 1 to 128 characters of A-Z, a-z, 0-9, _, . or -"));
            return;
        }
        if !self.names.insert(v.to_string()) {
            self.error(pos, "webmcp/duplicate", format!("toolname \"{v}\" is already used on this page: give each tool a unique name"));
        }
        if !snake_case(v) {
            let fixed = to_snake_case(v);
            let hint = if snake_case(&fixed) { format!("toolname=\"{fixed}\"") } else { "a verb_object name such as toolname=\"search_products\"".to_string() };
            self.warn(pos, "webmcp/name-style", format!("toolname \"{v}\" is not lowercase snake_case verb_object: use {hint}"));
        }
        if autosubmit && UNSAFE_PREFIXES.iter().any(|p| v.starts_with(*p)) {
            self.warn(pos, "webmcp/autosubmit", format!("toolautosubmit on \"{v}\" lets an agent run it without a user click: remove toolautosubmit from forms that delete, pay or send"));
        }
    }

    fn field(&mut self, e: &Element, name: &str, in_label: bool) {
        let kind = e.value("type").unwrap_or("").to_ascii_lowercase();
        if NOT_PARAMS.contains(&kind.as_str()) {
            return;
        }
        if !has(e, "name") {
            self.error(e.pos, "webmcp/param-name", format!("<{name}> in a tool form without name: add name=\"…\" (it is the tool parameter)"));
        }
        let labelled = in_label || e.value("id").is_some_and(|id| self.label_for.contains(id)) || e.has("data-attr-id");
        if kind != "hidden" && !labelled && !has(e, "toolparamdescription") && !has(e, "aria-description") {
            let param = e.value("name").map_or(String::new(), |n| format!(" name=\"{n}\""));
            self.warn(e.pos, "webmcp/param-description", format!("<{name}{param}> has no description for agents: add toolparamdescription=\"…\", a <label> or aria-description"));
        }
    }
}

pub fn check(files: &[PathBuf], body: &[Node]) -> Vec<Diagnostic> {
    let mut label_for = HashSet::new();
    collect_labels(body, &mut label_for);
    let mut t = Tools { files, out: Vec::new(), label_for, names: HashSet::new() };
    t.nodes(body, false, false);
    t.out
}
