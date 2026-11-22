use crate::specifier::{is_package, is_workers_module, node_builtin_message, workers_module_message};
use crate::{LineIndex, legacy, parse, static_specifier};
use oxc_allocator::Allocator;
use oxc_ast::Comment;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, ComputedMemberExpression, ExportAllDeclaration, ExportFromDeclaration, Expression, FormalParameters,
    IdentifierReference, ImportDeclaration, ImportExpression, ObjectProperty, StaticMemberExpression, StringLiteral, TemplateLiteral,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use pragma_diag::Diagnostic;
use std::path::{Component, Path};

const RULE_SYNTAX: &str = "js/syntax";
const RULE_HTML: &str = "js/html-in-js";
const RULE_CONSOLE: &str = "js/console";
const RULE_JSDOC: &str = "js/jsdoc-types";
pub const RULE_IMPORT: &str = "js/import";
pub const RULE_PACKAGE: &str = "app/package";
const RULE_COMMENT: &str = "js/comment";
pub const RULE_NAME: &str = "js/vague-name";

pub const SHORT_OK: &[&str] = &["$", "i", "j", "k", "id", "ok", "el", "ctx", "url", "log", "db", "kv"];
const VAGUE: &[&str] = &[
    "tmp", "temp", "aux", "foo", "bar", "baz", "qux", "obj", "arr", "val", "res", "ret", "fn", "func", "cb",
    "data", "dado", "dados", "info", "stuff", "thing", "things", "coisa", "coisas", "algo", "something", "whatever", "teste", "test",
    "handle", "handler", "helper", "util", "utils", "misc", "doit", "fazer", "processar", "process",
];
const DIGIT_OK: &[&str] = &["base64", "sha1", "sha256", "md5", "utf8", "i18n"];

// A single trailing digit marks a numbered copy (`item2`, `x1`). Longer numbers
// (`pode42`, `ec141`) and dimension pairs (`12x36`, `5x2`) are domain terms.
fn copy_suffix(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let n = bytes.len();
    if n == 0 || !bytes[n - 1].is_ascii_digit() {
        return false;
    }
    if n >= 2 && bytes[n - 2].is_ascii_digit() {
        return false;
    }
    !(n >= 3 && bytes[n - 2] == b'x' && bytes[n - 3].is_ascii_digit())
}

fn vague_name(name: &str, short_ok: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    VAGUE.contains(&lower.as_str())
        || (copy_suffix(&lower) && !DIGIT_OK.contains(&lower.as_str()))
        || (!short_ok && name.chars().count() < 3 && !SHORT_OK.contains(&name))
}

pub fn lint(path: &Path, source: &str) -> Vec<Diagnostic> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    let mut found: Vec<(u32, &'static str, String)> = Vec::new();

    for err in ret.diagnostics.errors() {
        let offset = err.labels.first().map_or(0, |l| l.offset());
        found.push((
            offset,
            RULE_SYNTAX,
            format!("not plain JS (ES module): {}", err.message),
        ));
    }

    let mut linter = Linter {
        server: is_server_file(path),
        middleware: path.file_name().is_some_and(|n| n == "_middleware.server.js"),
        short_ok: false,
        concise_arrow: false,
        found: &mut found,
    };
    linter.visit_program(&ret.program);
    for comment in &ret.program.comments {
        check_comment(source, comment, &mut found);
    }

    let lines = LineIndex::new(source);
    found.sort_by_key(|f| f.0);
    found
        .into_iter()
        .map(|(offset, rule, message)| {
            let (line, column) = lines.position(source, offset);
            Diagnostic::error(path, line, column, rule, message)
        })
        .collect()
}

fn is_server_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".server.js"))
    {
        return true;
    }
    let parts: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();
    parts.windows(2).any(|w| w == [".pragma", "server"] || w == legacy::SERVER_DIR)
}

struct Linter<'c> {
    server: bool,
    middleware: bool,
    short_ok: bool,
    concise_arrow: bool,
    found: &'c mut Vec<(u32, &'static str, String)>,
}

