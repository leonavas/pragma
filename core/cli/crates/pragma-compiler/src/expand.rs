//! Component expansion (macro, SPEC §3) and scoped validation on the expanded page.
use crate::directive::{self, Dir, Part};
use crate::html::{Attr, Element, Node, Pos, escape_text, is_blank};
use crate::project::{Component, Page, Project, component_tag, is_component_tag, rel_str};
use crate::validate::diag;
use crate::{Fnv, short_hash};
use pragma_diag::Diagnostic;
use pragma_jslint::script::ScriptInfo;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub struct Expanded {
    /// Layout `<head>` merged with the page's top-level `<title>`, `<meta>` and `<link>`.
    pub head: Vec<Node>,
    pub body: Vec<Node>,
    /// Content of `<main data-page>` (base of the SSR ops).
    pub page: Vec<Node>,
    /// By `data-c` id (`Component::id`).
    pub components: BTreeMap<String, Component>,
    /// Directories of components used directly in `_layout.html`.
    pub layout_components: BTreeSet<PathBuf>,
}

type Props = HashMap<String, Option<String>>;
type Slots = HashMap<String, Vec<Node>>;

#[derive(Clone)]
struct Scope<'s> {
    from: &'s Path,
    props: Option<&'s Props>,
    aliases: HashMap<String, String>,
    bound_props: HashMap<String, String>,
    /// The owner has a script: every `$x` is `state.x` (its instance scope, props included).
    script: bool,
    slots: Option<&'s Slots>,
    inst: u64,
}

enum Res {
    Path(String),
    /// Prop missing (`None`), boolean (`Some(None)`) or with a value.
    Prop(Option<Option<String>>),
}

fn prop_truthy(p: &Option<Option<String>>) -> bool {
    match p {
        None => false,
        Some(None) => true,
        Some(Some(v)) => !v.is_empty() && v != "false",
    }
}

pub struct Expander<'p> {
    p: &'p Project,
    pub diags: Vec<Diagnostic>,
    used: BTreeMap<String, Component>,
    layout_used: BTreeSet<PathBuf>,
    in_layout: bool,
    stack: Vec<&'p Path>,
    scripts: &'p HashMap<PathBuf, ScriptInfo>,
}

fn first_element(nodes: &[Node]) -> Option<&Element> {
    nodes.iter().find_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
}

fn has_runtime_directive(e: &Element) -> bool {
    let runtime = |a: &Attr| matches!(directive::parse(&a.name.to_ascii_lowercase()), Some(Some(Dir::Text | Dir::Show | Dir::Attr(_) | Dir::Class(_) | Dir::If | Dir::ElseIf | Dir::For | Dir::On(_) | Dir::Model)));
    e.attrs.iter().any(runtime) || e.element_children().any(has_runtime_directive)
}

/// Slot content belongs to whoever wrote it: `data-slot="n"` = owner n components up.
fn mark_slot(e: &mut Element) {
    if let Some(a) = e.attrs.iter_mut().find(|a| a.name == "data-slot") {
        a.value = Some((a.value.as_deref().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0) + 1).to_string());
    } else if has_runtime_directive(e) {
        e.attrs.push(Attr { name: "data-slot".into(), value: Some("1".into()), pos: e.pos });
    }
}

fn collect_slots(nodes: &[Node], out: &mut BTreeSet<String>) {
    for n in nodes {
        if let Node::Element(e) = n {
            if e.is("slot") {
                out.insert(e.value("name").unwrap_or("").to_string());
            }
            collect_slots(&e.children, out);
        }
    }
}

impl<'p> Expander<'p> {
    pub fn new(p: &'p Project, scripts: &'p HashMap<PathBuf, ScriptInfo>) -> Self {
        Expander { p, diags: Vec::new(), used: BTreeMap::new(), layout_used: BTreeSet::new(), in_layout: false, stack: Vec::new(), scripts }
    }

    fn err(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.diags.push(diag(&self.p.files, pos, rule, msg));
    }

    fn tid(&self, pos: Pos, inst: u64) -> String {
        let mut h = Fnv::new();
        h.write(rel_str(&self.p.files[pos.file as usize]).as_bytes());
        h.write(&pos.line.to_le_bytes());
        h.write(&pos.col.to_le_bytes());
        h.write(&inst.to_le_bytes());
        format!("t{}", &short_hash(h.finish())[..7])
    }

