//! Per-file validation (SPEC §3, §10): what does not depend on scope or expansion.
use crate::directive::{self, Dir};
use crate::html::{Element, Node, Pos, is_blank, stray_text};
use crate::project::{component_tag, is_component_name, is_component_tag};
use pragma_diag::Diagnostic;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Page,
    Layout,
    NestedLayout,
    Component,
}

pub fn diag(files: &[PathBuf], pos: Pos, rule: &'static str, msg: impl Into<String>) -> Diagnostic {
    Diagnostic::error(files[pos.file as usize].clone(), pos.line, pos.col, rule, msg)
}

pub struct ClassCheck {
    pub project: HashSet<String>,
    pub theme: pragma_css::Theme,
    cache: HashMap<String, bool>,
}

impl ClassCheck {
    pub fn new(project: HashSet<String>, theme: pragma_css::Theme) -> Self {
        ClassCheck { project, theme, cache: HashMap::new() }
    }

    pub fn known(&mut self, c: &str) -> bool {
        if self.project.contains(c) {
            return true;
        }
        if let Some(&k) = self.cache.get(c) {
            return k;
        }
        let k = pragma_css::is_utility(c, &self.theme);
        self.cache.insert(c.to_string(), k);
        k
    }

    pub fn check(&mut self, files: &[PathBuf], pos: Pos, class: &str, out: &mut Vec<Diagnostic>) {
        if self.known(class) {
            return;
        }
        if pragma_css::is_arbitrary(class) {
            out.push(diag(files, pos, "css/arbitrary-value", format!("arbitrary value \"{class}\" is not supported; use a utility from .pragma/docs/classes.md or add a class in lib/theme.css")));
        } else {
            out.push(diag(files, pos, "css/unknown-class", format!("class \"{class}\" is not a supported utility nor defined in the project CSS")));
        }
    }
}

/// Classes defined in CSS (`.name` outside comments, strings and `url()`).
pub fn css_classes(css: &str, out: &mut HashSet<String>) {
    let b = css.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = css[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 2);
            }
            q @ (b'"' | b'\'') => {
                i += 1;
                while i < b.len() && b[i] != q {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'(' if css[..i].ends_with("url") => {
                i = css[i..].find(')').map_or(b.len(), |k| i + k + 1);
            }
            b'.' => {
                let start = i + 1;
                let mut j = start;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_' || b[j] == b'\\' || b[j] >= 0x80) {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                let j = j.min(b.len());
                let first_ok = b.get(start).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b'-' || *c == b'\\');
                if j > start && first_ok {
                    out.insert(css[start..j].replace('\\', ""));
                }
                i = j.max(i + 1);
            }
            _ => i += 1,
        }
    }
}

pub struct FileCheck<'a> {
    pub files: &'a [PathBuf],
    pub kind: Kind,
    pub classes: &'a mut ClassCheck,
    pub diags: Vec<Diagnostic>,
}

pub const BLOCKS: &[&str] = &["data-if", "data-else-if", "data-else", "data-for", "data-empty"];

fn template_kind(e: &Element) -> Option<&'static str> {
    if !e.is("template") {
        return None;
    }
    BLOCKS.iter().copied().find(|b| e.has(b))
}

/// Paths read by a value directive: the `{}` paths of a template (`data-text`, `data-attr-*` with `{`), or the path itself.
pub fn values(d: Dir, v: &str) -> Vec<String> {
    match d {
        Dir::Text => directive::template_paths(v),
        Dir::Attr(_) if directive::has_braces(v) => directive::template_paths(v),
        _ => vec![v.to_string()],
    }
}

pub fn tag_of(block: &str) -> &'static str {
    match block {
        "data-if" => "<if>",
        "data-else-if" => "<else-if>",
        "data-else" => "<else>",
        "data-for" => "<for>",
        _ => "<empty>",
    }
}

