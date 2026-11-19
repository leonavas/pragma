use pragma_compiler::{BuildOptions, Mode, build, check};
use pragma_diag::Diagnostic;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tmp() -> Tmp {
    let p = std::env::temp_dir().join(format!("pragma-compiler-test-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Tmp(p)
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &t);
        } else {
            fs::copy(e.path(), t).unwrap();
        }
    }
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/app")
}

/// Copia o app de fixture e aplica trocas: `Some(conteúdo)` escreve, `None` apaga.
fn app_with(changes: &[(&str, Option<&str>)]) -> Tmp {
    let t = tmp();
    copy_dir(&fixture(), &t.0.join("app"));
    for (f, c) in changes {
        let path = t.0.join("app").join(f);
        match c {
            Some(c) => {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, c).unwrap();
            }
            None => fs::remove_file(path).unwrap(),
        }
    }
    t
}

fn check_with(changes: &[(&str, Option<&str>)]) -> Vec<Diagnostic> {
    let t = app_with(changes);
    check(&t.0.join("app"))
}

#[track_caller]
fn expect(d: &[Diagnostic], rule: &str, file: &str, line: u32) {
    let found = d.iter().any(|x| x.rule == rule && x.file == Path::new(file) && x.line == line);
    let list: Vec<String> = d.iter().map(|x| x.to_string()).collect();
    assert!(found, "esperava {rule} em {file}:{line}; veio:\n{}", list.join("\n"));
}

const PAGE: &str = "routes/index.html";

fn page(body: &str) -> Vec<Diagnostic> {
    check_with(&[(PAGE, Some(body))])
}

#[test]
fn fixture_is_clean() {
    let d = check(&fixture());
    assert!(d.is_empty(), "{:#?}", d);
}

#[test]
fn dev_snapshot() {
    let t = tmp();
    let out = t.0.join("dev");
    let r = build(&BuildOptions { root: fixture(), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    assert!(r.has_server);
    let snap_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let files = [
        "client/index.html",
        "client/loja/[id].html",
        "client/blog/[slug].html",
        "client/_404.html",
        "client/_pragma/manifest.json",
        "client/assets/routes/index.js",
        "templates/blog/[slug].json",
        "server/worker.js",
        "server/node.mjs",
        "server/wrangler.jsonc",
    ];
    let mut failed = Vec::new();
    for f in files {
        let got = fs::read_to_string(out.join(f)).unwrap_or_else(|_| panic!("faltou {f}"));
        let snap = snap_dir.join(f.replace('/', "__"));
        if update {
            fs::create_dir_all(&snap_dir).unwrap();
            fs::write(&snap, &got).unwrap();
        } else if fs::read_to_string(&snap).ok().as_deref() != Some(got.as_str()) {
            failed.push(f);
        }
    }
    assert!(failed.is_empty(), "snapshots diferentes (rode com UPDATE_SNAPSHOTS=1 e revise): {failed:?}");
    for f in ["client/assets/app.css", "client/robots.txt", "client/img/icon.svg", "server/routes/loja/[id].server.js", "server/utils/db.js", "server/core/server/render.js", "client/assets/core/lazy.js"] {
        assert!(out.join(f).is_file(), "faltou {f}");
    }
    assert!(!out.join("templates/loja/[id].json").exists(), "template só para rota ssr");
}

fn all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        if e.file_type().unwrap().is_dir() {
            all_files(&e.path(), out);
        } else {
            out.push(e.path());
        }
    }
}

fn urls(html: &str, attr: &str) -> Vec<String> {
    html.split(&format!("{attr}=\"")).skip(1).map(|s| s[..s.find('"').unwrap()].to_string()).collect()
}

