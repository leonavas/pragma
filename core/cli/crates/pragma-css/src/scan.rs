//! Reading CSS text: strings, comments, `()`/`[]` nesting, at-rule names and `var()` names.

/// Index after the string or comment that starts at `i`, if one starts there. A string ends at its
/// closing quote (a backslash escapes the next byte); an open one or an open comment runs to the end.
fn skip_string_or_comment(s: &str, i: usize) -> Option<usize> {
    let b = s.as_bytes();
    match b[i] {
        quote @ (b'"' | b'\'') => {
            let mut j = i + 1;
            while j < b.len() {
                match b[j] {
                    b'\\' => j += 2,
                    c if c == quote => return Some(j + 1),
                    _ => j += 1,
                }
            }
            Some(b.len())
        }
        b'/' if b.get(i + 1) == Some(&b'*') => Some(s[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 2)),
        _ => None,
    }
}

/// Bytes of `s` outside strings, comments and `()`/`[]`, with their offsets. An opening `(` or `[`
/// at the top level is included; what it holds and its closing byte are not.
pub(crate) fn top_level(s: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    let b = s.as_bytes();
    let (mut i, mut depth) = (0, 0i32);
    std::iter::from_fn(move || {
        while i < b.len() {
            if let Some(next) = skip_string_or_comment(s, i) {
                i = next;
                continue;
            }
            let (at, c) = (i, b[i]);
            i += 1;
            match c {
                b'(' | b'[' => {
                    depth += 1;
                    if depth == 1 {
                        return Some((at, c));
                    }
                }
                b')' | b']' => depth -= 1,
                _ if depth == 0 => return Some((at, c)),
                _ => {}
            }
        }
        None
    })
}

/// First of `chars` at the top level (see [`top_level`]).
pub(crate) fn find_top(s: &str, chars: &[char]) -> Option<usize> {
    top_level(s).find(|(_, c)| chars.contains(&(*c as char))).map(|(i, _)| i)
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

/// Index of the `}` closing the `{` at `open` (or the end).
pub(crate) fn block_end(s: &str, open: usize) -> usize {
    open + 1 + block_end_with(&s[open + 1..], '{', '}')
}

/// Index of the `close` matching an already opened `open`, in `s` (or `s.len()`). Only `open` and
/// `close` count, outside strings and comments.
pub(crate) fn block_end_with(s: &str, open: char, close: char) -> usize {
    let b = s.as_bytes();
    let (mut depth, mut i) = (0i32, 0);
    while i < b.len() {
        if let Some(next) = skip_string_or_comment(s, i) {
            i = next;
            continue;
        }
        if b[i] as char == open {
            depth += 1;
        } else if b[i] as char == close {
            if depth == 0 {
                return i;
            }
            depth -= 1;
        }
        i += 1;
    }
    b.len()
}

/// Byte ranges of the compound selectors (split at the top-level combinators), in order.
pub(crate) fn compounds(s: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None::<usize>;
    for (i, c) in top_level(s) {
        if matches!(c, b' ' | b'\t' | b'\n' | b'>' | b'+' | b'~') {
            if let Some(from) = start.take() {
                out.push((from, i));
            }
        } else {
            start.get_or_insert(i);
        }
    }
    if let Some(from) = start {
        out.push((from, s.len()));
    }
    out
}

/// Lowercase name of the at-rule that `prelude` starts (`@media (…)` → `media`).
pub(crate) fn at_name(prelude: &str) -> Option<String> {
    let at = prelude.trim_start().strip_prefix('@')?;
    Some(at.split(|c: char| c.is_whitespace() || c == '{' || c == '(').next().unwrap_or("").to_ascii_lowercase())
}

pub(crate) fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i + 2..].find("*/").map_or("", |j| &rest[i + 2 + j + 2..]);
    }
    out.push_str(rest);
    out
}

/// The name in each `var(` of `css` (`--x` in `var( --x, 1px)`), as written.
pub(crate) fn var_names(css: &str) -> impl Iterator<Item = &str> {
    css.match_indices("var(").map(|(i, _)| {
        let after = css[i + 4..].trim_start();
        let len = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(after.len());
        &after[..len]
    })
}
