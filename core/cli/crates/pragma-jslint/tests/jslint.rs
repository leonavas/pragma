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
fn codigo_limpo_sem_diagnosticos() {
    let src = "import { store } from '/core/state.js'\nimport { fmt } from '../utils/fmt.js'\n\
               export const state = () => ({ n: 0 })\n\
               export const actions = { inc(ctx) { ctx.$.n++ } }\n\
               const sinal = 'a < b e c>d'\nconst texto = `x ${sinal} < y`\n";
    assert_eq!(app(src), vec![]);
}

#[test]
fn inner_html_e_outer_html() {
    assert_eq!(app("el.innerHTML = x"), vec![(1, 4, "js/html-em-js")]);
    assert_eq!(
        app("const total = 1\n  el['outerHTML'] = x"),
        vec![(2, 6, "js/html-em-js")]
    );
    assert_eq!(app("el.textContent = x"), vec![]);
}

#[test]
fn insert_adjacent_html_document_write_e_domparser() {
    assert_eq!(
        app("el.insertAdjacentHTML('beforeend', x)"),
        vec![(1, 4, "js/html-em-js")]
    );
    assert_eq!(app("document.write(x)"), vec![(1, 10, "js/html-em-js")]);
    assert_eq!(
        app("new DOMParser().parseFromString(x, 'text/html')"),
        vec![(1, 17, "js/html-em-js")]
    );
    assert_eq!(app("stream.write(x)"), vec![]);
}

#[test]
fn string_e_template_com_markup() {
    assert_eq!(
        app("const marca = '<div class=\"x\">'"),
        vec![(1, 15, "js/html-em-js")]
    );
    assert_eq!(app("const marca = \"fim</p>\""), vec![(1, 15, "js/html-em-js")]);
    assert_eq!(
        app("const marca = `\n  <li>${x}</li>`"),
        vec![(1, 16, "js/html-em-js")]
    );
    assert_eq!(
        app("const marca = '<!-- c -->'"),
        vec![(1, 15, "js/html-em-js")]
    );
    assert_eq!(
        app("const menor = 'a<b', conta = '1 < 2', simbolo = '<=', molde = `<${x}>`"),
        vec![]
    );
}

#[test]
fn create_element_so_no_core() {
    let src = "const item = document.createElement('li')\nitem.append(document.createTextNode(x))";
    assert_eq!(
        app(src),
        vec![(1, 23, "js/html-em-js"), (2, 22, "js/html-em-js")]
    );
    assert_eq!(diags("core/runtime/directives.js", src, &CORE), vec![]);
}

#[test]
fn core_continua_proibindo_o_resto() {
    let src = "el.innerHTML = ''\nconsole.log(1)";
    assert_eq!(
        diags("core/runtime/boot.js", src, &CORE),
        vec![(1, 4, "js/html-em-js"), (2, 1, "js/console")]
    );
}

#[test]
fn console_proibido() {
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
fn typescript_e_erro_de_sintaxe() {
    assert_eq!(rules("let x: number = 1"), vec!["js/sintaxe"]);
    assert!(rules("interface A { x: string }").contains(&"js/sintaxe"));
    assert!(rules("const a = b as string").contains(&"js/sintaxe"));
    assert!(rules("enum Cor { A }").contains(&"js/sintaxe"));
    let d = app("const total = 1\nconst soma = (\n");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].0, 3);
    assert_eq!(app("const total = 1"), vec![]);
}

#[test]
fn jsx_nao_e_js_puro() {
    assert!(rules("const a = <div />").contains(&"js/sintaxe"));
}

