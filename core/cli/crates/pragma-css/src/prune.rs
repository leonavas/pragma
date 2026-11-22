//! Route-level CSS trimming: theme rules that cannot match a route's markup, and the fonts it uses.

use crate::scope::{NESTING, block_end, block_end_with, compounds, find_top, split_top};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Element names and classes a route's markup can have (static classes and `.x` toggles).
#[derive(Debug, Default, Clone)]
pub struct Markup {
    pub tags: HashSet<String>,
    pub classes: HashSet<String>,
}

const ALWAYS_PRESENT: &[&str] = &["html", "body", "head"];

enum Item<'a> {
    Text(&'a str),
    Statement(&'a str),
    Block { prelude: &'a str, body: &'a str },
}

fn items(css: &str) -> Vec<Item<'_>> {
    let mut out = Vec::new();
    let mut rest = css;
    while !rest.is_empty() {
        let ws = rest.len() - rest.trim_start().len();
        if ws > 0 {
            out.push(Item::Text(&rest[..ws]));
            rest = &rest[ws..];
            continue;
        }
        let Some(open) = find_top(rest, &['{', ';']) else {
            out.push(Item::Text(rest));
            break;
        };
        if rest.as_bytes()[open] == b';' {
            out.push(Item::Statement(&rest[..=open]));
            rest = &rest[open + 1..];
            continue;
        }
        let close = block_end(rest, open);
        out.push(Item::Block { prelude: &rest[..open], body: &rest[open + 1..close.min(rest.len())] });
        rest = &rest[(close + 1).min(rest.len())..];
    }
    out
}

fn at_name(prelude: &str) -> Option<String> {
    let at = prelude.trim_start().strip_prefix('@')?;
    Some(at.split(|c: char| c.is_whitespace() || c == '{' || c == '(').next().unwrap_or("").to_ascii_lowercase())
}

fn at_param(prelude: &str) -> &str {
    let t = prelude.trim_start().trim_start_matches('@');
    t.find(char::is_whitespace).map_or("", |i| t[i..].trim())
}

/// Keeps every block for which `keep(prelude, body)` is true; style rules inside `@media`,
/// `@supports`, `@layer`… are filtered one by one and an emptied group is dropped.
fn filter(css: &str, keep: &mut dyn FnMut(&str, &str) -> bool) -> String {
    let mut out = String::with_capacity(css.len());
    for item in items(css) {
        match item {
            Item::Text(t) | Item::Statement(t) => out.push_str(t),
            Item::Block { prelude, body } => {
                if at_name(prelude).is_some_and(|n| NESTING.contains(&n.as_str())) {
                    let inner = filter(body, keep);
                    if !inner.trim().is_empty() {
                        out.push_str(prelude);
                        out.push('{');
                        out.push_str(&inner);
                        out.push('}');
                    }
                } else if keep(prelude, body) {
                    out.push_str(prelude);
                    out.push('{');
                    out.push_str(body);
                    out.push('}');
                }
            }
        }
    }
    out
}

/// Calls `f(selector_list, declarations)` for each style rule, inside grouping at-rules too.
fn each_rule(css: &str, f: &mut dyn FnMut(&str, &str)) {
    for item in items(css) {
        if let Item::Block { prelude, body } = item {
            match at_name(prelude) {
                Some(n) if NESTING.contains(&n.as_str()) => each_rule(body, f),
                Some(_) => {}
                None => f(prelude, body),
            }
        }
    }
}

/// Top-level declarations of a rule body (nested rules are skipped).
fn declarations(body: &str) -> Vec<(String, &str)> {
    let mut out = Vec::new();
    let mut rest = body;
    while !rest.trim().is_empty() {
        let Some(end) = find_top(rest, &[';', '{']) else {
            push_decl(rest, &mut out);
            break;
        };
        if rest.as_bytes()[end] == b'{' {
            let close = block_end(rest, end);
            rest = &rest[(close + 1).min(rest.len())..];
            continue;
        }
        push_decl(&rest[..end], &mut out);
        rest = &rest[end + 1..];
    }
    out
}

fn push_decl<'a>(decl: &'a str, out: &mut Vec<(String, &'a str)>) {
    if let Some((name, value)) = decl.split_once(':') {
        out.push((name.trim().to_ascii_lowercase(), value.trim()));
    }
}

