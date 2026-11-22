//! Directives (SPEC §3): parsing names and paths.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir<'a> {
    Text,
    Attr(&'a str),
    /// `:--name="path"`: CSS custom property `--name` on the element.
    Var(&'a str),
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
    /// `data-c`, `data-tid`, `data-of`, `data-slot`, `data-self`, `data-def`, `data-prop-*`: written only by the build.
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
    /// The DOM event.
    Event,
    /// The element with the `@event` directive.
    El,
}

/// `fn`, `fn(arg, …)`, or a store function `shared.cart.add(…)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call<'a> {
    pub name: &'a str,
    /// `None` without parentheses: the function gets the event.
    pub args: Option<Vec<Arg<'a>>>,
}

impl Call<'_> {
    /// `shared.<store>.<fn>`.
    pub fn is_store(&self) -> bool {
        self.name.starts_with("shared.")
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.args.iter().flatten().filter_map(|a| if let Arg::Path(p) = a { Some(*p) } else { None })
    }
}

/// `fn`, `fn(arg, …)` or `shared.store.fn(…)`. Err: what is wrong.
pub fn parse_call(v: &str) -> Result<Call<'_>, String> {
    let v = v.trim();
    let (name, args) = match v.find('(') {
        None => (v, None),
        Some(i) => match v.strip_suffix(')') {
            Some(inner) => (v[..i].trim_end(), Some(&inner[i + 1..])),
            None => return Err(format!("\"{v}\": missing `)`")),
        },
    };
    if let Some(bare) = name.strip_prefix('$') {
        return Err(format!("\"{v}\": no `$` in events, write `{}` (`$` is for values: `{{$x}}`, `:attr=\"$x\"`)", v.replacen(name, bare, 1)));
    }
    let store = name.split('.').count() == 3 && name.starts_with("shared.") && name.split('.').all(is_ident);
    if !is_ident(name) && !store {
        return Err(format!("\"{v}\": call a function of the script (`toggle`, `toggle(task)`) or of a store (`shared.cart.add(item)`)"));
    }
    let Some(args) = args else { return Ok(Call { name, args: None }) };
    let mut out = Vec::new();
    if args.trim().is_empty() {
        return Ok(Call { name, args: Some(out) });
    }
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
            "el" => Arg::El,
            "true" | "false" | "null" => Arg::Literal(arg),
            _ if arg.starts_with(['\'', '"']) || arg.parse::<f64>().is_ok_and(|n| n.is_finite()) => Arg::Literal(arg),
            _ if check_path(arg, false).is_ok() => Arg::Path(arg),
            _ => return Err(format!("\"{v}\": argument `{}` must be a path, a simple literal ('text', 2, true), event or el; no expressions: compute it in a function of the script", source_form(arg))),
        });
        let after = t[len..].trim_start();
        match after.strip_prefix(',') {
            Some(next) => rest = next,
            None if after.is_empty() => return Ok(Call { name, args: Some(out) }),
            None => return Err(format!("\"{v}\": separate arguments with `,`")),
        }
    }
}

/// Event call back to text, with each path argument replaced by `map` (`fn` and `fn()` stay apart).
pub fn join_call(call: &Call, mut map: impl FnMut(&str) -> String) -> String {
    let Some(args) = &call.args else { return call.name.to_string() };
    let args: Vec<String> = args
        .iter()
        .map(|a| match a {
            Arg::Path(p) => map(p),
            Arg::Literal(l) => l.to_string(),
            Arg::Event => "event".into(),
            Arg::El => "el".into(),
        })
        .collect();
    format!("{}({})", call.name, args.join(", "))
}

/// The closest name to `wrong` among `names` (edit distance ≤ 2, or ≤ 1 for short names).
pub fn did_you_mean<'a>(wrong: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let max = if wrong.chars().count() <= 3 { 1 } else { 2 };
    names
        .into_iter()
        .filter(|n| *n != wrong)
        .map(|n| (distance(&wrong.to_lowercase(), &n.to_lowercase()), n))
        .filter(|(d, _)| *d <= max)
        .min_by_key(|(d, n)| (*d, *n))
        .map(|(_, n)| n)
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
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
        "c" | "tid" | "of" | "slot" | "self" | "def" => Dir::Generated,
        _ if rest.starts_with("prop-") => Dir::Generated,
        _ => {
            if let Some(n) = rest.strip_prefix("attr-") {
                Dir::Attr(n)
            } else if let Some(n) = rest.strip_prefix("var-") {
                Dir::Var(n)
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
    let empty_arg = matches!(d, Dir::Attr("") | Dir::Var("") | Dir::Class("") | Dir::On("") | Dir::With(""));
    Some(if empty_arg { None } else { Some(d) })
}

/// Name after `--` in `:--name`: lowercase kebab-case (HTML lowercases attribute names, CSS does not).
pub fn is_css_var_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase()) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `barLeft`, `bar_left` → `bar-left`.
pub fn kebab(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() && !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        out.push(if c == '_' { '-' } else { c.to_ascii_lowercase() });
    }
    out.trim_start_matches('-').to_string()
}

