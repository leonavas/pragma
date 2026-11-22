//! Drops the `data-s` tokens that no scoped selector of their owner can use, once the page is whole.
use crate::directive::{self, Dir};
use crate::html::{Element, Node};
use crate::validate::BLOCKS;
use pragma_css::prune::{Ancestor, Markup, OwnSelector};
use std::collections::{BTreeSet, HashMap};

pub type Owned = HashMap<String, BTreeSet<OwnSelector>>;

/// Every class an element can have: static ones and `.x` toggles.
pub fn classes(el: &Element) -> Vec<String> {
    let mut out: Vec<String> = el.attrs.iter().filter(|a| a.name.eq_ignore_ascii_case("class")).flat_map(|a| a.value.as_deref().unwrap_or("").split_whitespace().map(str::to_string)).collect();
    out.extend(el.attrs.iter().filter_map(|a| match directive::parse(&a.name.to_ascii_lowercase()) {
        Some(Some(Dir::Class(n))) => Some(n.to_string()),
        _ => None,
    }));
    out
}

/// An element an owner writes: its name and every class it can have, or `None` when unknown.
pub type Written = Option<(String, Vec<String>)>;

/// The root of a component body as its caller sees it, `None` when it is not a plain element.
pub fn root_of(tree: &[Node]) -> Written {
    let e = tree.iter().find_map(|n| if let Node::Element(e) = n { Some(e) } else { None })?;
    let plain = !crate::project::is_component_tag(&e.name) && !e.is("slot") && !e.is("template");
    plain.then(|| (e.name.to_ascii_lowercase(), classes(e)))
}

fn written(nodes: &[Node], root: &dyn Fn(&str) -> Written, out: &mut Vec<Written>) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if crate::project::is_component_tag(&e.name) {
            if e.has("class") {
                out.push(root(&e.name).map(|(tag, mut cls)| {
                    cls.extend(classes(e));
                    (tag, cls)
                }));
            }
        } else if !e.is("slot") && !(e.is("template") && BLOCKS.iter().any(|b| e.has(b))) {
            out.push(Some((e.name.to_ascii_lowercase(), classes(e))));
        }
        written(&e.children, root, out);
    }
}

/// Scoped selectors of one owner whose subject matches no element the owner writes, as
/// `(offset in css, selector, the same with its subject in :global)`. Slot content and child
/// component markup belong to their writer, so a component selector for them never matches.
/// `root` gives the root of a child component tag (it also belongs to a caller that gives it a
/// `class`).
pub fn unmatched(css: &str, marker: &str, tree: &[Node], root: &dyn Fn(&str) -> Written) -> Vec<(usize, String, String)> {
    let mut own = Vec::new();
    written(tree, root, &mut own);
    pragma_css::prune::scoped_subjects(css, marker)
        .into_iter()
        .filter(|s| !s.own.tag.as_deref().is_some_and(|t| ["html", "body", "head"].contains(&t)))
        .filter(|s| !own.iter().any(|w| w.as_ref().is_none_or(|(tag, cls)| s.own.matches(tag, &|c| cls.iter().any(|x| x == c)))))
        .map(|s| (s.offset, s.selector, s.global))
        .collect()
}

/// `body` is the whole document (empty without a layout) and `page` the content of its
/// `<main data-page>`: both keep a token only where one of its owner's selectors can match.
pub fn trim(body: &mut [Node], page: &mut [Node], owned: &Owned) {
    let mut markup = Markup::default();
    crate::markup_of(if body.is_empty() { page } else { body }, &mut markup);
    let outer = if body.is_empty() { Vec::new() } else { above_page(body, &mut Vec::new()).unwrap_or_else(|| vec![Ancestor::Unknown]) };
    walk(body, &mut Vec::new(), owned, &markup);
    walk(page, &mut outer.clone(), owned, &markup);
}

fn above_page(nodes: &[Node], stack: &mut Vec<Ancestor>) -> Option<Vec<Ancestor>> {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        let pushed = push(e, stack);
        let found = if e.has("data-page") { Some(stack.clone()) } else { above_page(&e.children, stack) };
        stack.truncate(stack.len() - pushed);
        if found.is_some() {
            return found;
        }
    }
    None
}

fn push(e: &Element, stack: &mut Vec<Ancestor>) -> usize {
    if e.is("template") && BLOCKS.iter().any(|b| e.has(b)) {
        return 0;
    }
    stack.push(if e.is("template") { Ancestor::Unknown } else { Ancestor::Element { tag: e.name.clone(), classes: classes(e) } });
    1
}

fn walk(nodes: &mut [Node], stack: &mut Vec<Ancestor>, owned: &Owned, markup: &Markup) {
    for n in nodes {
        let Node::Element(e) = n else { continue };
        if let Some(at) = e.attrs.iter().position(|a| a.name == "data-s") {
            let own = classes(e);
            let tokens: Vec<String> = e.attrs[at].value.as_deref().unwrap_or("").split_whitespace().filter(|t| owned.get(*t).is_none_or(|sels| sels.iter().any(|s| s.may_match(&e.name, &own, stack, markup)))).map(str::to_string).collect();
            if tokens.is_empty() {
                e.attrs.remove(at);
            } else {
                e.attrs[at].value = Some(tokens.join(" "));
            }
        }
        let pushed = push(e, stack);
        walk(&mut e.children, stack, owned, markup);
        stack.truncate(stack.len() - pushed);
    }
}
