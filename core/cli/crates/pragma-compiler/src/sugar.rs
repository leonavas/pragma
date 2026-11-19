use crate::directive::{self, Dir};
use crate::html::{Node, Pos};
use crate::project::is_component_tag;

const BOUND: &[&str] = &["text", "show", "if", "else", "for", "key", "empty", "model"];

pub fn desugar(nodes: &mut [Node], errors: &mut Vec<(Pos, String)>) {
    for node in nodes {
        let Node::Element(element) = node else { continue };
        let component = is_component_tag(&element.name);
        for attr in &mut element.attrs {
            let lname = attr.name.to_ascii_lowercase();
            if let Some(Some(old)) = directive::parse(&lname)
                && !matches!(old, Dir::Marker | Dir::Generated)
            {
                errors.push((attr.pos, format!("sintaxe antiga `{}`: use `{}`", attr.name, short_form(old))));
                continue;
            }
            let internal = if let Some(name) = lname.strip_prefix(':') {
                if BOUND.contains(&name) {
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
        desugar(&mut element.children, errors);
    }
}

fn expand_dollar(value: &str, pos: Pos, errors: &mut Vec<(Pos, String)>) -> String {
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
                errors.push((pos, format!("`{path}` é sintaxe antiga: use `${rest}`")));
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
        Dir::If => ":if".into(),
        Dir::Else => ":else".into(),
        Dir::For => ":for".into(),
        Dir::Key => ":key".into(),
        Dir::Empty => ":empty".into(),
        Dir::Model => ":model".into(),
        Dir::Attr(name) | Dir::With(name) => format!(":{name}"),
        Dir::Class(name) => format!(".{name}"),
        Dir::On(event) => format!("@{event}"),
        Dir::Open => "@open".into(),
        Dir::Close => "@close".into(),
        Dir::Marker | Dir::Generated => String::new(),
    }
}