#[test]
fn prod_hashes_and_rewrites() {
    let t = tmp();
    let out = t.0.join("dist");
    let start = std::time::Instant::now();
    let r = build(&BuildOptions { root: fixture(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 2000, "build lento: {elapsed:?}");
    assert_eq!(r.pages.len(), 4);
    assert!(r.css_gzip_bytes > 0);
    let client = out.join("client");
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    for u in urls(&index, "href").into_iter().chain(urls(&index, "src")) {
        if u.starts_with("/assets/") {
            assert!(client.join(&u[1..]).is_file(), "{u} não existe");
            assert!(u.rsplit('-').next().unwrap().len() == 11 || u.ends_with(".css"), "{u} sem hash");
        }
    }
    assert!(!index.contains("/_pragma/dev.js") && !index.contains("__pragmaDev"));
    assert!(!index.contains("__pd"));
    let produto = fs::read_to_string(client.join("loja/[id].html")).unwrap();
    assert!(produto.contains("<script>self.__pd=fetch("));
    let blog = fs::read_to_string(client.join("blog/[slug].html")).unwrap();
    assert!(!blog.contains("__pd"), "página ssr não leva __pd");
    let mut files = Vec::new();
    all_files(&client.join("assets"), &mut files);
    for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "js")) {
        let src = fs::read_to_string(f).unwrap();
        assert!(!src.contains("'./") && !src.contains("\"./") && !src.contains("../"), "import sem reescrever em {f:?}: {src}");
        assert!(!src.contains("\n  "), "não minificado: {f:?}");
        for spec in src.split("/assets/").skip(1) {
            let end = spec.find(['"', '\'', '`']).unwrap();
            assert!(client.join("assets").join(&spec[..end]).is_file(), "import quebrado {spec} em {f:?}");
        }
    }
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    assert!(worker.contains("service: \"loja-teste\""));
    assert!(worker.contains("ssr: true, template: [[\"html\""));
    assert!(out.join("templates/blog/[slug].json").is_file());
}

#[test]
fn hash_follows_dependencies() {
    let t = app_with(&[]);
    let root = t.0.join("app");
    let names = |out: &Path| {
        let mut f = Vec::new();
        all_files(&out.join("client/assets"), &mut f);
        let mut v: Vec<String> = f.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        v.sort();
        v
    };
    let a = t.0.join("a");
    build(&BuildOptions { root: root.clone(), out_dir: a.clone(), mode: Mode::Prod }).unwrap();
    fs::write(root.join("utils/fmt.js"), "export function formatar(valor) {\n  return `R$ ${valor}`\n}\n").unwrap();
    let b = t.0.join("b");
    build(&BuildOptions { root: root.clone(), out_dir: b.clone(), mode: Mode::Prod }).unwrap();
    let (na, nb) = (names(&a), names(&b));
    let changed: Vec<&String> = nb.iter().filter(|n| !na.contains(n)).collect();
    let prefix = |s: &str| changed.iter().any(|n| n.starts_with(s));
    assert!(prefix("fmt-") && prefix("index-"), "fmt e quem importa mudam: {changed:?}");
    assert!(!prefix("boot-") && !prefix("carrinho-"), "o resto não muda: {changed:?}");
}

#[test]
fn dev_rebuild_removes_stale_files() {
    let t = app_with(&[]);
    let root = t.0.join("app");
    let out = t.0.join("dev");
    let opts = BuildOptions { root: root.clone(), out_dir: out.clone(), mode: Mode::Dev };
    build(&opts).unwrap();
    assert!(out.join("client/loja/index.html").is_file());
    fs::remove_file(root.join("routes/loja/index.html")).unwrap();
    build(&opts).unwrap();
    assert!(!out.join("client/loja/index.html").exists());
    assert!(out.join("client/loja/[id].html").is_file());
}

#[test]
fn diagnostics_are_sorted() {
    let d = check_with(&[
        ("routes/loja/index.html", Some("<p style=\"x\">a</p>\n<p onclick=\"x\">b</p>")),
        (PAGE, Some("<div data-foo=\"x\"></div>\n<p class=\"nada-disso\"></p>")),
    ]);
    let keys: Vec<_> = d.iter().map(|x| (x.file.clone(), x.line, x.column)).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert_eq!(d.len(), 4, "{d:#?}");
}

// Um teste por diagnóstico.

#[test]
fn missing_layout() {
    expect(&check_with(&[("routes/_layout.html", None)]), "app/estrutura", "routes/_layout.html", 1);
}

#[test]
fn route_file_and_index_conflict() {
    expect(&check_with(&[("routes/loja.html", Some("<p>x</p>"))]), "app/rota-conflito", "routes/loja.html", 1);
}

