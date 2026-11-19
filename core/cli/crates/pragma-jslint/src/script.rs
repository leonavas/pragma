//! Page, component and store scripts (SPEC §4): the top-level script is the component. It becomes
//! a per-instance factory `export default function ($, $pragma) { …; return { functions, consts } }`:
//! top-level `let x` → reactive state `$.x`, `export let p = d` → prop `$pragma.prop("p", d)` read as
//! `$.p`, names imported from the virtual module `pragma` → `$pragma.name`; imports are hoisted.
use crate::{LineIndex, parse};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpressionElement, ArrowFunctionExpression, AssignmentExpression, AssignmentTargetPropertyIdentifier, AwaitExpression,
    BindingPattern, CallExpression, Declaration, ExportDefaultDeclarationKind, Expression, ForOfStatement, Function,
    IdentifierReference, ImportDeclarationSpecifier, ModuleExportName, ObjectProperty, Program, Statement, StaticMemberExpression,
    UnaryExpression, UnaryOperator, UpdateExpression, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use pragma_diag::Diagnostic;
use std::collections::HashMap;
use std::path::Path;

pub const RULE_LEGACY: &str = "js/legacy-api";
pub const RULE_PROP_WRITE: &str = "js/prop-write";
pub const RULE_SCRIPT: &str = "js/component-script";

