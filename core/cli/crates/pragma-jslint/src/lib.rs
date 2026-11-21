//! JS rules of the dialect. Contract: core/SPEC.md §10.
use oxc_allocator::Allocator;
use oxc_ast::Comment;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, ComputedMemberExpression, ExportAllDeclaration, ExportFromDeclaration, Expression, FormalParameters,
    IdentifierReference, ImportDeclaration, ImportExpression, ObjectProperty, ObjectPropertyKind, Program,
    Statement, StaticMemberExpression, StringLiteral, TemplateLiteral,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::module_record::{ExportExportName, ExportImportName, ImportImportName};
use pragma_diag::Diagnostic;
use std::path::{Component, Path};

pub mod script;

pub struct LintContext {
    pub is_core: bool,
}

pub const RULE_SYNTAX: &str = "js/syntax";
pub const RULE_HTML: &str = "js/html-in-js";
pub const RULE_CONSOLE: &str = "js/console";
pub const RULE_JSDOC: &str = "js/jsdoc-types";
pub const RULE_IMPORT: &str = "js/import";
pub const RULE_PACKAGE: &str = "app/package";
pub const RULE_COMMENT: &str = "js/comment";
pub const RULE_NAME: &str = "js/vague-name";

const SHORT_OK: &[&str] = &["$", "i", "j", "k", "id", "ok", "el", "ctx", "url", "log", "db", "kv"];
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

fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> ParserReturn<'a> {
    Parser::new(allocator, source, SourceType::mjs()).parse()
}

pub fn lint(path: &Path, source: &str, ctx: &LintContext) -> Vec<Diagnostic> {
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
        ctx,
        server: is_server_file(path, ctx),
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

fn is_server_file(path: &Path, ctx: &LintContext) -> bool {
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
    // In the app the server runtime lives in .pragma/server/ (or legacy core/server/); in the repo, in core/runtime/server/.
    parts.windows(2).any(|w| w == [".pragma", "server"] || w == ["core", "server"]) || (ctx.is_core && parts.contains(&"server"))
}

struct Linter<'c> {
    ctx: &'c LintContext,
    server: bool,
    middleware: bool,
    short_ok: bool,
    concise_arrow: bool,
    found: &'c mut Vec<(u32, &'static str, String)>,
}

