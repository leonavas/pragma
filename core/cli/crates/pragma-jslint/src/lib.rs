//! Regras de JS do dialeto. Contrato: core/SPEC.md §10.
use oxc_allocator::Allocator;
use oxc_ast::Comment;
use oxc_ast::ast::{
    ComputedMemberExpression, ExportAllDeclaration, ExportFromDeclaration, Expression,
    IdentifierReference, ImportDeclaration, ImportExpression, ObjectPropertyKind, Program,
    Statement, StaticMemberExpression, StringLiteral, TemplateLiteral,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{SourceType, Span};
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
    found: &'c mut Vec<(u32, &'static str, String)>,
}

impl Linter<'_> {
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
            "console" if matches!(object_name, Some("globalThis" | "window" | "self")) => {
                self.report(span, RULE_CONSOLE, console_message());
            }
            _ => {}
        }
    }

    fn check_specifier(&mut self, specifier: &str, span: Span) {
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
            format!("import remoto `{specifier}` é proibido; copie o código para o projeto")
        } else {
            format!(
                "import `{specifier}` é proibido; use caminho relativo (./, ../) ou /core/ (sem pacotes npm)"
            )
        };
        self.report(span, RULE_IMPORT, message);
    }
}

fn console_message() -> String {
    "`console` é proibido; use `ctx.log` (ou `log` de core/log.js)".to_string()
}

impl<'a> Visit<'a> for Linter<'_> {
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
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span);
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span);
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        self.check_specifier(it.source.value.as_str().unwrap_or_default(), it.source.span);
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        match &it.source {
            Expression::StringLiteral(lit) => {
                self.check_specifier(lit.value.as_str().unwrap_or_default(), lit.span);
            }
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
                self.check_specifier(t.quasis[0].value.raw.as_str(), t.span);
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
