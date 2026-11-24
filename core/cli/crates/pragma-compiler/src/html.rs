//! HTML dialect parser: strict (every non-void tag closes), with a position on each node.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Pos {
    pub file: u32,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    pub value: Option<String>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(String, Pos),
    Comment,
}

pub struct ParseError {
    pub pos: Pos,
    pub message: String,
}

pub const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr",
];
const RAW: &[&str] = &["script", "style", "textarea", "title"];

impl Element {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|a| a.name.eq_ignore_ascii_case(name))
    }

    pub fn has(&self, name: &str) -> bool {
        self.attr(name).is_some()
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.attr(name).and_then(|a| a.value.as_deref())
    }

    pub fn remove(&mut self, name: &str) {
        self.attrs.retain(|a| !a.name.eq_ignore_ascii_case(name));
    }

    pub fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    pub fn element_children(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
    }
}

/// Line and column of a byte offset, both from 1; a column counts characters, not bytes.
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(src: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(src.match_indices('\n').map(|(i, _)| i + 1));
        LineIndex { starts }
    }

    pub fn pos(&self, src: &str, offset: usize, file: u32) -> Pos {
        let offset = offset.min(src.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let col = src[self.starts[line]..offset].chars().count();
        Pos { file, line: line as u32 + 1, col: col as u32 + 1 }
    }
}

pub fn line_col(src: &str, offset: usize) -> (u32, u32) {
    let p = LineIndex::new(src).pos(src, offset, 0);
    (p.line, p.col)
}

impl Pos {
    /// Position of byte `offset` of `text`, a text that starts at `self`.
    pub fn advance(self, text: &str, offset: usize) -> Pos {
        let (line, col) = line_col(text, offset);
        if line == 1 { Pos { col: self.col + col - 1, ..self } } else { Pos { line: self.line + line - 1, col, ..self } }
    }
}

struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    i: usize,
    file: u32,
    lines: LineIndex,
    errors: Vec<ParseError>,
}

pub fn parse(src: &str, file: u32) -> (Vec<Node>, Vec<ParseError>) {
    let mut p = Parser { src, b: src.as_bytes(), i: 0, file, lines: LineIndex::new(src), errors: Vec::new() };
    let nodes = p.run();
    (nodes, p.errors)
}

fn is_name_char(c: u8) -> bool {
    !c.is_ascii_whitespace() && c != b'/' && c != b'>' && c != b'=' && c != b'"' && c != b'\'' && c != b'<'
}

impl Parser<'_> {
    fn pos(&self, at: usize) -> Pos {
        self.lines.pos(self.src, at, self.file)
    }

    fn err(&mut self, at: usize, message: String) {
        let pos = self.pos(at);
        self.errors.push(ParseError { pos, message });
    }

    fn starts_tag(&self, at: usize) -> bool {
        let next = self.b.get(at + 1).copied().unwrap_or(0);
        next.is_ascii_alphabetic()
            || next == b'!'
            || next == b'?'
            || (next == b'/' && self.b.get(at + 2).is_some_and(|c| c.is_ascii_alphabetic()))
    }

    fn run(&mut self) -> Vec<Node> {
        let mut stack: Vec<(Element, usize)> = Vec::new();
        let mut root: Vec<Node> = Vec::new();
        macro_rules! push {
            ($n:expr) => {
                match stack.last_mut() {
                    Some((e, _)) => e.children.push($n),
                    None => root.push($n),
                }
            };
        }
        while self.i < self.b.len() {
            let start = self.i;
            if self.b[start] != b'<' || !self.starts_tag(start) {
                let mut j = start + 1;
                while j < self.b.len() && !(self.b[j] == b'<' && self.starts_tag(j)) {
                    j += 1;
                }
                self.i = j;
                push!(Node::Text(self.src[start..j].to_string(), self.pos(start)));
                continue;
            }
            if self.src[start..].starts_with("<!--") {
                match self.src[start + 4..].find("-->") {
                    Some(k) => self.i = start + 4 + k + 3,
                    None => {
                        self.err(start, "unclosed comment".into());
                        self.i = self.b.len();
                    }
                }
                push!(Node::Comment);
                continue;
            }
            if self.b[start + 1] == b'!' || self.b[start + 1] == b'?' {
                if !self.src[start..].get(..9).is_some_and(|s| s.eq_ignore_ascii_case("<!doctype")) {
                    self.err(start, "unexpected declaration".into());
                }
                self.i = self.src[start..].find('>').map_or(self.b.len(), |k| start + k + 1);
                continue;
            }
            if self.b[start + 1] == b'/' {
                let mut j = start + 2;
                while j < self.b.len() && is_name_char(self.b[j]) {
                    j += 1;
                }
                let name = self.src[start + 2..j].to_string();
                self.i = self.src[j..].find('>').map_or(self.b.len(), |k| j + k + 1);
                match stack.iter().rposition(|(e, _)| e.name.eq_ignore_ascii_case(&name)) {
                    Some(idx) => {
                        while stack.len() > idx + 1 {
                            let (e, at) = stack.pop().unwrap();
                            self.err(at, format!("<{}> is not closed", e.name));
                            push!(Node::Element(e));
                        }
                        let (e, _) = stack.pop().unwrap();
                        push!(Node::Element(e));
                    }
                    None => self.err(start, format!("</{name}> has no opening tag")),
                }
                continue;
            }
            let (el, self_closing) = self.start_tag();
            let lower = el.name.to_ascii_lowercase();
            if VOID.contains(&lower.as_str()) || self_closing {
                push!(Node::Element(el));
            } else if RAW.contains(&lower.as_str()) {
                let mut el = el;
                let body_start = self.i;
                let end = find_close(self.src, body_start, &lower);
                match end {
                    Some(k) => {
                        if k > body_start {
                            el.children.push(Node::Text(self.src[body_start..k].to_string(), self.pos(body_start)));
                        }
                        self.i = self.src[k..].find('>').map_or(self.b.len(), |m| k + m + 1);
                    }
                    None => {
                        self.err(start, format!("<{}> is not closed", el.name));
                        self.i = self.b.len();
                    }
                }
                push!(Node::Element(el));
            } else {
                stack.push((el, start));
            }
        }
        while let Some((e, at)) = stack.pop() {
            self.err(at, format!("<{}> is not closed", e.name));
            push!(Node::Element(e));
        }
        root
    }

    fn start_tag(&mut self) -> (Element, bool) {
        let start = self.i;
        let mut j = start + 1;
        while j < self.b.len() && is_name_char(self.b[j]) {
            j += 1;
        }
        let mut el = Element { name: self.src[start + 1..j].to_string(), attrs: Vec::new(), children: Vec::new(), pos: self.pos(start) };
        self.i = j;
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.i >= self.b.len() {
                self.err(start, format!("incomplete tag <{}>", el.name));
                return (el, true);
            }
            match self.b[self.i] {
                b'>' => {
                    self.i += 1;
                    return (el, false);
                }
                b'/' if self.b.get(self.i + 1) == Some(&b'>') => {
                    self.i += 2;
                    return (el, true);
                }
                b'/' => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let a_start = self.i;
            let mut k = self.i;
            while k < self.b.len() && is_name_char(self.b[k]) {
                k += 1;
            }
            if k == a_start {
                self.err(a_start, "unexpected character in tag".into());
                self.i += 1;
                continue;
            }
            let name = self.src[a_start..k].to_string();
            self.i = k;
            let mut w = self.i;
            while w < self.b.len() && self.b[w].is_ascii_whitespace() {
                w += 1;
            }
            let mut value = None;
            if self.b.get(w) == Some(&b'=') {
                w += 1;
                while w < self.b.len() && self.b[w].is_ascii_whitespace() {
                    w += 1;
                }
                match self.b.get(w) {
                    Some(&q) if q == b'"' || q == b'\'' => match self.src[w + 1..].find(q as char) {
                        Some(e) => {
                            value = Some(self.src[w + 1..w + 1 + e].to_string());
                            self.i = w + 1 + e + 1;
                        }
                        None => {
                            self.err(w, "unclosed quotes".into());
                            value = Some(self.src[w + 1..].to_string());
                            self.i = self.b.len();
                        }
                    },
                    _ => {
                        let mut e = w;
                        while e < self.b.len() && !self.b[e].is_ascii_whitespace() && self.b[e] != b'>' {
                            e += 1;
                        }
                        value = Some(self.src[w..e].to_string());
                        self.i = e;
                    }
                }
            }
            if el.attrs.iter().any(|a| a.name.eq_ignore_ascii_case(&name)) {
                self.err(a_start, format!("duplicate attribute {name}"));
                continue;
            }
            let pos = self.pos(a_start);
            el.attrs.push(Attr { name, value, pos });
        }
    }
}