impl Linter<'_> {
    fn check_name(&mut self, name: &str, span: Span) {
        let framework_name = self.middleware && name == "handle";
        if !self.ctx.is_core && !framework_name && vague_name(name, self.short_ok) {
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
            "createElement" | "createElementNS" | "createTextNode" if !self.ctx.is_core => self.report(
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
            || specifier.starts_with("/core/")
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

pub fn is_node_builtin(specifier: &str) -> bool {
    specifier.starts_with("node:") || nodejs_built_in_modules::BUILTINS.binary_search(&specifier).is_ok()
}

pub fn node_builtin_message(specifier: &str) -> String {
    format!("`{specifier}` is a Node module and doesn't run in the browser; use it in a .server.js")
}

pub fn is_workers_module(specifier: &str) -> bool {
    specifier.starts_with("cloudflare:")
}

pub fn workers_module_message(specifier: &str) -> String {
    format!("`{specifier}` is a Cloudflare Workers module and doesn't run in the browser; use it in a .server.js")
}

pub fn is_package(specifier: &str) -> bool {
    let name = package_name(specifier);
    !name.is_empty()
        && !specifier.starts_with('.')
        && !specifier.starts_with('/')
        && !name.contains(':')
        && name
            .trim_start_matches('@')
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

pub fn package_name(specifier: &str) -> &str {
    let mut parts = specifier.splitn(3, '/');
    let first = parts.next().unwrap_or_default();
    if first.starts_with('@') {
        match parts.next() {
            Some(second) if !second.is_empty() => &specifier[..first.len() + 1 + second.len()],
            _ => "",
        }
    } else {
        first
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
        match &it.source {
            Expression::StringLiteral(lit) => {
                self.check_specifier(lit.value.as_str().unwrap_or_default(), lit.span, true);
            }
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
                self.check_specifier(t.quasis[0].value.raw.as_str(), t.span, false);
            }
            source if !self.ctx.is_core => self.report(
                source.span(),
                RULE_IMPORT,
                "`import()` needs a quoted path or package name (`import('./chart.js')`, `import('dayjs')`); the build cannot follow a computed specifier".to_string(),
            ),
            _ => {}
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

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { starts }
    }

    /// 1-based line and column; column in characters, not bytes.
    fn position(&self, source: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let column = source[self.starts[line]..offset].chars().count() + 1;
        (line as u32 + 1, column as u32)
    }
}

/// Names exported by the module (`state`, `actions`, `mount`, `load`, `ssr`, `default`…), in source
/// order. Empty if the file does not parse.
pub fn exported_names(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        return Vec::new();
    }
    let record = &ret.module_record;
    let mut names: Vec<(u32, String)> = record
        .local_export_entries
        .iter()
        .chain(&record.indirect_export_entries)
        .chain(&record.star_export_entries)
        .filter_map(|e| match &e.export_name {
            ExportExportName::Name(n) => Some((n.span.start, n.name.to_string())),
            ExportExportName::Default(span) => Some((span.start, "default".to_string())),
            ExportExportName::Null => None,
        })
        .collect();
    names.sort_by_key(|n| n.0);
    names.into_iter().map(|n| n.1).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Imported {
    Name(String),
    All,
    Bare,
}

pub fn imported_bindings(source: &str) -> Vec<(String, Imported)> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        return Vec::new();
    }
    let record = &ret.module_record;
    let mut out: Vec<(String, Imported)> = Vec::new();
    for e in record.import_entries.iter().filter(|e| !e.is_type) {
        let what = match &e.import_name {
            ImportImportName::Name(n) => Imported::Name(n.name.to_string()),
            ImportImportName::NamespaceObject => Imported::All,
            ImportImportName::Default(_) => Imported::Name("default".into()),
        };
        out.push((e.module_request.name.to_string(), what));
    }
    for e in record.indirect_export_entries.iter().chain(&record.star_export_entries).filter(|e| !e.is_type) {
        let Some(req) = &e.module_request else { continue };
        let what = match &e.import_name {
            ExportImportName::Name(n) => Imported::Name(n.name.to_string()),
            ExportImportName::All | ExportImportName::AllButDefault => Imported::All,
            ExportImportName::Null => continue,
        };
        out.push((req.name.to_string(), what));
    }
    for (spec, reqs) in &record.requested_modules {
        if reqs.iter().any(|r| r.is_import && !r.is_type) && !out.iter().any(|(s, _)| s == spec.as_str()) {
            out.push((spec.to_string(), Imported::Bare));
        }
    }
    for d in &record.dynamic_imports {
        let raw = d.module_request.source_text(source);
        let quoted = raw.len() >= 2 && (raw.starts_with('"') || raw.starts_with('\'')) && raw.ends_with(&raw[..1]);
        if quoted {
            out.push((raw[1..raw.len() - 1].to_string(), Imported::All));
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn package_alias(index: u32, name: Option<&str>) -> String {
    format!("${index}${}", name.unwrap_or_default())
}

pub fn alias_package_imports(source: &str, alias: impl Fn(&str) -> Option<u32>) -> Result<String, String> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        return Ok(source.to_string());
    }
    let record = &ret.module_record;
    let mut statements: Vec<(Span, &'static str, String, Vec<String>)> = Vec::new();
    let mut push = |span: Span, kind: &'static str, spec: &str, part: String| match statements.iter_mut().find(|s| s.0 == span) {
        Some(s) => s.3.push(part),
        None => statements.push((span, kind, spec.to_string(), vec![part])),
    };
    for e in &record.import_entries {
        let spec = e.module_request.name.as_str();
        let Some(k) = alias(spec) else { continue };
        let local = e.local_name.name.as_str();
        let imported = match &e.import_name {
            ImportImportName::Name(n) => package_alias(k, Some(n.name.as_str())),
            ImportImportName::Default(_) => package_alias(k, Some("default")),
            ImportImportName::NamespaceObject => package_alias(k, None),
        };
        push(e.statement_span, "import", spec, format!("{imported} as {local}"));
    }
    for e in &record.indirect_export_entries {
        let Some(req) = &e.module_request else { continue };
        let spec = req.name.as_str();
        let Some(k) = alias(spec) else { continue };
        let imported = match &e.import_name {
            ExportImportName::Name(n) => package_alias(k, Some(n.name.as_str())),
            ExportImportName::All => package_alias(k, None),
            _ => continue,
        };
        let exported = match &e.export_name {
            ExportExportName::Name(n) => n.name.to_string(),
            ExportExportName::Default(_) => "default".to_string(),
            ExportExportName::Null => continue,
        };
        push(e.statement_span, "export", spec, format!("{imported} as {exported}"));
    }
    for e in &record.star_export_entries {
        if let Some(req) = &e.module_request
            && alias(req.name.as_str()).is_some()
        {
            return Err(format!("`export * from '{}'` does not work alongside another import of the same package; re-export by name", req.name));
        }
    }
    let mut edits: Vec<(u32, u32, String)> = statements
        .into_iter()
        .map(|(span, kind, spec, parts)| {
            let lines = span.source_text(source).matches('\n').count();
            (span.start, span.end, format!("{kind} {{ {} }} from '{spec}';{}", parts.join(", "), "\n".repeat(lines)))
        })
        .collect();
    for d in &record.dynamic_imports {
        let raw = d.module_request.source_text(source);
        let quoted = raw.len() >= 2 && (raw.starts_with('"') || raw.starts_with('\'')) && raw.ends_with(&raw[..1]);
        if !quoted {
            continue;
        }
        if let Some(k) = alias(&raw[1..raw.len() - 1]) {
            let text = format!("{}.then((m) => m.{})", d.span.source_text(source), package_alias(k, None));
            edits.push((d.span.start, d.span.end, text));
        }
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = source.to_string();
    for (start, end, text) in edits {
        out.replace_range(start as usize..end as usize, &text);
    }
    Ok(out)
}

/// Keys of `export const actions = { ... }` (properties, methods and shorthands), in source
/// order. Also accepts `const x = {...}; export { x as actions }`. Spreads and non-static
/// computed keys are ignored.
pub fn exported_actions(source: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        return Vec::new();
    }
    let local = ret
        .module_record
        .local_export_entries
        .iter()
        .find_map(|e| match &e.export_name {
            ExportExportName::Name(n) if n.name.as_str() == "actions" => e.local_name.name(),
            _ => None,
        });
    let Some(local) = local else {
        return Vec::new();
    };
    let Some(Expression::ObjectExpression(obj)) = top_level_init(&ret.program, local.as_str())
    else {
        return Vec::new();
    };
    obj.properties
        .iter()
        .filter_map(|p| match p {
            ObjectPropertyKind::ObjectProperty(p) => {
                p.key.static_name()?.as_str().map(str::to_string)
            }
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect()
}

fn top_level_init<'p, 'a>(program: &'p Program<'a>, name: &str) -> Option<&'p Expression<'a>> {
    program.body.iter().find_map(|stmt| {
        let decl = match stmt {
            Statement::VariableDeclaration(d) => d,
            Statement::ExportDeclaration(e) => match &e.declaration {
                oxc_ast::ast::Declaration::VariableDeclaration(d) => d,
                _ => return None,
            },
            _ => return None,
        };
        decl.declarations
            .iter()
            .find(|d| {
                d.id.get_identifier_name()
                    .is_some_and(|n| n.as_str() == name)
            })
            .and_then(|d| d.init.as_ref())
    })
}

pub fn minify(source: &str) -> Result<String, String> {
    minify_with_imports(source, |_| None)
}

/// Minifies and rewrites literal import/export/`import()` specifiers: `rewrite` returns the
/// new specifier or `None` to keep it. Exported names are preserved.
pub fn minify_with_imports(
    source: &str,
    rewrite: impl Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        let lines = LineIndex::new(source);
        let errors: Vec<String> = ret
            .diagnostics
            .errors()
            .map(|e| {
                let (line, column) =
                    lines.position(source, e.labels.first().map_or(0, |l| l.offset()));
                format!("{line}:{column} {}", e.message)
            })
            .collect();
        return Err(errors.join("\n"));
    }
    let mut program = ret.program;
    ImportRewriter {
        allocator: &allocator,
        rewrite: &rewrite,
    }
    .visit_program(&mut program);

    let options = MinifierOptions {
        mangle: Some(MangleOptions::default()),
        mangle_properties: None,
        compress: Some(CompressOptions::smallest()),
    };
    let ret = Minifier::new(options).minify(&allocator, &mut program);
    Ok(Codegen::new()
        .with_options(CodegenOptions::minify())
        .with_scoping(ret.scoping)
        .with_private_member_mappings(ret.class_private_mappings)
        .build(&program)
        .code)
}

struct ImportRewriter<'a, 'r> {
    allocator: &'a Allocator,
    rewrite: &'r dyn Fn(&str) -> Option<String>,
}

impl<'a> ImportRewriter<'a, '_> {
    fn apply(&self, lit: &mut StringLiteral<'a>) {
        if let Some(new) = lit.value.as_str().and_then(|s| (self.rewrite)(s)) {
            lit.value = self.allocator.alloc_str(&new).into();
            lit.raw = None;
        }
    }
}

impl<'a> VisitMut<'a> for ImportRewriter<'a, '_> {
    fn visit_import_declaration(&mut self, it: &mut ImportDeclaration<'a>) {
        self.apply(&mut it.source);
    }

    fn visit_export_all_declaration(&mut self, it: &mut ExportAllDeclaration<'a>) {
        self.apply(&mut it.source);
    }

    fn visit_export_from_declaration(&mut self, it: &mut ExportFromDeclaration<'a>) {
        self.apply(&mut it.source);
    }

    fn visit_import_expression(&mut self, it: &mut ImportExpression<'a>) {
        if let Expression::StringLiteral(lit) = &mut it.source {
            self.apply(lit);
        }
        walk_mut::walk_import_expression(self, it);
    }
}
