//! Directives (SPEC §3): parsing names and paths.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir<'a> {
    Text,
    Attr(&'a str),
    Class(&'a str),
    Show,
    If,
    ElseIf,
    Else,
    For,
    Key,
    Empty,
    On(&'a str),
    Model,
    With(&'a str),
    Open,
    Close,
    /// `data-page`, `data-reload`, `data-theme`: markers without a path.
    Marker,
    /// `data-c`, `data-tid`, `data-of`, `data-slot`, `data-prop-*`: written only by the build.
    Generated,
}

/// Internal element for `{path}` beside child elements; written as `<!--{path}-->text<!--}-->`.
pub const INTERP: &str = "#interp";

/// Part of a `{}` template: literal text (source HTML, `\{` already unescaped) or a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    Path(String),
}

/// Splits `a {path} b` (`\{` = literal brace). Err: an unclosed `{` (byte offset).
pub fn template(v: &str) -> Result<Vec<Part>, usize> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = v;
    while let Some(i) = rest.find(['{', '\\']) {
        text.push_str(&rest[..i]);
        if rest[i..].starts_with("\\{") {
            text.push('{');
            rest = &rest[i + 2..];
            continue;
        }
        if rest.as_bytes()[i] == b'\\' {
            text.push('\\');
            rest = &rest[i + 1..];
            continue;
        }
        let Some(end) = rest[i..].find('}') else { return Err(v.len() - rest.len() + i) };
        if !text.is_empty() {
            parts.push(Part::Text(std::mem::take(&mut text)));
        }
        parts.push(Part::Path(rest[i + 1..i + end].trim().to_string()));
        rest = &rest[i + end + 1..];
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

/// Whether text or an attribute value has `{` (an interpolation or a `\{` escape).
pub fn has_braces(v: &str) -> bool {
    v.contains('{')
}

/// Template string back from parts (literal `{` escaped as `\{`).
pub fn join_template(parts: &[Part]) -> String {
    let mut out = String::new();
    for p in parts {
        match p {
            Part::Text(t) => out.push_str(&t.replace('{', "\\{")),
            Part::Path(p) => {
                out.push('{');
                out.push_str(p);
                out.push('}');
            }
        }
    }
    out
}

/// Paths of a compiled template (`data-text`, `data-attr-*` with `{`).
pub fn template_paths(v: &str) -> Vec<String> {
    template(v).map(|parts| parts.into_iter().filter_map(|p| if let Part::Path(p) = p { Some(p) } else { None }).collect()).unwrap_or_default()
}

/// Argument of an event call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg<'a> {
    Path(&'a str),
    /// `'text'`, `"text"`, number, `true`, `false`, `null`: written as is.
    Literal(&'a str),
    Event,
}

/// `name` or `name(arg, …)`. Err: what is wrong.
pub fn parse_call(v: &str) -> Result<(&str, Vec<Arg<'_>>), String> {
    let v = v.trim();
    let (name, args) = match v.find('(') {
        None => (v, None),
        Some(i) => match v.strip_suffix(')') {
            Some(inner) => (v[..i].trim_end(), Some(&inner[i + 1..])),
            None => return Err(format!("\"{v}\": missing `)`")),
        },
    };
    if !is_ident(name) {
        return Err(format!("\"{v}\": write a function name, optionally with arguments: `toggle` or `toggle(task)`"));
    }
    let mut out = Vec::new();
    let Some(args) = args.filter(|a| !a.trim().is_empty()) else { return Ok((name, out)) };
    let mut rest = args;
    loop {
        let t = rest.trim_start();
        let len = match t.chars().next() {
            Some(q @ ('\'' | '"')) => t[1..].find(q).map(|k| k + 2).ok_or_else(|| format!("\"{v}\": unclosed quote"))?,
            _ => t.find(',').unwrap_or(t.len()),
        };
        let arg = t[..len].trim();
        out.push(match arg {
            "event" => Arg::Event,
            "true" | "false" | "null" => Arg::Literal(arg),
            _ if arg.starts_with(['\'', '"']) || arg.parse::<f64>().is_ok_and(|n| n.is_finite()) => Arg::Literal(arg),
            _ if check_path(arg, false).is_ok() => Arg::Path(arg),
            _ => return Err(format!("\"{v}\": argument `{arg}` must be a path, a simple literal ('text', 2, true) or event; no expressions")),
        });
        let after = t[len..].trim_start();
        match after.strip_prefix(',') {
            Some(next) => rest = next,
            None if after.is_empty() => return Ok((name, out)),
            None => return Err(format!("\"{v}\": separate arguments with `,`")),
        }
    }
}

/// Event call back to text, with each path argument replaced by `map`.
pub fn join_call(name: &str, args: &[Arg], mut map: impl FnMut(&str) -> String) -> String {
    if args.is_empty() {
        return name.to_string();
    }
    let args: Vec<String> = args
        .iter()
        .map(|a| match a {
            Arg::Path(p) => map(p),
            Arg::Literal(l) => l.to_string(),
            Arg::Event => "event".into(),
        })
        .collect();
    format!("{name}({})", args.join(", "))
}

pub const ROOTS: &[&str] = &["data", "state", "shared", "params", "query", "page", "props", "$"];

/// `None`: not `data-*`. `Some(None)`: unknown directive.
pub fn parse(name: &str) -> Option<Option<Dir<'_>>> {
    let rest = name.strip_prefix("data-")?;
    let d = match rest {
        "text" => Dir::Text,
        "show" => Dir::Show,
        "if" => Dir::If,
        "else-if" => Dir::ElseIf,
        "else" => Dir::Else,
        "for" => Dir::For,
        "key" => Dir::Key,
        "empty" => Dir::Empty,
        "model" => Dir::Model,
        "open" => Dir::Open,
        "close" => Dir::Close,
        "page" | "reload" | "theme" => Dir::Marker,
        "c" | "tid" | "of" | "slot" => Dir::Generated,
        _ if rest.starts_with("prop-") => Dir::Generated,
        _ => {
            if let Some(n) = rest.strip_prefix("attr-") {
                Dir::Attr(n)
            } else if let Some(n) = rest.strip_prefix("class-") {
                Dir::Class(n)
            } else if let Some(n) = rest.strip_prefix("on-") {
                Dir::On(n)
            } else if let Some(n) = rest.strip_prefix("with-") {
                Dir::With(n)
            } else {
                return Some(None);
            }
        }
    };
    let empty_arg = matches!(d, Dir::Attr("") | Dir::Class("") | Dir::On("") | Dir::With(""));
    Some(if empty_arg { None } else { Some(d) })
}

pub fn is_ident(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Path `root.a.b`, with optional `!` when `neg_ok`.
pub fn check_path(v: &str, neg_ok: bool) -> Result<(), String> {
    let p = v.strip_prefix('!').unwrap_or(v);
    if p.len() != v.len() && !neg_ok {
        return Err("`!` is only valid in <if>, <else-if>, :show and .class".into());
    }
    if p.is_empty() {
        return Err("empty path".into());
    }
    if p.split('.').all(is_ident) {
        Ok(())
    } else {
        Err(format!("invalid path \"{v}\": use only root.a.b (no expressions, calls or indexes)"))
    }
}

pub fn split_neg(v: &str) -> (bool, &str) {
    match v.strip_prefix('!') {
        Some(p) => (true, p),
        None => (false, v),
    }
}

pub fn root(path: &str) -> &str {
    split_neg(path).1.split('.').next().unwrap_or("")
}

/// `x in path` -> (x, path).
pub fn parse_for(v: &str) -> Option<(&str, &str)> {
    let mut it = v.split_whitespace();
    let (var, kw, path) = (it.next()?, it.next()?, it.next()?);
    (kw == "in" && it.next().is_none() && is_ident(var)).then_some((var, path))
}

/// Replaces the path root (keeping `!`).
pub fn replace_root(v: &str, new_root: &str) -> String {
    let (neg, p) = split_neg(v);
    let rest = &p[root(p).len()..];
    format!("{}{new_root}{rest}", if neg { "!" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(t: &str) -> Part {
        Part::Text(t.into())
    }

    fn path(p: &str) -> Part {
        Part::Path(p.into())
    }

    #[test]
    fn templates() {
        assert_eq!(template("Hi {$name}, {data.n} items").unwrap(), [text("Hi "), path("$name"), text(", "), path("data.n"), text(" items")]);
        assert_eq!(template("{ task.title }").unwrap(), [path("task.title")]);
        assert_eq!(template(r"a \{b} c \ d").unwrap(), [text(r"a {b} c \ d")]);
        assert_eq!(template("x } y").unwrap(), [text("x } y")]);
        assert_eq!(template("a {b"), Err(2));
        assert_eq!(join_template(&template(r"/p/{p.id} \{x}").unwrap()), r"/p/{p.id} \{x}");
        assert_eq!(template_paths("{a.b} and {c}"), ["a.b", "c"]);
    }

    #[test]
    fn calls() {
        assert_eq!(parse_call("toggle").unwrap(), ("toggle", vec![]));
        assert_eq!(parse_call(" toggle() ").unwrap(), ("toggle", vec![]));
        assert_eq!(
            parse_call("search(event, $.q, 'a, b', \"x\", -2.5, true, null, task.id)").unwrap(),
            ("search", vec![Arg::Event, Arg::Path("$.q"), Arg::Literal("'a, b'"), Arg::Literal("\"x\""), Arg::Literal("-2.5"), Arg::Literal("true"), Arg::Literal("null"), Arg::Path("task.id")])
        );
        for bad in ["toggle(task", "toggle(a + b)", "toggle(f(x))", "obj.method", "toggle(a b)", "toggle('x)", "1x"] {
            assert!(parse_call(bad).is_err(), "{bad}");
        }
        let (name, args) = parse_call("set(task, 'done', event)").unwrap();
        assert_eq!(join_call(name, &args, |p| format!("state.{p}")), "set(state.task, 'done', event)");
    }

    #[test]
    fn generated_prop_attribute() {
        assert_eq!(parse("data-prop-step"), Some(Some(Dir::Generated)));
    }
}
