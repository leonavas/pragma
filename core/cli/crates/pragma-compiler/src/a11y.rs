//! Avisos de acessibilidade sobre a página expandida (layout + página + componentes).
use crate::html::{Element, Node, Pos};
use pragma_diag::Diagnostic;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

const INTERACTIVE: &[&str] = &["a", "button", "input", "select", "textarea", "summary", "option", "label", "details"];
const NO_LABEL: &[&str] = &["hidden", "submit", "reset", "button", "image"];

struct A11y<'a> {
    files: &'a [PathBuf],
    out: Vec<Diagnostic>,
    label_for: HashSet<String>,
    ids: HashMap<String, Pos>,
    heading: Option<u8>,
    h1: bool,
}

/// Atributo estático ou ligado em runtime por `data-attr-<nome>`.
fn has(e: &Element, name: &str) -> bool {
    e.has(name) || e.has(&format!("data-attr-{name}"))
}

fn named(e: &Element) -> bool {
    has(e, "aria-label") || has(e, "aria-labelledby") || has(e, "title")
}

fn has_text(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| match n {
        Node::Text(t, _) => !t.trim().is_empty(),
        Node::Element(e) => {
            e.has("data-text") || (e.is("img") && e.value("alt").is_some_and(|a| !a.trim().is_empty())) || (!e.has("aria-hidden") && (named(e) && e.is("svg") || has_text(&e.children)))
        }
        Node::Comment => false,
    })
}

fn collect_labels(nodes: &[Node], out: &mut HashSet<String>) {
    for n in nodes {
        if let Node::Element(e) = n {
            if e.is("label") && let Some(f) = e.value("for") {
                out.insert(f.to_string());
            }
            collect_labels(&e.children, out);
        }
    }
}

impl A11y<'_> {
    fn warn(&mut self, pos: Pos, rule: &'static str, msg: impl Into<String>) {
        self.out.push(Diagnostic::warning(self.files[pos.file as usize].clone(), pos.line, pos.col, rule, msg));
    }

    fn nodes(&mut self, nodes: &[Node], in_label: bool) {
        // data-else/data-empty excluem o bloco anterior: cada ramo conta a partir do mesmo ponto.
        let mut before: Option<(bool, Option<u8>)> = None;
        for n in nodes {
            let Node::Element(e) = n else { continue };
            let other_branch = e.is("template") && (e.has("data-else") || e.has("data-empty"));
            let mut after_first = None;
            if let (true, Some((h1, heading))) = (other_branch, before) {
                after_first = Some(self.h1);
                (self.h1, self.heading) = (h1, heading);
            }
            before = (e.is("template") && (e.has("data-if") || e.has("data-for"))).then_some((self.h1, self.heading));
            self.element(e, in_label);
            if let Some(h1) = after_first {
                self.h1 |= h1;
            }
        }
    }

    fn element(&mut self, e: &Element, in_label: bool) {
        let name = e.name.to_ascii_lowercase();
        match name.as_str() {
            "img" if !has(e, "alt") => self.warn(e.pos, "a11y/img-alt", "<img> sem alt; use alt=\"\" se a imagem for decorativa"),
            "a" if !has(e, "href") => self.warn(e.pos, "a11y/a-sem-href", "<a> sem href; para ação use <button>"),
            "dialog" if !has(e, "aria-label") && !has(e, "aria-labelledby") => self.warn(e.pos, "a11y/dialog-nome", "<dialog> sem nome: use aria-label ou aria-labelledby"),
            "iframe" if !has(e, "title") => self.warn(e.pos, "a11y/iframe-title", "<iframe> sem title"),
            "input" | "select" | "textarea" => {
                let kind = e.value("type").unwrap_or("").to_ascii_lowercase();
                let by_id = e.value("id").is_some_and(|id| self.label_for.contains(id)) || e.has("data-attr-id");
                if !NO_LABEL.contains(&kind.as_str()) && !in_label && !by_id && !has(e, "aria-label") && !has(e, "aria-labelledby") {
                    self.warn(e.pos, "a11y/label", format!("<{name}> sem label: use <label for>, envolva num <label> ou dê aria-label"));
                }
            }
            _ => {}
        }
        if (name == "a" || name == "button") && !named(e) && !e.has("data-text") && !has_text(&e.children) {
            self.warn(e.pos, "a11y/nome-acessivel", format!("<{name}> sem nome acessível: dê um texto, data-text ou aria-label"));
        }
        if e.has("data-on-click") && !INTERACTIVE.contains(&name.as_str()) && !(e.has("role") && has(e, "tabindex")) {
            self.warn(e.pos, "a11y/clique-nao-interativo", format!("data-on-click em <{name}>: use <button>, ou dê role e tabindex"));
        }
        if e.value("tabindex").and_then(|t| t.trim().parse::<i32>().ok()).is_some_and(|t| t > 0) {
            self.warn(e.pos, "a11y/tabindex", "tabindex maior que 0 bagunça a ordem do foco; use 0 ou -1");
        }
        if let Some(id) = e.value("id") {
            match self.ids.get(id) {
                Some(_) => self.warn(e.pos, "a11y/id-duplicado", format!("id \"{id}\" repetido na página")),
                None => {
                    self.ids.insert(id.to_string(), e.pos);
                }
            }
        }
        if let Some(level) = name.strip_prefix('h').and_then(|l| l.parse::<u8>().ok()).filter(|l| (1..=6).contains(l)) {
            if level == 1 && std::mem::replace(&mut self.h1, true) {
                self.warn(e.pos, "a11y/h1-unico", "mais de um <h1> na página");
            }
            if let Some(prev) = self.heading.filter(|p| level > p + 1) {
                self.warn(e.pos, "a11y/heading", format!("nível de heading pulado: <h{prev}> seguido de <h{level}>"));
            }
            self.heading = Some(level);
        }
        self.nodes(&e.children, in_label || name == "label");
    }
}

pub fn check(files: &[PathBuf], body: &[Node]) -> Vec<Diagnostic> {
    let mut label_for = HashSet::new();
    collect_labels(body, &mut label_for);
    let mut a = A11y { files, out: Vec::new(), label_for, ids: HashMap::new(), heading: None, h1: false };
    a.nodes(body, false);
    a.out
}