impl FileCheck<'_> {
    fn err(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.diags.push(diag(self.files, pos, rule, msg));
    }

    pub fn run(&mut self, tree: &[Node], file_pos: Pos) {
        match self.kind {
            Kind::Component => {
                let roots: Vec<&Element> = tree.iter().filter_map(|n| if let Node::Element(e) = n { Some(e) } else { None }).collect();
                let text = stray_text(tree);
                if roots.len() != 1 || text.is_some() {
                    self.err(text.or(roots.get(1).map(|e| e.pos)).unwrap_or(file_pos), "html/component", "a component needs exactly 1 root element");
                }
                if let Some(r) = roots.first()
                    && (r.is("template") || r.is("slot") || is_component_tag(&r.name)) {
                        self.err(r.pos, "html/component", "the component root cannot be <template>, <slot> or another component");
                    }
            }
            Kind::Layout => self.layout(tree, file_pos),
            Kind::NestedLayout => self.nested_layout(tree, file_pos),
            Kind::Page => {}
        }
        self.nodes(tree, None, true);
    }

    fn nested_layout(&mut self, tree: &[Node], file_pos: Pos) {
        let mut slots = Vec::new();
        find_slots(tree, &mut slots);
        match slots.as_slice() {
            [] => self.err(file_pos, "html/layout", "a nested layout needs <slot></slot>"),
            [slot] => {
                if slot.has("name") {
                    self.err(slot.pos, "html/layout", "nested layout <slot> cannot have a name");
                }
            }
            [_, second, ..] => self.err(second.pos, "html/layout", "a nested layout cannot have more than one <slot></slot>"),
        }
    }

    fn layout(&mut self, tree: &[Node], file_pos: Pos) {
        let mut mains = Vec::new();
        find_mains(tree, &mut mains);
        match mains.as_slice() {
            [] => self.err(file_pos, "html/layout", "the layout needs <main data-page><slot></slot></main>"),
            [main] => {
                let kids: Vec<&Node> = main.children.iter().filter(|n| !is_blank(n)).collect();
                let ok = matches!(kids.as_slice(), [Node::Element(s)] if s.is("slot") && !s.has("name"));
                if !ok {
                    self.err(main.pos, "html/layout", "<main data-page> must contain only <slot></slot>");
                }
            }
            [_, second, ..] => self.err(second.pos, "html/layout", "the layout has more than one <main data-page>"),
        }
        let mut first = true;
        for n in tree {
            if is_blank(n) {
                continue;
            }
            if let Node::Element(e) = n
                && e.is("head") && !first {
                    self.err(e.pos, "html/layout", "the <head> block must come first in _layout.html");
                }
            first = false;
        }
    }

    fn nodes(&mut self, nodes: &[Node], parent: Option<&Element>, top: bool) {
        let mut prev: Option<&Element> = None;
        for n in nodes {
            match n {
                Node::Element(e) => {
                    self.element(e, parent, prev, top);
                    prev = Some(e);
                }
                Node::Text(t, pos) if !is_blank(n) => {
                    prev = None;
                    if !parent.is_some_and(|p| p.is("code") || p.is("pre")) {
                        self.interpolation(t, *pos, true);
                    }
                }
                n if !is_blank(n) => prev = None,
                _ => {}
            }
        }
    }

    fn element(&mut self, e: &Element, parent: Option<&Element>, prev: Option<&Element>, top: bool) {
        let name = e.name.to_ascii_lowercase();
        let in_head = parent.is_some_and(|p| p.is("head"));
        match name.as_str() {
            "script" if directive::is_json_ld(e) && top && self.kind == Kind::Page => self.json_ld(e),
            "script" if directive::is_json_ld(e) => self.err(e.pos, "html/script", "JSON-LD <script type=\"application/ld+json\"> goes at the top level of a page"),
            "script" => self.err(e.pos, "html/script", "<script> goes once, at the top level of a page or component .html (the layout has none)"),
            "style" => self.err(e.pos, "html/style-inline", "<style> goes once, at the top level of a page or component .html; otherwise use utility classes or lib/theme.css"),
            "html" | "body" => self.err(e.pos, "html/layout", format!("<{name}> is generated by the build; do not write it in the fragment")),
            "head" if !(self.kind == Kind::Layout && top) => self.err(e.pos, "html/layout", "<head> only at the start of _layout.html"),
            "title" if !(top && self.kind == Kind::Page) && !in_head => self.err(e.pos, "html/title", "<title> only at the top of the page fragment"),
            "meta" | "link" | "base" if !(top && self.kind == Kind::Page && name != "base") && !in_head => {
                self.err(e.pos, "html/head", format!("<{name}> belongs in <head>: write it at the top level of the page fragment (or in the layout <head>)"))
            }
            "slot" if self.kind == Kind::Page => self.err(e.pos, "html/component", "<slot> only exists in components and the layout"),
            "main" if self.kind == Kind::Layout && e.has("data-page") => {}
            _ => {}
        }
        if let Some((_, c)) = component_tag(&name)
            && !is_component_name(c) {
                self.err(e.pos, "html/unknown-component", format!("invalid component name <{name}>"));
            }
        let head_tag = matches!(name.as_str(), "title" | "meta" | "link") && (in_head || (top && self.kind == Kind::Page));
        let for_var = e.value("data-for").and_then(|v| directive::parse_for(v)).map(|(v, _)| v.to_string());
        for a in &e.attrs {
            let an = a.name.to_ascii_lowercase();
            let v = a.value.as_deref().unwrap_or("");
            if an == "style" {
                self.err(a.pos, "html/style-inline", format!("the style attribute is not allowed; use classes, and for a computed value (position, size, color) {CSS_VAR_FIX}"));
                continue;
            }
            if let Some(m) = foreign_directive(&an) {
                self.err(a.pos, "html/unknown-directive", m);
                continue;
            }
            if an.starts_with("on") {
                self.err(a.pos, "html/on-inline", format!("inline {an} is not allowed; use @<event>=\"action\""));
                continue;
            }
            if v.contains("{{") || v.contains("${") {
                self.interpolation(v, a.pos, false);
                continue;
            }
            if an == "class" {
                for c in v.split_whitespace() {
                    self.classes.check(self.files, a.pos, c, &mut self.diags);
                }
                continue;
            }
            if an == "slot" && !parent.is_some_and(|p| is_component_tag(&p.name)) {
                self.err(a.pos, "html/component", "the slot attribute is only valid on direct children of <c-…> or <p-…>");
                continue;
            }
            let Some(d) = directive::parse(&an) else { continue };
            let Some(d) = d else {
                self.err(a.pos, "html/data-attribute", format!("hand-written `{an}` is not allowed; for state bind `:{an}=\"$path\"`, for styling use `.class=\"$flag\"` or `aria-*`, for tests select by role, label or id"));
                continue;
            };
            if head_tag {
                self.head_directive(&name, a.pos, d, v, in_head);
            } else {
                self.directive(e, &name, a.pos, d, v, for_var.as_deref());
            }
        }
        if e.is("template") {
            let blocks: Vec<&str> = BLOCKS.iter().copied().filter(|b| e.has(b)).collect();
            if blocks.is_empty() {
                self.err(e.pos, "html/outside-template", "bare <template> is not allowed in source: use <if>, <else-if>, <else>, <for> or <empty>");
            } else if blocks.len() > 1 {
                self.err(e.pos, "html/outside-template", format!("block with more than one kind ({})", blocks.join(", ")));
            }
            let tag = blocks.first().map_or("<template>", |b| tag_of(b));
            let roots = e.element_children().count();
            let text = stray_text(&e.children);
            if !blocks.is_empty() && (roots != 1 || text.is_some()) {
                self.err(text.unwrap_or(e.pos), "html/block-single-root", format!("the content of {tag} needs exactly 1 root element"));
            }
            if let Some(r) = e.element_children().next()
                && r.is("template") {
                    self.err(r.pos, "html/block-single-root", format!("the root of {tag} cannot be another block; wrap it in an element"));
                }
            let after_if = prev.is_some_and(|p| matches!(template_kind(p), Some("data-if" | "data-else-if")));
            match blocks.first().copied() {
                Some("data-for") if !e.has("data-key") => {
                    let var = for_var.as_deref().unwrap_or("x");
                    self.err(e.pos, "html/for-without-key", format!("<for> requires a stable, unique key: add key={var}.id (any field of {var} that identifies it)"))
                }
                Some(b @ ("data-else" | "data-else-if")) if !after_if => {
                    self.err(e.pos, "html/misplaced-else", format!("{} must come right after </if> or </else-if>", tag_of(b)))
                }
                Some("data-empty") if !prev.is_some_and(|p| template_kind(p) == Some("data-for")) => {
                    self.err(e.pos, "html/misplaced-empty", "<empty> must come right after </for>")
                }
                _ => {}
            }
        }
        self.nodes(&e.children, Some(e), false);
    }

    fn path(&mut self, pos: Pos, v: &str, neg_ok: bool, props_ok: bool) {
        if let Err(m) = directive::check_path(v, neg_ok) {
            self.err(pos, "html/invalid-path", m);
            return;
        }
        if directive::root(v) == "props" {
            let p = directive::split_neg(v).1;
            if !props_ok {
                self.err(pos, "html/invalid-root", "props is resolved at build time and is not valid in this directive");
            } else if self.kind != Kind::Component {
                self.err(pos, "html/invalid-root", "props only exists inside components");
            } else if p.split('.').count() != 2 {
                self.err(pos, "html/invalid-path", "props are text: use props.name, without subfields");
            }
        }
    }

    fn interpolation(&mut self, s: &str, pos: Pos, in_text: bool) {
        if let Some((i, m)) = ["{{", "${"].iter().filter_map(|m| s.find(*m).map(|i| (i, *m))).min() {
            let pos = if in_text { pos.advance(s, i) } else { pos };
            let close = if m == "{{" { "}}" } else { "}" };
            let dollar = if m == "${" { "; for a literal $ before a value write &#36;{path}" } else { "" };
            self.err(pos, "html/interpolation", format!("{m} {close} is not pragma syntax: write {{path}} in text and attributes (e.g. <p>Hi {{$name}}</p>, alt=\"Photo of {{p.name}}\"){dollar}; inside <code> or <pre> it is shown as text"));
        }
    }

    /// JSON-LD of a page: valid JSON; `{path}` only inside strings, with roots the server knows.
    fn json_ld(&mut self, e: &Element) {
        let (text, start) = match e.children.first() {
            Some(Node::Text(t, p)) => (t.as_str(), *p),
            _ => ("", e.pos),
        };
        let at = |off: usize| start.advance(text, off);
        if let Err((off, msg)) = directive::json_ld_check(text) {
            self.err(at(off), "html/json-ld", format!("invalid JSON-LD: {msg}"));
            return;
        }
        for (off, _, p) in directive::json_ld_paths(text).unwrap_or_default() {
            if let Err(m) = directive::check_path(p, false) {
                self.err(at(off), "html/invalid-path", m);
            } else if !matches!(directive::root(p), "data" | "params" | "query") {
                self.err(at(off), "html/head", format!("JSON-LD is rendered by the server and reads only data, params or query; \"{p}\""));
            }
        }
    }

    fn head_directive(&mut self, name: &str, pos: Pos, d: Dir, v: &str, in_layout: bool) {
        if in_layout {
            self.err(pos, "html/head", "the layout <head> is static (the same on every page); put bound tags at the top of the page fragment");
            return;
        }
        let ok = match d {
            Dir::Text => name == "title",
            Dir::Attr(n) => name != "title" && !n.starts_with("on") && n != "style",
            _ => false,
        };
        if !ok {
            self.err(pos, "html/head", format!("<{name}> in the head takes only {}", if name == "title" { "{path} text" } else { ":<attribute> or {path} in an attribute (e.g. :content, href=\"/p/{data.id}\")" }));
            return;
        }
        for p in values(d, v) {
            if let Err(m) = directive::check_path(&p, false) {
                self.err(pos, "html/invalid-path", m);
            } else if !matches!(directive::root(&p), "data" | "params" | "query") {
                self.err(pos, "html/head", format!("head tags are rendered by the server and read only data, params or query; \"{}\"", p.replace("$.", "$")));
            }
        }
    }

    fn directive(&mut self, e: &Element, name: &str, pos: Pos, d: Dir, v: &str, for_var: Option<&str>) {
        let is_tpl = name == "template";
        let on_tpl = |d: Dir| matches!(d, Dir::If | Dir::ElseIf | Dir::Else | Dir::For | Dir::Empty | Dir::Key);
        if on_tpl(d) && !is_tpl {
            self.err(pos, "html/outside-template", "conditions and loops are tags: <if>, <else-if>, <else>, <for> and <empty>");
            return;
        }
        if !on_tpl(d) && is_tpl {
            self.err(pos, "html/outside-template", "<if>, <else-if>, <else>, <for> and <empty> take no other directives; put them on the inner element");
            return;
        }
        match d {
            Dir::Text => {
                for p in values(d, v) {
                    self.path(pos, &p, false, true);
                }
            }
            Dir::Show | Dir::If | Dir::ElseIf => self.path(pos, v, true, true),
            Dir::Class(c) => {
                self.path(pos, v, true, true);
                self.classes.check(self.files, pos, c, &mut self.diags);
            }
            Dir::Attr(n) => {
                if n == "style" {
                    self.err(pos, "html/style-inline", format!(":style is not allowed: toggle a class with .name=\"path\" (e.g. .hidden=\"$closed\"); for a value with many options bind :data-x=\"path\" and style [data-x=value] in CSS; for a computed value {CSS_VAR_FIX}"));
                } else if let Some(ev) = n.strip_prefix("on") {
                    self.err(pos, "html/on-inline", format!(":{n} is not allowed; use @{ev}=\"action\""));
                } else if n == "class" {
                    self.err(pos, "html/class", ":class is not allowed: write static classes in class=\"…\" and toggle each one with .name=\"path\" (e.g. .text-warning-700=\"$warn\")");
                }
                for p in values(d, v).into_iter().filter(|_| directive::literal(v).is_none()) {
                    self.path(pos, &p, false, true);
                }
            }
            Dir::Var(_) if directive::literal(v).is_some() => {}
            Dir::Var(_) => self.path(pos, v, false, true),
            Dir::Model => {
                if !matches!(name, "input" | "select" | "textarea") {
                    self.err(pos, "html/unknown-directive", ":model only on input, select and textarea");
                }
                self.path(pos, v, false, false);
            }
            Dir::Else | Dir::Empty => {
                if !v.is_empty() {
                    self.err(pos, "html/invalid-path", "<else> and <empty> take no attributes");
                }
            }
            Dir::For => match directive::parse_for(v) {
                None => self.err(pos, "html/invalid-path", "use <for x in path key=x.field>"),
                Some((var, p)) => {
                    if directive::ROOTS.contains(&var) || var == "event" || var == "el" {
                        self.err(pos, "html/invalid-root", format!("<for> variable \"{var}\" cannot use a root name"));
                    }
                    self.path(pos, p, false, false);
                }
            },
            Dir::Key => {
                if !e.has("data-for") {
                    self.err(pos, "html/for-without-key", "key only on <for>");
                } else if let Err(m) = directive::check_path(v, false) {
                    self.err(pos, "html/invalid-path", m);
                } else if let Some(var) = for_var
                    && directive::root(v) != var {
                        self.err(pos, "html/invalid-root", format!("key must start from the <for> variable ({var}.field)"));
                    }
            }
            Dir::On(ev) => {
                if !ev.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                    self.err(pos, "html/unknown-directive", format!("invalid event \"{ev}\""));
                }
                match directive::parse_call(v) {
                    Ok(call) => {
                        for p in call.paths() {
                            self.path(pos, p, false, false);
                        }
                    }
                    Err(m) => self.err(pos, "html/invalid-path", format!("@{ev}: {m}")),
                }
            }
            Dir::With(alias) => {
                if !is_component_tag(name) {
                    self.err(pos, "html/component", "dynamic prop (:name) only on <c-…> or <p-…>");
                } else if !directive::is_ident(alias) || directive::ROOTS.contains(&alias) {
                    self.err(pos, "html/component", format!("invalid alias \"{alias}\""));
                }
                if directive::literal(v).is_none() {
                    self.path(pos, v, false, false);
                }
            }
            Dir::Open => {
                if v.is_empty() {
                    self.err(pos, "html/unknown-directive", "@open needs the overlay id");
                }
            }
            Dir::Close => {}
            Dir::Marker => {
                if e.attrs.iter().any(|a| a.pos == pos && a.name.eq_ignore_ascii_case("data-page")) && !(self.kind == Kind::Layout && name == "main") {
                    self.err(pos, "html/layout", "data-page only on the <main> of _layout.html");
                }
            }
            Dir::Generated => {
                let an = e.attrs.iter().find(|a| a.pos == pos).map_or(String::new(), |a| a.name.to_ascii_lowercase());
                let hint = match an.as_str() {
                    "data-c" => "write the component tag `<c-name>`",
                    "data-slot" => "put `slot=\"name\"` on the direct child of the component",
                    _ => "the build generates it; remove it",
                };
                self.err(pos, "html/data-attribute", format!("`{an}` is internal; {hint}"));
            }
        }
    }
}