impl Linter<'_> {
    fn check_name(&mut self, name: &str, span: Span) {
        let framework_name = self.middleware && name == "handle";
        if !framework_name && vague_name(name, self.short_ok) {
            self.report(span, RULE_NAME, format!("name `{name}` does not say what it is or does; use a self-explanatory name (e.g. `product`, `totalItems`, `addToCart`)"));
        }
    }

    fn report(&mut self, span: Span, rule: &'static str, message: String) {
        self.found.push((span.start, rule, message));
    }

    fn check_member(&mut self, name: &str, span: Span, object: &Expression) {
        let object_name = match object {
            Expression::Identifier(id) => Some(id.name.as_str()),
            _ => None,
        };
        match name {
            "innerHTML" | "outerHTML" | "insertAdjacentHTML" | "setHTMLUnsafe" | "parseHTMLUnsafe"
            | "createContextualFragment" | "parseFromString" => self.report(
                span,
                RULE_HTML,
                format!("`{name}` builds HTML in JS; markup lives in .html (use <template> and directives)"),
            ),
            "write" | "writeln" if object_name == Some("document") => self.report(
                span,
                RULE_HTML,
                format!("`document.{name}` builds HTML in JS; markup lives in .html"),
            ),
            "createElement" | "createElementNS" | "createTextNode" => self.report(
                span,
                RULE_HTML,
                format!("`{name}` is only allowed in the runtime (.pragma/); markup lives in .html"),
            ),
            "console" if matches!(object_name, Some("globalThis" | "window" | "self")) => {
                self.report(span, RULE_CONSOLE, console_message());
            }
            _ => {}
        }
    }

    fn check_specifier(&mut self, specifier: &str, span: Span, literal: bool) {
        if specifier.starts_with("./")
            || specifier.starts_with("../")
            || specifier.starts_with("/.pragma/")
            || specifier.starts_with(legacy::IMPORT_PREFIX)
        {
            return;
        }
        let message = if specifier.starts_with("node:") {
            if !self.server {
                self.report(span, RULE_PACKAGE, node_builtin_message(specifier));
            }
            return;
        } else if is_workers_module(specifier) {
            if !self.server {
                self.report(span, RULE_PACKAGE, workers_module_message(specifier));
            }
            return;
        } else if specifier.starts_with("http:")
            || specifier.starts_with("https:")
            || specifier.starts_with("//")
        {
            format!("remote import `{specifier}` is not allowed; install the package (npm i, bun add) or copy the code into the project")
        } else if is_package(specifier) {
            if literal {
                return;
            }
            format!("package import `{specifier}` must use quotes, not a template literal")
        } else {
            format!(
                "import `{specifier}` is not allowed; use a relative path (./, ../), /.pragma/ or a package from package.json"
            )
        };
        self.report(span, RULE_IMPORT, message);
    }
}
fn console_message() -> String {
    "`console` is not allowed; use `log` (`import { log } from 'pragma'`, or .pragma/log.js outside scripts)".to_string()
}

