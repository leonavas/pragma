use pragma_jslint::{
    LintContext, exported_actions, exported_names, lint, minify, minify_with_imports,
};
use std::path::Path;

const APP: LintContext = LintContext { is_core: false };
const CORE: LintContext = LintContext { is_core: true };

fn diags(path: &str, src: &str, ctx: &LintContext) -> Vec<(u32, u32, &'static str)> {
    lint(Path::new(path), src, ctx)
        .into_iter()
        .map(|d| (d.line, d.column, d.rule))
        .collect()
}

fn app(src: &str) -> Vec<(u32, u32, &'static str)> {
    diags("routes/index.js", src, &APP)
}

fn rules(src: &str) -> Vec<&'static str> {
    app(src).into_iter().map(|d| d.2).collect()
}

#[test]
fn clean_code_has_no_diagnostics() {
    let src = "import { store } from '/core/state.js'\nimport { fmt } from '../utils/fmt.js'\n\
               export const state = () => ({ n: 0 })\n\
               export const actions = { inc(ctx) { ctx.$.n++ } }\n\
               const sign = 'a < b and c>d'\nconst text = `x ${sign} < y`\n";
    assert_eq!(app(src), vec![]);
}

#[test]
fn inner_html_and_outer_html() {
    assert_eq!(app("el.innerHTML = x"), vec![(1, 4, "js/html-in-js")]);
    assert_eq!(
        app("const total = 1\n  el['outerHTML'] = x"),
        vec![(2, 6, "js/html-in-js")]
    );
    assert_eq!(app("el.textContent = x"), vec![]);
}

#[test]
fn insert_adjacent_html_document_write_and_domparser() {
    assert_eq!(
        app("el.insertAdjacentHTML('beforeend', x)"),
        vec![(1, 4, "js/html-in-js")]
    );
    assert_eq!(app("document.write(x)"), vec![(1, 10, "js/html-in-js")]);
    assert_eq!(
        app("new DOMParser().parseFromString(x, 'text/html')"),
        vec![(1, 17, "js/html-in-js")]
    );
    assert_eq!(app("stream.write(x)"), vec![]);
}

#[test]
fn string_and_template_with_markup() {
    assert_eq!(
        app("const label = '<div class=\"x\">'"),
        vec![(1, 15, "js/html-in-js")]
    );
    assert_eq!(app("const label = \"end</p>\""), vec![(1, 15, "js/html-in-js")]);
    assert_eq!(
        app("const label = `\n  <li>${x}</li>`"),
        vec![(1, 16, "js/html-in-js")]
    );
    assert_eq!(
        app("const label = '<!-- c -->'"),
        vec![(1, 15, "js/html-in-js")]
    );
    assert_eq!(
        app("const less = 'a<b', sum = '1 < 2', symbol = '<=', shape = `<${x}>`"),
        vec![]
    );
}

#[test]
fn create_element_only_in_core() {
    let src = "const item = document.createElement('li')\nitem.append(document.createTextNode(x))";
    assert_eq!(
        app(src),
        vec![(1, 23, "js/html-in-js"), (2, 22, "js/html-in-js")]
    );
    assert_eq!(diags("core/runtime/directives.js", src, &CORE), vec![]);
}

#[test]
fn core_still_forbids_the_rest() {
    let src = "el.innerHTML = ''\nconsole.log(1)";
    assert_eq!(
        diags("core/runtime/boot.js", src, &CORE),
        vec![(1, 4, "js/html-in-js"), (2, 1, "js/console")]
    );
}

#[test]
fn console_forbidden() {
    assert_eq!(
        app("if (x) {\n\tconsole.log(x)\n}"),
        vec![(2, 2, "js/console")]
    );
    assert_eq!(
        app("globalThis.console.warn(1)"),
        vec![(1, 12, "js/console")]
    );
    assert_eq!(app("ctx.log.info('ok')"), vec![]);
}

#[test]
fn typescript_and_syntax_error() {
    assert_eq!(rules("let x: number = 1"), vec!["js/syntax"]);
    assert!(rules("interface A { x: string }").contains(&"js/syntax"));
    assert!(rules("const a = b as string").contains(&"js/syntax"));
    assert!(rules("enum Color { A }").contains(&"js/syntax"));
    let d = app("const total = 1\nconst sum = (\n");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, 3);
    assert_eq!(app("const total = 1"), vec![]);
}

#[test]
fn jsx_is_not_plain_js() {
    assert!(rules("const a = <div />").contains(&"js/syntax"));
}