    pub fn page(mut self, page: &Page) -> (Expanded, Vec<Diagnostic>) {
        let p = self.p;
        let tree = &p.trees[page.html as usize];
        let mut own_head: Vec<Element> = Vec::new();
        let mut content: Vec<Node> = Vec::new();
        for n in tree {
            match n {
                Node::Element(e) if is_head_tag(e) => own_head.push(e.clone()),
                _ => content.push(n.clone()),
            }
        }
        let sc = Scope { from: &page.dir, props: None, aliases: HashMap::new(), bound_props: HashMap::new(), script: true, slots: None, inst: 0 };
        let mut page_nodes = self.expand_nodes(&content, &sc);
        if page.out_name != "_404" {
            for dir in page.dir.ancestors() {
                if dir == Path::new("routes") || !dir.starts_with("routes") {
                    break;
                }
                if let Some(&nl_id) = p.nested_layouts.get(dir) {
                    let slots: Slots = HashMap::from([(String::new(), page_nodes)]);
                    let nl_sc = Scope { from: dir, props: None, aliases: HashMap::new(), bound_props: HashMap::new(), script: true, slots: Some(&slots), inst: 0 };
                    page_nodes = self.expand_nodes(&p.trees[nl_id as usize], &nl_sc);
                }
            }
        }
        let mut head = Vec::new();
        let mut body = Vec::new();
        if let Some(l) = p.layout {
            let ltree = &p.trees[l as usize];
            let mut rest: Vec<Node> = Vec::new();
            let mut seen_content = false;
            for n in ltree {
                match n {
                    Node::Element(e) if e.is("head") && !seen_content => head = e.children.iter().filter(|n| !matches!(n, Node::Comment)).cloned().collect(),
                    n => {
                        seen_content |= !is_blank(n);
                        rest.push(n.clone());
                    }
                }
            }
            let slots: Slots = HashMap::from([(String::new(), page_nodes.clone())]);
            let lsc = Scope { from: Path::new("routes"), props: None, aliases: HashMap::new(), bound_props: HashMap::new(), script: true, slots: Some(&slots), inst: 0 };
            self.in_layout = true;
            body = self.expand_nodes(&rest, &lsc);
            self.in_layout = false;
            if page.css.is_some() {
                name_page_main(&mut body, &page.out_name);
            }
        }
        merge_head(&mut head, own_head);
        (Expanded { head, body, page: page_nodes, components: self.used, layout_components: self.layout_used }, self.diags)
    }

    fn resolve(&self, v: &str, sc: &Scope) -> Res {
        if directive::check_path(v, true).is_err() {
            return Res::Path(v.to_string());
        }
        let (negated, p) = directive::split_neg(v);
        let r = directive::root(p);
        if r == "$" {
            let bang = if negated { "!" } else { "" };
            let rest = &p[2..];
            let name = rest.split('.').next().unwrap_or("");
            let tail = &rest[name.len()..];
            // a page, or a component with a script: its instance scope (state, props, functions)
            let (Some(props), false) = (sc.props, sc.script) else {
                return Res::Path(format!("{bang}state.{rest}"));
            };
            // component without a script: `:x` props are the caller's paths, the rest build-time text
            if let Some(target) = sc.bound_props.get(name) {
                return Res::Path(format!("{bang}{target}{tail}"));
            }
            if !tail.is_empty() {
                return Res::Path(v.to_string());
            }
            return Res::Prop(props.get(&name.to_ascii_lowercase()).cloned());
        }
        if r == "props" {
            if let Some(props) = sc.props {
                let name = p.split('.').nth(1).unwrap_or("");
                return Res::Prop(props.get(&name.to_ascii_lowercase()).cloned());
            }
        } else if let Some(t) = sc.aliases.get(r) {
            return Res::Path(directive::replace_root(v, t));
        }
        Res::Path(v.to_string())
    }

    /// `:toolname="$x"` in a component with a script: a WebMCP tool attribute is static, so it takes
    /// the caller's fixed text at build time (`None` inside: the caller left the prop out). `None`:
    /// not a prop of a scripted component, or the caller bound it to a path.
    fn caller_text(&self, v: &str, sc: &Scope) -> Option<Option<Option<String>>> {
        let props = sc.props.filter(|_| sc.script)?;
        let name = v.strip_prefix("$.").filter(|n| directive::is_ident(n))?.to_ascii_lowercase();
        if sc.bound_props.contains_key(&name) {
            return None;
        }
        Some(props.get(&name).cloned())
    }

