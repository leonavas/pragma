//! Scoped `<style>` / `.css` of components and pages: each style rule applies only inside its owner.

use crate::scan::{at_name, block_end, block_end_with, compounds, find_top, split_top, top_level};

/// At-rules whose block holds style rules (scoped recursively); any other block is copied as is
/// (`@keyframes`, `@font-face`, `@property`, `@page`…).
pub(crate) const NESTING: &[&str] = &["media", "supports", "layer", "container", "starting-style", "document"];

/// The selector of the elements a page or component wrote: the build writes `data-s="<token>"` on
/// them.
pub fn owner_selector(token: &str) -> String {
    format!("[data-s~=\"{token}\"]")
}

/// What [`scope`] puts in a selector's own-element compound, up to the token (`OWNED` + `"s1"` +
/// [`OWNED_END`]).
pub const OWNED: &str = ":where([data-s~=";
pub(crate) const OWNED_END: &str = "])";

/// Pseudo-elements written with one colon (CSS 2): the scope goes before them.
const OLD_PSEUDO_ELEMENTS: &[&str] = &[":before", ":after", ":first-line", ":first-letter"];

/// Scopes every style rule of `css` to `scope` (a selector of the owner's own elements, such as
/// `[data-s~="s1a2b3"]`): a selector that does not mention any of `spellings` gets `:where(scope)`
/// on its subject, so it matches only elements the owner wrote, with no added specificity.
/// `:global(…)` opts out only what is inside it (the wrapper is removed): the scope goes on the last
/// compound that does not start with it (`:global(html.dark) .row` → `html.dark .row:where(…)`);
/// a selector made only of `:global(…)` compounds (`:global(:root)`) is not scoped.
pub fn scope(css: &str, scope: &str, spellings: &[String]) -> String {
    let mut out = String::with_capacity(css.len() + css.len() / 4);
    let mut rest = css;
    while !rest.is_empty() {
        let ws = rest.len() - rest.trim_start().len();
        out.push_str(&rest[..ws]);
        rest = &rest[ws..];
        if rest.is_empty() {
            break;
        }
        let Some(open) = find_top(rest, &['{', ';']) else {
            out.push_str(rest);
            break;
        };
        let prelude = &rest[..open];
        if rest.as_bytes()[open] == b';' {
            out.push_str(&rest[..=open]);
            rest = &rest[open + 1..];
            continue;
        }
        let close = block_end(rest, open);
        let block = &rest[open + 1..close];
        if let Some(name) = at_name(prelude) {
            out.push_str(prelude);
            out.push('{');
            if NESTING.contains(&name.as_str()) {
                out.push_str(&self::scope(block, scope, spellings));
            } else {
                out.push_str(block);
            }
        } else {
            let selectors: Vec<String> = split_top(prelude, ',').iter().map(|s| selector(s, scope, spellings)).collect();
            out.push_str(&selectors.join(","));
            out.push('{');
            out.push_str(block);
        }
        if close < rest.len() {
            out.push('}');
        }
        rest = &rest[(close + 1).min(rest.len())..];
    }
    out
}

fn selector(s: &str, scope: &str, spellings: &[String]) -> String {
    let (lead, trimmed, trail) = (&s[..s.len() - s.trim_start().len()], s.trim(), &s[s.trim_end().len()..]);
    let written = spellings.iter().any(|sp| trimmed.contains(sp.as_str()));
    let target = compounds(trimmed).into_iter().rev().find(|(start, _)| !trimmed[*start..].starts_with(":global("));
    let scoped = match target {
        Some((start, end)) if !written => {
            let at = pseudo_element(&trimmed[start..end]).map_or(end, |i| start + i);
            format!("{}:where({scope}){}", &trimmed[..at], &trimmed[at..])
        }
        _ => trimmed.to_string(),
    };
    format!("{lead}{}{trail}", unwrap_global(&scoped))
}

/// `:global(x)` → `x`, everywhere in the selector.
fn unwrap_global(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(":global(") {
        out.push_str(&rest[..i]);
        let inner = &rest[i + ":global(".len()..];
        let end = block_end_with(inner, '(', ')');
        out.push_str(&inner[..end]);
        rest = &inner[(end + 1).min(inner.len())..];
    }
    out.push_str(rest);
    out
}