/// Whether a selector (one item of a list) can match an element of `markup`: each compound outside
/// `:is()`, `:not()`, `[…]` and the like needs its element name and all its classes in the markup.
/// Attributes, ids and pseudo-classes never rule a selector out.
pub fn may_match(selector: &str, markup: &Markup) -> bool {
    let s = selector.trim();
    if s.contains('&') || s.contains('|') {
        return true;
    }
    compounds(s).into_iter().all(|(start, end)| compound_may_match(&s[start..end], markup))
}

fn compound_may_match(compound: &str, markup: &Markup) -> bool {
    let b = compound.as_bytes();
    let tag_len = compound.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(b.len());
    if tag_len > 0 && b[0].is_ascii_alphabetic() {
        let tag = compound[..tag_len].to_ascii_lowercase();
        if !ALWAYS_PRESENT.contains(&tag.as_str()) && !markup.tags.contains(&tag) {
            return false;
        }
    }
    let (mut depth, mut i, mut quote) = (0i32, tag_len, 0u8);
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else {
            match c {
                b'"' | b'\'' => quote = c,
                b':' if depth == 0 && (compound[i..].starts_with(":is(") || compound[i..].starts_with(":where(")) => {
                    let open = i + compound[i..].find('(').unwrap();
                    let close = open + 1 + block_end_with(&compound[open + 1..], '(', ')');
                    if !split_top(&compound[open + 1..close.min(b.len())], ',').iter().any(|s| may_match(s, markup)) {
                        return false;
                    }
                    i = close;
                }
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b'.' if depth == 0 => {
                    let (class, len) = ident(&compound[i + 1..]);
                    if !class.is_empty() && !markup.classes.contains(&class) {
                        return false;
                    }
                    i += len;
                }
                _ => {}
            }
        }
        i += 1;
    }
    true
}

/// Unescaped CSS identifier at the start of `s`, and its length in `s`.
fn ident(s: &str) -> (String, usize) {
    let mut out = String::new();
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            let hex: String = s[i + 1..].chars().take_while(char::is_ascii_hexdigit).take(6).collect();
            if hex.is_empty() {
                match chars.next() {
                    Some((_, n)) => out.push(n),
                    None => return (out, s.len()),
                }
            } else {
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
                for _ in 0..hex.len() {
                    chars.next();
                }
                if chars.peek().is_some_and(|(_, n)| *n == ' ') {
                    chars.next();
                }
            }
        } else if c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() {
            out.push(c);
        } else {
            return (out, i);
        }
    }
    (out, s.len())
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i + 2..].find("*/").map_or("", |j| &rest[i + 2 + j + 2..]);
    }
    out.push_str(rest);
    out
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string()
}

fn font_face_family(body: &str) -> Option<String> {
    declarations(body).into_iter().find(|(n, _)| n == "font-family").map(|(_, v)| unquote(v))
}

fn has_word(text: &str, word: &str) -> bool {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    text.match_indices(word).any(|(i, _)| {
        !text[..i].chars().next_back().is_some_and(is_ident) && !text[i + word.len()..].chars().next().is_some_and(is_ident)
    })
}

/// The theme CSS a route needs: style rules whose selectors may match `markup` (see [`may_match`]),
/// every other at-rule, and `@keyframes` / `@font-face` only when the kept CSS or `route_css` (the
/// route's component, page and utility CSS) names them.
pub fn prune_theme(theme_css: &str, markup: &Markup, route_css: &str) -> String {
    let css = strip_comments(theme_css);
    let rules = filter(&css, &mut |prelude, _| at_name(prelude).is_some() || split_top(prelude, ',').iter().any(|s| may_match(s, markup)));
    let mut named = filter(&rules, &mut |prelude, _| !matches!(at_name(prelude).as_deref(), Some("keyframes" | "-webkit-keyframes" | "font-face")));
    named.push_str(route_css);
    let named_lower = named.to_ascii_lowercase();
    filter(&rules, &mut |prelude, body| match at_name(prelude).as_deref() {
        Some("keyframes" | "-webkit-keyframes") => has_word(&named, &unquote(at_param(prelude))),
        Some("font-face") => font_face_family(body).is_none_or(|f| named_lower.contains(&f.to_ascii_lowercase())),
        _ => true,
    })
}

