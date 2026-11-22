use crate::directive::{self, Dir, INTERP, Part};
use crate::html::{Attr, Element, Node, Pos};
use crate::project::is_component_tag;

pub type SugarError = (Pos, &'static str, String);

const BOUND: &[&str] = &["show", "model"];
const OLD_BLOCK: &[&str] = &["if", "else-if", "else", "for", "key", "empty"];
/// Content is not HTML: nothing inside is processed.
const RAW: &[&str] = &["script", "style"];
/// `{` is literal in text whose nearest parent element is one of these; child elements are processed.
const LITERAL: &[&str] = &["code", "pre"];

pub fn desugar(nodes: &mut Vec<Node>, errors: &mut Vec<SugarError>) {
    desugar_in(nodes, false, errors);
}

/// `literal`: `nodes` are the children of `<pre>` or `<code>`.
fn desugar_in(nodes: &mut Vec<Node>, literal: bool, errors: &mut Vec<SugarError>) {
    if !literal {
        split_text(nodes, errors);
    }
    for node in nodes.iter_mut() {
        let Node::Element(element) = node else { continue };
        if element.name == INTERP {
            continue;
        }
        if !block_tag(element, errors) {
            attrs(element, errors);
        }
        if RAW.iter().any(|t| element.is(t)) {
            continue;
        }
        let literal = LITERAL.iter().any(|t| element.is(t));
        if literal || !text_template(element, errors) {
            desugar_in(&mut element.children, literal, errors);
        }
    }
}

/// `{{ }}` and `${ }` are left for `html/interpolation`.
fn foreign(text: &str) -> bool {
    text.contains("{{") || text.contains("${")
}

/// Template of a text or attribute value, with `$x` expanded; `None` without `{`.
fn sugar_template(v: &str, pos: Pos, errors: &mut Vec<SugarError>) -> Option<Result<Vec<Part>, ()>> {
    if !directive::has_braces(v) || foreign(v) {
        return None;
    }
    let parts = match directive::template(v) {
        Ok(parts) => parts,
        Err(_) => {
            errors.push((pos, "html/invalid-path", "unclosed `{`: write {path} for a value, `\\{` for a literal brace".into()));
            return Some(Err(()));
        }
    };
    Some(Ok(parts
        .into_iter()
        .map(|p| match p {
            Part::Text(t) => Part::Text(t),
            Part::Path(p) => Part::Path(expand_dollar(&p, pos, errors)),
        })
        .collect()))
}

fn has_path(parts: &[Part]) -> bool {
    parts.iter().any(|p| matches!(p, Part::Path(_)))
}

fn literal_text(parts: &[Part]) -> String {
    parts.iter().map(|p| if let Part::Text(t) = p { t.as_str() } else { "" }).collect()
}

/// Element whose children are only text: `{}` makes the whole text a `data-text` template.
fn text_template(e: &mut Element, errors: &mut Vec<SugarError>) -> bool {
    if e.is("template") || is_component_tag(&e.name) || e.children.iter().any(|n| matches!(n, Node::Element(_))) {
        return false;
    }
    let text: String = e.children.iter().map(|n| if let Node::Text(t, _) = n { t.as_str() } else { "" }).collect();
    let pos = e.children.iter().find_map(|n| if let Node::Text(_, p) = n { Some(*p) } else { None }).unwrap_or(e.pos);
    match sugar_template(&text, pos, errors) {
        Some(Ok(parts)) if has_path(&parts) => {
            e.children.clear();
            e.attrs.push(Attr { name: "data-text".into(), value: Some(directive::join_template(&parts)), pos });
        }
        Some(Ok(parts)) => e.children = vec![Node::Text(literal_text(&parts), pos)],
        _ => {}
    }
    true
}

/// Text beside elements: each `{path}` becomes an internal `#interp` element (written as comments).
fn split_text(nodes: &mut Vec<Node>, errors: &mut Vec<SugarError>) {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes.drain(..) {
        let Node::Text(t, pos) = &node else {
            out.push(node);
            continue;
        };
        match sugar_template(t, *pos, errors) {
            Some(Ok(parts)) => {
                for part in parts {
                    out.push(match part {
                        Part::Text(text) => Node::Text(text, *pos),
                        Part::Path(p) => Node::Element(Element {
                            name: INTERP.into(),
                            attrs: vec![Attr { name: "data-text".into(), value: Some(format!("{{{p}}}")), pos: *pos }],
                            children: Vec::new(),
                            pos: *pos,
                        }),
                    });
                }
            }
            _ => out.push(node),
        }
    }
    *nodes = out;
}

fn block_tag(e: &mut Element, errors: &mut Vec<SugarError>) -> bool {
    let tag = e.name.to_ascii_lowercase();
    let attrs: Vec<Attr> = std::mem::take(&mut e.attrs);
    let bare = |a: &Attr| a.value.is_none();
    let operators = "no operators or quotes in a block tag: write a function in the script (`const hasTasks = () => tasks.length > 0`) and use";
    let internal = match tag.as_str() {
        "if" | "else-if" => match attrs.as_slice() {
            [cond] if bare(cond) => {
                // `<if $a.length > 0>`: the tag ended at the operator's `>`; the rest became text
                if let Some(Node::Text(t, pos)) = e.children.first_mut()
                    && let Some(end) = t.find('>').filter(|&i| !t[..i].contains('<') && !t[..i].trim().is_empty())
                {
                    errors.push((*pos, "html/invalid-path", format!("`<{tag} {} >{}`: {operators} <{tag} $hasTasks>", cond.name, &t[..end])));
                    *t = t[end + 1..].to_string();
                }
                vec![internal(cond, &format!("data-{tag}"), Some(expand_dollar(&cond.name, cond.pos, errors)))]
            }
            _ => {
                errors.push((e.pos, "html/block", format!("use <{tag} path>: one unquoted path (e.g. <{tag} page.loading>, <{tag} !$open>); {operators} <{tag} $hasTasks>")));
                vec![Attr { name: format!("data-{tag}"), value: Some(String::new()), pos: e.pos }]
            }
        },
        "else" | "empty" => {
            if let Some(a) = attrs.first() {
                errors.push((a.pos, "html/block", format!("<{tag}> takes no attributes")));
            }
            vec![Attr { name: format!("data-{tag}"), value: None, pos: e.pos }]
        }
        "for" => match attrs.as_slice() {
            [var, kw, path, key]
                if bare(var) && bare(kw) && bare(path) && kw.name == "in" && key.name.eq_ignore_ascii_case("key") && key.value.is_some() =>
            {
                let path = expand_dollar(&path.name, path.pos, errors);
                vec![internal(var, "data-for", Some(format!("{} in {path}", var.name))), internal(key, "data-key", key.value.clone())]
            }
            // no key: one error (`html/for-without-key`, from check), and the body still knows the variable
            [var, kw, path] if bare(var) && bare(kw) && bare(path) && kw.name == "in" => {
                let path = expand_dollar(&path.name, path.pos, errors);
                vec![internal(var, "data-for", Some(format!("{} in {path}", var.name)))]
            }
            _ => {
                errors.push((e.pos, "html/block", "use <for x in path key=x.field>".into()));
                vec![Attr { name: "data-for".into(), value: Some(String::new()), pos: e.pos }]
            }
        },
        _ => {
            e.attrs = attrs;
            return false;
        }
    };
    e.name = "template".into();
    e.attrs = internal;
    true
}

fn internal(a: &Attr, name: &str, value: Option<String>) -> Attr {
    Attr { name: name.into(), value, pos: a.pos }
}

fn attrs(element: &mut Element, errors: &mut Vec<SugarError>) {
    let component = is_component_tag(&element.name);
    let template = element.is("template");
    let mut out = Vec::with_capacity(element.attrs.len());
    for mut attr in std::mem::take(&mut element.attrs) {
        let lname = attr.name.to_ascii_lowercase();
        if let Some(Some(old)) = directive::parse(&lname)
            && !matches!(old, Dir::Marker | Dir::Generated)
        {
            errors.push((attr.pos, "html/data-attribute", format!("`{lname}` is internal; write `{}`", written_form(old))));
            out.push(attr);
            continue;
        }
        if lname == ":text" {
            let path = attr.value.as_deref().unwrap_or("path");
            errors.push((attr.pos, "html/text-removed", format!("`:text` was removed: write {{{path}}} inside the element (e.g. <p>{{$name}}</p>)")));
            continue;
        }
        let internal = if let Some(name) = lname.strip_prefix(':') {
            if let Some(v) = attr.value.as_deref().filter(|v| v.contains('{')) {
                let bare = v.trim().trim_start_matches('{').trim_end_matches('}');
                let msg = format!("`:{name}` takes a bare path: `:{name}=\"{bare}\"`; to build text write `{name}=\"… {{{bare}}}\"` (no colon)");
                errors.push((attr.pos, "html/invalid-path", msg));
                continue;
            }
            if let Some(var) = name.strip_prefix("--") {
                let written = &attr.name[3..];
                if !directive::is_css_var_name(written) {
                    let fix = directive::kebab(written);
                    errors.push((attr.pos, "html/css-var", format!("`:--{written}`: write the custom property name in lowercase kebab-case (`:--{fix}`), and the same name in CSS (`var(--{fix})`)")));
                }
                format!("data-var-{var}")
            } else if OLD_BLOCK.contains(&name) {
                let hint = if template || name != "key" { block_hint(name) } else { "<for x in path key=x.field>" };
                errors.push((attr.pos, "html/legacy-syntax", format!("legacy syntax `{}`: use `{hint}`", attr.name)));
                format!("data-{name}")
            } else if BOUND.contains(&name) {
                format!("data-{name}")
            } else if component {
                format!("data-with-{name}")
            } else {
                format!("data-attr-{name}")
            }
        } else if let Some(name) = lname.strip_prefix('@') {
            if matches!(name, "open" | "close") { format!("data-{name}") } else { format!("data-on-{name}") }
        } else if let Some(name) = lname.strip_prefix('.') {
            format!("data-class-{name}")
        } else {
            attr_template(&mut attr, &lname, component, errors);
            out.push(attr);
            continue;
        };
        if let Some(value) = &attr.value {
            let takes_literal = ["data-attr-", "data-var-", "data-with-"].iter().any(|p| internal.starts_with(p));
            attr.value = Some(if internal.starts_with("data-on-") {
                expand_call(value, attr.pos, errors)
            } else if takes_literal && directive::literal(value).is_some() {
                value.trim().to_string()
            } else {
                expand_dollar(value, attr.pos, errors)
            });
        }
        attr.name = internal;
        out.push(attr);
    }
    element.attrs = out;
}

/// `alt="Photo of {$name}"` → `data-attr-alt` template; `\{` alone → literal brace.
fn attr_template(attr: &mut Attr, lname: &str, component: bool, errors: &mut Vec<SugarError>) {
    let Some(value) = attr.value.as_deref() else { return };
    match sugar_template(value, attr.pos, errors) {
        Some(Ok(parts)) if has_path(&parts) => {
            if component {
                errors.push((attr.pos, "html/component", format!("a prop value cannot use {{}}: pass a path with :{lname}=\"path\" (build the text in the script)")));
            } else if lname == "class" {
                errors.push((attr.pos, "html/class", "no {} in class: write static classes in class=\"…\" and toggle each one with .name=\"path\"".to_string()));
            } else {
                attr.name = format!("data-attr-{lname}");
                attr.value = Some(directive::join_template(&parts));
            }
        }
        Some(Ok(parts)) => attr.value = Some(literal_text(&parts)),
        _ => {}
    }
}

/// `toggle($item, 'x')` → `toggle($.item, 'x')`; invalid calls are kept for `check`.
fn expand_call(value: &str, pos: Pos, errors: &mut Vec<SugarError>) -> String {
    match directive::parse_call(value) {
        Ok(call) => directive::join_call(&call, |p| expand_dollar(p, pos, errors)),
        Err(_) => value.to_string(),
    }
}

fn block_hint(name: &str) -> &'static str {
    match name {
        "if" => "<if path>",
        "else-if" => "<else-if path>",
        "else" => "<else>",
        "empty" => "<empty>",
        _ => "<for x in path key=x.field>",
    }
}