    fn expand_nodes(&mut self, nodes: &[Node], sc: &Scope) -> Vec<Node> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            match &nodes[i] {
                Node::Comment => {}
                Node::Text(..) => out.push(nodes[i].clone()),
                Node::Element(e) => {
                    let lname = e.name.to_ascii_lowercase();
                    if lname == directive::INTERP {
                        out.push(self.interp(e, sc));
                    } else if is_component_tag(&lname) {
                        out.extend(self.component(e, sc));
                    } else if lname == "slot" && sc.slots.is_some() {
                        let name = e.value("name").unwrap_or("");
                        match sc.slots.unwrap().get(name) {
                            Some(v) if v.iter().any(|n| !is_blank(n)) => out.extend(v.iter().cloned()),
                            _ => out.extend(self.expand_nodes(&e.children, sc)),
                        }
                    } else if lname == "template" && e.has("data-if") {
                        let mut chain = vec![e];
                        let mut j = i + 1;
                        let start = i;
                        loop {
                            let mut k = j;
                            while k < nodes.len() && is_blank(&nodes[k]) {
                                k += 1;
                            }
                            match nodes.get(k) {
                                Some(Node::Element(x)) if x.is("template") && (x.has("data-else-if") || x.has("data-else")) => {
                                    chain.push(x);
                                    i = k;
                                    j = k + 1;
                                    if x.has("data-else") {
                                        break;
                                    }
                                }
                                _ => break,
                            }
                        }
                        let is_static = |t: &&Element| {
                            t.value("data-if").or_else(|| t.value("data-else-if")).is_some_and(|v| matches!(self.resolve(v, sc), Res::Prop(_)))
                        };
                        if chain.iter().any(is_static) {
                            out.extend(self.chain(&chain, sc));
                        } else {
                            i = start;
                            out.push(Node::Element(self.element(e, sc)));
                        }
                    } else {
                        out.push(Node::Element(self.element(e, sc)));
                    }
                }
            }
            i += 1;
        }
        out
    }

    /// if/else-if/else chain: branches on a static prop are decided at build time; the rest becomes
    /// the runtime chain (the first dynamic branch becomes the if).
    fn chain(&mut self, chain: &[&Element], sc: &Scope) -> Vec<Node> {
        let mut kept: Vec<Element> = Vec::new();
        for t in chain {
            let cond = t.value("data-if").or_else(|| t.value("data-else-if"));
            let taken = match cond.map(|v| (v, self.resolve(v, sc))) {
                None => true,
                Some((v, Res::Prop(prop))) => prop_truthy(&prop) != v.starts_with('!'),
                Some((v, Res::Path(_))) => {
                    let mut b = (*t).clone();
                    let name = if kept.is_empty() { "data-if" } else { "data-else-if" };
                    b.attrs.retain(|a| !a.name.eq_ignore_ascii_case("data-if") && !a.name.eq_ignore_ascii_case("data-else-if"));
                    b.attrs.insert(0, Attr { name: name.into(), value: Some(v.to_string()), pos: t.pos });
                    kept.push(b);
                    continue;
                }
            };
            if !taken {
                continue;
            }
            if kept.is_empty() {
                return self.expand_nodes(&t.children, sc);
            }
            let mut b = (*t).clone();
            b.attrs.retain(|a| !a.name.eq_ignore_ascii_case("data-else-if") && !a.name.eq_ignore_ascii_case("data-else"));
            b.attrs.insert(0, Attr { name: "data-else".into(), value: None, pos: t.pos });
            kept.push(b);
            break;
        }
        let mut out = Vec::new();
        for (n, b) in kept.iter().enumerate() {
            if n > 0 {
                out.push(Node::Text("\n".into(), b.pos));
            }
            out.push(Node::Element(self.element(b, sc)));
        }
        out
    }

    fn element(&mut self, e: &Element, sc: &Scope) -> Element {
        let mut el = Element { name: e.name.clone(), attrs: Vec::with_capacity(e.attrs.len()), children: Vec::new(), pos: e.pos };
        let mut text: Option<String> = None;
        let mut classes: Vec<String> = Vec::new();
        let mut hidden = false;
        let mut for_var: Option<String> = None;
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            let Some(Some(d)) = directive::parse(&lname) else {
                el.attrs.push(a.clone());
                continue;
            };
            let v = a.value.as_deref().unwrap_or("");
            if let Dir::Attr(n) = d
                && crate::webmcp::is_tool_attr(n)
                && let Some(text) = self.caller_text(v, sc)
            {
                if let Some(value) = text {
                    el.attrs.push(Attr { name: n.to_string(), value, pos: a.pos });
                }
                continue;
            }
            match d {
                Dir::Text => {
                    match self.template(v, sc, true) {
                        Ok(t) => el.attrs.push(Attr { value: Some(t), ..a.clone() }),
                        Err(literal) => text = Some(literal),
                    }
                    continue;
                }
                Dir::Attr(n) if directive::has_braces(v) => {
                    match self.template(v, sc, false) {
                        Ok(t) => el.attrs.push(Attr { value: Some(t), ..a.clone() }),
                        Err(literal) => el.attrs.push(Attr { name: n.to_string(), value: Some(literal), pos: a.pos }),
                    }
                    continue;
                }
                Dir::On(_) => {
                    el.attrs.push(Attr { value: Some(self.event_call(v, sc)), ..a.clone() });
                    continue;
                }
                _ => {}
            }
            let res = match d {
                Dir::Show | Dir::Attr(_) | Dir::Class(_) | Dir::If | Dir::ElseIf | Dir::Model => self.resolve(v, sc),
                Dir::For => {
                    if let Some((var, path)) = directive::parse_for(v) {
                        for_var = Some(var.to_string());
                        if let Res::Path(p) = self.resolve(path, sc) {
                            el.attrs.push(Attr { value: Some(format!("{var} in {p}")), ..a.clone() });
                            continue;
                        }
                    }
                    Res::Path(v.to_string())
                }
                _ => Res::Path(v.to_string()),
            };
            match res {
                Res::Path(p) => el.attrs.push(Attr { value: a.value.as_ref().map(|_| p), ..a.clone() }),
                Res::Prop(prop) => {
                    let neg = v.starts_with('!');
                    match d {
                        Dir::Attr(n) => {
                            if let Some(val) = prop {
                                el.attrs.push(Attr { name: n.to_string(), value: val, pos: a.pos });
                            }
                        }
                        Dir::Class(n) => {
                            if prop_truthy(&prop) != neg {
                                classes.push(n.to_string());
                            }
                        }
                        Dir::Show => hidden |= prop_truthy(&prop) == neg,
                        _ => {}
                    }
                }
            }
        }
        if !classes.is_empty() {
            merge_class(&mut el, &classes.join(" "));
        }
        if hidden && !el.has("hidden") {
            el.attrs.push(Attr { name: "hidden".into(), value: None, pos: e.pos });
        }
        if el.is("template") && crate::validate::BLOCKS.iter().any(|b| el.has(b)) {
            let tid = self.tid(e.pos, sc.inst);
            el.attrs.push(Attr { name: "data-tid".into(), value: Some(tid), pos: e.pos });
        }
        el.children = match text {
            Some(t) => vec![Node::Text(t, e.pos)],
            None => match for_var.filter(|v| sc.aliases.contains_key(v)) {
                Some(var) => {
                    let mut inner = sc.clone();
                    inner.aliases.remove(&var);
                    self.expand_nodes(&e.children, &inner)
                }
                None => self.expand_nodes(&e.children, sc),
            },
        };
        el
    }

    fn component(&mut self, e: &Element, sc: &Scope) -> Vec<Node> {
        let p = self.p;
        let tag = e.name.to_ascii_lowercase();
        let (local, name) = component_tag(&tag).map(|(l, n)| (l, n.to_string())).unwrap();
        let Some(comp) = p.find_component(sc.from, local, &name) else {
            let dirs: Vec<String> = p.component_search(sc.from, local).iter().map(|d| rel_str(d)).collect();
            let msg = if dirs.is_empty() {
                format!("<{tag}> only exists inside routes/; elsewhere use <c-{name}> in lib/components")
            } else {
                format!("component <{tag}> not found (searched in {})", dirs.join(", "))
            };
            self.err(e.pos, "html/unknown-component", msg);
            return Vec::new();
        };
        if self.stack.contains(&comp.dir.as_path()) {
            self.err(e.pos, "html/component", format!("component <{tag}> uses itself (cycle)"));
            return Vec::new();
        }
        let mut props: Props = HashMap::new();
        let aliases = sc.aliases.clone();
        let mut bound_props = HashMap::new();
        let mut forward = Vec::new();
        let mut class_extra = None;
        // a component with a script declares its props (`export let`); they reach the instance as
        // data-prop-<name> (text) and data-with-<name> (a path in the caller's scope)
        let script = comp.module.as_ref().and_then(|m| self.scripts.get(m));
        let mut static_props: Vec<(String, String)> = Vec::new();
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            let prop = lname.strip_prefix("data-with-").unwrap_or(&lname);
            if let Some(s) = script
                && prop != "class"
                && prop != "slot"
                && !(prop.starts_with("data-") && prop == lname)
                && !s.props.iter().any(|x| x.eq_ignore_ascii_case(prop))
            {
                let has = if s.props.is_empty() { "its script declares no props".to_string() } else { format!("its props: {}", s.props.join(", ")) };
                let lowered: Vec<String> = s.props.iter().map(|p| p.to_ascii_lowercase()).collect();
                let fix = match directive::did_you_mean(prop, lowered.iter().map(String::as_str)) {
                    Some(near) => format!("did you mean `{near}`?"),
                    None => format!("declare it with `export let {prop} = …`"),
                };
                self.err(a.pos, "html/unknown-prop", format!("<{tag}> has no prop `{prop}` ({has}); {fix}"));
            }
            if let Some(alias) = lname.strip_prefix("data-with-") {
                match self.resolve(a.value.as_deref().unwrap_or(""), sc) {
                    Res::Path(p) if script.is_some() => {
                        bound_props.insert(alias.to_string(), p.clone());
                        forward.push(Attr { name: format!("data-with-{alias}"), value: Some(p), pos: a.pos });
                    }
                    Res::Path(p) => {
                        bound_props.insert(alias.to_string(), p);
                    }
                    Res::Prop(value) => {
                        if let Some(v) = &value {
                            static_props.push((alias.to_string(), v.clone().unwrap_or_else(|| "true".into())));
                        }
                        props.insert(alias.to_string(), value.flatten());
                    }
                }
            } else if lname == "slot" {
            } else if lname.starts_with("data-") {
                forward.push(a.clone());
            } else {
                if lname == "class" {
                    class_extra = a.value.clone();
                } else {
                    static_props.push((lname.clone(), a.value.clone().unwrap_or_else(|| "true".into())));
                }
                props.insert(lname, a.value.clone());
            }
        }
        let fwd = self.element(&Element { name: "div".into(), attrs: forward, children: Vec::new(), pos: e.pos }, sc).attrs;
        let kids = self.expand_nodes(&e.children, sc);
        let mut slots: Slots = HashMap::new();
        for n in kids {
            match n {
                Node::Element(mut k) => {
                    let s = k.value("slot").unwrap_or("").to_string();
                    k.remove("slot");
                    mark_slot(&mut k);
                    slots.entry(s).or_default().push(Node::Element(k));
                }
                n => slots.entry(String::new()).or_default().push(n),
            }
        }
        let tree = &p.trees[comp.html as usize];
        let mut names = BTreeSet::new();
        collect_slots(tree, &mut names);
        for (k, v) in &slots {
            if !names.contains(k) && v.iter().any(|n| !is_blank(n)) {
                let at = first_element(v).map_or(e.pos, |x| x.pos);
                let what = if k.is_empty() { "children (it has no <slot>)".to_string() } else { format!("slot \"{k}\"") };
                self.err(at, "html/component", format!("<{tag}> does not accept {what}"));
            }
        }
        let Some(root) = tree.iter().find(|n| matches!(n, Node::Element(_))) else { return Vec::new() };
        let mut h = Fnv::new();
        h.write(&sc.inst.to_le_bytes());
        h.write(rel_str(&p.files[e.pos.file as usize]).as_bytes());
        h.write(&e.pos.line.to_le_bytes());
        h.write(&e.pos.col.to_le_bytes());
        let inner = Scope { from: &comp.scope, props: Some(&props), aliases, bound_props, script: comp.module.is_some(), slots: Some(&slots), inst: h.finish() };
        self.stack.push(&comp.dir);
        let mut nodes = self.expand_nodes(std::slice::from_ref(root), &inner);
        self.stack.pop();
        match self.used.get(&comp.id) {
            Some(prev) if prev.dir != comp.dir => {
                let msg = format!("two components with the id \"{}\" on the same page ({} and {}); rename one", comp.id, rel_str(&prev.dir), rel_str(&comp.dir));
                self.err(e.pos, "html/component", msg);
            }
            _ => {
                self.used.insert(comp.id.clone(), comp.clone());
            }
        }
        if self.in_layout {
            self.layout_used.insert(comp.dir.clone());
        }
        let mut dup = Vec::new();
        if let Some(Node::Element(r)) = nodes.iter_mut().find(|n| matches!(n, Node::Element(_))) {
            r.attrs.insert(0, Attr { name: "data-c".into(), value: Some(comp.id.clone()), pos: r.pos });
            if comp.module.is_some() {
                for (k, v) in static_props {
                    r.attrs.push(Attr { name: format!("data-prop-{k}"), value: Some(v), pos: r.pos });
                }
            }
            if let Some(c) = class_extra {
                merge_class(r, &c);
            }
            for a in fwd {
                if r.has(&a.name) {
                    dup.push(a.clone());
                } else {
                    r.attrs.push(a);
                }
            }
        }
        for a in dup {
            self.err(a.pos, "html/component", format!("{} already exists on the root of <{tag}>", a.name));
        }
        nodes
    }
}