/// Offset of the pseudo-element (`::x` or an old one-colon form) in a compound selector.
fn pseudo_element(compound: &str) -> Option<usize> {
    top_level(compound).find(|&(i, c)| {
        if c != b':' {
            return false;
        }
        let tail = compound[i..].to_ascii_lowercase();
        tail.starts_with("::") || OLD_PSEUDO_ELEMENTS.iter().any(|p| tail.starts_with(p) && !tail[p.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-'))
    }).map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::scope;

    fn card(css: &str) -> String {
        let s = "[data-c=\"card\"]";
        scope(css, &super::owner_selector("s1"), &[s.to_string(), "[data-c='card']".into(), "[data-c=card]".into()])
    }

    const W: &str = ":where([data-s~=\"s1\"])";

    #[test]
    fn the_scope_of_an_owner_is_its_marker() {
        let scoped = super::scope(".a{x:1}", &super::owner_selector("s1"), &[]);
        assert_eq!(scoped, format!(".a{}\"s1\"{}{{x:1}}", super::OWNED, super::OWNED_END));
    }

    #[test]
    fn a_quoted_bracket_does_not_hide_the_pseudo_element() {
        assert_eq!(card("a[title=\")\"]::before{x:1}"), format!("a[title=\")\"]{W}::before{{x:1}}"));
    }

    #[test]
    fn subjects_get_the_scope() {
        assert_eq!(card(".card { display: flex } button { color: red }"), format!(".card{W} {{ display: flex }} button{W} {{ color: red }}"));
        assert_eq!(card(".a .b > li + p ~ i{x:1}"), format!(".a .b > li + p ~ i{W}{{x:1}}"));
        assert_eq!(card("a:hover, .b::before, .c:after, li::marker{x:1}"), format!("a:hover{W}, .b{W}::before, .c{W}:after, li{W}::marker{{x:1}}"));
        assert_eq!(card(":is(a, b) > [data-x=\"a b\"]{x:1}"), format!(":is(a, b) > [data-x=\"a b\"]{W}{{x:1}}"));
    }

    #[test]
    fn written_scope_and_global_are_kept() {
        let css = "[data-c=\"card\"] .t{x:1} .dark [data-c='card']{x:1} [data-c=\"card\"][open]{x:1}";
        assert_eq!(card(css), css);
        assert_eq!(card("[data-c=card] :global(.x){a:1}"), "[data-c=card] .x{a:1}");
    }

    #[test]
    fn global_opts_out_only_what_is_inside_it() {
        assert_eq!(card(":global(html[data-theme=\"dark\"]) .row{x:1}"), format!("html[data-theme=\"dark\"] .row{W}{{x:1}}"));
        assert_eq!(card(":global(body.locked) .x::before, :global(:root){--a:1}"), format!("body.locked .x{W}::before, :root{{--a:1}}"));
        assert_eq!(card(":global(:root), :global(body.locked):hover, :global(.a) > :global(.b){x:1}"), ":root, body.locked:hover, .a > .b{x:1}");
        assert_eq!(card(".list :global(.hit){x:1}"), format!(".list{W} .hit{{x:1}}"));
        assert_eq!(card("button:global(.active){x:1}"), format!("button.active{W}{{x:1}}"));
        assert_eq!(card(":global(.dark) li:nth-child(2n + 1) > a[title~=\"a b\"]{x:1}"), format!(".dark li:nth-child(2n + 1) > a[title~=\"a b\"]{W}{{x:1}}"));
    }

    #[test]
    fn at_rules() {
        let css = "@media (min-width: 40rem) { .a { x: 1 } @supports (x: y) { b { y: 2 } } } @keyframes spin { from { rotate: 0deg } to { rotate: 1turn } } @import url(\"x.css\"); @font-face { font-family: X; }";
        let want = format!("@media (min-width: 40rem) {{ .a{W} {{ x: 1 }} @supports (x: y) {{ b{W} {{ y: 2 }} }} }} @keyframes spin {{ from {{ rotate: 0deg }} to {{ rotate: 1turn }} }} @import url(\"x.css\"); @font-face {{ font-family: X; }}");
        assert_eq!(card(css), want);
    }

    #[test]
    fn brackets_parentheses_and_escapes_in_selectors() {
        assert_eq!(card(".md\\:p-0 > a[href$=\")\"]{x:1}"), format!(".md\\:p-0 > a[href$=\")\"]{W}{{x:1}}"));
        assert_eq!(card("li:not(.a, [b=\"c d\"]) + p:nth-of-type(2n){x:1}"), format!("li:not(.a, [b=\"c d\"]) + p:nth-of-type(2n){W}{{x:1}}"));
        assert_eq!(card("a[title=\"x::y\"]::after{x:1}"), format!("a[title=\"x::y\"]{W}::after{{x:1}}"));
        assert_eq!(card(":is(.a,.b)::before,\n.c:BEFORE{x:1}"), format!(":is(.a,.b){W}::before,\n.c{W}:BEFORE{{x:1}}"));
    }

    #[test]
    fn nested_rules_and_strings_are_not_touched() {
        assert_eq!(card(".a { & .b { c: d } content: \"}\" }"), format!(".a{W} {{ & .b {{ c: d }} content: \"}}\" }}"));
        assert_eq!(card(""), "");
    }
}