fn expand_dollar(value: &str, pos: Pos, errors: &mut Vec<SugarError>) -> String {
    value
        .split(' ')
        .map(|token| {
            let (negated, path) = directive::split_neg(token);
            let bang = if negated { "!" } else { "" };
            if let Some(rest) = path.strip_prefix('$') {
                return format!("{bang}$.{rest}");
            }
            if matches!(directive::root(path), "state" | "props") && path.contains('.') {
                let rest = path.split_once('.').map_or("", |(_, r)| r);
                errors.push((pos, "html/legacy-syntax", format!("`{path}` is legacy syntax: use `${rest}`")));
            }
            token.to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn written_form(old: Dir) -> String {
    let short = short_form(old);
    match old {
        Dir::Text | Dir::If | Dir::ElseIf | Dir::Else | Dir::For | Dir::Key | Dir::Empty | Dir::Close => short,
        Dir::Open => format!("{short}=\"overlay-id\""),
        _ => format!("{short}=\"...\""),
    }
}

fn short_form(old: Dir) -> String {
    match old {
        Dir::Text => "{path} inside the element".into(),
        Dir::Show => ":show".into(),
        Dir::If => block_hint("if").into(),
        Dir::ElseIf => block_hint("else-if").into(),
        Dir::Else => block_hint("else").into(),
        Dir::For | Dir::Key => block_hint("for").into(),
        Dir::Empty => block_hint("empty").into(),
        Dir::Model => ":model".into(),
        Dir::Attr(name) | Dir::With(name) => format!(":{name}"),
        Dir::Var(name) => format!(":--{name}"),
        Dir::Class(name) => format!(".{name}"),
        Dir::On(event) => format!("@{event}"),
        Dir::Open => "@open".into(),
        Dir::Close => "@close".into(),
        Dir::Marker | Dir::Generated => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use crate::compile::{hoist, without_ids, write_nodes};
    use crate::html::parse;

    /// The HTML the build writes for `src` (before component expansion), block bodies after `|`.
    fn sugar(src: &str) -> (String, Vec<(&'static str, String)>) {
        let (mut nodes, _) = parse(src, 0);
        let mut errors = Vec::new();
        super::desugar(&mut nodes, &mut errors);
        let bodies = hoist(&mut nodes);
        let mut out = String::new();
        write_nodes(&nodes, &mut out, false);
        for b in bodies {
            out.push('|');
            write_nodes(&b.children, &mut out, false);
        }
        (without_ids(&out), errors.into_iter().map(|(_, rule, msg)| (rule, msg)).collect())
    }

    #[test]
    fn text_only_element_becomes_template() {
        assert_eq!(sugar("<p>Hi {$name}, {$count} items</p>").0, "<p data-text=\"Hi {$.name}, {$.count} items\"></p>");
        assert_eq!(sugar("<li>{task.title}</li>").0, "<li data-text=\"{task.title}\"></li>");
        assert_eq!(sugar("<title>{data.product.name} | Shop</title>").0, "<title data-text=\"{data.product.name} | Shop\"></title>");
        assert_eq!(sugar("<p>a \\{b} c</p>").0, "<p>a {b} c</p>");
    }

    #[test]
    fn text_beside_elements_gets_markers() {
        let (out, errors) = sugar("<label><input type=\"checkbox\"> {task.title} ({$n})</label>");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(out, "<label><input type=\"checkbox\"> <!--{task.title}--><!--}--> (<!--{$.n}--><!--}-->)</label>");
        assert_eq!(sugar("<c-button>Save {$n}</c-button>").0, "<c-button>Save <!--{$.n}--><!--}--></c-button>");
    }

    #[test]
    fn literal_in_code_and_pre() {
        assert_eq!(sugar("<p>Use <code>{$name}</code></p>").0, "<p>Use <code>{$name}</code></p>");
        assert_eq!(sugar("<pre>{ a: 1 }</pre>").0, "<pre>{ a: 1 }</pre>");
        assert_eq!(sugar("<script>if (a) { b() }</script><style>a { color: red }</style>").0, "<script>if (a) { b() }</script><style>a { color: red }</style>");
    }

    #[test]
    fn directives_blocks_and_child_text_inside_pre_and_code() {
        let (out, errors) = sugar("<pre :aria-label=\"$path\"><code><for line in $lines key=line.n><span :data-n=\"line.n\">{line.text}</span></for></code></pre>");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(out, "<pre data-attr-aria-label=\"$.path\"><code><!--p:for * line $.lines line.n--></code></pre>|<span data-attr-data-n=\"line.n\" data-text=\"{line.text}\"></span>");
        assert_eq!(sugar("<pre>$ curl {url} <span>{$path}</span></pre>").0, "<pre>$ curl {url} <span data-text=\"{$.path}\"></span></pre>");
        assert_eq!(sugar("<code>a <b>{$x} and <i>y</i> {$z}</b> {raw}</code>").0, "<code>a <b><!--{$.x}--><!--}--> and <i>y</i> <!--{$.z}--><!--}--></b> {raw}</code>");
        assert_eq!(sugar("<pre><button @click=\"copy(line)\" .on=\"$on\">c</button></pre>").0, "<pre><button data-on-click=\"copy(line)\" data-class-on=\"$.on\">c</button></pre>");
        assert_eq!(sugar("<pre><span :text=\"$x\"></span></pre>").1[0].0, "html/text-removed");
    }

    #[test]
    fn attribute_templates() {
        assert_eq!(sugar("<img alt=\"Photo of {$name}\" src=\"/a.png\">").0, "<img data-attr-alt=\"Photo of {$.name}\" src=\"/a.png\">");
        assert_eq!(sugar("<a href=\"/shop/{p.id}\">x</a>").0, "<a data-attr-href=\"/shop/{p.id}\">x</a>");
        assert_eq!(sugar("<input pattern=\"\\d\\{3}\">").0, "<input pattern=\"\\d{3}\">");
        let (_, errors) = sugar("<p class=\"a {$b}\">x</p>");
        assert_eq!(errors[0].0, "html/class");
        let (_, errors) = sugar("<c-card title=\"Hi {$name}\"></c-card>");
        assert_eq!(errors[0].0, "html/component");
    }

    #[test]
    fn event_calls_expand_dollar() {
        assert_eq!(sugar("<button @click=\"set($item, 'x', event)\">x</button>").0, "<button data-on-click=\"set($.item, 'x', event)\">x</button>");
        assert_eq!(sugar("<input @change=\"toggle(task)\">").0, "<input data-on-change=\"toggle(task)\">");
    }

    #[test]
    fn literal_values_stay_as_written() {
        let (out, errors) = sugar("<p :title=\"'$5 off'\" :--x=\"-2.5\" :aria-busy=\"false\">x</p><c-day :holiday=\"false\" :label=\"'a b'\"></c-day>");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(out, "<p data-attr-title=\"'$5 off'\" data-var-x=\"-2.5\" data-attr-aria-busy=\"false\">x</p><c-day data-with-holiday=\"false\" data-with-label=\"'a b'\"></c-day>");
    }

    #[test]
    fn text_directive_removed_and_errors() {
        let (out, errors) = sugar("<p :text=\"$name\"></p>");
        assert_eq!(out, "<p></p>");
        assert_eq!(errors, [("html/text-removed", "`:text` was removed: write {$name} inside the element (e.g. <p>{$name}</p>)".to_string())]);
        assert_eq!(sugar("<p>a {b</p>").1[0].0, "html/invalid-path");
        assert!(sugar("<p>{{ x }}</p>").1.is_empty(), "left to html/interpolation");
        assert_eq!(sugar("<p data-text=\"x\"></p>").1[0].1, "`data-text` is internal; write `{path} inside the element`");
    }
}
