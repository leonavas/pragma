//! Expansão de componentes (macro, SPEC §3) e validação com escopo sobre a página expandida.
use crate::directive::{self, Dir};
use crate::html::{Attr, Element, Node, Pos, escape_text, is_blank};
use crate::project::{Component, Page, Project, component_tag, is_component_tag, rel_str};
use crate::validate::diag;
use crate::{Fnv, short_hash};
use pragma_diag::Diagnostic;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct Expanded {
    pub title: Option<String>,
    pub head: Vec<Node>,
    pub body: Vec<Node>,
    /// Conteúdo de `<main data-page>` (base das ops de SSR).
    pub page: Vec<Node>,
    pub components: BTreeMap<String, Component>,
}

type Props = HashMap<String, Option<String>>;
type Slots = HashMap<String, Vec<Node>>;

#[derive(Clone)]
struct Scope<'s> {
    from: &'s Path,
    props: Option<&'s Props>,
    aliases: HashMap<String, String>,
    bound_props: HashMap<String, String>,
    stateful: bool,
    known_props: Option<&'s HashSet<String>>,
    slots: Option<&'s Slots>,
    inst: u64,
}

enum Res {
    Path(String),
    /// Prop ausente (`None`), booleana (`Some(None)`) ou com valor.
    Prop(Option<Option<String>>),
}

fn prop_truthy(p: &Option<Option<String>>) -> bool {
    match p {
        None => false,
        Some(None) => true,
        Some(Some(v)) => v != "false",
    }
}

pub struct Expander<'p> {
    p: &'p Project,
    pub diags: Vec<Diagnostic>,
    used: BTreeMap<String, Component>,
    stack: Vec<&'p Path>,
    known_props: HashMap<String, HashSet<String>>,
    stateful: HashSet<PathBuf>,
}

fn collect_props(nodes: &[Node], out: &mut HashMap<String, HashSet<String>>) {
    for node in nodes {
        let Node::Element(element) = node else { continue };
        let tag = element.name.to_ascii_lowercase();
        if let Some((_, name)) = component_tag(&tag) {
            let names = out.entry(name.to_string()).or_default();
            for attr in &element.attrs {
                let lname = attr.name.to_ascii_lowercase();
                if let Some(bound) = lname.strip_prefix("data-with-") {
                    names.insert(bound.to_string());
                } else if !lname.starts_with("data-") && lname != "slot" {
                    names.insert(lname);
                }
            }
        }
        collect_props(&element.children, out);
    }
}

fn first_element(nodes: &[Node]) -> Option<&Element> {
    nodes.iter().find_map(|n| if let Node::Element(e) = n { Some(e) } else { None })
}

fn has_runtime_directive(e: &Element) -> bool {
    let runtime = |a: &Attr| matches!(directive::parse(&a.name.to_ascii_lowercase()), Some(Some(Dir::Text | Dir::Show | Dir::Attr(_) | Dir::Class(_) | Dir::If | Dir::For | Dir::On(_) | Dir::Model)));
    e.attrs.iter().any(runtime) || e.element_children().any(has_runtime_directive)
}

