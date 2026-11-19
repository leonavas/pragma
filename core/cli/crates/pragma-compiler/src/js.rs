//! Leitura leve de JS: só o necessário para achar imports e alguns exports.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ident,
    Str,
    Punct,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
pub struct Import {
    /// Intervalo do conteúdo da string (sem aspas).
    pub start: usize,
    pub end: usize,
    pub spec: String,
    /// `import('x')`: carregado sob demanda, fora do modulepreload.
    pub dynamic: bool,
}

const REGEX_AFTER: &[&str] = &[
    "return", "typeof", "case", "do", "else", "in", "instanceof", "new", "delete", "void", "throw", "yield", "await", "of",
];

fn tokens(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut out: Vec<Tok> = Vec::new();
    // profundidade de chaves de cada `${` aberto
    let mut tpl: Vec<u32> = Vec::new();
    let mut depth: u32 = 0;
    let mut i = 0;
    let regex_ok = |out: &Vec<Tok>| match out.last() {
        None => true,
        Some(t) => match t.kind {
            Kind::Ident => REGEX_AFTER.contains(&&src[t.start..t.end]),
            Kind::Str | Kind::Other => false,
            Kind::Punct => !matches!(b[t.start], b')' | b']' | b'}'),
        },
    };
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i = src[i + 2..].find("*/").map_or(b.len(), |k| i + 2 + k + 2);
        } else if c == b'"' || c == b'\'' {
            let s = i;
            i += 1;
            while i < b.len() && b[i] != c && b[i] != b'\n' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(b.len());
            out.push(Tok { kind: Kind::Str, start: s, end: i });
        } else if c == b'`' || (c == b'}' && tpl.last() == Some(&depth)) {
            if c == b'}' {
                tpl.pop();
            }
            let s = i;
            i += 1;
            loop {
                if i >= b.len() {
                    break;
                }
                match b[i] {
                    b'\\' => i += 2,
                    b'`' => {
                        i += 1;
                        break;
                    }
                    b'$' if b.get(i + 1) == Some(&b'{') => {
                        i += 2;
                        tpl.push(depth);
                        break;
                    }
                    _ => i += 1,
                }
            }
            out.push(Tok { kind: Kind::Other, start: s, end: i.min(b.len()) });
        } else if c == b'/' && regex_ok(&out) {
            let s = i;
            i += 1;
            let mut class = false;
            while i < b.len() && b[i] != b'\n' {
                match b[i] {
                    b'\\' => i += 1,
                    b'[' => class = true,
                    b']' => class = false,
                    b'/' if !class => break,
                    _ => {}
                }
                i += 1;
            }
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            out.push(Tok { kind: Kind::Other, start: s, end: i.min(b.len()) });
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'$' || c >= 0x80 {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$' || b[i] >= 0x80) {
                i += 1;
            }
            out.push(Tok { kind: Kind::Ident, start: s, end: i });
        } else if c.is_ascii_digit() {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'.' || b[i] == b'_') {
                i += 1;
            }
            out.push(Tok { kind: Kind::Other, start: s, end: i });
        } else {
            match c {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            out.push(Tok { kind: Kind::Punct, start: i, end: i + 1 });
            i += 1;
        }
    }
    out
}

fn text<'a>(src: &'a str, t: &Tok) -> &'a str {
    &src[t.start..t.end]
}

fn str_import(src: &str, t: &Tok, dynamic: bool) -> Import {
    Import { start: t.start + 1, end: t.end - 1, spec: src[t.start + 1..t.end - 1].to_string(), dynamic }
}

pub fn imports(src: &str) -> Vec<Import> {
    let toks = tokens(src);
    let mut out = Vec::new();
    let is = |i: usize, s: &str| toks.get(i).is_some_and(|t| text(src, t) == s);
    for (i, t) in toks.iter().enumerate() {
        if t.kind != Kind::Ident || (i > 0 && is(i - 1, ".")) {
            continue;
        }
        let word = text(src, t);
        if word != "import" && word != "export" {
            continue;
        }
        let next = match toks.get(i + 1) {
            Some(n) => n,
            None => continue,
        };
        if word == "import" && next.kind == Kind::Str {
            out.push(str_import(src, next, false));
            continue;
        }
        if word == "import" && text(src, next) == "(" {
            if let (Some(s), true) = (toks.get(i + 2), is(i + 3, ")"))
                && s.kind == Kind::Str {
                    out.push(str_import(src, s, true));
                }
            continue;
        }
        if word == "import" && text(src, next) == "." {
            continue;
        }
        if word == "export" && !matches!(text(src, next), "*" | "{") {
            continue;
        }
        let mut j = i + 1;
        while j < toks.len() && !is(j, ";") && !(toks[j].kind == Kind::Ident && matches!(text(src, &toks[j]), "import" | "export" | "const" | "let" | "function")) {
            if is(j, "from") {
                if let Some(s) = toks.get(j + 1).filter(|s| s.kind == Kind::Str) {
                    out.push(str_import(src, s, false));
                }
                break;
            }
            j += 1;
        }
    }
    out
}