#[test]
fn type_jsdoc() {
    assert_eq!(
        app("/** @type {number} */\nlet total = 1"),
        vec![(1, 5, "js/jsdoc-types")]
    );
    assert_eq!(
        app("/**\n * Sum.\n * @param {number} part\n */\nfunction add(part) {}"),
        vec![(3, 4, "js/jsdoc-types")]
    );
    assert_eq!(
        rules("/** @returns {string} */\nfunction label() {}"),
        vec!["js/jsdoc-types"]
    );
    assert_eq!(
        rules("/** @return {string} */\nfunction label() {}"),
        vec!["js/jsdoc-types"]
    );
    assert_eq!(rules("/** @typedef {Object} X */"), vec!["js/jsdoc-types"]);
    assert_eq!(rules("/** @template T */"), vec!["js/jsdoc-types"]);
    assert_eq!(
        app("let total = 1 // @ts-ignore"),
        vec![(1, 18, "js/jsdoc-types")]
    );
    assert_eq!(rules("// @ts-check"), vec!["js/jsdoc-types"]);
}

#[test]
fn comments_are_forbidden() {
    assert_eq!(
        rules("/**\n * @param ctx action context\n * @returns nothing\n */\nfunction save(ctx) {}"),
        vec!["js/comment"]
    );
    assert_eq!(rules("// see @types/node and @typescript"), vec!["js/comment"]);
    assert_eq!(rules("const total = 1 /* x */"), vec!["js/comment"]);
}

#[test]
fn allowed_imports() {
    let src = "import a from './a.js'\nimport b from '../b.js'\nimport { c } from '/core/state.js'\n\
               export { d } from './d.js'\nexport * from '../e.js'\nimport('./f.js')";
    assert_eq!(app(src), vec![]);
    assert_eq!(diags("core/runtime/router.js", "import(url)", &CORE), vec![]);
}

#[test]
fn computed_dynamic_import() {
    assert_eq!(app("const pkg = 'date-fns'\nawait import(pkg)"), vec![(2, 14, "js/import")]);
    assert_eq!(app("await import(`${name}`)"), vec![(1, 14, "js/import")]);
    assert_eq!(app("await import(`./locales/${lang}.js`)"), vec![(1, 14, "js/import")]);
    assert_eq!(diags("routes/shop/[id].server.js", "await import(driver)", &APP), vec![(1, 14, "js/import")]);
}

#[test]
fn forbidden_imports() {
    assert_eq!(
        app("import x from 'https://esm.sh/x'"),
        vec![(1, 15, "js/import")]
    );
    assert_eq!(
        app("import x from 'http://a/x.js'"),
        vec![(1, 15, "js/import")]
    );
    assert_eq!(app("import fs from 'node:fs'"), vec![(1, 16, "app/package")]);
    assert_eq!(app("import x from '/lib/x.js'"), vec![(1, 15, "js/import")]);
    assert_eq!(
        app("const loaded = await import(`pkg`)"),
        vec![(1, 29, "js/import")]
    );
}

#[test]
fn npm_packages_allowed() {
    assert_eq!(app("import dayjs from 'dayjs'"), vec![]);
    assert_eq!(app("import { debounce } from 'lodash-es'"), vec![]);
    assert_eq!(app("import utc from '@scope/pkg/sub.js'"), vec![]);
    assert_eq!(app("export * from 'pkg'"), vec![]);
    assert_eq!(app("export { debounce } from 'pkg'"), vec![]);
    assert_eq!(app("const loaded = await import('pkg')"), vec![]);
    assert_eq!(pragma_jslint::package_name("@scope/pkg/sub.js"), "@scope/pkg");
    assert_eq!(pragma_jslint::package_name("dayjs/plugin/utc.js"), "dayjs");
}

#[test]
fn node_builtins_detected() {
    for spec in ["fs", "fs/promises", "path", "node:fs", "node:test", "crypto", "buffer"] {
        assert!(pragma_jslint::is_node_builtin(spec), "{spec}");
    }
    for spec in ["dayjs", "fs-extra", "pathe", "@scope/fs", "./fs.js"] {
        assert!(!pragma_jslint::is_node_builtin(spec), "{spec}");
    }
    assert!(pragma_jslint::node_builtin_message("fs").starts_with("`fs` is a Node module and doesn't run in the browser"));
}

#[test]
fn node_only_on_server() {
    let src = "import { readFile } from 'node:fs/promises'";
    assert_eq!(diags("routes/shop/[id].server.js", src, &APP), vec![]);
    assert_eq!(diags("app/core/server/node.mjs", src, &APP), vec![]);
    assert_eq!(diags("core/runtime/server/node.mjs", src, &CORE), vec![]);
    assert_eq!(
        diags("core/runtime/router.js", src, &CORE),
        vec![(1, 26, "app/package")]
    );
    assert_eq!(
        diags("routes/shop/[id].js", src, &APP),
        vec![(1, 26, "app/package")]
    );
    assert_eq!(
        diags("routes/shop/[id].server.js", "import postgres from 'pg'", &APP),
        vec![]
    );
}

#[test]
fn column_counts_chars_not_bytes() {
    assert_eq!(
        app("const résumé = 'café'; el.innerHTML = résumé"),
        vec![(1, 27, "js/html-in-js")]
    );
}

#[test]
fn diagnostic_has_file_and_message() {
    let d = lint(Path::new("routes/x.js"), "console.log(1)", &APP);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].file, Path::new("routes/x.js"));
    assert_eq!(d[0].severity, pragma_diag::Severity::Error);
    assert!(d[0].message.contains("console"));
}

