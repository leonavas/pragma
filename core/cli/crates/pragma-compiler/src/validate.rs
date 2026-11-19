//! Validação por arquivo (SPEC §3, §10): o que não depende de escopo nem de expansão.
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
            out.push(diag(files, pos, "css/valor-arbitrario", format!("valor arbitrário \"{class}\" não é suportado; use um utilitário da lista (core/docs/classes.md) ou crie uma classe em lib/theme.css")));
        } else {
            out.push(diag(files, pos, "css/classe-desconhecida", format!("classe \"{class}\" não é utilitário suportado nem está definida no CSS do projeto")));
        }
    }
}

/// Classes definidas em CSS (`.nome` fora de comentários, strings e `url()`).
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

const BLOCKS: &[&str] = &["data-if", "data-else", "data-for", "data-empty"];

fn template_kind(e: &Element) -> Option<&'static str> {
    if !e.is("template") {
        return None;
    }
    BLOCKS.iter().copied().find(|b| e.has(b))
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
                    self.err(text.or(roots.get(1).map(|e| e.pos)).unwrap_or(file_pos), "html/componente", "componente precisa de exatamente 1 elemento raiz");
                }
                if let Some(r) = roots.first()
                    && (r.is("template") || r.is("slot") || is_component_tag(&r.name)) {
                        self.err(r.pos, "html/componente", "a raiz do componente não pode ser <template>, <slot> nem outro componente");
                    }
            }
            Kind::Layout => self.layout(tree, file_pos),
            Kind::Page => {}
        }
        self.nodes(tree, None, true);
    }

    fn layout(&mut self, tree: &[Node], file_pos: Pos) {
        let mut mains = Vec::new();
        find_mains(tree, &mut mains);
        match mains.as_slice() {
            [] => self.err(file_pos, "html/layout", "o layout precisa de <main data-page><slot></slot></main>"),
            [main] => {
                let kids: Vec<&Node> = main.children.iter().filter(|n| !is_blank(n)).collect();
                let ok = matches!(kids.as_slice(), [Node::Element(s)] if s.is("slot") && !s.has("name"));
                if !ok {
                    self.err(main.pos, "html/layout", "<main data-page> deve conter só <slot></slot>");
                }
            }
            [_, second, ..] => self.err(second.pos, "html/layout", "o layout tem mais de um <main data-page>"),
        }
        let mut first = true;
        for n in tree {
            if is_blank(n) {
                continue;
            }
            if let Node::Element(e) = n
                && e.is("head") && !first {
                    self.err(e.pos, "html/layout", "o bloco <head> deve vir no início do _layout.html");
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
                n if !is_blank(n) => prev = None,
                _ => {}
            }
        }
    }

    fn element(&mut self, e: &Element, parent: Option<&Element>, prev: Option<&Element>, top: bool) {
        let name = e.name.to_ascii_lowercase();
        let in_head = parent.is_some_and(|p| p.is("head"));
        match name.as_str() {
            "script" => self.err(e.pos, "html/script", "<script> é proibido em routes/ e lib/; o comportamento vai no .js da página ou do componente"),
            "style" => self.err(e.pos, "html/style-inline", "<style> é proibido; use classes utilitárias, lib/theme.css ou o .css do componente"),
            "html" | "body" => self.err(e.pos, "html/layout", format!("<{name}> é gerado pelo build; não escreva no fragmento")),
            "head" if !(self.kind == Kind::Layout && top) => self.err(e.pos, "html/layout", "<head> só no início do _layout.html"),
            "title" if !(top && self.kind == Kind::Page) && !in_head => self.err(e.pos, "html/title", "<title> só no topo do fragmento da página"),
            "slot" if self.kind == Kind::Page => self.err(e.pos, "html/componente", "<slot> só existe em componentes e no layout"),
            "main" if self.kind == Kind::Layout && e.has("data-page") => {}
            _ => {}
        }
        if let Some((_, c)) = component_tag(&name)
            && !is_component_name(c) {
                self.err(e.pos, "html/componente-inexistente", format!("nome de componente inválido <{name}>"));
            }
        let for_var = e.value("data-for").and_then(|v| directive::parse_for(v)).map(|(v, _)| v.to_string());
        for a in &e.attrs {
            let an = a.name.to_ascii_lowercase();
            let v = a.value.as_deref().unwrap_or("");
            if an == "style" {
                self.err(a.pos, "html/style-inline", "atributo style é proibido; use classes");
                continue;
            }
            if an.starts_with("on") {
                self.err(a.pos, "html/on-inline", format!("{an} inline é proibido; use @<evento>=\"acao\""));
                continue;
            }
            if an == "class" {
                for c in v.split_whitespace() {
                    self.classes.check(self.files, a.pos, c, &mut self.diags);
                }
                continue;
            }
            if an == "slot" && !parent.is_some_and(|p| is_component_tag(&p.name)) {
                self.err(a.pos, "html/componente", "atributo slot só em filhos diretos de <c-…> ou <p-…>");
                continue;
            }
            let Some(d) = directive::parse(&an) else { continue };
            let Some(d) = d else {
                self.err(a.pos, "html/diretiva-desconhecida", format!("diretiva desconhecida {an} (veja a lista fechada em core/docs)"));
                continue;
            };
            self.directive(e, &name, a.pos, d, v, for_var.as_deref());
        }
        if e.is("template") {
            let blocks: Vec<&str> = BLOCKS.iter().copied().filter(|b| e.has(b)).collect();
            if blocks.is_empty() {
                self.err(e.pos, "html/fora-de-template", "<template> precisa de :if, :else, :for ou :empty");
            } else if blocks.len() > 1 {
                self.err(e.pos, "html/fora-de-template", format!("<template> com mais de um bloco ({})", blocks.join(", ")));
            }
            let roots = e.element_children().count();
            let text = stray_text(&e.children);
            if !blocks.is_empty() && (roots != 1 || text.is_some()) {
                self.err(text.unwrap_or(e.pos), "html/bloco-sem-raiz-unica", "o conteúdo do <template> precisa ter exatamente 1 elemento raiz");
            }
            if let Some(r) = e.element_children().next()
                && r.is("template") {
                    self.err(r.pos, "html/bloco-sem-raiz-unica", "a raiz de um bloco não pode ser outro <template>; envolva-o num elemento");
                }
            match blocks.first().copied() {
                Some("data-for") if !e.has("data-key") => self.err(e.pos, "html/for-sem-key", ":for exige :key=\"x.campo\""),
                Some("data-else") if !prev.is_some_and(|p| template_kind(p) == Some("data-if")) => {
                    self.err(e.pos, "html/else-fora-de-lugar", ":else deve vir imediatamente após um <template :if>")
                }
                Some("data-empty") if !prev.is_some_and(|p| template_kind(p) == Some("data-for")) => {
                    self.err(e.pos, "html/empty-fora-de-lugar", ":empty deve vir imediatamente após um <template :for>")
                }
                _ => {}
            }
        }
        self.nodes(&e.children, Some(e), false);
    }

    fn path(&mut self, pos: Pos, v: &str, neg_ok: bool, props_ok: bool) {
        if let Err(m) = directive::check_path(v, neg_ok) {
            self.err(pos, "html/caminho-invalido", m);
            return;
        }
        if directive::root(v) == "props" {
            let p = directive::split_neg(v).1;
            if !props_ok {
                self.err(pos, "html/raiz-invalida", "props é resolvido no build e não vale nesta diretiva");
            } else if self.kind != Kind::Component {
                self.err(pos, "html/raiz-invalida", "props só existe dentro de componentes");
            } else if p.split('.').count() != 2 {
                self.err(pos, "html/caminho-invalido", "props é texto: use props.nome, sem subcampos");
            }
        }
    }

    fn directive(&mut self, e: &Element, name: &str, pos: Pos, d: Dir, v: &str, for_var: Option<&str>) {
        let is_tpl = name == "template";
        let on_tpl = |d: Dir| matches!(d, Dir::If | Dir::Else | Dir::For | Dir::Empty | Dir::Key);
        if on_tpl(d) && !is_tpl {
            self.err(pos, "html/fora-de-template", ":if, :else, :for, :empty e :key só em <template>");
            return;
        }
        if !on_tpl(d) && is_tpl {
            self.err(pos, "html/fora-de-template", "<template> só aceita :if, :else, :for, :empty e :key");
            return;
        }
        match d {
            Dir::Text => self.path(pos, v, false, true),
            Dir::Show | Dir::If => self.path(pos, v, true, true),
            Dir::Class(c) => {
                self.path(pos, v, true, true);
                self.classes.check(self.files, pos, c, &mut self.diags);
            }
            Dir::Attr(n) => {
                if n.starts_with("on") || n == "style" {
                    self.err(pos, "html/on-inline", format!(":{n} é proibido"));
                }
                self.path(pos, v, false, true);
            }
            Dir::Model => {
                if !matches!(name, "input" | "select" | "textarea") {
                    self.err(pos, "html/diretiva-desconhecida", ":model só em input, select e textarea");
                }
                self.path(pos, v, false, false);
                let alias = self.kind == Kind::Component && !directive::ROOTS.contains(&directive::root(v));
                if !alias && !matches!(directive::root(v), "state" | "shared" | "props" | "$") && directive::check_path(v, false).is_ok() {
                    self.err(pos, "html/raiz-invalida", ":model só aceita $ ou shared");
                }
            }
            Dir::Else | Dir::Empty => {
                if !v.is_empty() {
                    self.err(pos, "html/caminho-invalido", ":else e :empty não têm valor");
                }
            }
            Dir::For => match directive::parse_for(v) {
                None => self.err(pos, "html/caminho-invalido", "use :for=\"x in caminho\""),
                Some((var, p)) => {
                    if directive::ROOTS.contains(&var) {
                        self.err(pos, "html/raiz-invalida", format!("variável \"{var}\" do :for não pode ter nome de raiz"));
                    }
                    self.path(pos, p, false, false);
                }
            },
            Dir::Key => {
                if !e.has("data-for") {
                    self.err(pos, "html/for-sem-key", ":key só junto de :for");
                } else if let Err(m) = directive::check_path(v, false) {
                    self.err(pos, "html/caminho-invalido", m);
                } else if let Some(var) = for_var
                    && directive::root(v) != var {
                        self.err(pos, "html/raiz-invalida", format!(":key deve partir da variável do :for ({var}.campo)"));
                    }
            }
            Dir::On(ev) => {
                if !ev.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
                    self.err(pos, "html/diretiva-desconhecida", format!("evento inválido \"{ev}\""));
                }
                if !directive::is_ident(v) {
                    self.err(pos, "html/acao-inexistente", format!("@{ev} espera o nome de uma action, não \"{v}\""));
                }
            }
            Dir::With(alias) => {
                if !is_component_tag(name) {
                    self.err(pos, "html/componente", "prop dinâmica (:nome) só em <c-…> ou <p-…>");
                } else if !directive::is_ident(alias) || directive::ROOTS.contains(&alias) {
                    self.err(pos, "html/componente", format!("alias inválido \"{alias}\""));
                }
                self.path(pos, v, false, false);
            }
            Dir::Open => {
                if v.is_empty() {
                    self.err(pos, "html/diretiva-desconhecida", "@open precisa do id do overlay");
                }
            }
            Dir::Close => {}
            Dir::Marker => {
                if e.attrs.iter().any(|a| a.pos == pos && a.name.eq_ignore_ascii_case("data-page")) && !(self.kind == Kind::Layout && name == "main") {
                    self.err(pos, "html/layout", "data-page só no <main> do _layout.html");
                }
            }
            Dir::Generated => self.err(pos, "html/diretiva-desconhecida", "data-c, data-tid, data-of e data-slot são gerados pelo build"),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_class_scan() {
        let mut s = HashSet::new();
        css_classes("/* .no */ .btn, a.link:hover { margin: .5rem; background: url(x.y.png) } @media (x) { .md\\:a > .b-c {} } [data-c=\"card\"] .titulo{}", &mut s);
        let mut v: Vec<_> = s.into_iter().collect();
        v.sort();
        assert_eq!(v, ["b-c", "btn", "link", "md:a", "titulo"]);
    }
}