/// Scoped page or component CSS without the rules whose own-element compound (the one with
/// `marker`, e.g. `:where([data-s~=`) needs an element name or class the route's markup lacks.
/// Other compounds may be outside the owner (`:global(…)`, ancestors), so they never drop a rule.
pub fn prune_scoped(css: &str, markup: &Markup, marker: &str) -> String {
    let css = strip_comments(css);
    filter(&css, &mut |prelude, _| {
        at_name(prelude).is_some()
            || split_top(prelude, ',').iter().any(|sel| {
                let sel = sel.trim();
                compounds(sel).into_iter().find(|(start, end)| sel[*start..*end].contains(marker)).is_none_or(|(start, end)| compound_may_match(&sel[start..end], markup))
            })
    })
}

/// What a compound of scoped CSS needs from an element: its name, its classes, and one alternative
/// of each `:is()`/`:where()` (outside other parentheses and brackets). An empty one matches any
/// element.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnCompound {
    pub tag: Option<String>,
    pub classes: Vec<String>,
    pub any_of: Vec<Vec<OwnCompound>>,
}

impl OwnCompound {
    pub fn matches(&self, tag: &str, has_class: &dyn Fn(&str) -> bool) -> bool {
        self.tag.as_deref().is_none_or(|t| t.eq_ignore_ascii_case(tag)) && self.classes.iter().all(|c| has_class(c)) && self.any_of.iter().all(|alts| alts.iter().any(|a| a.matches(tag, has_class)))
    }

    fn is_any(&self) -> bool {
        self.tag.is_none() && self.classes.is_empty() && self.any_of.is_empty()
    }

    /// Whether an element pragma did not write (`<html>`, `<body>`, or one with a name or class
    /// the page's markup lacks) can match.
    fn may_be_outside(&self, markup: &Markup) -> bool {
        match self.tag.as_deref().map(str::to_ascii_lowercase) {
            Some(tag) => ALWAYS_PRESENT.contains(&tag.as_str()) || !markup.tags.contains(&tag),
            None => (self.classes.is_empty() || self.classes.iter().any(|c| !markup.classes.contains(c))) && self.any_of.iter().all(|alts| alts.iter().any(|a| a.may_be_outside(markup))),
        }
    }
}

/// An element above the one being checked: one pragma wrote, or an unknown one (the body of a
/// component that renders itself is cloned under any element).
#[derive(Debug, Clone)]
pub enum Ancestor {
    Element { tag: String, classes: Vec<String> },
    Unknown,
}

/// A scoped selector seen from the element that holds the token: its own-element compound and the
/// compounds above it, nearest first. Sibling compounds (`+`, `~`) are left out and `>` counts as
/// any ancestor, so the check can only say yes too often.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnSelector {
    pub own: OwnCompound,
    pub ancestors: Vec<OwnCompound>,
}

impl OwnSelector {
    /// Whether an element (`tag`, every class it can have) under `ancestors` (outermost first) can
    /// match. `<html>`, `<body>` and elements pragma did not write may be above all of them.
    pub fn may_match(&self, tag: &str, classes: &[String], ancestors: &[Ancestor], markup: &Markup) -> bool {
        if !self.own.matches(tag, &|c| classes.iter().any(|x| x == c)) {
            return false;
        }
        let mut above = ancestors.len();
        for need in &self.ancestors {
            let found = ancestors[..above].iter().rposition(|a| match a {
                Ancestor::Unknown => true,
                Ancestor::Element { tag, classes } => need.matches(tag, &|c| classes.iter().any(|x| x == c)),
            });
            match found {
                Some(i) => above = if matches!(ancestors[i], Ancestor::Unknown) { i + 1 } else { i },
                None if need.may_be_outside(markup) => above = 0,
                None => return false,
            }
        }
        true
    }
}

/// The own-element selectors (those holding `marker`) of every style rule in scoped CSS. A selector
/// with nesting (`&`) or a namespace (`|`) can match any element.
pub fn own_selectors(css: &str, marker: &str) -> BTreeSet<OwnSelector> {
    let mut out = BTreeSet::new();
    each_rule(&strip_comments(css), &mut |prelude, _| {
        for sel in split_top(prelude, ',') {
            let sel = sel.trim();
            let parts = compounds(sel);
            let Some(at) = parts.iter().position(|(start, end)| sel[*start..*end].contains(marker)) else { continue };
            if sel.contains('&') || sel.contains('|') {
                out.insert(OwnSelector { own: own_compound(""), ancestors: Vec::new() });
                continue;
            }
            let ancestors = (0..at).rev().filter(|&i| !sel[parts[i].1..parts[i + 1].0].contains(['+', '~'])).map(|i| own_compound(&sel[parts[i].0..parts[i].1])).filter(|c| !c.is_any()).collect();
            out.insert(OwnSelector { own: own_compound(&sel[parts[at].0..parts[at].1]), ancestors });
        }
    });
    out
}