/// Names the virtual module `pragma` exports.
pub const PRAGMA_EXPORTS: &[&str] = &["shared", "data", "params", "query", "page", "log", "call", "navigate", "open", "close", "confirm"];
const MUTATORS: &[&str] = &["push", "pop", "shift", "unshift", "splice", "sort", "reverse", "fill", "copyWithin", "set", "add", "delete", "clear"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScriptKind {
    Page,
    Component,
    Store,
}

/// What a script declares at the top level (the names HTML can use).
#[derive(Debug, Default, Clone)]
pub struct ScriptInfo {
    pub lets: Vec<String>,
    pub props: Vec<String>,
    /// Top-level `const` and `class` that are not functions.
    pub consts: Vec<String>,
    /// Top-level function declarations and `const f = () => …` / `function () {}`.
    pub functions: Vec<String>,
    /// Top-level `const` changed in place (`items.push(x)`), with the line and column of its name:
    /// not reactive, so HTML that reads it would never update.
    pub mutated_consts: Vec<(String, u32, u32)>,
    /// Store: `export const persist = ['field']`.
    pub persist: Vec<String>,
    /// The script has errors (old API, syntax…): its names are not known, so HTML is not checked
    /// against them.
    pub incomplete: bool,
}

impl ScriptInfo {
    pub fn declares(&self, name: &str) -> bool {
        [&self.lets, &self.props, &self.consts, &self.functions].iter().any(|v| v.iter().any(|n| n == name))
    }

    pub fn is_function(&self, name: &str) -> bool {
        self.functions.iter().any(|n| n == name)
    }
}

pub struct Script {
    /// The factory module; the source unchanged when the script has errors.
    pub code: String,
    pub info: ScriptInfo,
    pub diags: Vec<Diagnostic>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sym {
    State,
    Prop,
    Const,
    Pragma(&'static str),
}

struct Transform<'s> {
    source: &'s str,
    kind: ScriptKind,
    edits: Vec<(u32, u32, String)>,
    found: Vec<(u32, &'static str, String)>,
    info: ScriptInfo,
    imports: Vec<String>,
    exposed: Vec<String>,
    syms: HashMap<SymbolId, Sym>,
    persist_at: Option<u32>,
    /// Where the factory opens: after the imports that come before any other statement (they stay
    /// in place, so import diagnostics keep their positions). Later imports are hoisted to the top.
    open_at: u32,
    leading: bool,
}

/// Rewrites a page, component or store script into its factory module and reports the script rules
/// (`js/legacy-api`, `js/prop-write`, `js/component-script`). Syntax errors are left to `lint`.
pub fn transform(path: &Path, source: &str, kind: ScriptKind) -> Script {
    let allocator = Allocator::default();
    let ret = parse(&allocator, source);
    if ret.diagnostics.has_errors() {
        return Script { code: source.to_string(), info: ScriptInfo { incomplete: true, ..ScriptInfo::default() }, diags: Vec::new() };
    }
    let semantic = SemanticBuilder::new().build(&ret.program).semantic;
    let scoping = semantic.scoping();
    let mut t = Transform {
        source,
        kind,
        edits: Vec::new(),
        found: Vec::new(),
        info: ScriptInfo::default(),
        imports: Vec::new(),
        exposed: Vec::new(),
        syms: HashMap::new(),
        persist_at: None,
        open_at: 0,
        leading: true,
    };
    t.top_level(&ret.program);
    t.reserved(scoping);
    let mut refs = Refs { t: &mut t, scoping, depth: 0 };
    refs.visit_program(&ret.program);
    if let Some(at) = t.persist_at {
        for key in t.info.persist.clone() {
            if !t.info.lets.contains(&key) {
                t.found.push((at, RULE_SCRIPT, format!("persist lists \"{key}\", which is not a top-level let of the store")));
            }
        }
    }
    let lines = LineIndex::new(source);
    let failed = !t.found.is_empty();
    t.info.incomplete = failed;
    t.found.sort_by_key(|f| f.0);
    let diags = t
        .found
        .iter()
        .map(|(offset, rule, message)| {
            let (line, column) = lines.position(source, *offset);
            Diagnostic::error(path, line, column, rule, message.clone())
        })
        .collect();
    let code = if failed { source.to_string() } else { t.assemble() };
    Script { code, info: t.info, diags }
}

fn binding_names<'a>(p: &'a BindingPattern<'a>) -> Vec<(&'a str, Span, Option<SymbolId>)> {
    p.get_binding_identifiers().into_iter().map(|b| (b.name.as_str(), b.span, b.symbol_id.get())).collect()
}

fn is_function_expr(e: &Expression) -> bool {
    matches!(e.without_parentheses(), Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
}

impl Transform<'_> {
    fn text(&self, span: Span) -> &str {
        &self.source[span.start as usize..span.end as usize]
    }

    fn edit(&mut self, start: u32, end: u32, text: impl Into<String>) {
        self.edits.push((start, end, text.into()));
    }

    /// Removes a statement, keeping its line breaks so the rest keeps its line numbers.
    fn remove(&mut self, span: Span) {
        let blank: String = self.text(span).chars().filter(|c| *c == '\n').collect();
        self.edit(span.start, span.end, blank);
    }

    fn err(&mut self, at: u32, rule: &'static str, message: impl Into<String>) {
        self.found.push((at, rule, message.into()));
    }

    fn top_level(&mut self, program: &Program) {
        for stmt in &program.body {
            if !matches!(stmt, Statement::ImportDeclaration(_)) {
                self.leading = false;
            }
            match stmt {
                Statement::ImportDeclaration(d) => {
                    if d.source.value.as_str() == Some("pragma") {
                        for s in d.specifiers.iter().flatten() {
                            match s {
                                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    let imported = match &s.imported {
                                        ModuleExportName::IdentifierName(n) => n.name.as_str(),
                                        ModuleExportName::IdentifierReference(n) => n.name.as_str(),
                                        ModuleExportName::StringLiteral(l) => l.value.as_str().unwrap_or_default(),
                                    };
                                    match PRAGMA_EXPORTS.iter().find(|n| **n == imported) {
                                        Some(n) => {
                                            if let Some(id) = s.local.symbol_id.get() {
                                                self.syms.insert(id, Sym::Pragma(n));
                                            }
                                        }
                                        None => self.err(s.span.start, RULE_SCRIPT, format!("'pragma' has no `{imported}`; it exports {}", PRAGMA_EXPORTS.join(", "))),
                                    }
                                }
                                _ => self.err(s.span().start, RULE_SCRIPT, "import names from 'pragma': `import { data, log } from 'pragma'`"),
                            }
                        }
                        self.remove(d.span);
                    } else if d.source.value.as_str().is_some_and(|s| s.ends_with("core/state.js")) {
                        self.err(d.span.start, RULE_LEGACY, "old API: /core/state.js `store()` → a store is plain top-level `let` values and functions; persist fields with `export const persist = ['field']`");
                    } else if !self.leading {
                        self.imports.push(self.text(d.span).split_whitespace().collect::<Vec<_>>().join(" "));
                        self.remove(d.span);
                    }
                    if self.leading {
                        self.open_at = d.span.end;
                    }
                }
                Statement::VariableDeclaration(d) => self.declaration(d, d.span.start),
                Statement::FunctionDeclaration(f) => self.function(f),
                Statement::ClassDeclaration(c) => {
                    if let Some(id) = &c.id {
                        self.info.consts.push(id.name.to_string());
                        self.exposed.push(id.name.to_string());
                    }
                }
                Statement::ExportDeclaration(e) => self.export(&e.declaration, e.span),
                Statement::ExportDefaultDeclaration(e) => {
                    let is_store_call = matches!(&e.declaration, ExportDefaultDeclarationKind::CallExpression(c) if matches!(&c.callee, Expression::Identifier(i) if i.name == "store"));
                    if is_store_call {
                        self.err(e.span.start, RULE_LEGACY, "old API: `export default store({...})` → top-level `let` values and functions; persist fields with `export const persist = ['field']`");
                    } else {
                        self.err(e.span.start, RULE_SCRIPT, "no `export default`: the top-level script is the component itself");
                    }
                }
                Statement::ExportNamedDeclaration(e) => self.err(e.span.start, RULE_SCRIPT, self.export_message()),
                Statement::ExportAllDeclaration(e) => self.err(e.span.start, RULE_SCRIPT, self.export_message()),
                Statement::ExportFromDeclaration(e) => self.err(e.span.start, RULE_SCRIPT, self.export_message()),
                _ => {}
            }
        }
    }

    fn export_message(&self) -> String {
        match self.kind {
            ScriptKind::Component => "`export` only declares props: `export let name = default`".into(),
            ScriptKind::Page => "a page script exports nothing (pages have no props; use `params` and `query` from 'pragma')".into(),
            ScriptKind::Store => "a store exports only `export const persist = ['field']`; everything else is reached through `shared`".into(),
        }
    }

    fn function(&mut self, f: &Function) {
        if let Some(id) = &f.id {
            self.info.functions.push(id.name.to_string());
            self.exposed.push(id.name.to_string());
            if let Some(sym) = id.symbol_id.get() {
                self.syms.insert(sym, Sym::Const);
            }
        }
    }

    fn declaration(&mut self, d: &VariableDeclaration, start: u32) {
        match d.kind {
            VariableDeclarationKind::Let => {
                for (i, decl) in d.declarations.iter().enumerate() {
                    let BindingPattern::BindingIdentifier(id) = &decl.id else {
                        self.err(decl.span.start, RULE_SCRIPT, "a top-level `let` declares one state value per name; destructure inside a function, or use `const`");
                        continue;
                    };
                    self.info.lets.push(id.name.to_string());
                    if let Some(sym) = id.symbol_id.get() {
                        self.syms.insert(sym, Sym::State);
                    }
                    if i == 0 {
                        self.edit(start, id.span.start, "");
                    }
                    let value = if decl.init.is_some() { format!("$.{}", id.name) } else { format!("$.{} = undefined", id.name) };
                    self.edit(id.span.start, id.span.end, value);
                }
            }
            VariableDeclarationKind::Const => {
                for decl in &d.declarations {
                    let function = matches!(decl.id, BindingPattern::BindingIdentifier(_)) && decl.init.as_ref().is_some_and(is_function_expr);
                    for (name, _, sym) in binding_names(&decl.id) {
                        if function { self.info.functions.push(name.to_string()) } else { self.info.consts.push(name.to_string()) }
                        self.exposed.push(name.to_string());
                        if let Some(sym) = sym {
                            self.syms.insert(sym, Sym::Const);
                        }
                    }
                }
            }
            _ => self.err(d.span.start, RULE_SCRIPT, "use `let` (state) or `const`, not `var`"),
        }
    }

    fn export(&mut self, decl: &Declaration, span: Span) {
        match decl {
            Declaration::VariableDeclaration(d) => {
                let names: Vec<String> = d.declarations.iter().flat_map(|x| binding_names(&x.id)).map(|(n, _, _)| n.to_string()).collect();
                if let Some(old) = names.iter().find(|n| matches!(n.as_str(), "state" | "actions" | "mount")) {
                    self.err(span.start, RULE_LEGACY, legacy_export(old));
                } else if d.kind == VariableDeclarationKind::Let && self.kind == ScriptKind::Component {
                    self.props(d, span.start);
                } else if self.kind == ScriptKind::Store && d.kind == VariableDeclarationKind::Const && names == ["persist"] {
                    self.persist(d, span);
                } else {
                    self.err(span.start, RULE_SCRIPT, self.export_message());
                }
            }
            Declaration::FunctionDeclaration(f) => match f.id.as_ref().map(|i| i.name.as_str()) {
                Some(old @ ("state" | "mount")) => self.err(span.start, RULE_LEGACY, legacy_export(old)),
                _ => self.err(span.start, RULE_SCRIPT, self.export_message()),
            },
            _ => self.err(span.start, RULE_SCRIPT, self.export_message()),
        }
    }

    fn props(&mut self, d: &VariableDeclaration, start: u32) {
        for (i, decl) in d.declarations.iter().enumerate() {
            let BindingPattern::BindingIdentifier(id) = &decl.id else {
                self.err(decl.span.start, RULE_SCRIPT, "a prop is one name: `export let name = default`");
                continue;
            };
            self.info.props.push(id.name.to_string());
            if let Some(sym) = id.symbol_id.get() {
                self.syms.insert(sym, Sym::Prop);
            }
            let from = if i == 0 { start } else { decl.span.start };
            match &decl.init {
                Some(init) => {
                    self.edit(from, init.span().start, format!("$pragma.prop({:?}, ", id.name.as_str()));
                    self.edit(init.span().end, init.span().end, ")");
                }
                None => self.edit(from, decl.span.end, format!("$pragma.prop({:?}, undefined)", id.name.as_str())),
            }
        }
    }

    fn persist(&mut self, d: &VariableDeclaration, span: Span) {
        self.persist_at = Some(span.start);
        let keys = d.declarations[0].init.as_ref().and_then(|init| match init.without_parentheses() {
            Expression::ArrayExpression(a) => a
                .elements
                .iter()
                .map(|e| match e {
                    ArrayExpressionElement::StringLiteral(s) => s.value.as_str().map(str::to_string),
                    _ => None,
                })
                .collect::<Option<Vec<String>>>(),
            _ => None,
        });
        match keys {
            Some(keys) => self.info.persist = keys,
            None => self.err(span.start, RULE_SCRIPT, "persist is a list of field names: `export const persist = ['items']`"),
        }
        self.remove(span);
    }

    fn reserved(&mut self, scoping: &Scoping) {
        let taken = scoping.symbol_names().any(|n| n == "$" || n == "$pragma")
            || scoping.root_unresolved_references().keys().any(|n| n.as_str() == "$" || n.as_str() == "$pragma");
        if taken {
            self.err(0, RULE_SCRIPT, "`$` and `$pragma` are reserved in page, component and store scripts; rename that variable");
        }
    }

    fn assemble(&mut self) -> String {
        let mut hoisted = String::new();
        for i in &self.imports {
            hoisted.push_str(i.trim_end_matches(';'));
            hoisted.push_str("; ");
        }
        let mut open = String::from(if self.open_at > 0 { "; " } else { "" });
        if !self.info.persist.is_empty() {
            open.push_str(&format!("export const persist = {:?}; ", self.info.persist));
        }
        open.push_str("export default function ($, $pragma) {");
        let mut edits = vec![(0, 0, hoisted), (self.open_at, self.open_at, open)];
        edits.append(&mut self.edits);
        edits.sort_by_key(|e| (e.0, e.1));
        let mut body = String::with_capacity(self.source.len() + 256);
        let mut at = 0usize;
        for (start, end, text) in &edits {
            let (start, end) = (*start as usize, *end as usize);
            if start < at {
                continue;
            }
            body.push_str(&self.source[at..start]);
            body.push_str(text);
            at = end;
        }
        body.push_str(&self.source[at..]);
        let exposed = self.exposed.join(", ");
        format!("{body}\n;return {{ {exposed} }}\n}}\n")
    }
}

fn legacy_export(name: &str) -> String {
    match name {
        "state" => "old API: `export const state` → declare each value as a top-level `let` (`let count = 0`); props are `export let step = 1`".into(),
        "actions" => "old API: `export const actions` → write top-level functions (`function add() {…}`) and call them from HTML (`@click=\"add\"`, `@change=\"toggle(task)\"`)".into(),
        _ => "old API: `export function mount(ctx)` → `function mount() { …; return cleanup }` without export; services come from `import { … } from 'pragma'`".into(),
    }
}

/// Root identifier of `a.b[c].d`.
fn member_root<'a, 'b>(mut e: &'b Expression<'a>) -> Option<&'b IdentifierReference<'a>> {
    loop {
        match e {
            Expression::Identifier(id) => return Some(id),
            Expression::StaticMemberExpression(m) => e = &m.object,
            Expression::ComputedMemberExpression(m) => e = &m.object,
            Expression::PrivateFieldExpression(m) => e = &m.object,
            _ => return None,
        }
    }
}

struct Refs<'t, 's, 'c> {
    t: &'t mut Transform<'s>,
    scoping: &'c Scoping,
    /// Function nesting (0 = top level).
    depth: u32,
}