#[test]
fn param_routes_conflict() {
    let d = check_with(&[("routes/loja/[slug].html", Some("<p>x</p>"))]);
    assert!(d.iter().any(|x| x.rule == "app/rota-conflito"), "{d:#?}");
}

#[test]
fn orphan_module() {
    expect(&check_with(&[("routes/sobre.js", Some("export const actions = {}\n"))]), "app/estrutura", "routes/sobre.js", 1);
}

#[test]
fn unknown_pragma_json_key() {
    expect(&check_with(&[("pragma.json", Some("{\n  \"name\": \"x\",\n  \"tema\": 1\n}\n"))]), "app/pragma-json", "pragma.json", 3);
}

#[test]
fn unclosed_tag() {
    expect(&page("<section>\n  <div>\n</section>"), "html/sintaxe", PAGE, 2);
}

#[test]
fn unknown_directive() {
    expect(&page("<p>a</p>\n<p data-texto=\"data.x\">b</p>"), "html/diretiva-desconhecida", PAGE, 2);
}

#[test]
fn invalid_path() {
    expect(&page("<p :text=\"$itens[0]\"></p>"), "html/caminho-invalido", PAGE, 1);
}

#[test]
fn negation_only_where_allowed() {
    expect(&page("<p :text=\"!$aberto\"></p>"), "html/caminho-invalido", PAGE, 1);
}

#[test]
fn unknown_root() {
    expect(&page("<p :text=\"usuario.nome\"></p>"), "html/raiz-invalida", PAGE, 1);
}

#[test]
fn for_variable_out_of_scope() {
    let src = "<ul><template :for=\"x in $itens\" :key=\"x.id\"><li :text=\"x.nome\"></li></template></ul>\n<p :text=\"x.nome\"></p>";
    expect(&page(src), "html/raiz-invalida", PAGE, 2);
}

#[test]
fn data_without_loader() {
    expect(&page("<p :text=\"data.x\"></p>"), "html/raiz-invalida", PAGE, 1);
}

#[test]
fn state_without_module() {
    let d = check_with(&[("routes/loja/index.html", Some("<p :text=\"$x\"></p>"))]);
    expect(&d, "html/raiz-invalida", "routes/loja/index.html", 1);
}

#[test]
fn unknown_store() {
    expect(&page("<p :text=\"shared.sacola.total\"></p>"), "html/raiz-invalida", PAGE, 1);
}

#[test]
fn unknown_param() {
    let d = check_with(&[("routes/loja/[id].html", Some("<p :text=\"params.slug\"></p>"))]);
    expect(&d, "html/raiz-invalida", "routes/loja/[id].html", 1);
}

#[test]
fn props_outside_component() {
    expect(&page("<p data-text=\"state.titulo\"></p>"), "html/sintaxe-antiga", PAGE, 1);
}

#[test]
fn alias_missing_in_component() {
    let d = check_with(&[("routes/loja/[id].html", Some("<p-preco></p-preco>"))]);
    expect(&d, "html/raiz-invalida", "routes/loja/_components/preco/preco.html", 1);
}

#[test]
fn for_without_key() {
    expect(&page("<ul>\n<template :for=\"x in $itens\"><li></li></template></ul>"), "html/for-sem-key", PAGE, 2);
}

#[test]
fn key_must_use_for_variable() {
    expect(&page("<ul><template :for=\"x in $itens\" :key=\"y.id\"><li></li></template></ul>"), "html/raiz-invalida", PAGE, 1);
}

#[test]
fn if_outside_template() {
    expect(&page("<div :if=\"$aberto\"></div>"), "html/fora-de-template", PAGE, 1);
}

#[test]
fn else_out_of_place() {
    expect(&page("<p>a</p>\n<template :else><p>b</p></template>"), "html/else-fora-de-lugar", PAGE, 2);
}

#[test]
fn empty_out_of_place() {
    let src = "<template :if=\"$aberto\"><p>a</p></template>\n<template :empty><p>b</p></template>";
    expect(&page(src), "html/empty-fora-de-lugar", PAGE, 2);
}

#[test]
fn block_needs_single_root() {
    expect(&page("<template :if=\"$aberto\">\n  <p>a</p>\n  <p>b</p>\n</template>"), "html/bloco-sem-raiz-unica", PAGE, 1);
}