/// The own-element compound of each scoped selector in `css`, with the selector as written (the
/// scope `marker` taken out, its subject in `:global(…)` as the fix) and its offset in `css`.
/// Selectors with nesting (`&`) or a namespace (`|`) are left out: they can match any element.
pub fn scoped_subjects(css: &str, marker: &str) -> Vec<ScopedSubject> {
    let mut out = Vec::new();
    each_rule(css, &mut |prelude, _| {
        for sel in split_top(prelude, ',') {
            let sel = sel.trim();
            if sel.contains('&') || sel.contains('|') {
                continue;
            }
            let parts = compounds(sel);
            let Some(&(start, end)) = parts.iter().find(|(start, end)| sel[*start..*end].contains(marker)) else { continue };
            let compound = &sel[start..end];
            let at = compound.find(marker).unwrap();
            let close = at + compound[at..].find("])").map_or(compound.len() - at, |i| i + 2);
            let bare = format!("{}{}", &compound[..at], &compound[close..]);
            out.push(ScopedSubject {
                own: own_compound(&bare),
                selector: format!("{}{bare}{}", &sel[..start], &sel[end..]),
                global: format!("{}:global({bare}){}", &sel[..start], &sel[end..]),
                offset: sel.as_ptr() as usize - css.as_ptr() as usize,
            });
        }
    });
    out
}

/// A scoped selector seen from its subject: see [`scoped_subjects`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedSubject {
    pub own: OwnCompound,
    pub selector: String,
    pub global: String,
    pub offset: usize,
}

fn own_compound(compound: &str) -> OwnCompound {
    let b = compound.as_bytes();
    let tag_len = compound.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(b.len());
    let mut tag = (tag_len > 0 && b[0].is_ascii_alphabetic()).then(|| compound[..tag_len].to_ascii_lowercase());
    let mut classes = Vec::new();
    let mut any_of = Vec::new();
    let (mut depth, mut i, mut quote) = (0i32, tag_len, 0u8);
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else {
            match c {
                b'"' | b'\'' => quote = c,
                b':' if depth == 0 && [":is(", ":where(", ":matches(", ":-webkit-any("].iter().any(|p| compound[i..].to_ascii_lowercase().starts_with(p)) => {
                    let open = i + compound[i..].find('(').unwrap();
                    let close = open + 1 + block_end_with(&compound[open + 1..], '(', ')');
                    let alts: Vec<OwnCompound> = split_top(&compound[open + 1..close.min(b.len())], ',').into_iter().map(|alt| subject(alt.trim())).collect();
                    if !alts.iter().any(OwnCompound::is_any) {
                        any_of.push(alts);
                    }
                    i = close;
                }
                b':' if depth == 0 && ident(&compound[i + 1..]).0.eq_ignore_ascii_case("root") => {
                    tag.get_or_insert_with(|| "html".into());
                }
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                b'.' if depth == 0 => {
                    let (class, len) = ident(&compound[i + 1..]);
                    if !class.is_empty() {
                        classes.push(class);
                    }
                    i += len;
                }
                _ => {}
            }
        }
        i += 1;
    }
    OwnCompound { tag, classes, any_of }
}

fn subject(selector: &str) -> OwnCompound {
    match compounds(selector).last() {
        Some(&(start, end)) if !selector.contains(['&', '|']) => own_compound(&selector[start..end]),
        _ => own_compound(""),
    }
}

struct Face {
    family: String,
    italic: bool,
    weight: (u32, u32),
    latin: bool,
    href: Option<String>,
}