/// Conteúdo de slot pertence a quem o escreveu: `data-slot="n"` = dono n componentes acima.
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
    pub fn new(p: &'p Project) -> Self {
        let mut known_props = HashMap::new();
        for tree in &p.trees {
            collect_props(tree, &mut known_props);
        }
        let stateful = p
            .components
            .values()
            .flat_map(|m| m.values())
            .filter_map(|c| c.module.as_ref())
            .filter(|m| p.read(m).is_some_and(|src| crate::js::exports(&src).iter().any(|(n, _)| n == "state")))
            .cloned()
            .collect();
        Expander { p, diags: Vec::new(), used: BTreeMap::new(), stack: Vec::new(), known_props, stateful }
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
        let mut title = None;
        let mut content: Vec<Node> = Vec::new();
        for n in tree {
            match n {
                Node::Element(e) if e.is("title") && title.is_none() => {
                    let mut s = String::new();
                    crate::html::serialize(&e.children, &mut s);
                    title = Some(s.trim().to_string());
                }
                _ => content.push(n.clone()),
            }
        }
        let sc = Scope { from: &page.dir, props: None, aliases: HashMap::new(), bound_props: HashMap::new(), stateful: true, known_props: None, slots: None, inst: 0 };
        let page_nodes = self.expand_nodes(&content, &sc);
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
            let lsc = Scope { from: Path::new("routes"), props: None, aliases: HashMap::new(), bound_props: HashMap::new(), stateful: true, known_props: None, slots: Some(&slots), inst: 0 };
            body = self.expand_nodes(&rest, &lsc);
        }
        (Expanded { title, head, body, page: page_nodes, components: self.used }, self.diags)
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
            let Some(props) = sc.props else {
                return Res::Path(format!("{bang}state.{rest}"));
            };
            if let Some(target) = sc.bound_props.get(name) {
                return Res::Path(format!("{bang}{target}{tail}"));
            }
            let key = name.to_ascii_lowercase();
            let is_state = sc.stateful && !sc.known_props.is_some_and(|k| k.contains(&key));
            if !tail.is_empty() && (props.contains_key(&key) || !is_state) {
                return Res::Path(v.to_string());
            }
            if let Some(value) = props.get(&key) {
                return Res::Prop(Some(value.clone()));
            }
            if is_state {
                return Res::Path(format!("{bang}state.{rest}"));
            }
            return Res::Prop(None);
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

    fn expand_nodes(&mut self, nodes: &[Node], sc: &Scope) -> Vec<Node> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            match &nodes[i] {
                Node::Comment => {}
                Node::Text(..) => out.push(nodes[i].clone()),
                Node::Element(e) => {
                    let lname = e.name.to_ascii_lowercase();
                    if is_component_tag(&lname) {
                        out.extend(self.component(e, sc));
                    } else if lname == "slot" && sc.slots.is_some() {
                        let name = e.value("name").unwrap_or("");
                        match sc.slots.unwrap().get(name) {
                            Some(v) if v.iter().any(|n| !is_blank(n)) => out.extend(v.iter().cloned()),
                            _ => out.extend(self.expand_nodes(&e.children, sc)),
                        }
                    } else if let Some(Res::Prop(prop)) = (lname == "template").then(|| e.value("data-if").map(|v| self.resolve(v, sc))).flatten() {
                        let neg = e.value("data-if").is_some_and(|v| v.starts_with('!'));
                        let mut j = i + 1;
                        while j < nodes.len() && is_blank(&nodes[j]) {
                            j += 1;
                        }
                        let else_el = match nodes.get(j) {
                            Some(Node::Element(x)) if x.is("template") && x.has("data-else") => Some(x),
                            _ => None,
                        };
                        let chosen = if prop_truthy(&prop) != neg { Some(e) } else { else_el };
                        if let Some(t) = chosen {
                            out.extend(self.expand_nodes(&t.children, sc));
                        }
                        if else_el.is_some() {
                            i = j;
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
            let res = match d {
                Dir::Text | Dir::Show | Dir::Attr(_) | Dir::Class(_) | Dir::If | Dir::Model => self.resolve(v, sc),
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
                        Dir::Text => text = Some(prop.flatten().unwrap_or_default()),
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
        if el.is("template") && ["data-if", "data-else", "data-for", "data-empty"].iter().any(|b| el.has(b)) {
            let tid = self.tid(e.pos, sc.inst);
            el.attrs.push(Attr { name: "data-tid".into(), value: Some(tid), pos: e.pos });
        }
        el.children = match text {
            Some(t) => vec![Node::Text(escape_text(&t), e.pos)],
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
                format!("<{tag}> só existe dentro de routes/; fora dela use <c-{name}> em lib/components")
            } else {
                format!("componente <{tag}> não encontrado (procurado em {})", dirs.join(", "))
            };
            self.err(e.pos, "html/componente-inexistente", msg);
            return Vec::new();
        };
        if self.stack.contains(&comp.dir.as_path()) {
            self.err(e.pos, "html/componente", format!("componente <{tag}> usa a si mesmo (ciclo)"));
            return Vec::new();
        }
        let mut props: Props = HashMap::new();
        let aliases = sc.aliases.clone();
        let mut bound_props = HashMap::new();
        let mut forward = Vec::new();
        let mut class_extra = None;
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            if let Some(alias) = lname.strip_prefix("data-with-") {
                match self.resolve(a.value.as_deref().unwrap_or(""), sc) {
                    Res::Path(p) => {
                        if comp.module.is_some() {
                            if directive::root(&p) == "state" {
                                self.err(a.pos, "html/componente", format!(":{alias}: <{tag}> tem .js e não recebe $ de quem usa; passe data., shared. ou o item do :for, ou use um store"));
                            }
                            forward.push(Attr { name: format!("data-attr-data-{alias}"), value: Some(p.clone()), pos: a.pos });
                        }
                        bound_props.insert(alias.to_string(), p);
                    }
                    Res::Prop(value) => {
                        props.insert(alias.to_string(), value.flatten());
                    }
                }
            } else if lname == "slot" {
            } else if lname.starts_with("data-") {
                forward.push(a.clone());
            } else {
                if lname == "class" {
                    class_extra = a.value.clone();
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
                let what = if k.is_empty() { "filhos (não tem <slot>)".to_string() } else { format!("slot \"{k}\"") };
                self.err(at, "html/componente", format!("<{tag}> não aceita {what}"));
            }
        }
        let Some(root) = tree.iter().find(|n| matches!(n, Node::Element(_))) else { return Vec::new() };
        let mut h = Fnv::new();
        h.write(&sc.inst.to_le_bytes());
        h.write(rel_str(&p.files[e.pos.file as usize]).as_bytes());
        h.write(&e.pos.line.to_le_bytes());
        h.write(&e.pos.col.to_le_bytes());
        let known = self.known_props.get(&name).cloned().unwrap_or_default();
        let inner = Scope { from: &comp.scope, props: Some(&props), aliases, bound_props, stateful: comp.module.as_ref().is_some_and(|m| self.stateful.contains(m)), known_props: Some(&known), slots: Some(&slots), inst: h.finish() };
        self.stack.push(&comp.dir);
        let mut nodes = self.expand_nodes(std::slice::from_ref(root), &inner);
        self.stack.pop();
        match self.used.get(&name) {
            Some(prev) if prev.dir != comp.dir => {
                let msg = format!("dois componentes \"{name}\" na mesma página ({} e {})", rel_str(&prev.dir), rel_str(&comp.dir));
                self.err(e.pos, "html/componente", msg);
            }
            _ => {
                self.used.insert(name.clone(), comp.clone());
            }
        }
        let mut dup = Vec::new();
        if let Some(Node::Element(r)) = nodes.iter_mut().find(|n| matches!(n, Node::Element(_))) {
            r.attrs.insert(0, Attr { name: "data-c".into(), value: Some(name.clone()), pos: r.pos });
            if comp.module.is_some() {
                for a in &e.attrs {
                    let k = a.name.to_ascii_lowercase();
                    let attr = format!("data-{k}");
                    if props.contains_key(&k) && k != "class" && k != "id" && !r.has(&attr) {
                        r.attrs.push(Attr { name: attr, value: Some(a.value.clone().unwrap_or_default()), pos: r.pos });
                    }
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
            self.err(a.pos, "html/componente", format!("{} já existe na raiz de <{tag}>", a.name));
        }
        nodes
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

/// Dados da página para a validação com escopo.
pub struct PageCtx<'a> {
    pub loader: bool,
    pub params: Vec<String>,
    pub page_actions: Option<&'a [String]>,
    pub page_file: &'a Path,
    /// Componentes usados com módulo → actions exportadas.
    pub comp_actions: HashMap<String, &'a [String]>,
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
    pub classes: BTreeSet<String>,
    /// Dono e variáveis de fora de cada componente aberto (para `data-slot`).
    pub outer: Vec<(Owner, Vec<String>)>,
}

impl Scoped<'_> {
    fn err(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.diags.push(diag(&self.p.files, pos, rule, msg));
    }

    pub fn run(&mut self, body: &[Node]) {
        self.walk(body, &mut Vec::new(), Owner::None, false);
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
        if e.is("main") && e.has("data-page") {
            in_page = true;
            owner = Owner::Page;
        }
        let comp = e.value("data-c");
        if let Some(c) = comp {
            self.outer.push((owner.clone(), vars.clone()));
            if self.ctx.comp_actions.contains_key(c) {
                owner = Owner::Comp(c.to_string());
            }
        }
        let mut pushed = false;
        for a in &e.attrs {
            let lname = a.name.to_ascii_lowercase();
            if lname == "class" {
                if let Some(v) = &a.value {
                    self.classes.extend(v.split_whitespace().map(str::to_string));
                }
                continue;
            }
            let Some(Some(d)) = directive::parse(&lname) else { continue };
            let v = a.value.as_deref().unwrap_or("");
            match d {
                Dir::Model if !matches!(directive::root(v), "state" | "shared") && directive::check_path(v, false).is_ok() => {
                    self.err(a.pos, "html/raiz-invalida", format!(":model só aceita $ ou shared; \"{v}\""));
                }
                Dir::Text | Dir::Show | Dir::Attr(_) | Dir::If | Dir::Model => self.path(a.pos, v, vars, &owner, in_page),
                Dir::Class(c) => {
                    self.classes.insert(c.to_string());
                    self.path(a.pos, v, vars, &owner, in_page);
                }
                Dir::For => {
                    if let Some((var, path)) = directive::parse_for(v) {
                        self.path(a.pos, path, vars, &owner, in_page);
                        vars.push(var.to_string());
                        pushed = true;
                    }
                }
                Dir::On(_) => self.action(a.pos, v, &owner),
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

    fn action(&mut self, pos: Pos, name: &str, owner: &Owner) {
        if !directive::is_ident(name) {
            return;
        }
        let page_js = rel_str(&self.ctx.page_file.with_extension("js"));
        match owner {
            Owner::None => self.err(pos, "html/acao-inexistente", format!("action \"{name}\" sem dono: o layout não tem módulo; use um componente com .js")),
            Owner::Page => match self.ctx.page_actions {
                None => self.err(pos, "html/acao-inexistente", format!("action \"{name}\": a página não tem módulo ({page_js})")),
                Some(acts) if !acts.iter().any(|a| a == name) => self.err(pos, "html/acao-inexistente", format!("action \"{name}\" não existe em actions de {page_js}")),
                _ => {}
            },
            Owner::Comp(c) => {
                let acts = self.ctx.comp_actions.get(c.as_str()).copied().unwrap_or(&[]);
                if !acts.iter().any(|a| a == name) {
                    self.err(pos, "html/acao-inexistente", format!("action \"{name}\" não existe em actions do componente {c} (dono: componente mais próximo com módulo; conteúdo de slot pertence a quem o escreveu)"));
                }
            }
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
                    self.err(pos, "html/raiz-invalida", "data não existe no layout (use shared, params ou query)");
                } else if !self.ctx.loader {
                    let f = rel_str(&self.ctx.page_file.with_extension("server.js"));
                    self.err(pos, "html/raiz-invalida", format!("data exige um load em {f}"));
                }
            }
            "state" => match owner {
                Owner::None => self.err(pos, "html/raiz-invalida", "$ não existe no layout (o layout não tem módulo)"),
                Owner::Page if self.ctx.page_actions.is_none() => {
                    let f = rel_str(&self.ctx.page_file.with_extension("js"));
                    self.err(pos, "html/raiz-invalida", format!("$ exige o módulo {f} com export const state"));
                }
                _ => {}
            },
            "shared" => match second {
                Some(s) if self.p.stores.contains_key(s) => {
                    self.stores.insert(s.to_string());
                }
                Some(s) => self.err(pos, "html/raiz-invalida", format!("store \"{s}\" não existe (crie lib/state/{s}.js)")),
                None => self.err(pos, "html/caminho-invalido", "use shared.<store>.campo"),
            },
            "params" => {
                if in_page
                    && let Some(s) = second.filter(|s| !self.ctx.params.iter().any(|x| x == s)) {
                        let msg = if self.ctx.params.is_empty() { format!("params.{s}: a rota não tem parâmetros") } else { format!("params.{s} não existe; a rota tem: {}", self.ctx.params.join(", ")) };
                        self.err(pos, "html/raiz-invalida", msg);
                    }
            }
            "query" => {}
            "page" => {
                if !matches!(second, Some("loading" | "error")) {
                    self.err(pos, "html/caminho-invalido", "page só tem page.loading e page.error");
                }
            }
            "props" => {}
            "$" => {
                let name = segs.get(1).copied().unwrap_or("");
                self.err(pos, "html/raiz-invalida", format!("${name} não foi passado com :{name} (prop de texto não tem subcampo)"));
            }
            _ => self.err(pos, "html/raiz-invalida", format!("raiz desconhecida \"{root}\" (use $, data, shared, params, query, page ou a variável de um :for)")),
        }
    }
}
