//! Accessibility warnings on the expanded page (layout + page + components).
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

/// Static attribute or bound at runtime by `data-attr-<name>`.
pub(crate) fn has(e: &Element, name: &str) -> bool {
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

pub(crate) fn collect_labels(nodes: &[Node], out: &mut HashSet<String>) {
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
        let mut before: Option<(bool, Option<u8>)> = None;
        let mut seen_h1 = false;
        for n in nodes {
            let Node::Element(e) = n else { continue };
            let tpl = e.is("template");
            if tpl && e.has("data-def") {
                continue;
            }
            let other_branch = tpl && (e.has("data-else-if") || e.has("data-else") || e.has("data-empty"));
            match before {
                Some((h1, heading)) if other_branch => {
                    seen_h1 |= self.h1;
                    (self.h1, self.heading) = (h1, heading);
                }
                _ => {
                    self.h1 |= seen_h1;
                    seen_h1 = false;
                    before = None;
                }
            }
            if tpl && (e.has("data-if") || e.has("data-for")) {
                before = Some((self.h1, self.heading));
            }
            self.element(e, in_label);
        }
        self.h1 |= seen_h1;
    }

    fn element(&mut self, e: &Element, in_label: bool) {
        let name = e.name.to_ascii_lowercase();
        match name.as_str() {
            "img" if !has(e, "alt") => self.warn(e.pos, "a11y/img-alt", "<img> without alt; use alt=\"\" if the image is decorative"),
            "a" if !has(e, "href") => self.warn(e.pos, "a11y/a-no-href", "<a> without href; use <button> for actions"),
            "dialog" if !has(e, "aria-label") && !has(e, "aria-labelledby") => self.warn(e.pos, "a11y/dialog-name", "<dialog> without a name: use aria-label or aria-labelledby"),
            "iframe" if !has(e, "title") => self.warn(e.pos, "a11y/iframe-title", "<iframe> without title"),
            "input" | "select" | "textarea" => {
                let kind = e.value("type").unwrap_or("").to_ascii_lowercase();
                let by_id = e.value("id").is_some_and(|id| self.label_for.contains(id)) || e.has("data-attr-id");
                if !NO_LABEL.contains(&kind.as_str()) && !in_label && !by_id && !has(e, "aria-label") && !has(e, "aria-labelledby") {
                    self.warn(e.pos, "a11y/label", format!("<{name}> without label: use <label for>, wrap it in a <label> or add aria-label"));
                }
            }
            _ => {}
        }
        if (name == "a" || name == "button") && !named(e) && !e.has("data-text") && !has_text(&e.children) {
            self.warn(e.pos, "a11y/accessible-name", format!("<{name}> without an accessible name: add text, {{path}} or aria-label"));
        }
        if e.has("data-on-click") && !INTERACTIVE.contains(&name.as_str()) && !(e.has("role") && has(e, "tabindex")) {
            self.warn(e.pos, "a11y/click-non-interactive", format!("@click on <{name}>: use <button>, or add role and tabindex"));
        }
        if e.value("tabindex").and_then(|t| t.trim().parse::<i32>().ok()).is_some_and(|t| t > 0) {
            self.warn(e.pos, "a11y/tabindex", "tabindex greater than 0 breaks focus order; use 0 or -1");
        }
        if let Some(id) = e.value("id") {
            match self.ids.get(id) {
                Some(_) => self.warn(e.pos, "a11y/duplicate-id", format!("id \"{id}\" repeated on the page")),
                None => {
                    self.ids.insert(id.to_string(), e.pos);
                }
            }
        }
        if let Some(level) = name.strip_prefix('h').and_then(|l| l.parse::<u8>().ok()).filter(|l| (1..=6).contains(l)) {
            if level == 1 && std::mem::replace(&mut self.h1, true) {
                self.warn(e.pos, "a11y/single-h1", "more than one <h1> on the page");
            }
            if let Some(prev) = self.heading.filter(|p| level > p + 1) {
                self.warn(e.pos, "a11y/heading", format!("skipped heading level: <h{prev}> followed by <h{level}>"));
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
