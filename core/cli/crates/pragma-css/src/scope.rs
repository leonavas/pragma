//! Scoped `<style>` / `.css` of components and pages: each style rule applies only inside its owner.

/// At-rules whose block holds style rules (scoped recursively); any other block is copied as is
/// (`@keyframes`, `@font-face`, `@property`, `@page`…).
pub(crate) const NESTING: &[&str] = &["media", "supports", "layer", "container", "starting-style", "document"];

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
        if let Some(at) = prelude.trim_start().strip_prefix('@') {
            let name = at.split(|c: char| c.is_whitespace() || c == '{' || c == '(').next().unwrap_or("").to_ascii_lowercase();
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

/// Byte ranges of the compound selectors (split at the top-level combinators), in order.
pub(crate) fn compounds(s: &str) -> Vec<(usize, usize)> {
    let b = s.as_bytes();
    let (mut depth, mut i, mut quote, mut start) = (0i32, 0, 0u8, None::<usize>);
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if depth == 0 && matches!(c, b' ' | b'\t' | b'\n' | b'>' | b'+' | b'~') {
            if let Some(from) = start.take() {
                out.push((from, i));
            }
            i += 1;
            continue;
        } else {
            match c {
                b'"' | b'\'' => quote = c,
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                _ => {}
            }
        }
        start.get_or_insert(i);
        i += 1;
    }
    if let Some(from) = start {
        out.push((from, b.len()));
    }
    out
}

/// Offset of the pseudo-element (`::x` or an old one-colon form) in a compound selector.
fn pseudo_element(compound: &str) -> Option<usize> {
    let b = compound.as_bytes();
    let mut depth = 0i32;
    for i in 0..b.len() {
        match b[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b':' if depth == 0 => {
                if b.get(i + 1) == Some(&b':') {
                    return Some(i);
                }
                let tail = compound[i..].to_ascii_lowercase();
                if OLD_PSEUDO_ELEMENTS.iter().any(|p| tail.starts_with(p) && !tail[p.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-')) {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// First of `chars` at depth 0, outside strings and comments.
pub(crate) fn find_top(s: &str, chars: &[char]) -> Option<usize> {
    let b = s.as_bytes();
    let (mut depth, mut i, mut quote) = (0i32, 0, 0u8);
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = s[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 1);
        } else {
            match c {
                b'"' | b'\'' => quote = c,
                b'(' | b'[' => depth += 1,
                b')' | b']' => depth -= 1,
                _ if depth == 0 && chars.contains(&(c as char)) => return Some(i),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Index of the `}` closing the `{` at `open` (or the end).
pub(crate) fn block_end(s: &str, open: usize) -> usize {
    open + 1 + block_end_with(&s[open + 1..], '{', '}')
}

/// Index of the `close` matching an already opened `open`, in `s` (or `s.len()`).
pub(crate) fn block_end_with(s: &str, open: char, close: char) -> usize {
    let b = s.as_bytes();
    let (mut depth, mut i, mut quote) = (0i32, 0, 0u8);
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = s[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 1);
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c as char == open {
            depth += 1;
        } else if c as char == close {
            if depth == 0 {
                return i;
            }
            depth -= 1;
        }
        i += 1;
    }
    b.len()
}

pub(crate) fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = s;
    while let Some(i) = find_top(rest, &[sep]) {
        parts.push(&rest[..i]);
        rest = &rest[i + 1..];
    }
    parts.push(rest);
    parts
}

#[cfg(test)]
mod tests {
    use super::scope;

    fn card(css: &str) -> String {
        let s = "[data-c=\"card\"]";
        scope(css, "[data-s~=\"s1\"]", &[s.to_string(), "[data-c='card']".into(), "[data-c=card]".into()])
    }

    const W: &str = ":where([data-s~=\"s1\"])";

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
    fn nested_rules_and_strings_are_not_touched() {
        assert_eq!(card(".a { & .b { c: d } content: \"}\" }"), format!(".a{W} {{ & .b {{ c: d }} content: \"}}\" }}"));
        assert_eq!(card(""), "");
    }
}