/// A CSS custom property value known at build time; `None` when it could add declarations or markup.
pub fn css_value(v: &str) -> Option<&str> {
    (!v.contains([';', '{', '}', '<'])).then_some(v.trim())
}

/// A JS literal written as the value of `:attr`, `:--name` or a prop: `true`, `false`, `null`,
/// a number (`-2.5`) or a quoted string (`'x'`, `"x"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Null,
    Number(String),
    Text(String),
}

impl Literal {
    /// As a static attribute: `None` leaves it out, `Some(None)` writes it empty.
    pub fn attr(&self) -> Option<Option<String>> {
        match self {
            Literal::Bool(false) | Literal::Null => None,
            Literal::Bool(true) => Some(None),
            Literal::Number(n) => Some(Some(n.clone())),
            Literal::Text(t) => Some(Some(t.clone())),
        }
    }

    pub fn json(&self) -> serde_json::Value {
        match self {
            Literal::Bool(b) => serde_json::Value::Bool(*b),
            Literal::Null => serde_json::Value::Null,
            Literal::Number(n) => serde_json::from_str(n).unwrap_or(serde_json::Value::Null),
            Literal::Text(t) => serde_json::Value::String(t.clone()),
        }
    }
}

pub fn literal(v: &str) -> Option<Literal> {
    let v = v.trim();
    match v {
        "true" => return Some(Literal::Bool(true)),
        "false" => return Some(Literal::Bool(false)),
        "null" => return Some(Literal::Null),
        _ => {}
    }
    if let Some(q @ ('\'' | '"')) = v.chars().next() {
        let inner = v[1..].strip_suffix(q)?;
        return (!inner.contains(q)).then(|| Literal::Text(inner.to_string()));
    }
    let digits = v.strip_prefix('-').unwrap_or(v);
    let (int, frac) = digits.split_once('.').unwrap_or((digits, "0"));
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (all_digits(int) && all_digits(frac)).then(|| Literal::Number(v.to_string()))
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
        let hint = if p.find('(').is_some_and(|i| !p[i + 1..].trim_start().starts_with(')')) {
            "a value per item is a field: add it in the function that returns the list (`const rows = () => tasks.map(task => ({ ...task, price: format(task.amount) }))`) and use row.price"
        } else {
            "move the logic into a function in <script> (`const total = () => …`) and use $total"
        };
        Err(format!("invalid path \"{}\": use only root.a.b (no expressions, calls or indexes); {hint}", source_form(v)))
    }
}