/// `{}` templates and event calls (@sfc-template).
impl Expander<'_> {
    /// Resolves the paths of a `{}` template. Ok: the template still has runtime paths.
    /// Err: only static props were used; the literal text (HTML-escaped when `text`).
    fn template(&self, v: &str, sc: &Scope, text: bool) -> Result<String, String> {
        let Ok(parts) = directive::template(v) else { return Ok(v.to_string()) };
        let mut dynamic = false;
        let parts: Vec<Part> = parts
            .into_iter()
            .map(|part| match part {
                Part::Path(p) => match self.resolve(&p, sc) {
                    Res::Path(p) => {
                        dynamic = true;
                        Part::Path(p)
                    }
                    Res::Prop(prop) => {
                        let value = prop.flatten().unwrap_or_default();
                        Part::Text(if text { escape_text(&value) } else { value })
                    }
                },
                text => text,
            })
            .collect();
        if dynamic {
            Ok(directive::join_template(&parts))
        } else {
            Err(parts.into_iter().map(|p| if let Part::Text(t) = p { t } else { String::new() }).collect())
        }
    }

    /// `toggle(task, $q)`: path arguments resolved (`state.q`, caller paths); static props become literals.
    fn event_call(&self, v: &str, sc: &Scope) -> String {
        let Ok(call) = directive::parse_call(v) else { return v.to_string() };
        directive::join_call(&call, |p| match self.resolve(p, sc) {
            Res::Path(p) => p,
            Res::Prop(None) => "null".into(),
            Res::Prop(Some(None)) => "true".into(),
            Res::Prop(Some(Some(value))) => {
                let quote = if value.contains('\'') { '"' } else { '\'' };
                format!("{quote}{}{quote}", value.replace(quote, ""))
            }
        })
    }

    /// `#interp` (`{}` beside elements): a runtime marker, or plain text when only static props are used.
    fn interp(&mut self, e: &Element, sc: &Scope) -> Node {
        let v = e.value("data-text").unwrap_or("");
        match self.template(v, sc, true) {
            Ok(_) => Node::Element(self.element(e, sc)),
            Err(literal) => Node::Text(literal, e.pos),
        }
    }
}

