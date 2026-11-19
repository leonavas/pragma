//! Diretivas (SPEC §3): leitura dos nomes e dos caminhos.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir<'a> {
    Text,
    Attr(&'a str),
    Class(&'a str),
    Show,
    If,
    Else,
    For,
    Key,
    Empty,
    On(&'a str),
    Model,
    With(&'a str),
    Open,
    Close,
    /// `data-page`, `data-reload`, `data-theme`: marcadores sem caminho.
    Marker,
    /// `data-c`, `data-tid`, `data-of`, `data-slot`: escritos só pelo build.
    Generated,
}

pub const ROOTS: &[&str] = &["data", "state", "shared", "params", "query", "page", "props", "$"];

/// `None`: não é `data-*`. `Some(None)`: diretiva desconhecida.
pub fn parse(name: &str) -> Option<Option<Dir<'_>>> {
    let rest = name.strip_prefix("data-")?;
    let d = match rest {
        "text" => Dir::Text,
        "show" => Dir::Show,
        "if" => Dir::If,
        "else" => Dir::Else,
        "for" => Dir::For,
        "key" => Dir::Key,
        "empty" => Dir::Empty,
        "model" => Dir::Model,
        "open" => Dir::Open,
        "close" => Dir::Close,
        "page" | "reload" | "theme" => Dir::Marker,
        "c" | "tid" | "of" | "slot" => Dir::Generated,
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

/// Caminho `raiz.a.b`, com `!` opcional quando `neg_ok`.
pub fn check_path(v: &str, neg_ok: bool) -> Result<(), String> {
    let p = v.strip_prefix('!').unwrap_or(v);
    if p.len() != v.len() && !neg_ok {
        return Err("`!` só vale em data-if, data-show e data-class-*".into());
    }
    if p.is_empty() {
        return Err("caminho vazio".into());
    }
    if p.split('.').all(is_ident) {
        Ok(())
    } else {
        Err(format!("caminho inválido \"{v}\": use só raiz.a.b (sem expressões, chamadas ou índices)"))
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

/// `x in caminho` → (x, caminho).
pub fn parse_for(v: &str) -> Option<(&str, &str)> {
    let mut it = v.split_whitespace();
    let (var, kw, path) = (it.next()?, it.next()?, it.next()?);
    (kw == "in" && it.next().is_none() && is_ident(var)).then_some((var, path))
}

/// Troca a raiz do caminho (preservando `!`).
pub fn replace_root(v: &str, new_root: &str) -> String {
    let (neg, p) = split_neg(v);
    let rest = &p[root(p).len()..];
    format!("{}{new_root}{rest}", if neg { "!" } else { "" })
}
