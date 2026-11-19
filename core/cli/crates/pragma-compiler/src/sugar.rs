use crate::directive::{self, Dir};
use crate::html::{Attr, Element, Node, Pos};
use crate::project::is_component_tag;

pub type SugarError = (Pos, &'static str, String);

const BOUND: &[&str] = &["text", "show", "model"];
const OLD_BLOCK: &[&str] = &["if", "else-if", "else", "for", "key", "empty"];

pub fn desugar(nodes: &mut [Node], errors: &mut Vec<SugarError>) {
    for node in nodes {
        let Node::Element(element) = node else { continue };
        if !block_tag(element, errors) {
            attrs(element, errors);
        }
        desugar(&mut element.children, errors);
    }
}

fn block_tag(e: &mut Element, errors: &mut Vec<SugarError>) -> bool {
    let tag = e.name.to_ascii_lowercase();
    let attrs: Vec<Attr> = std::mem::take(&mut e.attrs);
    let bare = |a: &Attr| a.value.is_none();
    let internal = match tag.as_str() {
        "if" | "else-if" => match attrs.as_slice() {
            [cond] if bare(cond) => vec![internal(cond, &format!("data-{tag}"), Some(expand_dollar(&cond.name, cond.pos, errors)))],
            _ => {
                errors.push((e.pos, "html/block", format!("use <{tag} path>, with the path unquoted (e.g. <{tag} page.loading>, <{tag} !$open>)")));
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
    for attr in &mut element.attrs {
        let lname = attr.name.to_ascii_lowercase();
        if let Some(Some(old)) = directive::parse(&lname)
            && !matches!(old, Dir::Marker | Dir::Generated)
        {
            errors.push((attr.pos, "html/legacy-syntax", format!("legacy syntax `{}`: use `{}`", attr.name, short_form(old))));
            continue;
        }
        let internal = if let Some(name) = lname.strip_prefix(':') {
            if OLD_BLOCK.contains(&name) {
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
            continue;
        };
        attr.name = internal;
        if let Some(value) = &attr.value {
            attr.value = Some(expand_dollar(value, attr.pos, errors));
        }
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

fn short_form(old: Dir) -> String {
    match old {
        Dir::Text => ":text".into(),
        Dir::Show => ":show".into(),
        Dir::If => block_hint("if").into(),
        Dir::ElseIf => block_hint("else-if").into(),
        Dir::Else => block_hint("else").into(),
        Dir::For | Dir::Key => block_hint("for").into(),
        Dir::Empty => block_hint("empty").into(),
        Dir::Model => ":model".into(),
        Dir::Attr(name) | Dir::With(name) => format!(":{name}"),
        Dir::Class(name) => format!(".{name}"),
        Dir::On(event) => format!("@{event}"),
        Dir::Open => "@open".into(),
        Dir::Close => "@close".into(),
        Dir::Marker | Dir::Generated => String::new(),
    }
}