#[test]
fn jsdoc_de_tipos() {
    assert_eq!(
        app("/** @type {number} */\nlet total = 1"),
        vec![(1, 5, "js/jsdoc-tipos")]
    );
    assert_eq!(
        app("/**\n * Soma.\n * @param {number} parcela\n */\nfunction somar(parcela) {}"),
        vec![(3, 4, "js/jsdoc-tipos")]
    );
    assert_eq!(
        rules("/** @returns {string} */\nfunction rotulo() {}"),
        vec!["js/jsdoc-tipos"]
    );
    assert_eq!(
        rules("/** @return {string} */\nfunction rotulo() {}"),
        vec!["js/jsdoc-tipos"]
    );
    assert_eq!(rules("/** @typedef {Object} X */"), vec!["js/jsdoc-tipos"]);
    assert_eq!(rules("/** @template T */"), vec!["js/jsdoc-tipos"]);
    assert_eq!(
        app("let total = 1 // @ts-ignore"),
        vec![(1, 18, "js/jsdoc-tipos")]
    );
    assert_eq!(rules("// @ts-check"), vec!["js/jsdoc-tipos"]);
}

#[test]
fn comentario_e_proibido() {
    assert_eq!(
        rules("/**\n * @param ctx contexto da action\n * @returns nada\n */\nfunction salvar(ctx) {}"),
        vec!["js/comentario"]
    );
    assert_eq!(rules("// ver @types/node e @typescript"), vec!["js/comentario"]);
    assert_eq!(rules("const total = 1 /* x */"), vec!["js/comentario"]);
}

#[test]
fn imports_permitidos() {
    let src = "import a from './a.js'\nimport b from '../b.js'\nimport { c } from '/core/state.js'\n\
               export { d } from './d.js'\nexport * from '../e.js'\nimport('./f.js')\nimport(url)";
    assert_eq!(app(src), vec![]);
}

#[test]
fn imports_proibidos() {
    assert_eq!(
        app("import x from 'https://esm.sh/x'"),
        vec![(1, 15, "js/import")]
    );
    assert_eq!(
        app("import x from 'http://a/x.js'"),
        vec![(1, 15, "js/import")]
    );
    assert_eq!(app("import fs from 'node:fs'"), vec![(1, 16, "js/import")]);
    assert_eq!(app("import x from '/lib/x.js'"), vec![(1, 15, "js/import")]);
    assert_eq!(
        app("const modulo = await import(`pkg`)"),
        vec![(1, 29, "js/import")]
    );
}

#[test]
fn pacotes_npm_permitidos() {
    assert_eq!(app("import dayjs from 'dayjs'"), vec![]);
    assert_eq!(app("import { debounce } from 'lodash-es'"), vec![]);
    assert_eq!(app("import utc from '@scope/pkg/sub.js'"), vec![]);
    assert_eq!(app("export * from 'pkg'"), vec![]);
    assert_eq!(app("export { debounce } from 'pkg'"), vec![]);
    assert_eq!(app("const modulo = await import('pkg')"), vec![]);
    assert_eq!(pragma_jslint::package_name("@scope/pkg/sub.js"), "@scope/pkg");
    assert_eq!(pragma_jslint::package_name("dayjs/plugin/utc.js"), "dayjs");
}

#[test]
fn node_so_no_servidor() {
    let src = "import { readFile } from 'node:fs/promises'";
    assert_eq!(diags("routes/loja/[id].server.js", src, &APP), vec![]);
    assert_eq!(diags("app/core/server/node.mjs", src, &APP), vec![]);
    assert_eq!(diags("core/runtime/server/node.mjs", src, &CORE), vec![]);
    assert_eq!(
        diags("core/runtime/router.js", src, &CORE),
        vec![(1, 26, "js/import")]
    );
    assert_eq!(
        diags("routes/loja/[id].js", src, &APP),
        vec![(1, 26, "js/import")]
    );
    assert_eq!(
        diags("routes/loja/[id].server.js", "import postgres from 'pg'", &APP),
        vec![]
    );
}

#[test]
fn coluna_conta_caracteres_e_nao_bytes() {
    assert_eq!(
        app("const cédula = 'ção'; el.innerHTML = cédula"),
        vec![(1, 26, "js/html-em-js")]
    );
}

#[test]
fn diagnostico_tem_arquivo_e_mensagem() {
    let d = lint(Path::new("routes/x.js"), "console.log(1)", &APP);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].file, Path::new("routes/x.js"));
    assert_eq!(d[0].severity, pragma_diag::Severity::Error);
    assert!(d[0].message.contains("console"));
}

