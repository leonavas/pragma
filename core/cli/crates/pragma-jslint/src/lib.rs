//! JS of the dialect: the lint rules (core/SPEC.md §10), the module facts the bundler needs, and
//! the factory transform of page, component and store scripts.
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;

pub mod legacy;
mod lint;
mod module;
pub mod script;
mod specifier;

pub use lint::{RULE_COMMENT, RULE_CONSOLE, RULE_HTML, RULE_IMPORT, RULE_JSDOC, RULE_NAME, RULE_PACKAGE, RULE_SYNTAX, SHORT_OK, lint};
pub use module::{Imported, alias_package_imports, exported_names, imported_bindings, minify_with_imports, package_alias};
pub use specifier::{is_node_builtin, is_package, is_workers_module, node_builtin_message, package_name, workers_module_message};

fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> ParserReturn<'a> {
    Parser::new(allocator, source, SourceType::mjs()).parse()
}

/// The specifier of an `import()` the build can follow: a string, or a template literal without
/// `${}` (its raw text).
fn static_specifier<'e>(source: &'e Expression) -> Option<&'e str> {
    match source {
        Expression::StringLiteral(lit) => Some(lit.value.as_str().unwrap_or_default()),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => Some(t.quasis[0].value.raw.as_str()),
        _ => None,
    }
}

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let starts = std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1)).collect();
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