/// `<main data-page="<page>">`: the scope of the page's own CSS (the router copies it on navigation).
fn name_page_main(nodes: &mut [Node], page: &str) -> bool {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if e.is("main") && let Some(a) = e.attrs.iter_mut().find(|a| a.name.eq_ignore_ascii_case("data-page")) {
            a.value = Some(page.to_string());
            return true;
        }
        if name_page_main(&mut e.children, page) {
            return true;
        }
    }
    false
}

pub fn is_head_tag(e: &Element) -> bool {
    e.is("title") || e.is("meta") || e.is("link") || directive::is_json_ld(e)
}

/// Identity of a head tag: a page tag replaces the layout tag with the same key.
fn head_key(e: &Element) -> Option<String> {
    let v = |n: &str| e.value(n).map(|s| s.to_ascii_lowercase());
    if e.is("title") {
        return Some("title".into());
    }
    if e.is("meta") {
        return ["name", "property", "http-equiv"].iter().find_map(|k| v(k).map(|x| format!("meta {k}={x}")));
    }
    if e.is("link") {
        return v("rel").map(|r| format!("link {r} {}", v("hreflang").unwrap_or_default()));
    }
    None
}

fn merge_head(head: &mut Vec<Node>, own: Vec<Element>) {
    for e in own {
        let key = head_key(&e);
        let same = |n: &Node| matches!(n, Node::Element(x) if key.is_some() && head_key(x) == key);
        match head.iter().position(same) {
            Some(i) => {
                head[i] = Node::Element(e);
                let mut j = i + 1;
                while j < head.len() {
                    if same(&head[j]) {
                        head.remove(j);
                    } else {
                        j += 1;
                    }
                }
            }
            None => {
                if !head.last().is_some_and(is_blank) {
                    head.push(Node::Text("\n".into(), e.pos));
                }
                head.push(Node::Element(e));
            }
        }
    }
}

