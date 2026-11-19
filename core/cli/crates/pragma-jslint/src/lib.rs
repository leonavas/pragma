//! Regras de JS do dialeto. Contrato: core/SPEC.md §10.
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
use oxc_syntax::module_record::ExportExportName;
use pragma_diag::Diagnostic;
use std::path::{Component, Path};

pub struct LintContext {
    pub is_core: bool,
}

pub const RULE_SYNTAX: &str = "js/sintaxe";
pub const RULE_HTML: &str = "js/html-em-js";
pub const RULE_CONSOLE: &str = "js/console";
pub const RULE_JSDOC: &str = "js/jsdoc-tipos";
pub const RULE_IMPORT: &str = "js/import";
pub const RULE_COMMENT: &str = "js/comentario";
pub const RULE_NAME: &str = "js/nome-vago";
pub const RULE_CTX: &str = "js/ctx";

const SHORT_OK: &[&str] = &["$", "i", "j", "k", "id", "ok", "el", "ctx", "url", "log"];
const VAGUE: &[&str] = &[
    "tmp", "temp", "aux", "foo", "bar", "baz", "qux", "obj", "arr", "val", "res", "ret", "fn", "func", "cb",
    "data", "dado", "dados", "info", "stuff", "thing", "things", "coisa", "coisas", "algo", "teste", "test",
    "handle", "handler", "helper", "util", "utils", "misc", "doit", "fazer", "processar", "process",
];
const DIGIT_OK: &[&str] = &["base64", "sha1", "sha256", "md5", "utf8", "i18n"];

fn vague_name(name: &str, short_ok: bool) -> bool {
    let lower = name.to_ascii_lowercase();
    VAGUE.contains(&lower.as_str())
        || (lower.ends_with(|c: char| c.is_ascii_digit()) && !DIGIT_OK.contains(&lower.as_str()))
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
            format!("não é JS puro (ES module): {}", err.message),
        ));
    }

    let mut linter = Linter {
        ctx,
        server: is_server_file(path, ctx),
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
    // No app o runtime de servidor fica em core/server/; no repositório, em core/runtime/server/.
    parts.windows(2).any(|w| w == ["core", "server"]) || (ctx.is_core && parts.contains(&"server"))
}

struct Linter<'c> {
    ctx: &'c LintContext,
    server: bool,
    short_ok: bool,
    concise_arrow: bool,
    found: &'c mut Vec<(u32, &'static str, String)>,
}

impl Linter<'_> {
    fn check_name(&mut self, name: &str, span: Span) {
        if !self.ctx.is_core && vague_name(name, self.short_ok) {
            self.report(span, RULE_NAME, format!("nome `{name}` não diz o que é nem o que faz; use um nome que se explique (ex.: `produto`, `totalItens`, `adicionarAoCarrinho`)"));
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
                format!("`{name}` monta HTML em JS; a marcação vive no .html (use <template> e diretivas)"),
            ),
            "write" | "writeln" if object_name == Some("document") => self.report(
                span,
                RULE_HTML,
                format!("`document.{name}` monta HTML em JS; a marcação vive no .html"),
            ),
            "createElement" | "createElementNS" | "createTextNode" if !self.ctx.is_core => self.report(
                span,
                RULE_HTML,
                format!("`{name}` só é permitido no runtime (core/); a marcação vive no .html"),
            ),
            "state" | "props" if object_name == Some("ctx") && !self.ctx.is_core => self.report(
                span,
                RULE_CTX,
                format!("`ctx.{name}` não existe; use `ctx.$` (estado e props)"),
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
            || specifier.starts_with("/core/")
        {
            return;
        }
        let message = if let Some(rest) = specifier.strip_prefix("node:") {
            if self.server {
                return;
            }
            format!("`node:{rest}` só pode ser importado em *.server.js e core/server/")
        } else if specifier.starts_with("http:")
            || specifier.starts_with("https:")
            || specifier.starts_with("//")
        {
            format!("import remoto `{specifier}` é proibido; instale o pacote (npm i, bun add) ou copie o código para o projeto")
        } else if is_package(specifier) {
            if literal {
                return;
            }
            format!("import de pacote `{specifier}` deve usar aspas, não template literal")
        } else {
            format!(
                "import `{specifier}` é proibido; use caminho relativo (./, ../), /core/ ou um pacote do package.json"
            )
        };
        self.report(span, RULE_IMPORT, message);
    }
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
    "`console` é proibido; use `ctx.log` (ou `log` de core/log.js)".to_string()
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

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if it.value.as_str().is_some_and(has_markup) {
            self.report(
                it.span,
                RULE_HTML,
                "string com marcação HTML; a marcação vive no .html".to_string(),
            );
        }
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        if let Some(quasi) = it.quasis.iter().find(|q| has_markup(q.value.raw.as_str())) {
            self.report(
                quasi.span,
                RULE_HTML,
                "template literal com marcação HTML; a marcação vive no .html".to_string(),
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
            _ => {}
        }
        walk::walk_import_expression(self, it);
    }
}

/// Heurística de marcação: `<tag` seguido de espaço, `>` ou `/`; `</tag`; `<!--`; `<!doctype`.
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
                format!("JSDoc de tipos (`@{tag}`) é proibido; o pragma é JS puro, sem tipos"),
            ));
        }
    }
    if found.len() == before {
        found.push((comment.span.start, RULE_COMMENT, comment_message()));
    }
}

fn comment_message() -> String {
    "comentário é proibido; o código é a documentação: nomes claros e funções pequenas".to_string()
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

    /// Linha e coluna 1-based; coluna em caracteres, não bytes.
    fn position(&self, source: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source.len());
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let column = source[self.starts[line]..offset].chars().count() + 1;
        (line as u32 + 1, column as u32)
    }
}

/// Nomes exportados pelo módulo (`state`, `actions`, `mount`, `load`, `ssr`, `default`…), na ordem
/// do código. Vazio se o arquivo não parseia.
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

/// Chaves de `export const actions = { ... }` (propriedades, métodos e atalhos), na ordem do
/// código. Também aceita `const x = {...}; export { x as actions }`. Spreads e chaves computadas
/// não estáticas são ignorados.
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

/// Minifica e troca especificadores de import/export/`import()` literais: `rewrite` devolve o
/// novo especificador ou `None` para manter. Nomes exportados são preservados.
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