pub fn has_module_syntax(src: &str) -> bool {
    let toks = tokens(src);
    toks.iter().enumerate().any(|(i, t)| {
        let word = text(src, t);
        t.kind == Kind::Ident
            && (word == "import" || word == "export")
            && (i == 0 || text(src, &toks[i - 1]) != ".")
            && toks.get(i + 1).is_some_and(|n| !matches!(text(src, n), "(" | "."))
    })
}

/// Nome exportado diretamente: `export [async] function n`, `export const|let|var n`.
pub fn exports(src: &str) -> Vec<(String, Option<String>)> {
    let toks = tokens(src);
    let mut out = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if text(src, t) != "export" || (i > 0 && text(src, &toks[i - 1]) == ".") {
            continue;
        }
        let mut j = i + 1;
        if toks.get(j).is_some_and(|t| text(src, t) == "async") {
            j += 1;
        }
        let Some(kw) = toks.get(j) else { continue };
        match text(src, kw) {
            "function" => {
                if let Some(n) = toks.get(j + 1).filter(|t| t.kind == Kind::Ident) {
                    out.push((text(src, n).to_string(), None));
                }
            }
            "const" | "let" | "var" => {
                if let Some(n) = toks.get(j + 1).filter(|t| t.kind == Kind::Ident) {
                    let value = if toks.get(j + 2).is_some_and(|t| text(src, t) == "=") {
                        toks.get(j + 3).map(|t| text(src, t).to_string())
                    } else {
                        None
                    };
                    out.push((text(src, n).to_string(), value));
                }
            }
            "{" => {
                let mut k = j + 1;
                while let Some(t) = toks.get(k) {
                    let s = text(src, t);
                    if s == "}" {
                        break;
                    }
                    if t.kind == Kind::Ident && s != "as" && toks.get(k + 1).is_none_or(|n| text(src, n) != "as") {
                        out.push((s.to_string(), None));
                    }
                    k += 1;
                }
            }
            _ => {}
        }
    }
    out
}

pub fn rewrite(src: &str, imports: &[Import], mut url: impl FnMut(&Import) -> Option<String>) -> String {
    let mut out = String::with_capacity(src.len() + 64);
    let mut last = 0;
    for imp in imports {
        if let Some(u) = url(imp) {
            out.push_str(&src[last..imp.start]);
            out.push_str(&u);
            last = imp.end;
        }
    }
    out.push_str(&src[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs(src: &str) -> Vec<String> {
        imports(src).into_iter().map(|i| i.spec).collect()
    }

    #[test]
    fn finds_imports() {
        let src = r#"
import a from './a.js'
import { b, c } from "../b.js";
import * as d from './d.js'
import './side.js'
export { x } from './x.js'
export * from './y.js'
const s = 'import z from "./no.js"'
const t = `import ${a} from './no2.js'`
const r = /from '.\/no3.js'/g
// import n from './no4.js'
const m = import('./lazy.js')
console.log(import.meta.url)
export const q = 1
"#;
        assert_eq!(specs(src), ["./a.js", "../b.js", "./d.js", "./side.js", "./x.js", "./y.js", "./lazy.js"]);
        let dynamic: Vec<bool> = imports(src).iter().map(|i| i.dynamic).collect();
        assert_eq!(dynamic, [false, false, false, false, false, false, true]);
    }

    #[test]
    fn minified_imports() {
        assert_eq!(specs(r#"import{a as b}from"./a.js";import"./c.js";export*from"./d.js";"#), ["./a.js", "./c.js", "./d.js"]);
    }

    #[test]
    fn finds_exports() {
        let e = exports("export async function load() {}\nexport const ssr = true\nexport { a, b as c }");
        assert_eq!(e[0], ("load".into(), None));
        assert_eq!(e[1], ("ssr".into(), Some("true".into())));
        assert_eq!(e[2].0, "a");
        assert_eq!(e.len(), 4);
    }
}
