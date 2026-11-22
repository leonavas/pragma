//! `url(…)` references in CSS.

/// Replaces the target of each `url(…)` for which `map` returns a new URL and keeps the rest of
/// `css` as written. Comments and strings outside `url()` are skipped.
pub fn map(css: &str, mut map: impl FnMut(&str) -> Option<String>) -> String {
    let bytes = css.as_bytes();
    let mut out = String::with_capacity(css.len());
    let mut copied = 0;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = css[i + 2..].find("*/").map_or(bytes.len(), |e| i + 2 + e + 2),
            quote @ (b'"' | b'\'') => i = string_end(bytes, i, quote),
            b'u' | b'U' if starts_url(css, i) => {
                let Some((start, end, close)) = target(bytes, i + 4) else {
                    i += 4;
                    continue;
                };
                if let Some(url) = map(&css[start..end]) {
                    out.push_str(&css[copied..start]);
                    out.push_str(&url);
                    copied = end;
                }
                i = close + 1;
            }
            _ => i += 1,
        }
    }
    out.push_str(&css[copied..]);
    out
}

fn starts_url(css: &str, i: usize) -> bool {
    let before = i.checked_sub(1).map(|b| css.as_bytes()[b]);
    css[i..].get(..4).is_some_and(|s| s.eq_ignore_ascii_case("url(")) && !before.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'\\')
}

fn string_end(bytes: &[u8], open: usize, quote: u8) -> usize {
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == quote || b == b'\n' => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `(target start, target end, index of ')')` of the `url(` body that starts at `from`.
fn target(bytes: &[u8], from: usize) -> Option<(usize, usize, usize)> {
    let skip_space = |mut i: usize| {
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        i
    };
    let start = skip_space(from);
    match *bytes.get(start)? {
        quote @ (b'"' | b'\'') => {
            let after = string_end(bytes, start, quote);
            if bytes.get(after - 1) != Some(&quote) {
                return None;
            }
            let close = skip_space(after);
            (bytes.get(close) == Some(&b')')).then_some((start + 1, after - 1, close))
        }
        _ => {
            let end = start + bytes[start..].iter().position(|b| *b == b')' || b.is_ascii_whitespace())?;
            let close = skip_space(end);
            (bytes.get(close) == Some(&b')')).then_some((start, end, close))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::map;

    fn hashed(css: &str) -> String {
        map(css, |url| url.starts_with("/f/").then(|| format!("/assets{url}-h")))
    }

    #[test]
    fn rewrites_quoted_and_unquoted_targets() {
        assert_eq!(
            hashed("@font-face{src:url('/f/a.woff2') format('woff2'),URL( \"/f/b.woff\" ),url(/f/c.ttf)}"),
            "@font-face{src:url('/assets/f/a.woff2-h') format('woff2'),URL( \"/assets/f/b.woff-h\" ),url(/assets/f/c.ttf-h)}"
        );
    }

    #[test]
    fn leaves_other_text_alone() {
        let css = ".a{content:\"url(/f/x)\";mask:url(\"data:image/svg+xml,%3Csvg url(/f/y)%3E\");b:myurl(/f/z)}/* url(/f/w) */.b{c:url(/x/y.png)}.c{d:url(";
        assert_eq!(hashed(css), css);
        assert_eq!(hashed("a{b:url( /f/é.png )}é"), "a{b:url( /assets/f/é.png-h )}é");
    }
}