fn merge_class(el: &mut Element, extra: &str) {
    match el.attrs.iter_mut().find(|a| a.name.eq_ignore_ascii_case("class")) {
        Some(a) => {
            let cur = a.value.take().unwrap_or_default();
            a.value = Some(if cur.trim().is_empty() { extra.to_string() } else { format!("{} {extra}", cur.trim_end()) });
        }
        None => el.attrs.push(Attr { name: "class".into(), value: Some(extra.to_string()), pos: el.pos }),
    }
}

/// The script of a page or component: where it is (for messages) and what it declares.
#[derive(Clone, Copy)]
pub struct OwnerScript<'a> {
    /// The `.html` for an embedded `<script>`, else the `.js`.
    pub file: &'a Path,
    pub info: &'a ScriptInfo,
}

/// Page data for scoped validation.
pub struct PageCtx<'a> {
    pub loader: bool,
    pub ssr: bool,
    pub params: Vec<String>,
    pub page_script: Option<OwnerScript<'a>>,
    pub page_file: &'a Path,
    /// Components used with a script: `data-c` id -> (tag for messages, script).
    pub comp_scripts: HashMap<String, (&'a str, OwnerScript<'a>)>,
    /// Every script by module path (stores included: `@click="shared.cart.add(…)"`).
    pub scripts: &'a HashMap<PathBuf, pragma_jslint::script::ScriptInfo>,
}

#[derive(Clone)]
pub enum Owner {
    None,
    Page,
    Comp(String),
}

pub struct Scoped<'a> {
    pub p: &'a Project,
    pub ctx: &'a PageCtx<'a>,
    pub diags: Vec<Diagnostic>,
    pub stores: BTreeSet<String>,
    pub shared_classes: BTreeSet<String>,
    pub page_classes: BTreeSet<String>,
    /// Owner and outer variables of each open component (for `data-slot`).
    pub outer: Vec<(Owner, Vec<String>)>,
}