#[test]
fn unknown_page_action() {
    expect(&page("<button @click=\"sumir\">x</button>"), "html/acao-inexistente", PAGE, 1);
}

const CAIXA: [(&str, Option<&str>); 2] = [
    ("lib/components/caixa/caixa.html", Some("<section><button @click=\"fechar\">x</button><slot></slot></section>")),
    ("lib/components/caixa/caixa.js", Some("export const actions = { fechar() {} }")),
];

#[test]
fn slot_content_belongs_to_writer() {
    let src = "<c-caixa><button @click=\"fechar\">x</button></c-caixa>";
    expect(&check_with(&[CAIXA[0], CAIXA[1], (PAGE, Some(src))]), "html/acao-inexistente", PAGE, 1);
    let ok = "<c-card titulo=\"x\"><c-caixa><button @click=\"alternar\" :text=\"$aberto\">x</button></c-caixa></c-card>";
    let d = check_with(&[CAIXA[0], CAIXA[1], (PAGE, Some(ok))]);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn slot_marks_lexical_owner_and_module_props() {
    let src = "<c-card titulo=\"x\"><p>fixo</p><div><c-caixa passo=\"2\" class=\"p-4\"><b :text=\"$aberto\"></b></c-caixa></div></c-card>";
    let t = app_with(&[CAIXA[0], CAIXA[1], (PAGE, Some(src))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(html.contains("<p>fixo</p>"), "{html}");
    assert!(html.contains("<div data-slot=\"1\"><section data-c=\"caixa\" data-passo=\"2\" class=\"p-4\">"), "{html}");
    assert!(html.contains("<b data-text=\"state.aberto\" data-slot=\"1\">"), "{html}");
    assert!(html.contains("<div data-c=\"card\" class=\"card p-4\" data-titulo=\"x\">"), "{html}");
}

#[test]
fn data_slot_is_generated() {
    expect(&page("<p data-slot=\"1\">x</p>"), "html/diretiva-desconhecida", PAGE, 1);
}

#[test]
fn layout_action_without_owner() {
    let layout = "<button @click=\"abrir\">x</button>\n<main data-page><slot></slot></main>";
    expect(&check_with(&[("routes/_layout.html", Some(layout))]), "html/acao-inexistente", "routes/_layout.html", 1);
}

#[test]
fn unknown_component() {
    expect(&page("<section>\n  <c-tabela></c-tabela>\n</section>"), "html/componente-inexistente", PAGE, 2);
}

#[test]
fn route_component_not_visible_elsewhere() {
    expect(&page("<p-preco></p-preco>"), "html/componente-inexistente", PAGE, 1);
}

#[test]
fn unknown_slot() {
    expect(&page("<c-card titulo=\"x\">\n  <p slot=\"rodape\">x</p>\n</c-card>"), "html/componente", PAGE, 2);
}

#[test]
fn component_single_root() {
    let d = check_with(&[("lib/components/badge/badge.html", Some("<span></span>\n<span></span>"))]);
    expect(&d, "html/componente", "lib/components/badge/badge.html", 2);
}

#[test]
fn component_cycle() {
    let d = check_with(&[("lib/components/badge/badge.html", Some("<span><c-badge></c-badge></span>")), (PAGE, Some("<c-badge></c-badge>"))]);
    expect(&d, "html/componente", "lib/components/badge/badge.html", 1);
}

#[test]
fn inline_style() {
    expect(&page("<p style=\"color: red\">x</p>"), "html/style-inline", PAGE, 1);
}

#[test]
fn inline_handler() {
    expect(&page("<p>\n<button onclick=\"x()\">x</button></p>"), "html/on-inline", PAGE, 2);
}

#[test]
fn script_tag() {
    expect(&page("<script>alert(1)</script>"), "html/script", PAGE, 1);
}

#[test]
fn layout_without_main() {
    expect(&check_with(&[("routes/_layout.html", Some("<div><slot></slot></div>"))]), "html/layout", "routes/_layout.html", 1);
}

#[test]
fn unknown_class() {
    expect(&page("<p class=\"p-4 bonito\">x</p>"), "css/classe-desconhecida", PAGE, 1);
}

#[test]
fn arbitrary_value() {
    expect(&page("<p class=\"w-[37px]\">x</p>"), "css/valor-arbitrario", PAGE, 1);
}

#[test]
fn budget_exceeded() {
    let cfg = "{\n  \"name\": \"x\",\n  \"budgets\": {\n    \"initialJsKb\": 0.1\n  }\n}\n";
    expect(&check_with(&[("pragma.json", Some(cfg))]), "app/orcamento", "pragma.json", 4);
}

#[test]
fn js_lint_runs_on_app_code() {
    let d = check_with(&[("utils/fmt.js", Some("export function formatar(n) {\n  console.log(n)\n  return n\n}\n"))]);
    expect(&d, "js/console", "utils/fmt.js", 2);
}

#[test]
fn missing_import() {
    let d = check_with(&[("routes/index.js", Some("import { x } from './nao-existe.js'\nexport const actions = { alternar() { return x } }\n"))]);
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn client_cannot_import_server_module() {
    let src = "import { load } from './loja/[id].server.js'\nexport const actions = { alternar() { return load } }\n";
    expect(&check_with(&[("routes/index.js", Some(src))]), "js/import", "routes/index.js", 1);
}

#[test]
fn build_fails_with_errors() {
    let t = app_with(&[(PAGE, Some("<p style=\"x\">a</p>"))]);
    let r = build(&BuildOptions { root: t.0.join("app"), out_dir: t.0.join("dist"), mode: Mode::Prod });
    assert!(r.is_err());
}

#[test]
fn real_starter_is_clean() {
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let (starter, runtime) = (core.join("starter"), core.join("runtime"));
    if !starter.join("routes").is_dir() || !runtime.join("boot.js").is_file() {
        return;
    }
    let t = tmp();
    let app = t.0.join("app");
    copy_dir(&starter, &app);
    copy_dir(&runtime, &app.join("core"));
    assert!(check(&app).is_empty(), "{:#?}", check(&app));
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let index = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(index.contains("data-c=\"modal\"") && !index.contains("props."));
}

#[test]
fn preload_only_what_first_paint_uses() {
    let t = app_with(&[
        ("core/overlay.js", Some("export const open = () => {}")),
        ("routes/sobre.html", Some("<dialog id=\"d\"><p>x</p></dialog>")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let index = fs::read_to_string(out.join("client/index.html")).unwrap();
    let sobre = fs::read_to_string(out.join("client/sobre.html")).unwrap();
    assert!(out.join("client/assets/core/lazy.js").is_file());
    assert!(!index.contains("lazy.js"), "import() dinâmico não entra no modulepreload");
    assert!(!index.contains("overlay.js"), "página sem overlay não pré-carrega overlay.js");
    assert!(sobre.contains("<link rel=\"modulepreload\" href=\"/assets/core/overlay.js\">"), "{sobre}");
}

#[test]
fn theme_script_only_when_used() {
    let t = app_with(&[
        ("core/theme.js", Some("export const setTheme = () => {}")),
        ("routes/index.js", Some("import { setTheme } from '../core/theme.js'\nexport const state = () => ({ aberto: false, itens: [] })\nexport const actions = { alternar() { setTheme('dark') } }\n")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/loja/[id].html")).unwrap();
    let theme = html.find("localStorage.getItem('pragma:theme')").expect("script de tema em toda página");
    assert!(theme < html.find("rel=\"stylesheet\"").unwrap(), "antes do CSS");
    let d = check(&fixture());
    assert!(d.is_empty());
    let plain = app_with(&[]);
    build(&BuildOptions { root: plain.0.join("app"), out_dir: plain.0.join("out"), mode: Mode::Dev }).unwrap();
    assert!(!fs::read_to_string(plain.0.join("out/client/index.html")).unwrap().contains("pragma:theme"));
}

#[test]
fn aria_current_on_links_to_the_page() {
    let t = app_with(&[(PAGE, Some("<a href=\"/\">aqui</a> <a href=\"/?x=1\">q</a> <a href=\"/loja/1\">l</a> <a href=\"/\" target=\"_blank\">n</a>"))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(html.contains("<a href=\"/\" aria-current=\"page\">aqui</a> <a href=\"/?x=1\">q</a> <a href=\"/loja/1\">l</a> <a href=\"/\" target=\"_blank\">n</a>"), "{html}");
}

#[track_caller]
fn warns(body: &str, rule: &str, line: u32) {
    let d = page(body);
    expect(&d, rule, PAGE, line);
    assert!(d.iter().all(|x| x.severity == pragma_diag::Severity::Warning), "{d:?}");
}

#[test]
fn a11y_img_alt() {
    warns("<img src=\"/a.png\">", "a11y/img-alt", 1);
    assert!(page("<img src=\"/a.png\" alt=\"\">").is_empty());
}

#[test]
fn a11y_accessible_name() {
    warns("<p>x</p>\n<button><svg aria-hidden=\"true\"></svg></button>", "a11y/nome-acessivel", 2);
    warns("<a href=\"/\"></a>", "a11y/nome-acessivel", 1);
    let ok = "<button :text=\"$aberto\"></button><button aria-label=\"Fechar\">×</button><a href=\"/\"><span class=\"sr-only\">Início</span></a>";
    assert!(page(ok).is_empty(), "{:?}", page(ok));
}

#[test]
fn a11y_label() {
    warns("<input name=\"q\">", "a11y/label", 1);
    warns("<select><option>a</option></select>", "a11y/label", 1);
    let ok = "<label>Nome <input></label><label for=\"e\">E</label><input id=\"e\"><textarea aria-label=\"T\"></textarea><input type=\"hidden\">";
    assert!(page(ok).is_empty(), "{:?}", page(ok));
}

#[test]
fn a11y_dialog_name() {
    warns("<dialog><p>x</p></dialog>", "a11y/dialog-nome", 1);
    assert!(page("<dialog aria-label=\"Sobre\"><p>x</p></dialog>").is_empty());
}

#[test]
fn a11y_click_on_non_interactive() {
    warns("<div @click=\"alternar\">x</div>", "a11y/clique-nao-interativo", 1);
    assert!(page("<div role=\"button\" tabindex=\"0\" @click=\"alternar\">x</div>").is_empty());
}

#[test]
fn a11y_headings() {
    warns("<h1>a</h1>\n<h3>b</h3>", "a11y/heading", 2);
    warns("<h1>a</h1>\n<h2>b</h2>\n<h1>c</h1>", "a11y/h1-unico", 3);
    assert!(page("<h1>a</h1><h2>b</h2><h3>c</h3><h2>d</h2>").is_empty());
    let branches = "<template :if=\"$aberto\"><h1>a</h1></template>\n<template :else><h1>b</h1></template>";
    assert!(page(branches).is_empty(), "{:?}", page(branches));
    warns(&format!("{branches}\n<h1>c</h1>"), "a11y/h1-unico", 3);
}

#[test]
fn a11y_tabindex() {
    warns("<p tabindex=\"2\">x</p>", "a11y/tabindex", 1);
}

#[test]
fn a11y_duplicate_id() {
    warns("<p id=\"x\">a</p>\n<p id=\"x\">b</p>", "a11y/id-duplicado", 2);
}

#[test]
fn a11y_iframe_title() {
    warns("<iframe src=\"/x\"></iframe>", "a11y/iframe-title", 1);
}

#[test]
fn a11y_link_without_href() {
    warns("<a>x</a>", "a11y/a-sem-href", 1);
}

#[test]
fn a11y_points_to_component_source() {
    let d = check_with(&[("lib/components/foto/foto.html", Some("<img src=\"/f.png\">")), (PAGE, Some("<c-foto></c-foto>\n<c-foto></c-foto>"))]);
    expect(&d, "a11y/img-alt", "lib/components/foto/foto.html", 1);
    assert_eq!(d.len(), 1, "{d:?}");
}

#[test]
fn alias_in_component_model() {
    let busca = ("lib/components/busca/busca.html", Some("<input aria-label=\"Buscar\" :model=\"$valor\">"));
    let ok = check_with(&[busca, (PAGE, Some("<c-busca :valor=\"$aberto\"></c-busca>"))]);
    assert!(ok.is_empty(), "{ok:?}");
    let d = check_with(&[busca, (PAGE, Some("<ul><template :for=\"x in $itens\" :key=\"x.id\"><li><c-busca :valor=\"x.nome\"></c-busca></li></template></ul>"))]);
    expect(&d, "html/raiz-invalida", "lib/components/busca/busca.html", 1);
}

#[test]
fn state_alias_into_module_component() {
    expect(&page("<c-contador :n=\"$aberto\"></c-contador>"), "html/componente", PAGE, 1);
}

const PAGINA_COM_PACOTE: &str = "import { carrinho } from '../lib/state/carrinho.js'\n\
import { formatar } from '../utils/fmt.js'\n\
import { hora, modo } from 'relogio'\n\
import { extra } from 'relogio/sub'\n\n\
export const state = () => ({ aberto: false, itens: [], agora: hora(), modo, extra })\n\
export const actions = {\n  alternar(ctx) {\n    ctx.$.aberto = !ctx.$.aberto\n    ctx.log.info(formatar(carrinho.total))\n  },\n}\n";

const PACOTE_RELOGIO: [(&str, Option<&str>); 8] = [
    ("package.json", Some("{ \"name\": \"loja\", \"dependencies\": { \"relogio\": \"1.0.0\" } }")),
    ("node_modules/relogio/package.json", Some("{ \"name\": \"relogio\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./sub\": \"./lib/sub.js\" } }")),
    ("node_modules/relogio/index.js", Some("import { ajuda } from './interno.js'\nimport { somar } from 'calculo'\nexport const modo = process.env.NODE_ENV\nexport function hora() {\n  return ajuda() + somar(1, 2)\n}\n")),
    ("node_modules/relogio/interno.js", Some("export function ajuda() {\n  return 'interno'\n}\n")),
    ("node_modules/relogio/lib/sub.js", Some("export const extra = 'subcaminho'\n")),
    ("node_modules/relogio/node_modules/calculo/package.json", Some("{ \"name\": \"calculo\", \"type\": \"module\", \"main\": \"main.js\" }")),
    ("node_modules/relogio/node_modules/calculo/main.js", Some("export function somar(parcela, outra) {\n  return parcela + outra\n}\n")),
    ("routes/index.js", Some(PAGINA_COM_PACOTE)),
];

fn com_relogio(extra: &[(&'static str, Option<&'static str>)]) -> Vec<(&'static str, Option<&'static str>)> {
    PACOTE_RELOGIO.iter().copied().chain(extra.iter().copied()).collect()
}

fn asset_com_prefixo(client: &Path, prefixo: &str) -> String {
    let mut files = Vec::new();
    all_files(&client.join("assets"), &mut files);
    let achados: Vec<String> = files
        .iter()
        .map(|f| format!("/{}", f.strip_prefix(client).unwrap().to_string_lossy()))
        .filter(|u| u.starts_with(prefixo) && u.ends_with(".js") && u.len() == prefixo.len() + 11)
        .collect();
    assert_eq!(achados.len(), 1, "esperava um asset {prefixo}*: {achados:?}");
    achados[0].clone()
}

#[test]
fn npm_package_in_prod_build() {
    let t = app_with(&com_relogio(&[]));
    let root = t.0.join("app");
    assert!(check(&root).is_empty(), "{:?}", check(&root));
    let out = t.0.join("dist");
    build(&BuildOptions { root: root.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let client = out.join("client");
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    let npm: Vec<String> = ["/assets/npm/relogio/index-", "/assets/npm/relogio/interno-", "/assets/npm/relogio/lib/sub-", "/assets/npm/relogio/node_modules/calculo/main-"]
        .iter()
        .map(|p| asset_com_prefixo(&client, p))
        .collect();
    for url in &npm {
        assert!(index.contains(&format!("<link rel=\"modulepreload\" href=\"{url}\">")), "sem modulepreload de {url}:\n{index}");
    }
    let pagina = fs::read_to_string(client.join(&asset_com_prefixo(&client, "/assets/routes/index-")[1..])).unwrap();
    assert!(pagina.contains(&npm[0]) && pagina.contains(&npm[2]), "{pagina}");
    assert!(!pagina.contains("'relogio") && !pagina.contains("\"relogio"), "{pagina}");
    let relogio = fs::read_to_string(client.join(&npm[0][1..])).unwrap();
    assert!(relogio.contains(&npm[1]) && relogio.contains(&npm[3]), "{relogio}");
    assert!(relogio.contains("production") && !relogio.contains("process.env"), "{relogio}");
    assert!(!client.join("assets/node_modules").exists());
}

#[test]
fn npm_package_in_dev_build() {
    let t = app_with(&com_relogio(&[]));
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let relogio = fs::read_to_string(out.join("client/assets/npm/relogio/index.js")).unwrap();
    assert!(relogio.contains("\"development\"") && relogio.contains("/assets/npm/relogio/interno.js"), "{relogio}");
    let index = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(index.contains("href=\"/assets/npm/relogio/lib/sub.js\""), "{index}");
}

#[test]
fn npm_package_not_listed() {
    let d = check_with(&com_relogio(&[("package.json", Some("{ \"dependencies\": {} }"))]));
    expect(&d, "js/import", "routes/index.js", 3);
    let d = check_with(&com_relogio(&[("package.json", None)]));
    expect(&d, "js/import", "routes/index.js", 3);
}

#[test]
fn npm_package_not_installed() {
    let d = check_with(&[
        ("package.json", Some("{ \"dependencies\": { \"relogio\": \"1.0.0\" } }")),
        ("routes/index.js", Some(PAGINA_COM_PACOTE)),
    ]);
    expect(&d, "js/import", "routes/index.js", 3);
    expect(&d, "js/import", "routes/index.js", 4);
}

#[test]
fn npm_package_commonjs() {
    let d = check_with(&com_relogio(&[
        ("node_modules/relogio/interno.js", Some("module.exports = { ajuda: () => 'interno' }\n")),
    ]));
    expect(&d, "app/pacote", "node_modules/relogio/interno.js", 1);
}

#[test]
fn npm_package_not_pure_js() {
    for main in ["dados.json", "index.ts"] {
        let manifest = format!("{{ \"name\": \"relogio\", \"main\": \"{main}\" }}");
        let manifest: &'static str = Box::leak(manifest.into_boxed_str());
        let d = check_with(&com_relogio(&[
            ("node_modules/relogio/package.json", Some(manifest)),
            ("node_modules/relogio/dados.json", Some("{}")),
            ("node_modules/relogio/index.ts", Some("export const hora = (): number => 1\n")),
            ("routes/index.js", Some("import { hora } from 'relogio'\n\nexport const state = () => ({ aberto: false, itens: [], agora: hora() })\nexport const actions = { alternar(ctx) {} }\n")),
        ]));
        expect(&d, "app/pacote", "routes/index.js", 1);
    }
}

#[test]
fn npm_ui_framework_denied() {
    let d = check_with(&com_relogio(&[
        ("package.json", Some("{ \"dependencies\": { \"relogio\": \"1.0.0\", \"react\": \"18.0.0\" } }")),
        ("node_modules/react/package.json", Some("{ \"name\": \"react\", \"type\": \"module\", \"main\": \"index.js\" }")),
        ("node_modules/react/index.js", Some("export const createElement = () => null\n")),
        ("routes/index.js", Some("import { createElement } from 'react'\n\nexport const state = () => ({ aberto: false, itens: [], vazio: createElement() })\nexport const actions = { alternar(ctx) {} }\n")),
    ]));
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn npm_package_on_server_only_needs_listing() {
    let server = "import postgres from 'postgres'\nimport { buscar } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { produto: await buscar(params.id), cliente: typeof postgres }\n}\n";
    let t = app_with(&[
        ("package.json", Some("{ \"dependencies\": { \"postgres\": \"3.0.0\" } }")),
        ("routes/loja/[id].server.js", Some(server)),
    ]);
    let root = t.0.join("app");
    let d = check(&root);
    assert!(d.is_empty(), "{d:?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: root.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    assert_eq!(fs::read_to_string(out.join("server/package.json")).unwrap().trim(), "{ \"type\": \"module\" }");
    assert!(fs::read_to_string(out.join("server/routes/loja/[id].server.js")).unwrap().contains("from 'postgres'"));
    let d = check_with(&[("routes/loja/[id].server.js", Some(server))]);
    expect(&d, "js/import", "routes/loja/[id].server.js", 1);
}