impl Refs<'_, '_, '_> {
    fn sym(&self, id: &IdentifierReference) -> Option<Sym> {
        let reference = self.scoping.get_reference(id.reference_id.get()?);
        self.t.syms.get(&reference.symbol_id()?).copied()
    }

    fn is_write(&self, id: &IdentifierReference) -> bool {
        id.reference_id.get().is_some_and(|r| self.scoping.get_reference(r).is_write())
    }

    fn replacement(&self, id: &IdentifierReference) -> Option<String> {
        match self.sym(id)? {
            Sym::State | Sym::Prop => Some(format!("$.{}", id.name)),
            Sym::Pragma(n) => Some(format!("$pragma.{n}")),
            Sym::Const => None,
        }
    }

    fn prop_write(&mut self, id: &IdentifierReference) {
        if self.sym(id) == Some(Sym::Prop) && self.is_write(id) {
            self.t.err(id.span.start, RULE_PROP_WRITE, format!("`{}` is a prop: props are read-only; copy it into a `let` (`let current = {}`) and change that", id.name, id.name));
        }
    }

    /// `root` is changed in place (`root.x = 1`, `root.push(x)`, `delete root.x`).
    fn mutated_root(&mut self, root: &IdentifierReference) {
        if self.sym(root) != Some(Sym::Const) {
            return;
        }
        let name = root.name.to_string();
        if self.t.info.mutated_consts.iter().any(|(n, _, _)| *n == name) || self.t.info.functions.contains(&name) {
            return;
        }
        let Some(decl) = root.reference_id.get().and_then(|r| self.scoping.get_reference(r).symbol_id()).map(|s| self.scoping.symbol_span(s)) else { return };
        let (line, col) = LineIndex::new(self.t.source).position(self.t.source, decl.start);
        self.t.info.mutated_consts.push((name, line, col));
    }
}