const CSS_VAR_FIX: &str = "bind a CSS variable with `:--name=\"path\"` and use it in a CSS rule (`.bar { left: calc(var(--x) * 1%) }` with `:--x=\"$left\"`)";

const ALPINE: &[&str] = &[
    "data", "init", "show", "bind", "on", "text", "html", "model", "modelable", "for", "if", "transition", "effect", "ignore",
    "ref", "cloak", "teleport", "id", "intersect", "mask", "trap", "collapse", "anchor", "sort", "resize",
];
const SVELTE: &[&str] = &["bind", "on", "class", "style", "use", "transition", "in", "out", "animate", "let"];

/// Directive syntax of other frameworks (Vue, Alpine, Svelte, Angular, React). In pragma it stays a
/// plain attribute that does nothing, so it is an error that names the pragma form. `name` is lowercase.
fn foreign_directive(name: &str) -> Option<String> {
    // `on:click.prevent` → ("on", "click"); modifiers (`.x`, Svelte `|x`) are dropped
    fn dir_arg(rest: &str) -> (&str, &str) {
        let end = rest.find([':', '.']).unwrap_or(rest.len());
        let arg = rest[end..].strip_prefix(':').unwrap_or("");
        (&rest[..end], &arg[..arg.find(['.', '|']).unwrap_or(arg.len())])
    }
    let (framework, dir, arg) = if let Some(rest) = name.strip_prefix("v-") {
        let (d, a) = dir_arg(rest);
        ("Vue", d, a)
    } else if let Some(rest) = name.strip_prefix("x-")
        && ALPINE.contains(&dir_arg(rest).0)
    {
        let (d, a) = dir_arg(rest);
        ("Alpine", d, a)
    } else if let Some((d, a)) = name.split_once(':')
        && SVELTE.contains(&d)
    {
        let a = &a[..a.find(['.', '|']).unwrap_or(a.len())];
        // `bind:value` is two-way
        ("Svelte", if d == "bind" && matches!(a, "value" | "checked" | "group") { "model" } else { d }, a)
    } else if let Some(inner) = name.strip_prefix("[(").and_then(|n| n.strip_suffix(")]")) {
        ("Angular", "model", inner)
    } else if let Some(inner) = name.strip_prefix('[').and_then(|n| n.strip_suffix(']')) {
        match inner.split_once('.') {
            Some(("class", c)) => ("Angular", "class", c),
            Some(("style", _)) => ("Angular", "style", ""),
            Some(("attr", a)) => ("Angular", "bind", a),
            _ => ("Angular", "bind", inner),
        }
    } else if let Some(ev) = name.strip_prefix('(').and_then(|n| n.strip_suffix(')')) {
        ("Angular", "on", ev)
    } else if let Some(d) = name.strip_prefix("*ng") {
        ("Angular", d, "")
    } else if name.starts_with('#') {
        ("Vue/Angular", "#", "")
    } else if name == "classname" || name == "htmlfor" {
        ("React", if name == "classname" { "class" } else { "for-attr" }, "")
    } else {
        return None;
    };
    let hint = match (dir, arg) {
        ("if", _) => "wrap the element in `<if path>…</if>`".to_string(),
        ("else-if", _) => "wrap the element in `<else-if path>…</else-if>`, right after `</if>`".into(),
        ("else", _) => "wrap the element in `<else>…</else>`, right after `</if>`".into(),
        ("for" | "each", _) => "wrap the element in `<for x in path key=x.id>…</for>`".into(),
        ("show", _) => "use `:show=\"path\"`".into(),
        ("text", _) => "write `{path}` inside the element".into(),
        ("html", _) => "write `{path}` inside the element (pragma does not render raw HTML)".into(),
        ("model" | "modelable", _) => "use `:model=\"$path\"` on input, select or textarea".into(),
        ("bind", "") => "use `:<attribute>=\"path\"`".into(),
        ("bind", "class") | ("class", "") => "use `class=\"…\"` for static classes and `.<class>=\"path\"` to toggle one".into(),
        ("style", v) if v.starts_with("--") => format!("use `:{v}=\"path\"` and read it in CSS with `var({v})`"),
        ("bind", "style") | ("style", _) => format!("use classes (toggle one with `.<class>=\"path\"`); for a computed value {CSS_VAR_FIX}"),
        ("bind", a) => format!("use `:{a}=\"path\"`"),
        ("on", "") => "use `@<event>=\"action\"`".into(),
        ("on", ev) => format!("use `@{ev}=\"action\"`"),
        ("class", c) => format!("use `.{c}=\"path\"`"),
        ("for-attr", _) => "use `for=\"…\"`".into(),
        ("data" | "init" | "setup", _) => "put state and actions in the page or component .js".into(),
        _ => "see the closed list in .pragma/docs/syntax.md".into(),
    };
    Some(format!("`{name}` is {framework} syntax and does nothing in pragma; {hint}"))
}