fn face(body: &str) -> Option<Face> {
    let decls = declarations(body);
    let get = |name: &str| decls.iter().find(|(n, _)| n == name).map(|(_, v)| *v);
    let family = unquote(get("font-family")?);
    let italic = get("font-style").is_some_and(|v| !v.trim().eq_ignore_ascii_case("normal"));
    let weights: Vec<u32> = get("font-weight")
        .unwrap_or("400")
        .split_whitespace()
        .filter_map(|w| match w {
            "normal" => Some(400),
            "bold" => Some(700),
            w => w.parse().ok(),
        })
        .collect();
    let weight = (*weights.first().unwrap_or(&400), *weights.last().unwrap_or(&400));
    let latin = get("unicode-range").is_none_or(|r| r.split(',').any(|r| range_covers(r.trim(), 'a' as u32)));
    let href = get("src").and_then(woff2_url);
    Some(Face { family, italic, weight, latin, href })
}

fn range_covers(range: &str, cp: u32) -> bool {
    let Some(r) = range.strip_prefix("U+").or_else(|| range.strip_prefix("u+")) else { return false };
    let (lo, hi) = match r.split_once('-') {
        Some((a, b)) => (u32::from_str_radix(a, 16).ok(), u32::from_str_radix(b, 16).ok()),
        None if r.contains('?') => (u32::from_str_radix(&r.replace('?', "0"), 16).ok(), u32::from_str_radix(&r.replace('?', "F"), 16).ok()),
        None => (u32::from_str_radix(r, 16).ok(), u32::from_str_radix(r, 16).ok()),
    };
    matches!((lo, hi), (Some(lo), Some(hi)) if (lo..=hi).contains(&cp))
}

/// First same-origin or absolute `url(…)` of a `src` that is a `.woff2` file.
fn woff2_url(src: &str) -> Option<String> {
    for part in split_top(src, ',') {
        let Some(i) = part.find("url(") else { continue };
        let end = part[i..].find(')').map_or(part.len(), |e| i + e);
        let url = unquote(&part[i + 4..end]);
        let woff2 = url.split(['?', '#']).next().is_some_and(|p| p.ends_with(".woff2")) || part.contains("woff2");
        if woff2 && (url.starts_with('/') && !url.starts_with("//") || url.starts_with("https://")) {
            return Some(url);
        }
    }
    None
}

/// Values of `--name` in every rule, and the font values (`font-family`, `font`) of rules that may
/// match `markup`.
fn font_values(css: &str, markup: &Markup) -> (HashMap<String, Vec<String>>, Vec<String>) {
    let mut vars: HashMap<String, Vec<String>> = HashMap::new();
    let mut used = Vec::new();
    each_rule(css, &mut |prelude, body| {
        let matches = split_top(prelude, ',').iter().any(|s| may_match(s, markup));
        for (name, value) in declarations(body) {
            if name.starts_with("--") {
                vars.entry(name).or_default().push(value.to_string());
            } else if matches && (name == "font-family" || name == "font") {
                used.push(value.to_string());
            }
        }
    });
    (vars, used)
}

fn resolve(value: &str, vars: &HashMap<String, Vec<String>>, seen: &mut BTreeSet<String>, out: &mut String) {
    out.push_str(value);
    out.push(',');
    let mut rest = value;
    while let Some(i) = rest.find("var(") {
        let after = rest[i + 4..].trim_start();
        let len = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(after.len());
        let name = after[..len].to_ascii_lowercase();
        if seen.insert(name.clone()) {
            for v in vars.get(&name).into_iter().flatten() {
                resolve(v, vars, seen, out);
            }
        }
        rest = &after[len..];
    }
}