#[test]
fn exported_actions_in_many_shapes() {
    let src = "export const actions = {\n  toggle(ctx) {},\n  async save(ctx) {},\n  clear: (ctx) => {},\n  \
               'with-hyphen': function () {},\n  shorthand,\n  ...others,\n  [dynamic]: 1,\n  get g() { return 1 },\n}";
    assert_eq!(
        exported_actions(src),
        vec!["toggle", "save", "clear", "with-hyphen", "shorthand", "g"]
    );

    let indirect = "const mine = { a() {}, b: 1 }\nexport { mine as actions }";
    assert_eq!(exported_actions(indirect), vec!["a", "b"]);

    let separate = "const actions = { x() {} }\nexport { actions }";
    assert_eq!(exported_actions(separate), vec!["x"]);

    assert_eq!(
        exported_actions("export const state = () => ({})"),
        Vec::<String>::new()
    );
    assert_eq!(
        exported_actions("export const actions = create()"),
        Vec::<String>::new()
    );
    assert_eq!(exported_actions("let x: number"), Vec::<String>::new());
}

#[test]
fn exported_names_in_order() {
    let src = "export const state = () => ({})\nexport const actions = {}\n\
               export function mount(ctx) {}\nexport async function load() {}\nexport const ssr = true\n\
               const a = 1\nexport { a as other }\nexport default store({})";
    assert_eq!(
        exported_names(src),
        vec![
            "state", "actions", "mount", "load", "ssr", "other", "default"
        ]
    );
}

#[test]
fn minify_keeps_exports_and_imports() {
    let src = "import { store } from '/core/state.js'\nimport './effect.js'\n\
               import { format as fmt } from \"../utils/fmt.js\"\n\
               export const state = () => ({ open: false, total: store.x })\n\
               export const actions = {\n  toggle(ctx) { ctx.$.open = !ctx.$.open },\n  \
               async save(ctx) { await ctx.call('save', { id: fmt(ctx.params.id) }) },\n}\n\
               export function mount(ctx) {\n  const ticker = setInterval(() => ctx.update(s => { s.now = Date.now() }), 1000)\n  \
               return () => clearInterval(ticker)\n}\nexport { mount as attach }\n\
               export * from './other.js'\nexport const loadLater = () => import('./lazy.js')\n";
    let out = minify(src).unwrap();
    assert!(out.len() < src.len());
    for spec in [
        "/core/state.js",
        "./effect.js",
        "../utils/fmt.js",
        "./other.js",
        "./lazy.js",
    ] {
        assert!(out.contains(spec), "missing {spec}: {out}");
    }
    assert_eq!(exported_names(&out), exported_names(src), "{out}");
    assert_eq!(exported_actions(&out), vec!["toggle", "save"]);
    assert!(!out.contains("ticker"), "should rename locals: {out}");
    let diagnostics: Vec<_> = lint(Path::new("x.js"), &out, &APP)
        .into_iter()
        .filter(|d| d.rule != pragma_jslint::RULE_NAME)
        .collect();
    assert_eq!(diagnostics, vec![]);
}

#[test]
fn minify_is_deterministic_and_stable() {
    let src = "export const state = () => ({ n: 0 })\nexport function mount() { const a = 1, b = 2; return () => a + b }";
    let a = minify(src).unwrap();
    assert_eq!(a, minify(src).unwrap());
    assert_eq!(minify(&a).unwrap(), minify(&minify(&a).unwrap()).unwrap());
}

#[test]
fn minify_syntax_error() {
    let err = minify("const a = 1\nlet x: number").unwrap_err();
    assert!(err.starts_with("2:"), "{err}");
}

#[test]
fn minify_rewrites_imports() {
    let src = "import { a } from './a.js'\nimport '/core/boot.js'\nexport * from './b.js'\n\
               export { c } from './c.js'\nexport const f = () => import('./d.js')\nconsole.info(a)";
    let out = minify_with_imports(src, |s| {
        (s != "/core/boot.js").then(|| s.replace(".js", "-h1.js"))
    })
    .unwrap();
    for spec in [
        "./a-h1.js",
        "/core/boot.js",
        "./b-h1.js",
        "./c-h1.js",
        "./d-h1.js",
    ] {
        assert!(out.contains(spec), "missing {spec}: {out}");
    }
}

#[test]
fn digit_suffix_flags_copies_not_domain_numbers() {
    assert_eq!(rules("const item2 = 1"), vec!["js/vague-name"]);
    assert_eq!(rules("const valor1 = 1"), vec!["js/vague-name"]);
    assert_eq!(rules("const pode42 = 1"), Vec::<&str>::new());
    assert_eq!(rules("const INICIO_EC141 = 1"), Vec::<&str>::new());
    assert_eq!(rules("const escala12x36 = 1"), Vec::<&str>::new());
    assert_eq!(rules("const carga5x2 = 1"), Vec::<&str>::new());
}