fn find_close(src: &str, from: usize, name: &str) -> Option<usize> {
    let lower = src[from..].to_ascii_lowercase();
    let pat = format!("</{name}");
    let mut base = 0;
    while let Some(k) = lower[base..].find(&pat) {
        let at = base + k;
        let after = lower.as_bytes().get(at + pat.len()).copied().unwrap_or(b'>');
        if after == b'>' || after.is_ascii_whitespace() || after == b'/' {
            return Some(from + at);
        }
        base = at + pat.len();
    }
    None
}

pub fn escape_attr(v: &str) -> String {
    v.replace('"', "&quot;")
}

pub fn escape_text(v: &str) -> String {
    v.replace('<', "&lt;").replace('>', "&gt;")
}

pub fn push_attr(out: &mut String, a: &Attr) {
    out.push_str(&a.name);
    if let Some(v) = &a.value {
        out.push_str("=\"");
        out.push_str(&escape_attr(v));
        out.push('"');
    }
}

pub fn is_blank(n: &Node) -> bool {
    match n {
        Node::Text(t, _) => t.trim().is_empty(),
        Node::Comment => true,
        Node::Element(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_and_structure() {
        let src = "<div class=\"a\">\n  <p data-text=\"data.x\">hi</p>\n  <br>\n  <c-card title='X' />\n</div>";
        let (nodes, errs) = parse(src, 0);
        assert!(errs.is_empty());
        let Node::Element(div) = &nodes[0] else { panic!() };
        let kids: Vec<_> = div.element_children().collect();
        assert_eq!(kids.len(), 3);
        assert_eq!((kids[0].pos.line, kids[0].pos.col), (2, 3));
        assert_eq!((kids[0].attrs[0].pos.line, kids[0].attrs[0].pos.col), (2, 6));
        assert_eq!(kids[2].value("title"), Some("X"));
        let mut out = String::new();
        crate::compile::write_nodes(&nodes, &mut out, false);
        assert!(out.contains("<c-card title=\"X\"></c-card>"));
    }

    #[test]
    fn unclosed_and_stray() {
        let (_, errs) = parse("<div><span></div></p>", 0);
        let msgs: Vec<_> = errs.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(msgs, ["<span> is not closed", "</p> has no opening tag"]);
    }

    #[test]
    fn raw_text_and_lt() {
        let (nodes, errs) = parse("<title>a < b</title><p>1 < 2</p>", 0);
        assert!(errs.is_empty());
        assert_eq!(nodes.len(), 2);
    }
}