/// Font files to preload for a route: for each `@font-face` family that a rule matching `markup`
/// uses (`font-family`/`font`, through `var(--x)` chains), the `.woff2` of its upright face that
/// covers Latin letters (the one with weight 400 when there are several).
pub fn font_preloads(css: &str, markup: &Markup) -> Vec<String> {
    let css = strip_comments(css);
    let mut faces = Vec::new();
    let mut collect = |css: &str| {
        for item in items(css) {
            if let Item::Block { prelude, body } = item
                && at_name(prelude).as_deref() == Some("font-face")
                && let Some(f) = face(body)
            {
                faces.push(f);
            }
        }
    };
    collect(&css);
    for item in items(&css) {
        if let Item::Block { prelude, body } = item
            && at_name(prelude).is_some_and(|n| n == "layer" || n == "media" || n == "supports")
        {
            collect(body);
        }
    }
    let (vars, used) = font_values(&css, markup);
    let mut resolved = String::new();
    let mut seen = BTreeSet::new();
    for v in &used {
        resolve(v, &vars, &mut seen, &mut resolved);
    }
    let names: Vec<String> = resolved.to_ascii_lowercase().split(',').map(|s| s.trim().to_string()).collect();
    let names_family = |family: &str| {
        let family = family.to_ascii_lowercase();
        names.iter().any(|n| {
            unquote(n) == family || [format!("'{family}'"), format!("\"{family}\""), format!(" {family}")].iter().any(|end| n.ends_with(end.as_str()))
        })
    };
    let mut out: Vec<String> = Vec::new();
    let mut families: Vec<&str> = Vec::new();
    for f in &faces {
        if !families.contains(&f.family.as_str()) {
            families.push(&f.family);
        }
    }
    for family in families {
        if !names_family(family) {
            continue;
        }
        let candidates: Vec<&Face> = faces.iter().filter(|f| f.family == family && !f.italic && f.latin && f.href.is_some()).collect();
        let pick = candidates.iter().find(|f| (f.weight.0..=f.weight.1).contains(&400)).or(candidates.first());
        if let Some(href) = pick.and_then(|f| f.href.clone())
            && !out.contains(&href)
        {
            out.push(href);
        }
    }
    out
}