impl<'a> Visit<'a> for Linter<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.check_name(it.name.as_str(), it.span);
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.concise_arrow = it.is_expression();
        walk::walk_arrow_function_expression(self, it);
    }

    fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
        let outer = self.short_ok;
        self.short_ok = std::mem::take(&mut self.concise_arrow);
        walk::walk_formal_parameters(self, it);
        self.short_ok = outer;
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        let is_function = it.method || matches!(it.value, Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_));
        if is_function && let Some(name) = it.key.static_name().and_then(|n| n.as_str().map(str::to_string)) {
            self.check_name(&name, it.key.span());
        }
        walk::walk_object_property(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.check_member(it.property.name.as_str(), it.property.span, &it.object);
        walk::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if let Expression::StringLiteral(lit) = &it.expression
            && let Some(name) = lit.value.as_str()
        {
            self.check_member(name, lit.span, &it.object);
        }
        walk::walk_computed_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name.as_str() == "console" {
            self.report(it.span, RULE_CONSOLE, console_message());
        }
    }

    // Server strings may hold markup that never reaches the page DOM: e-mail bodies, LLM prompts.
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if !self.server && it.value.as_str().is_some_and(has_markup) {
            self.report(
                it.span,
                RULE_HTML,
                "string with HTML markup; markup lives in .html".to_string(),
            );
        }
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        if self.server {
            walk::walk_template_literal(self, it);
            return;
        }
        if let Some(quasi) = it.quasis.iter().find(|q| has_markup(q.value.raw.as_str())) {
            self.report(
                quasi.span,
                RULE_HTML,
                "template literal with HTML markup; markup lives in .html".to_string(),
            );
        }
        walk::walk_template_literal(self, it);
    }

    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span, true);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span, true);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span, true);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match static_specifier(&it.source) {
            Some(specifier) => self.check_specifier(specifier, it.source.span(), matches!(it.source, Expression::StringLiteral(_))),
            None => self.report(
                it.source.span(),
                RULE_IMPORT,
                "`import()` needs a quoted path or package name (`import('./chart.js')`, `import('dayjs')`); the build cannot follow a computed specifier".to_string(),
            ),
        }
        walk::walk_import_expression(self, it);
    }
}

/// Markup heuristic: `<tag` followed by space, `>` or `/`; `</tag`; `<!--`; `<!doctype`.
fn has_markup(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len()).any(|i| {
        if b[i] != b'<' {
            return false;
        }
        let rest = &b[i + 1..];
        match rest {
            [b'/', c, ..] => c.is_ascii_alphabetic(),
            [b'!', b'-', b'-', ..] => true,
            [b'!', c, ..] => c.is_ascii_alphabetic(),
            [c, ..] if c.is_ascii_alphabetic() => {
                let end = rest
                    .iter()
                    .position(|c| !(c.is_ascii_alphanumeric() || *c == b'-'));
                end.is_some_and(|e| matches!(rest[e], b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r'))
            }
            _ => false,
        }
    })
}

const TYPE_TAGS: &[&str] = &[
    "type",
    "typedef",
    "template",
    "callback",
    "satisfies",
    "overload",
    "import",
    "enum",
    "implements",
];
const TYPED_TAGS: &[&str] = &[
    "param",
    "arg",
    "argument",
    "returns",
    "return",
    "prop",
    "property",
    "throws",
    "exception",
    "yields",
    "yield",
    "this",
    "extends",
    "augments",
];

fn check_comment(source: &str, comment: &Comment, found: &mut Vec<(u32, &'static str, String)>) {
    let span = comment.content_span();
    let text = &source[span.start as usize..span.end as usize];
    let b = text.as_bytes();
    let before = found.len();
    for (i, _) in text.match_indices('@') {
        if i > 0 && (b[i - 1].is_ascii_alphanumeric() || matches!(b[i - 1], b'_' | b'.' | b'-')) {
            continue;
        }
        let tag_len = b[i + 1..]
            .iter()
            .position(|c| !(c.is_ascii_alphanumeric() || *c == b'-' || *c == b'_'))
            .unwrap_or(b.len() - i - 1);
        let tag = &text[i + 1..i + 1 + tag_len];
        let after = text[i + 1 + tag_len..].trim_start();
        let forbidden = tag.starts_with("ts-")
            || TYPE_TAGS.contains(&tag)
            || (TYPED_TAGS.contains(&tag) && after.starts_with('{'));
        if forbidden {
            found.push((
                span.start + i as u32,
                RULE_JSDOC,
                format!("type JSDoc (`@{tag}`) is not allowed; pragma is plain JS, without types"),
            ));
        }
    }
    if found.len() == before {
        found.push((comment.span.start, RULE_COMMENT, comment_message()));
    }
}

fn comment_message() -> String {
    "comments are not allowed; the code is the documentation: clear names and small functions".to_string()
}

