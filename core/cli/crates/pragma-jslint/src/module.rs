use crate::{LineIndex, parse, static_specifier};
use oxc_allocator::Allocator;
use oxc_ast::ast::{ExportAllDeclaration, ExportFromDeclaration, Expression, ImportDeclaration, ImportExpression, Program, StringLiteral};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
use oxc_span::Span;
use oxc_syntax::module_record::{ExportExportName, ExportImportName, ImportImportName};

/// Names exported by the module (`load`, `actions`, `ssr`, `GET`, `default`…), in source order. Empty if the file does not parse.
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
    for (_, specifier) in dynamic_imports(&ret.program) {
        out.push((specifier, Imported::All));
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
    for (span, specifier) in dynamic_imports(&ret.program) {
        if let Some(k) = alias(&specifier) {
            let text = format!("{}.then((m) => m.{})", span.source_text(source), package_alias(k, None));
            edits.push((span.start, span.end, text));
        }
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = source.to_string();
    for (start, end, text) in edits {
        out.replace_range(start as usize..end as usize, &text);
    }
    Ok(out)
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
        match &mut it.source {
            Expression::StringLiteral(lit) => self.apply(lit),
            Expression::TemplateLiteral(t) if t.expressions.is_empty() => {
                if let Some(new) = (self.rewrite)(t.quasis[0].value.raw.as_str()) {
                    let new = self.allocator.alloc_str(&new);
                    t.quasis[0].value.raw = new.into();
                    t.quasis[0].value.cooked = Some(new.into());
                }
            }
            _ => {}
        }
        walk_mut::walk_import_expression(self, it);
    }
}

/// `import()` calls the build can follow: (span of the call, specifier).
fn dynamic_imports(program: &Program) -> Vec<(Span, String)> {
    struct Found(Vec<(Span, String)>);
    impl<'a> Visit<'a> for Found {
        fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
            if let Some(specifier) = static_specifier(&it.source) {
                self.0.push((it.span, specifier.to_string()));
            }
            walk::walk_import_expression(self, it);
        }
    }
    let mut found = Found(Vec::new());
    found.visit_program(program);
    found.0
}