/// Selectors of `css` (outside comments) made of one class that is also a utility, with pseudo-classes
/// at most (`.flex`, `.hidden:hover`): `(class, byte offset)`.
pub fn utility_redefinitions(css: &str, is_utility: &dyn Fn(&str) -> bool) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    each_rule(css, &mut |prelude, _| {
        for sel in split_top(prelude, ',') {
            let s = sel.trim();
            let Some(rest) = s.strip_prefix('.') else { continue };
            let (class, len) = ident(rest);
            let tail = &rest[len..];
            if class.is_empty() || !(tail.is_empty() || tail.starts_with(':')) || tail.contains(['.', ' ', '>', '+', '~', '[']) {
                continue;
            }
            if is_utility(&class) {
                let offset = s.as_ptr() as usize - css.as_ptr() as usize;
                out.push((class, offset));
            }
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markup(tags: &[&str], classes: &[&str]) -> Markup {
        Markup { tags: tags.iter().map(|s| s.to_string()).collect(), classes: classes.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn selectors_need_their_tags_and_classes() {
        let m = markup(&["div", "button", "dialog"], &["card", "md:p-4"]);
        for yes in [".card", "div.card:hover", ".card > button", ":root", "html[data-theme=\"dark\"] .card", "*", "::selection", "dialog::backdrop", ":not(.nope)", ":is(.nope, .card) button", ":where(div, a).card", "[data-x=\"a.b\"] .card", ".md\\:p-4", "::view-transition-group(*.mes)", "#main", "body"] {
            assert!(may_match(yes, &m), "{yes}");
        }
        for no in [".nope", "a", "div.nope", ".card .nope", "table td", ".card + p", ".nope:is(.card)", "body.nope", ":is(.a, .b) .card", ":where(a, p)"] {
            assert!(!may_match(no, &m), "{no}");
        }
    }

    #[test]
    fn theme_keeps_what_the_route_can_use() {
        let theme = "@font-face { font-family: 'Used'; src: url(/u.woff2) }\n@font-face { font-family: Unused; src: url(/x.woff2) }\n@layer tokens { :root { --font-sans: 'Used', sans-serif; } }\n@layer components {\n  .card { animation: pop 1s; }\n  .modal { animation: slide 1s; }\n}\n@media (width >= 40rem) { .modal { x: 1 } }\n@keyframes pop { to { opacity: 1 } }\n@keyframes slide { to { opacity: 1 } }\n@keyframes fade { to { opacity: 0 } }\n@page { margin: 1cm }\n";
        let out = prune_theme(theme, &markup(&["div"], &["card"]), ".x:where([data-s~=a]) { animation: fade 1s }");
        for kept in ["'Used'", "--font-sans", ".card", "@keyframes pop", "@keyframes fade", "@page"] {
            assert!(out.contains(kept), "{kept} missing in {out}");
        }
        for dropped in ["Unused", ".modal", "@media", "@keyframes slide"] {
            assert!(!out.contains(dropped), "{dropped} kept in {out}");
        }
    }

    #[test]
    fn scoped_rules_need_their_own_element() {
        let css = ".a:where([data-s~=\"s1\"]) { x: 1 }\n.b:where([data-s~=\"s1\"]) { x: 2 }\n.b:where([data-s~=\"s1\"]) .hit, .gone .a:where([data-s~=\"s1\"]) { x: 3 }\n.list:where([data-s~=\"s1\"]) .widget { x: 4 }\n[data-c=\"x\"] .nope { x: 5 }\n@keyframes k { to { x: 6 } }";
        let out = prune_scoped(css, &markup(&[], &["a", "list"]), ":where([data-s~=");
        for kept in ["x: 1", "x: 3", "x: 4", "x: 5", "x: 6"] {
            assert!(out.contains(kept), "{kept} missing in {out}");
        }
        assert!(!out.contains("x: 2"), "{out}");
    }

    fn oc(tag: Option<&str>, classes: &[&str]) -> OwnCompound {
        OwnCompound { tag: tag.map(str::to_string), classes: classes.iter().map(|c| c.to_string()).collect(), any_of: Vec::new() }
    }

    fn el(tag: &str, classes: &[&str]) -> Ancestor {
        Ancestor::Element { tag: tag.into(), classes: classes.iter().map(|c| c.to_string()).collect() }
    }

    fn owns(css: &str) -> Vec<OwnSelector> {
        own_selectors(css, ":where([data-s~=").into_iter().collect()
    }

    #[test]
    fn own_selectors_say_which_elements_need_the_token() {
        let css = ".a:where([data-s~=\"s1\"]) { x: 1 }\n@media (width >= 1px) { li.b.c:where([data-s~=\"s1\"]):hover, .ancestor :global(.x) { x: 2 } }\n.list:where([data-s~=\"s1\"]) .hit, :where([data-s~=\"s1\"])::marker { x: 3 }";
        let own: Vec<OwnCompound> = owns(css).into_iter().map(|s| s.own).collect();
        assert_eq!(own, [oc(None, &[]), oc(None, &["a"]), oc(None, &["list"]), oc(Some("li"), &["b", "c"])]);
        assert!(own[3].matches("LI", &|c| c == "b" || c == "c") && !own[3].matches("li", &|c| c == "b") && !own[3].matches("p", &|_| true));
    }

    #[test]
    fn own_selectors_keep_ancestors_and_alternatives() {
        let one = |css: &str| owns(css).remove(0);
        let s = one(".tabela tbody > tr.abre td:where([data-s~=\"s1\"]) { x: 1 }");
        assert_eq!((s.own, s.ancestors), (oc(Some("td"), &[]), vec![oc(Some("tr"), &["abre"]), oc(Some("tbody"), &[]), oc(None, &["tabela"])]));
        let s = one(".a .b + .c ~ .d .e:where([data-s~=\"s1\"]) { x: 1 }");
        assert_eq!(s.ancestors, [oc(None, &["d"]), oc(None, &["a"])], "sibling compounds share the parent of the next one");
        let s = one(".m[data-x] :is(.p, li.q, .r .s):where([data-s~=\"s1\"]):not(.z) { x: 1 }");
        assert_eq!(s.own.any_of, [vec![oc(None, &["p"]), oc(Some("li"), &["q"]), oc(None, &["s"])]]);
        assert!(s.own.matches("span", &|c| c == "s") && !s.own.matches("span", &|c| c == "z" || c == "r"));
        for any in [".k:is(:hover, .x):where([data-s~=\"s1\"]) { x: 1 }", ":where([data-s~=\"s1\"]) { x: 1 }", "* .x:where([data-s~=\"s1\"]) { x: 1 }"] {
            let s = one(any);
            assert!(s.own.any_of.is_empty() && s.ancestors.iter().all(|a| !a.is_any()), "{any}");
        }
        let s = one(".a { & .b:where([data-s~=\"s1\"]) { x: 1 } }\n.n &:where([data-s~=\"s1\"]) { x: 2 }");
        assert!(s.own.is_any() && s.ancestors.is_empty());
    }

    #[test]
    fn own_selectors_need_ancestors_the_page_can_give() {
        let m = markup(&["table", "tr", "td", "div", "span"], &["tabela", "mes", "abre"]);
        let s = owns(".tabela tr.abre td:where([data-s~=\"s1\"]) { x: 1 }").remove(0);
        let td = |ancestors: &[Ancestor]| s.may_match("td", &[], ancestors, &m);
        assert!(td(&[el("div", &["tabela"]), el("table", &[]), el("tr", &["abre"])]));
        assert!(td(&[el("div", &["tabela"]), el("tr", &["abre"]), el("span", &[])]), "a descendant at any depth");
        assert!(!td(&[el("div", &["mes"]), el("table", &[]), el("tr", &["abre"])]));
        assert!(!td(&[el("tr", &["abre"]), el("div", &["tabela"])]), "the order of ancestors counts");
        assert!(!td(&[]) && !s.may_match("span", &[], &[el("div", &["tabela"]), el("tr", &["abre"])], &m));
        assert!(td(&[Ancestor::Unknown]), "a self-rendered body can be anywhere");
        assert!(td(&[el("div", &["tabela"]), Ancestor::Unknown]));
        let outside = |css: &str| owns(css).remove(0).may_match("span", &[], &[el("div", &[])], &m);
        for yes in ["html[data-theme=\"dark\"] span:where([data-s~=\"s1\"]) { x: 1 }", "body.locked span:where([data-s~=\"s1\"]) { x: 1 }", ".dark span:where([data-s~=\"s1\"]) { x: 1 }", "x-widget span:where([data-s~=\"s1\"]) { x: 1 }", ":root.mes span:where([data-s~=\"s1\"]) { x: 1 }", ".mes.dark span:where([data-s~=\"s1\"]) { x: 1 }", "div > span:where([data-s~=\"s1\"]) { x: 1 }"] {
            assert!(outside(yes), "{yes}");
        }
        for no in [".mes span:where([data-s~=\"s1\"]) { x: 1 }", "table span:where([data-s~=\"s1\"]) { x: 1 }", ":is(.mes, .abre) span:where([data-s~=\"s1\"]) { x: 1 }", "html .mes span:where([data-s~=\"s1\"]) { x: 1 }"] {
            assert!(!outside(no), "{no}");
        }
    }

    #[test]
    fn preloads_fonts_the_route_uses() {
        let css = r#"
@font-face { font-family: 'Sans V'; font-weight: 100 900; src: url('/f/sans-ext.woff2') format('woff2'); unicode-range: U+0100-024F; }
@font-face { font-family: 'Sans V'; font-weight: 100 900; src: url('/f/sans-latin.woff2') format('woff2'); unicode-range: U+0000-00FF, U+0131; }
@font-face { font-family: 'Sans V'; font-style: italic; src: url('/f/sans-italic.woff2'); }
@font-face { font-family: Slab; font-weight: 700; src: url(/f/serif-700.woff2); }
@font-face { font-family: Slab; font-weight: 400; src: url(/f/serif-400.woff2); }
@font-face { font-family: Display; src: url(/f/display.woff2); }
@font-face { font-family: Hand; src: url(/f/hand.woff2); }
@layer tokens { :root { --font-sans: 'Sans V', sans-serif; --font-display: Display, serif; --title: var(--font-serif); --font-serif: Slab; } }
html { font-family: var(--default-font-family, sans-serif); }
:root { --default-font-family: var(--font-sans); }
.title { font: 700 2rem/1 var(--title); }
.hero { font-family: var(--font-display); }
.note { font-family: Hand; }
"#;
        assert_eq!(font_preloads(css, &markup(&["h1"], &["title"])), ["/f/sans-latin.woff2", "/f/serif-400.woff2"]);
        assert_eq!(font_preloads(css, &markup(&[], &[])), ["/f/sans-latin.woff2"]);
        assert_eq!(font_preloads(css, &markup(&[], &["note", "hero"])), ["/f/sans-latin.woff2", "/f/display.woff2", "/f/hand.woff2"]);
    }

    #[test]
    fn finds_classes_named_like_utilities() {
        let css = ".flex { display: grid }\n@layer components { .hidden:hover, .card { x: 1 } .heroi > .contents, .mr-auto.x, .p-4 .y { x: 1 } }";
        let found: Vec<String> = utility_redefinitions(css, &|c| ["flex", "hidden", "contents", "mr-auto", "p-4"].contains(&c)).into_iter().map(|(c, _)| c).collect();
        assert_eq!(found, ["flex", "hidden"]);
    }
}