impl Scoped<'_> {
    fn err(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.diags.push(diag(&self.p.files, pos, rule, msg));
    }

    pub fn run(&mut self, body: &[Node], in_page: bool) {
        self.walk(body, &mut Vec::new(), Owner::None, in_page);
    }

    /// Bindings in the page's head tags are rendered by the server, so they need `ssr`.
    pub fn head(&mut self, head: &[Node]) {
        let mut told = false;
        for n in head {
            let Node::Element(e) = n else { continue };
            if self.p.files[e.pos.file as usize] != self.ctx.page_file {
                continue;
            }
            if directive::is_json_ld(e) {
                let text = match e.children.first() {
                    Some(Node::Text(t, _)) => t.as_str(),
                    _ => "",
                };
                for (_, _, v) in directive::json_ld_paths(text).unwrap_or_default() {
                    if !matches!(directive::root(v), "data" | "params" | "query") || directive::check_path(v, false).is_err() {
                        continue;
                    }
                    if !self.ctx.ssr && !std::mem::replace(&mut told, true) {
                        let f = rel_str(&self.ctx.page_file.with_extension("server.js"));
                        self.err(e.pos, "html/head", format!("JSON-LD with {{path}} is rendered by the server so crawlers see it: add export const ssr = true to {f}"));
                    }
                    self.path(e.pos, v, &[], &Owner::Page, true);
                }
            }
            for a in &e.attrs {
                let lname = a.name.to_ascii_lowercase();
                let Some(Some(d @ (Dir::Text | Dir::Attr(_)))) = directive::parse(&lname) else { continue };
                for v in crate::validate::values(d, a.value.as_deref().unwrap_or("")) {
                    if !matches!(directive::root(&v), "data" | "params" | "query") {
                        continue;
                    }
                    if !self.ctx.ssr && !std::mem::replace(&mut told, true) {
                        let f = rel_str(&self.ctx.page_file.with_extension("server.js"));
                        self.err(a.pos, "html/head", format!("bound head tags are rendered by the server so crawlers see them: add export const ssr = true to {f}"));
                    }
                    self.path(a.pos, &v, &[], &Owner::Page, true);
                }
            }
        }
    }

    fn walk(&mut self, nodes: &[Node], vars: &mut Vec<String>, owner: Owner, in_page: bool) {
        for n in nodes {
            if let Node::Element(e) = n {
                self.element(e, vars, owner.clone(), in_page);
            }
        }
    }

    fn element(&mut self, e: &Element, vars: &mut Vec<String>, mut owner: Owner, mut in_page: bool) {
        let mut lexical;
        let vars = match e.value("data-slot").and_then(|n| n.parse::<usize>().ok()).and_then(|n| self.outer.len().checked_sub(n)) {
            Some(i) => {
                let (o, v) = self.outer[i].clone();
                owner = o;
                lexical = v;
                &mut lexical
            }
            None => vars,
        };
        let is_page_main = e.is("main") && e.has("data-page");
        if is_page_main {
            in_page = true;
            owner = Owner::Page;
        }
        let class_in_page = in_page && !is_page_main;
        let comp = e.value("data-c");
        if let Some(c) = comp {
            self.outer.push((owner.clone(), vars.clone()));
            if self.ctx.comp_scripts.contains_key(c) {
                owner = Owner::Comp(c.to_string());
            }
        }
        let mut pushed = false;
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            if lname == "class" {
                if let Some(v) = &a.value {
                    let target = if class_in_page { &mut self.page_classes } else { &mut self.shared_classes };
                    target.extend(v.split_whitespace().map(str::to_string));
                }
                continue;
            }
            let Some(Some(d)) = directive::parse(&lname) else { continue };
            let v = a.value.as_deref().unwrap_or("");
            match d {
                Dir::Model if !matches!(directive::root(v), "state" | "shared") && directive::check_path(v, false).is_ok() => {
                    self.err(a.pos, "html/invalid-root", format!(":model only accepts $ or shared; \"{v}\""));
                }
                Dir::Model if directive::root(v) == "state" => {
                    self.path(a.pos, v, vars, &owner, in_page);
                    self.model_target(a.pos, v, &owner);
                }
                Dir::Text | Dir::Attr(_) => {
                    for p in crate::validate::values(d, v) {
                        self.path(a.pos, &p, vars, &owner, in_page);
                    }
                }
                Dir::Show | Dir::If | Dir::ElseIf | Dir::Model => self.path(a.pos, v, vars, &owner, in_page),
                Dir::Class(c) => {
                    if class_in_page {
                        self.page_classes.insert(c.to_string());
                    } else {
                        self.shared_classes.insert(c.to_string());
                    }
                    self.path(a.pos, v, vars, &owner, in_page);
                }
                Dir::For => {
                    if let Some((var, path)) = directive::parse_for(v) {
                        self.path(a.pos, path, vars, &owner, in_page);
                        vars.push(var.to_string());
                        pushed = true;
                    }
                }
                Dir::On(_) => {
                    if let Ok(call) = directive::parse_call(v) {
                        if call.is_store() {
                            self.path(a.pos, call.name, vars, &owner, in_page);
                            self.store_function(a.pos, call.name);
                        } else {
                            self.action(a.pos, call.name, &owner);
                        }
                        for p in call.paths() {
                            self.path(a.pos, p, vars, &owner, in_page);
                        }
                    }
                }
                _ => {}
            }
        }
        self.walk(&e.children, vars, owner, in_page);
        if pushed {
            vars.pop();
        }
        if comp.is_some() {
            self.outer.pop();
        }
    }

    /// The script that owns `$x` and `@fn` here, and how to name it in messages. A script with errors
    /// counts as declaring everything (its own errors are reported already).
    fn owner_script(&self, owner: &Owner) -> Option<(String, OwnerScript<'_>)> {
        match owner {
            Owner::None => None,
            Owner::Page => self.ctx.page_script.map(|s| (format!("the script of {}", rel_str(self.ctx.page_file)), s)),
            Owner::Comp(c) => self.ctx.comp_scripts.get(c.as_str()).map(|(tag, s)| (format!("the script of {tag}"), *s)),
        }
    }

    fn no_script(&mut self, pos: Pos, what: &str, owner: &Owner) {
        match owner {
            Owner::None => self.err(pos, "html/unknown-name", format!("{what}: the layout has no script; use a component with a <script>")),
            _ => {
                let f = rel_str(self.ctx.page_file);
                self.err(pos, "html/unknown-name", format!("{what}: {f} has no <script> (owner: the page, or the nearest component with a script; slot content belongs to whoever wrote it)"));
            }
        }
    }

    fn action(&mut self, pos: Pos, name: &str, owner: &Owner) {
        if !directive::is_ident(name) {
            return;
        }
        let Some((whose, script)) = self.owner_script(owner) else {
            return self.no_script(pos, &format!("@… calls \"{name}\""), owner);
        };
        if !script.info.incomplete && !script.info.callable(name) {
            let what = if script.info.declares(name) { "is not a function" } else { "is not declared" };
            let hint = match directive::did_you_mean(name, script.info.callables()) {
                Some(m) => format!("did you mean \"{m}\"? Otherwise write"),
                None => "write".into(),
            };
            self.err(pos, "html/unknown-name", format!("\"{name}\" {what} in {whose}: {hint} `function {name}() {{ … }}` (or `const {name} = …` holding a function) at its top level"));
        }
    }

    /// `@click="shared.cart.add(…)"`: `add` must be a function of the store (its own path is checked by `path`).
    fn store_function(&mut self, pos: Pos, name: &str) {
        let mut parts = name.split('.').skip(1);
        let (Some(store), Some(f)) = (parts.next(), parts.next()) else { return };
        let Some(info) = self.p.stores.get(store).and_then(|m| self.ctx.scripts.get(m)) else { return };
        if info.incomplete || info.callable(f) {
            return;
        }
        let hint = directive::did_you_mean(f, info.callables()).map(|m| format!(" Did you mean shared.{store}.{m}?")).unwrap_or_default();
        self.err(pos, "html/unknown-name", format!("\"{f}\" is not a function of the store {store} (lib/state/{store}.js).{hint}"));
    }

    /// Every name the owner's script declares (state, props, constants, functions).
    fn script_names(&self, owner: &Owner) -> Vec<String> {
        let Some((_, s)) = self.owner_script(owner) else { return Vec::new() };
        [&s.info.lets, &s.info.props, &s.info.consts, &s.info.functions].into_iter().flatten().cloned().collect()
    }

    /// `:model` writes: only a top-level `let`.
    fn model_target(&mut self, pos: Pos, v: &str, owner: &Owner) {
        let name = v.split('.').nth(1).unwrap_or("");
        let Some((_, script)) = self.owner_script(owner) else { return };
        if script.info.props.iter().any(|p| p == name) {
            self.err(pos, "js/prop-write", format!(":model=\"${name}\" writes a prop, and props are read-only; copy it into a `let`"));
        } else if script.info.declares(name) && !script.info.lets.iter().any(|l| l == name) {
            self.err(pos, "html/invalid-root", format!(":model=\"${name}\" needs a top-level `let {name}` (state), not a const or a function"));
        }
    }

    /// `state.x`: `x` must be declared by the owner's script; a const it changes in place is not state.
    fn state_name(&mut self, pos: Pos, name: &str, owner: &Owner) {
        let Some((whose, script)) = self.owner_script(owner) else {
            return self.no_script(pos, &format!("${name}"), owner);
        };
        if script.info.incomplete {
            return;
        }
        if !script.info.declares(name) {
            let names = self.script_names(owner);
            let hint = match directive::did_you_mean(name, names.iter().map(String::as_str)) {
                Some(m) => format!("did you mean ${m}? Otherwise add"),
                None => "add".into(),
            };
            self.err(pos, "html/unknown-name", format!("${name} is not declared in {whose}: {hint} `let {name} = …` (state), `export let {name} = …` (prop) or `function {name}() {{ … }}`"));
        } else if let Some((_, line, col)) = script.info.mutated_consts.iter().find(|(n, _, _)| n == name) {
            let (file, line, col) = (script.file.to_path_buf(), *line, *col);
            self.diags.push(Diagnostic::error(file, line, col, "js/const-state", format!("`{name}` is changed in place and used in HTML, but a `const` is not state: declare it with `let {name}`")));
        }
    }

    fn path(&mut self, pos: Pos, v: &str, vars: &[String], owner: &Owner, in_page: bool) {
        if directive::check_path(v, true).is_err() {
            return;
        }
        let p = directive::split_neg(v).1;
        let segs: Vec<&str> = p.split('.').collect();
        let root = segs[0];
        if vars.iter().any(|x| x == root) {
            return;
        }
        let second = segs.get(1).copied();
        match root {
            "data" => {
                if !in_page {
                    self.err(pos, "html/invalid-root", "data does not exist in the layout (use shared, params or query)");
                } else if !self.ctx.loader {
                    let f = rel_str(&self.ctx.page_file.with_extension("server.js"));
                    self.err(pos, "html/invalid-root", format!("data requires a load in {f}"));
                }
            }
            "state" => self.state_name(pos, second.unwrap_or(""), owner),
            "shared" => match second {
                Some(s) if self.p.stores.contains_key(s) => {
                    self.stores.insert(s.to_string());
                }
                Some(s) => {
                    let hint = directive::did_you_mean(s, self.p.stores.keys().map(String::as_str)).map(|m| format!("did you mean shared.{m}? Otherwise ")).unwrap_or_default();
                    self.err(pos, "html/invalid-root", format!("store \"{s}\" does not exist ({hint}create lib/state/{s}.js)"));
                }
                None => self.err(pos, "html/invalid-path", "use shared.<store>.field"),
            },
            "params" => {
                if in_page
                    && let Some(s) = second.filter(|s| !self.ctx.params.iter().any(|x| x == s)) {
                        let msg = if self.ctx.params.is_empty() { format!("params.{s}: the route has no params") } else { format!("params.{s} does not exist; the route has: {}", self.ctx.params.join(", ")) };
                        self.err(pos, "html/invalid-root", msg);
                    }
            }
            "query" => {}
            "page" => {
                if !matches!(second, Some("loading" | "error")) {
                    self.err(pos, "html/invalid-path", "page only has page.loading and page.error");
                }
            }
            "props" => {}
            "$" => {
                let name = segs.get(1).copied().unwrap_or("");
                self.err(pos, "html/invalid-root", format!("${name} was not passed with :{name} (a text prop has no subfields)"));
            }
            _ => {
                let names = self.script_names(owner);
                let msg = if names.iter().any(|n| n == root) {
                    format!("\"{root}\" is a name of the script: in HTML write ${p} (`{{${p}}}`, `:attr=\"${p}\"`)")
                } else {
                    let known = vars.iter().map(String::as_str).chain(["data", "shared", "params", "query", "page"]);
                    let hint = if let Some(m) = directive::did_you_mean(root, known) {
                        format!("did you mean \"{m}\"? ")
                    } else if let Some(m) = directive::did_you_mean(root, names.iter().map(String::as_str)) {
                        format!("did you mean ${m}? ")
                    } else {
                        String::new()
                    };
                    format!("unknown root \"{root}\" ({hint}use $, data, shared, params, query, page or a <for> variable)")
                };
                self.err(pos, "html/invalid-root", msg)
            }
        }
    }
}