fn find_mains<'a>(nodes: &'a [Node], out: &mut Vec<&'a Element>) {
    for n in nodes {
        if let Node::Element(e) = n {
            if e.is("main") && e.has("data-page") {
                out.push(e);
            }
            find_mains(&e.children, out);
        }
    }
}

fn find_slots<'a>(nodes: &'a [Node], out: &mut Vec<&'a Element>) {
    for n in nodes {
        if let Node::Element(e) = n {
            if e.is("slot") {
                out.push(e);
            }
            find_slots(&e.children, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_class_scan() {
        let mut s = HashSet::new();
        css_classes("/* .no */ .btn, a.link:hover { margin: .5rem; background: url(x.y.png) } @media (x) { .md\\:a > .b-c {} } [data-c=\"card\"] .title{}", &mut s);
        let mut v: Vec<_> = s.into_iter().collect();
        v.sort();
        assert_eq!(v, ["b-c", "btn", "link", "md:a", "title"]);
    }

    #[test]
    fn foreign_directives() {
        let cases = [
            ("v-if", "Vue", "<if path>"),
            ("v-else-if", "Vue", "<else-if path>"),
            ("v-for", "Vue", "<for x in path key=x.id>"),
            ("v-bind:href.prop", "Vue", ":href=\"path\""),
            ("v-bind:class", "Vue", ".<class>=\"path\""),
            ("v-on:click.prevent", "Vue", "@click=\"action\""),
            ("v-model.trim", "Vue", ":model=\"$path\""),
            ("x-show", "Alpine", ":show=\"path\""),
            ("x-data", "Alpine", "page or component .js"),
            ("x-on:keydown.enter", "Alpine", "@keydown=\"action\""),
            ("on:click|preventdefault", "Svelte", "@click=\"action\""),
            ("bind:value", "Svelte", ":model=\"$path\""),
            ("class:active", "Svelte", ".active=\"path\""),
            ("*ngif", "Angular", "<if path>"),
            ("[disabled]", "Angular", ":disabled=\"path\""),
            ("[class.active]", "Angular", ".active=\"path\""),
            ("(click)", "Angular", "@click=\"action\""),
            ("[(ngmodel)]", "Angular", ":model=\"$path\""),
            ("classname", "React", "class=\"…\""),
        ];
        for (name, framework, want) in cases {
            let m = foreign_directive(name).unwrap_or_else(|| panic!("{name} passed"));
            assert!(m.contains(&format!("{framework} syntax")) && m.contains(want), "{name}: {m}");
        }
        for name in ["x-foo", "xlink:href", "xml:lang", "xmlns:xlink", "aria-label", "role", "for", "class", "data-x", "hidden"] {
            assert_eq!(foreign_directive(name), None, "{name}");
        }
    }
}