/// A path as the author wrote it: `$x` (the build reads it as `$.x`).
pub fn source_form(v: &str) -> String {
    v.replace("$.", "$")
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

/// `<script type="application/ld+json">`: structured data (a head tag of the page, never a module).
pub fn is_json_ld(e: &crate::html::Element) -> bool {
    e.is("script") && e.value("type").is_some_and(|t| t.trim().eq_ignore_ascii_case("application/ld+json"))
}

/// `{path}` inside the strings of a JSON-LD text: (byte offset of `{`, byte length of `{…}`, path).
/// Err: byte offset of a `{` without `}` in its string.
pub fn json_ld_paths(text: &str) -> Result<Vec<(usize, usize, &str)>, usize> {
    let b = text.as_bytes();
    let (mut out, mut i, mut in_string) = (Vec::new(), 0, false);
    while i < b.len() {
        match b[i] {
            b'\\' if in_string => i += 1,
            b'"' => in_string = !in_string,
            b'{' if in_string => {
                let rest = &text[i + 1..];
                match rest.find(['}', '"']) {
                    Some(k) if rest.as_bytes()[k] == b'}' => {
                        out.push((i, k + 2, rest[..k].trim()));
                        i += k + 2;
                        continue;
                    }
                    _ => return Err(i),
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(out)
}

/// Checks the JSON of a JSON-LD text (each `{path}` counts as string content).
/// Err: (byte offset, message).
pub fn json_ld_check(text: &str) -> Result<(), (usize, String)> {
    let mut plain = text.to_string();
    for (at, len, _) in json_ld_paths(text).map_err(|at| (at, "unclosed `{` in a JSON string: write {path}, or \\u007b for a literal brace".to_string()))? {
        plain.replace_range(at..at + len, &"x".repeat(len));
    }
    if plain.trim().is_empty() {
        return Err((0, "empty JSON-LD".into()));
    }
    serde_json::from_str::<serde_json::Value>(&plain).map(|_| ()).map_err(|e| {
        let at = plain.split_inclusive('\n').take(e.line().saturating_sub(1)).map(str::len).sum::<usize>() + e.column().saturating_sub(1);
        let msg = e.to_string();
        (at.min(text.len()), msg.split(" at line ").next().unwrap_or(&msg).to_string())
    })
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
        assert_eq!(parse_call("toggle").unwrap(), Call { name: "toggle", args: None });
        assert_eq!(parse_call(" toggle() ").unwrap(), Call { name: "toggle", args: Some(vec![]) });
        assert_eq!(
            parse_call("search(event, $.q, 'a, b', \"x\", -2.5, true, null, task.id, el)").unwrap().args.unwrap(),
            [Arg::Event, Arg::Path("$.q"), Arg::Literal("'a, b'"), Arg::Literal("\"x\""), Arg::Literal("-2.5"), Arg::Literal("true"), Arg::Literal("null"), Arg::Path("task.id"), Arg::El]
        );
        let store = parse_call("shared.cart.remove(item.id)").unwrap();
        assert!(store.is_store() && store.paths().eq(["item.id"]));
        for bad in ["toggle(task", "toggle(a + b)", "toggle(f(x))", "obj.method", "shared.cart", "shared.cart.a.b", "toggle(a b)", "toggle('x)", "1x"] {
            assert!(parse_call(bad).is_err(), "{bad}");
        }
        assert!(parse_call("$increase").unwrap_err().contains("no `$` in events, write `increase`"));
        assert!(parse_call("$set(x)").unwrap_err().contains("write `set(x)`"));
        let call = parse_call("set(task, 'done', event)").unwrap();
        assert_eq!(join_call(&call, |p| format!("state.{p}")), "set(state.task, 'done', event)");
        assert_eq!(join_call(&parse_call("reset()").unwrap(), |p| p.into()), "reset()");
        assert_eq!(join_call(&parse_call("reset").unwrap(), |p| p.into()), "reset");
    }

    #[test]
    fn suggestions() {
        assert_eq!(did_you_mean("cuont", ["count", "items", "total"]), Some("count"));
        assert_eq!(did_you_mean("itms", ["count", "items"]), Some("items"));
        assert_eq!(did_you_mean("zzz", ["count", "items"]), None);
        assert_eq!(did_you_mean("ab", ["ac", "xyz"]), Some("ac"));
        assert_eq!(did_you_mean("Count", ["count"]), Some("count"));
    }

    #[test]
    fn json_ld() {
        let text = "{\"@type\": \"Article\", \"author\": {\"name\": \"{data.post.author}\"}, \"headline\": \"News: { data.post.title }!\", \"q\": \"a\\\"{x}\"}";
        let paths: Vec<&str> = json_ld_paths(text).unwrap().into_iter().map(|p| p.2).collect();
        assert_eq!(paths, ["data.post.author", "data.post.title", "x"]);
        assert_eq!(json_ld_check(text), Ok(()));
        assert_eq!(json_ld_paths("{\"a\": \"{b\"}"), Err(7));
        let (at, msg) = json_ld_check("{\n  \"a\": 1,\n  \"b\": }").unwrap_err();
        assert_eq!((at, msg.as_str()), (19, "expected value"));
        assert!(json_ld_check("  ").is_err());
    }

    #[test]
    fn generated_prop_attribute() {
        assert_eq!(parse("data-prop-step"), Some(Some(Dir::Generated)));
        assert_eq!(parse("data-self"), Some(Some(Dir::Generated)));
        assert_eq!(parse("data-def"), Some(Some(Dir::Generated)));
    }

    #[test]
    fn literal_values() {
        assert_eq!(literal("false"), Some(Literal::Bool(false)));
        assert_eq!(literal(" true "), Some(Literal::Bool(true)));
        assert_eq!(literal("null"), Some(Literal::Null));
        assert_eq!(literal("-2.5"), Some(Literal::Number("-2.5".into())));
        assert_eq!(literal("42"), Some(Literal::Number("42".into())));
        assert_eq!(literal("'a b'"), Some(Literal::Text("a b".into())));
        assert_eq!(literal("\"it's\""), Some(Literal::Text("it's".into())));
        assert_eq!(literal("''"), Some(Literal::Text(String::new())));
        for path in ["$.x", "task.done", "falsey", "1.", ".5", "1e3", "'a'b'", "'a", "-", "NaN"] {
            assert_eq!(literal(path), None, "{path}");
        }
        assert_eq!(Literal::Bool(false).attr(), None);
        assert_eq!(Literal::Bool(true).attr(), Some(None));
        assert_eq!(Literal::Number("3".into()).attr(), Some(Some("3".into())));
        assert_eq!(Literal::Number("-2.5".into()).json(), serde_json::json!(-2.5));
        assert_eq!(Literal::Text("x".into()).json(), serde_json::json!("x"));
    }

    #[test]
    fn css_variable_directive() {
        assert_eq!(parse("data-var-left"), Some(Some(Dir::Var("left"))));
        assert_eq!(parse("data-var-"), Some(None));
        assert!(is_css_var_name("bar-left") && is_css_var_name("x2"));
        assert_eq!(kebab("barLeft_x"), "bar-left-x");
        assert!(!is_css_var_name("barLeft") && !is_css_var_name("-x") && !is_css_var_name("") && !is_css_var_name("a_b"));
        assert_eq!(css_value("12.5"), Some("12.5"));
        assert_eq!(css_value(" 4px "), Some("4px"));
        for bad in ["1;color:red", "a{", "}", "<b>"] {
            assert_eq!(css_value(bad), None, "{bad}");
        }
    }
}