impl<'a> Visit<'a> for Refs<'_, '_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.prop_write(it);
        if let Some(text) = self.replacement(it) {
            self.t.edit(it.span.start, it.span.end, text);
        }
    }

    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        if it.shorthand
            && let Expression::Identifier(id) = &it.value
            && let Some(text) = self.replacement(id)
        {
            self.t.edit(it.span.start, it.span.end, format!("{}: {text}", id.name));
            return;
        }
        walk::walk_object_property(self, it);
    }

    fn visit_assignment_target_property_identifier(&mut self, it: &AssignmentTargetPropertyIdentifier<'a>) {
        self.prop_write(&it.binding);
        match self.replacement(&it.binding) {
            Some(text) => {
                self.t.edit(it.binding.span.start, it.binding.span.end, format!("{}: {text}", it.binding.name));
                if let Some(init) = &it.init {
                    self.visit_expression(init);
                }
            }
            None => walk::walk_assignment_target_property_identifier(self, it),
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::StaticMemberExpression(m) = &it.callee
            && MUTATORS.contains(&m.property.name.as_str())
            && let Some(root) = member_root(&m.object)
        {
            self.mutated_root(root);
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(m) = it.left.as_member_expression() {
            if let Some(root) = member_root(m.object()) {
                self.mutated_root(root);
            }
        }
        walk::walk_assignment_expression(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let Some(m) = it.argument.as_member_expression() {
            if let Some(root) = member_root(m.object()) {
                self.mutated_root(root);
            }
        }
        walk::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete
            && let Some(m) = it.argument.as_member_expression()
            && let Some(root) = member_root(m.object())
        {
            self.mutated_root(root);
        }
        walk::walk_unary_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk::walk_function(self, it, flags);
        self.depth -= 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.depth += 1;
        walk::walk_arrow_function_expression(self, it);
        self.depth -= 1;
    }

    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        if self.depth == 0 {
            self.t.err(it.span.start, RULE_SCRIPT, "no top-level `await`: the script runs for each instance; await inside `mount()` or a function");
        }
        walk::walk_await_expression(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        if it.r#await && self.depth == 0 {
            self.t.err(it.span.start, RULE_SCRIPT, "no top-level `for await`: await inside `mount()` or a function");
        }
        walk::walk_for_of_statement(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if let Expression::Identifier(obj) = &it.object
            && obj.name == "ctx"
        {
            match it.property.name.as_str() {
                "$" => self.t.err(it.span.start, RULE_LEGACY, "old API: `ctx.$.x` → use the top-level `let x` or the prop `x` directly"),
                "update" => self.t.err(it.span.start, RULE_LEGACY, "old API: `ctx.update(fn)` → assign the top-level `let` directly; renders are batched"),
                _ => {}
            }
        }
        walk::walk_static_member_expression(self, it);
    }
}