#[test]
fn acoes_exportadas_em_varias_formas() {
    let src = "export const actions = {\n  alternar(ctx) {},\n  async salvar(ctx) {},\n  limpar: (ctx) => {},\n  \
               'com-hifen': function () {},\n  atalho,\n  ...outras,\n  [dinamica]: 1,\n  get g() { return 1 },\n}";
    assert_eq!(
        exported_actions(src),
        vec!["alternar", "salvar", "limpar", "com-hifen", "atalho", "g"]
    );

    let indireto = "const minhas = { a() {}, b: 1 }\nexport { minhas as actions }";
    assert_eq!(exported_actions(indireto), vec!["a", "b"]);

    let separado = "const actions = { x() {} }\nexport { actions }";
    assert_eq!(exported_actions(separado), vec!["x"]);

    assert_eq!(
        exported_actions("export const state = () => ({})"),
        Vec::<String>::new()
    );
    assert_eq!(
        exported_actions("export const actions = criar()"),
        Vec::<String>::new()
    );
    assert_eq!(exported_actions("let x: number"), Vec::<String>::new());
}

#[test]
fn nomes_exportados() {
    let src = "export const state = () => ({})\nexport const actions = {}\n\
               export function mount(ctx) {}\nexport async function load() {}\nexport const ssr = true\n\
               const a = 1\nexport { a as outro }\nexport default store({})";
    assert_eq!(
        exported_names(src),
        vec![
            "state", "actions", "mount", "load", "ssr", "outro", "default"
        ]
    );
}

#[test]
fn minify_preserva_exports_e_imports() {
    let src = "import { store } from '/core/state.js'\nimport './efeito.js'\n\
               import { formatar as fmt } from \"../utils/fmt.js\"\n\
               export const state = () => ({ aberto: false, total: store.x })\n\
               export const actions = {\n  alternar(ctx) { ctx.$.aberto = !ctx.$.aberto },\n  \
               async salvar(ctx) { await ctx.call('salvar', { id: fmt(ctx.params.id) }) },\n}\n\
               export function mount(ctx) {\n  const intervalo = setInterval(() => ctx.update(s => { s.agora = Date.now() }), 1000)\n  \
               return () => clearInterval(intervalo)\n}\nexport { mount as montar }\n\
               export * from './outro.js'\nexport const carregar = () => import('./lazy.js')\n";
    let out = minify(src).unwrap();
    assert!(out.len() < src.len());
    for spec in [
        "/core/state.js",
        "./efeito.js",
        "../utils/fmt.js",
        "./outro.js",
        "./lazy.js",
    ] {
        assert!(out.contains(spec), "falta {spec}: {out}");
    }
    assert_eq!(exported_names(&out), exported_names(src), "{out}");
    assert_eq!(exported_actions(&out), vec!["alternar", "salvar"]);
    assert!(!out.contains("intervalo"), "deveria renomear locais: {out}");
    let diagnosticos: Vec<_> = lint(Path::new("x.js"), &out, &APP)
        .into_iter()
        .filter(|d| d.rule != pragma_jslint::RULE_NAME)
        .collect();
    assert_eq!(diagnosticos, vec![]);
}

#[test]
fn minify_deterministico_e_estavel() {
    let src = "export const state = () => ({ n: 0 })\nexport function mount() { const a = 1, b = 2; return () => a + b }";
    let a = minify(src).unwrap();
    assert_eq!(a, minify(src).unwrap());
    assert_eq!(minify(&a).unwrap(), minify(&minify(&a).unwrap()).unwrap());
}

#[test]
fn minify_erro_de_sintaxe() {
    let err = minify("const a = 1\nlet x: number").unwrap_err();
    assert!(err.starts_with("2:"), "{err}");
}

#[test]
fn minify_reescreve_imports() {
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
        assert!(out.contains(spec), "falta {spec}: {out}");
    }
}
