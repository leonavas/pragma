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

/// Copies the fixture app and applies changes: `Some(content)` writes, `None` deletes.
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
    assert!(found, "expected {rule} at {file}:{line}; got:\n{}", list.join("\n"));
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
        "client/shop/[id].html",
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
        let got = fs::read_to_string(out.join(f)).unwrap_or_else(|_| panic!("missing {f}"));
        let snap = snap_dir.join(f.replace('/', "__"));
        if update {
            fs::create_dir_all(&snap_dir).unwrap();
            fs::write(&snap, &got).unwrap();
        } else if fs::read_to_string(&snap).ok().as_deref() != Some(got.as_str()) {
            failed.push(f);
        }
    }
    assert!(failed.is_empty(), "snapshots differ (run with UPDATE_SNAPSHOTS=1 and review): {failed:?}");
    for f in ["client/assets/app.css", "client/robots.txt", "client/img/icon.svg", "server/routes/shop/[id].server.js", "server/utils/db.js", "server/core/server/render.js", "client/assets/core/lazy.js"] {
        assert!(out.join(f).is_file(), "missing {f}");
    }
    assert!(!out.join("templates/shop/[id].json").exists(), "template only for ssr routes");
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
    assert!(elapsed.as_millis() < 2000, "slow build: {elapsed:?}");
    assert_eq!(r.pages.len(), 4);
    assert!(r.css_gzip_bytes > 0);
    let client = out.join("client");
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    for u in urls(&index, "href").into_iter().chain(urls(&index, "src")) {
        if u.starts_with("/assets/") {
            assert!(client.join(&u[1..]).is_file(), "{u} does not exist");
            assert!(u.rsplit('-').next().unwrap().len() == 11 || u.ends_with(".css"), "{u} has no hash");
        }
    }
    assert!(!index.contains("/_pragma/dev.js") && !index.contains("__pragmaDev"));
    assert!(!index.contains("__pd"));
    let product = fs::read_to_string(client.join("shop/[id].html")).unwrap();
    assert!(product.contains("<script>self.__pd=fetch("));
    let blog = fs::read_to_string(client.join("blog/[slug].html")).unwrap();
    assert!(!blog.contains("__pd"), "ssr page has no __pd");
    let mut files = Vec::new();
    all_files(&client.join("assets"), &mut files);
    for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "js")) {
        let src = fs::read_to_string(f).unwrap();
        assert!(!src.contains("'./") && !src.contains("\"./") && !src.contains("../"), "unrewritten import in {f:?}: {src}");
        assert!(!src.contains("\n  "), "not minified: {f:?}");
        for spec in src.split("/assets/").skip(1) {
            let end = spec.find(['"', '\'', '`']).unwrap();
            assert!(client.join("assets").join(&spec[..end]).is_file(), "broken import {spec} in {f:?}");
        }
    }
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    assert!(worker.contains("service: \"test-shop\""));
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
    fs::write(root.join("utils/fmt.js"), "export function format(value) {\n  return `$${value}`\n}\n").unwrap();
    let b = t.0.join("b");
    build(&BuildOptions { root: root.clone(), out_dir: b.clone(), mode: Mode::Prod }).unwrap();
    let (na, nb) = (names(&a), names(&b));
    let changed: Vec<&String> = nb.iter().filter(|n| !na.contains(n)).collect();
    let prefix = |s: &str| changed.iter().any(|n| n.starts_with(s));
    assert!(prefix("fmt-") && prefix("index-"), "fmt and its importers change: {changed:?}");
    assert!(!prefix("boot-") && !prefix("cart-"), "the rest does not change: {changed:?}");
}

#[test]
fn dev_rebuild_removes_stale_files() {
    let t = app_with(&[]);
    let root = t.0.join("app");
    let out = t.0.join("dev");
    let opts = BuildOptions { root: root.clone(), out_dir: out.clone(), mode: Mode::Dev };
    build(&opts).unwrap();
    assert!(out.join("client/shop/index.html").is_file());
    fs::remove_file(root.join("routes/shop/index.html")).unwrap();
    build(&opts).unwrap();
    assert!(!out.join("client/shop/index.html").exists());
    assert!(out.join("client/shop/[id].html").is_file());
}

#[test]
fn diagnostics_are_sorted() {
    let d = check_with(&[
        ("routes/shop/index.html", Some("<p style=\"x\">a</p>\n<p onclick=\"x\">b</p>")),
        (PAGE, Some("<div data-foo=\"x\"></div>\n<p class=\"none-of-this\"></p>")),
    ]);
    let keys: Vec<_> = d.iter().map(|x| (x.file.clone(), x.line, x.column)).collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert_eq!(d.len(), 4, "{d:#?}");
}

// One test per diagnostic.

#[test]
fn missing_layout() {
    expect(&check_with(&[("routes/_layout.html", None)]), "app/structure", "routes/_layout.html", 1);
}

#[test]
fn route_file_and_index_conflict() {
    expect(&check_with(&[("routes/shop.html", Some("<p>x</p>"))]), "app/route-conflict", "routes/shop.html", 1);
}

#[test]
fn param_routes_conflict() {
    let d = check_with(&[("routes/shop/[slug].html", Some("<p>x</p>"))]);
    assert!(d.iter().any(|x| x.rule == "app/route-conflict"), "{d:#?}");
}

#[test]
fn orphan_module() {
    expect(&check_with(&[("routes/about.js", Some("export const actions = {}\n"))]), "app/structure", "routes/about.js", 1);
}

#[test]
fn unknown_pragma_json_key() {
    expect(&check_with(&[("pragma.json", Some("{\n  \"name\": \"x\",\n  \"theme\": 1\n}\n"))]), "app/pragma-json", "pragma.json", 3);
}

#[test]
fn unclosed_tag() {
    expect(&page("<section>\n  <div>\n</section>"), "html/syntax", PAGE, 2);
}

#[test]
fn unknown_directive() {
    expect(&page("<p>a</p>\n<p data-label=\"data.x\">b</p>"), "html/data-attribute", PAGE, 2);
}

#[track_caller]
fn message(d: &[Diagnostic], rule: &str) -> String {
    let list: Vec<String> = d.iter().map(|x| x.to_string()).collect();
    d.iter().find(|x| x.rule == rule).map(|x| x.message.clone()).unwrap_or_else(|| panic!("expected {rule}; got:\n{}", list.join("\n")))
}

#[test]
fn reserved_data_attribute_names_short_form() {
    let cases = [
        ("<p data-text=\"$heading\"></p>", "`data-text` is internal; write `:text=\"...\"`"),
        ("<button data-on-click=\"save\">s</button>", "write `@click=\"...\"`"),
        ("<p data-class-active=\"$on\"></p>", "write `.active=\"...\"`"),
        ("<a data-attr-href=\"$url\">a</a>", "write `:href=\"...\"`"),
        ("<div data-if=\"$open\"></div>", "write `<if path>`"),
        ("<div data-c=\"card\"></div>", "write the component tag `<c-name>`"),
        ("<p data-slot=\"1\">x</p>", "put `slot=\"name\"`"),
    ];
    for (src, want) in cases {
        let d = page(src);
        expect(&d, "html/data-attribute", PAGE, 1);
        let m = message(&d, "html/data-attribute");
        assert!(m.contains(want), "{src}: {m}");
    }
}

#[test]
fn arbitrary_data_attribute_suggests_alternatives() {
    let d = page("<p data-testid=\"hero\">x</p>");
    expect(&d, "html/data-attribute", PAGE, 1);
    let m = message(&d, "html/data-attribute");
    for want in ["`data-testid`", ":data-testid=\"$path\"", ".class=\"$flag\"", "aria-*", "role, label or id"] {
        assert!(m.contains(want), "{m}");
    }
}

#[test]
fn bound_data_attribute_passes() {
    let src = fs::read_to_string(fixture().join("routes/blog/[slug].html")).unwrap();
    let src = src.replace("class=\"tag\"", ":data-hl=\"t.kind\" class=\"tag\"");
    assert!(src.contains(":data-hl"));
    let d = check_with(&[("routes/blog/[slug].html", Some(&src)), (PAGE, Some("<p :data-state=\"$heading\">x</p>"))]);
    assert!(!d.iter().any(|x| x.rule == "html/data-attribute" || x.rule == "html/unknown-directive"), "{:#?}", d);
}

#[test]
fn invalid_path() {
    expect(&page("<p :text=\"$items[0]\"></p>"), "html/invalid-path", PAGE, 1);
}

#[test]
fn negation_only_where_allowed() {
    expect(&page("<p :text=\"!$open\"></p>"), "html/invalid-path", PAGE, 1);
}

#[test]
fn unknown_root() {
    expect(&page("<p :text=\"user.name\"></p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn for_variable_out_of_scope() {
    let src = "<ul><for x in $items key=x.id><li :text=\"x.name\"></li></for></ul>\n<p :text=\"x.name\"></p>";
    expect(&page(src), "html/invalid-root", PAGE, 2);
}

#[test]
fn data_without_loader() {
    expect(&page("<p :text=\"data.x\"></p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn state_without_module() {
    let d = check_with(&[("routes/shop/index.html", Some("<p :text=\"$x\"></p>"))]);
    expect(&d, "html/invalid-root", "routes/shop/index.html", 1);
}

#[test]
fn unknown_store() {
    expect(&page("<p :text=\"shared.bag.total\"></p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn unknown_param() {
    let d = check_with(&[("routes/shop/[id].html", Some("<p :text=\"params.slug\"></p>"))]);
    expect(&d, "html/invalid-root", "routes/shop/[id].html", 1);
}

#[test]
fn props_outside_component() {
    expect(&page("<p data-text=\"state.heading\"></p>"), "html/data-attribute", PAGE, 1);
}

#[test]
fn alias_missing_in_component() {
    let d = check_with(&[("routes/shop/[id].html", Some("<p-price></p-price>"))]);
    expect(&d, "html/invalid-root", "routes/shop/_components/price/price.html", 1);
}

#[test]
fn for_without_key() {
    expect(&page("<ul>\n<for x in $items><li></li></for></ul>"), "html/for-without-key", PAGE, 2);
}

#[test]
fn key_must_use_for_variable() {
    expect(&page("<ul><for x in $items key=y.id><li></li></for></ul>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn old_block_syntax() {
    expect(&page("<div :if=\"$open\"></div>"), "html/legacy-syntax", PAGE, 1);
    expect(&page("<template :if=\"$open\"><p>a</p></template>"), "html/legacy-syntax", PAGE, 1);
    expect(&page("<p>a</p>\n<template><p>b</p></template>"), "html/outside-template", PAGE, 2);
}

#[test]
fn malformed_block_tags() {
    expect(&page("<if test=\"$open\"><p>a</p></if>"), "html/block", PAGE, 1);
    expect(&page("<ul><for x of $items key=x.id><li></li></for></ul>"), "html/block", PAGE, 1);
    expect(&page("<if $open><p>a</p></if>\n<else x><p>b</p></else>"), "html/block", PAGE, 2);
}

#[test]
fn else_if_chain() {
    let ok = "<if $open><p>a</p></if>\n<else-if !$open><p>b</p></else-if>\n<else-if $open><p>c</p></else-if>\n<else><p>d</p></else>";
    assert!(page(ok).is_empty(), "{:?}", page(ok));
    expect(&page("<p>a</p>\n<else-if $open><p>b</p></else-if>"), "html/misplaced-else", PAGE, 2);
}

#[test]
fn else_out_of_place() {
    expect(&page("<p>a</p>\n<else><p>b</p></else>"), "html/misplaced-else", PAGE, 2);
}

#[test]
fn empty_out_of_place() {
    let src = "<if $open><p>a</p></if>\n<empty><p>b</p></empty>";
    expect(&page(src), "html/misplaced-empty", PAGE, 2);
}

#[test]
fn block_needs_single_root() {
    expect(&page("<if $open>\n  <p>a</p>\n  <p>b</p>\n</if>"), "html/block-single-root", PAGE, 1);
}

#[test]
fn unknown_page_action() {
    expect(&page("<button @click=\"vanish\">x</button>"), "html/unknown-action", PAGE, 1);
}

const BOX: [(&str, Option<&str>); 2] = [
    ("lib/components/box/box.html", Some("<section><button @click=\"dismiss\">x</button><slot></slot></section>")),
    ("lib/components/box/box.js", Some("export const actions = { dismiss() {} }")),
];

#[test]
fn slot_content_belongs_to_writer() {
    let src = "<c-box><button @click=\"dismiss\">x</button></c-box>";
    expect(&check_with(&[BOX[0], BOX[1], (PAGE, Some(src))]), "html/unknown-action", PAGE, 1);
    let ok = "<c-card heading=\"x\"><c-box><button @click=\"toggle\" :text=\"$open\">x</button></c-box></c-card>";
    let d = check_with(&[BOX[0], BOX[1], (PAGE, Some(ok))]);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn slot_marks_lexical_owner_and_module_props() {
    let src = "<c-card heading=\"x\"><p>fixed</p><div><c-box step=\"2\" class=\"p-4\"><b :text=\"$open\"></b></c-box></div></c-card>";
    let t = app_with(&[BOX[0], BOX[1], (PAGE, Some(src))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(html.contains("<p>fixed</p>"), "{html}");
    assert!(html.contains("<div data-slot=\"1\"><section data-c=\"box\" data-step=\"2\" class=\"p-4\">"), "{html}");
    assert!(html.contains("<b data-text=\"state.open\" data-slot=\"1\">"), "{html}");
    assert!(html.contains("<div data-c=\"card\" class=\"card p-4\" data-heading=\"x\">"), "{html}");
}

#[test]
fn data_slot_is_generated() {
    expect(&page("<p data-slot=\"1\">x</p>"), "html/data-attribute", PAGE, 1);
}

#[test]
fn layout_action_without_owner() {
    let layout = "<button @click=\"expand\">x</button>\n<main data-page><slot></slot></main>";
    expect(&check_with(&[("routes/_layout.html", Some(layout))]), "html/unknown-action", "routes/_layout.html", 1);
}

#[test]
fn unknown_component() {
    expect(&page("<section>\n  <c-table></c-table>\n</section>"), "html/unknown-component", PAGE, 2);
}

#[test]
fn route_component_not_visible_elsewhere() {
    expect(&page("<p-price></p-price>"), "html/unknown-component", PAGE, 1);
}

#[test]
fn unknown_slot() {
    expect(&page("<c-card heading=\"x\">\n  <p slot=\"footer\">x</p>\n</c-card>"), "html/component", PAGE, 2);
}

#[test]
fn component_single_root() {
    let d = check_with(&[("lib/components/badge/badge.html", Some("<span></span>\n<span></span>"))]);
    expect(&d, "html/component", "lib/components/badge/badge.html", 2);
}

#[test]
fn component_cycle() {
    let d = check_with(&[("lib/components/badge/badge.html", Some("<span><c-badge></c-badge></span>")), (PAGE, Some("<c-badge></c-badge>"))]);
    expect(&d, "html/component", "lib/components/badge/badge.html", 1);
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
    expect(&page("<p class=\"p-4 pretty\">x</p>"), "css/unknown-class", PAGE, 1);
}

#[test]
fn arbitrary_value() {
    expect(&page("<p class=\"w-[37px]\">x</p>"), "css/arbitrary-value", PAGE, 1);
}

#[test]
fn budget_exceeded() {
    let cfg = "{\n  \"name\": \"x\",\n  \"budgets\": {\n    \"initialJsKb\": 0.1\n  }\n}\n";
    expect(&check_with(&[("pragma.json", Some(cfg))]), "app/budget", "pragma.json", 4);
}

#[test]
fn js_lint_runs_on_app_code() {
    let d = check_with(&[("utils/fmt.js", Some("export function format(n) {\n  console.log(n)\n  return n\n}\n"))]);
    expect(&d, "js/console", "utils/fmt.js", 2);
}

#[test]
fn missing_import() {
    let d = check_with(&[("routes/index.js", Some("import { x } from './missing.js'\nexport const actions = { toggle() { return x } }\n"))]);
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn client_cannot_import_server_module() {
    let src = "import { load } from './shop/[id].server.js'\nexport const actions = { toggle() { return load } }\n";
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
        ("routes/about.html", Some("<dialog id=\"d\"><p>x</p></dialog>")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let index = fs::read_to_string(out.join("client/index.html")).unwrap();
    let about = fs::read_to_string(out.join("client/about.html")).unwrap();
    assert!(out.join("client/assets/core/lazy.js").is_file());
    assert!(!index.contains("lazy.js"), "dynamic import() is not modulepreloaded");
    assert!(!index.contains("overlay.js"), "page without overlay does not preload overlay.js");
    assert!(about.contains("<link rel=\"modulepreload\" href=\"/assets/core/overlay.js\">"), "{about}");
}

#[test]
fn theme_script_only_when_used() {
    let t = app_with(&[
        ("core/theme.js", Some("export const setTheme = () => {}")),
        ("routes/index.js", Some("import { setTheme } from '../core/theme.js'\nexport const state = () => ({ open: false, items: [] })\nexport const actions = { toggle() { setTheme('dark') } }\n")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/shop/[id].html")).unwrap();
    let theme = html.find("localStorage.getItem('pragma:theme')").expect("theme script on every page");
    assert!(theme < html.find("rel=\"stylesheet\"").unwrap(), "before the CSS");
    let d = check(&fixture());
    assert!(d.is_empty());
    let plain = app_with(&[]);
    build(&BuildOptions { root: plain.0.join("app"), out_dir: plain.0.join("out"), mode: Mode::Dev }).unwrap();
    assert!(!fs::read_to_string(plain.0.join("out/client/index.html")).unwrap().contains("pragma:theme"));
}

#[test]
fn aria_current_on_links_to_the_page() {
    let t = app_with(&[(PAGE, Some("<a href=\"/\">here</a> <a href=\"/?x=1\">q</a> <a href=\"/shop/1\">l</a> <a href=\"/\" target=\"_blank\">n</a>"))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(html.contains("<a href=\"/\" aria-current=\"page\">here</a> <a href=\"/?x=1\">q</a> <a href=\"/shop/1\">l</a> <a href=\"/\" target=\"_blank\">n</a>"), "{html}");
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
    warns("<p>x</p>\n<button><svg aria-hidden=\"true\"></svg></button>", "a11y/accessible-name", 2);
    warns("<a href=\"/\"></a>", "a11y/accessible-name", 1);
    let ok = "<button :text=\"$open\"></button><button aria-label=\"Close\">×</button><a href=\"/\"><span class=\"sr-only\">Home</span></a>";
    assert!(page(ok).is_empty(), "{:?}", page(ok));
}

#[test]
fn a11y_label() {
    warns("<input name=\"q\">", "a11y/label", 1);
    warns("<select><option>a</option></select>", "a11y/label", 1);
    let ok = "<label>Name <input></label><label for=\"e\">E</label><input id=\"e\"><textarea aria-label=\"T\"></textarea><input type=\"hidden\">";
    assert!(page(ok).is_empty(), "{:?}", page(ok));
}

#[test]
fn a11y_dialog_name() {
    warns("<dialog><p>x</p></dialog>", "a11y/dialog-name", 1);
    assert!(page("<dialog aria-label=\"About\"><p>x</p></dialog>").is_empty());
}

#[test]
fn a11y_click_on_non_interactive() {
    warns("<div @click=\"toggle\">x</div>", "a11y/click-non-interactive", 1);
    assert!(page("<div role=\"button\" tabindex=\"0\" @click=\"toggle\">x</div>").is_empty());
}

#[test]
fn a11y_headings() {
    warns("<h1>a</h1>\n<h3>b</h3>", "a11y/heading", 2);
    warns("<h1>a</h1>\n<h2>b</h2>\n<h1>c</h1>", "a11y/single-h1", 3);
    assert!(page("<h1>a</h1><h2>b</h2><h3>c</h3><h2>d</h2>").is_empty());
    let branches = "<if $open><h1>a</h1></if>\n<else-if $open><h1>b</h1></else-if>\n<else><h1>c</h1></else>";
    assert!(page(branches).is_empty(), "{:?}", page(branches));
    warns(&format!("{branches}\n<h1>c</h1>"), "a11y/single-h1", 4);
}

#[test]
fn a11y_tabindex() {
    warns("<p tabindex=\"2\">x</p>", "a11y/tabindex", 1);
}

#[test]
fn a11y_duplicate_id() {
    warns("<p id=\"x\">a</p>\n<p id=\"x\">b</p>", "a11y/duplicate-id", 2);
}

#[test]
fn a11y_iframe_title() {
    warns("<iframe src=\"/x\"></iframe>", "a11y/iframe-title", 1);
}

#[test]
fn a11y_link_without_href() {
    warns("<a>x</a>", "a11y/a-no-href", 1);
}

#[test]
fn a11y_points_to_component_source() {
    let d = check_with(&[("lib/components/photo/photo.html", Some("<img src=\"/f.png\">")), (PAGE, Some("<c-photo></c-photo>\n<c-photo></c-photo>"))]);
    expect(&d, "a11y/img-alt", "lib/components/photo/photo.html", 1);
    assert_eq!(d.len(), 1, "{d:?}");
}

#[test]
fn alias_in_component_model() {
    let search = ("lib/components/search/search.html", Some("<input aria-label=\"Search\" :model=\"$value\">"));
    let ok = check_with(&[search, (PAGE, Some("<c-search :value=\"$open\"></c-search>"))]);
    assert!(ok.is_empty(), "{ok:?}");
    let d = check_with(&[search, (PAGE, Some("<ul><for x in $items key=x.id><li><c-search :value=\"x.name\"></c-search></li></for></ul>"))]);
    expect(&d, "html/invalid-root", "lib/components/search/search.html", 1);
}

#[test]
fn state_alias_into_module_component() {
    expect(&page("<c-counter :n=\"$open\"></c-counter>"), "html/component", PAGE, 1);
}

const PAGE_WITH_PACKAGE: &str = "import { cart } from '../lib/state/cart.js'\n\
import { format } from '../utils/fmt.js'\n\
import { time, mode } from 'clock'\n\
import { extra } from 'clock/sub'\n\n\
export const state = () => ({ open: false, items: [], now: time(), mode, extra })\n\
export const actions = {\n  toggle(ctx) {\n    ctx.$.open = !ctx.$.open\n    ctx.log.info(format(cart.total))\n  },\n}\n";

const CLOCK_PACKAGE: [(&str, Option<&str>); 8] = [
    ("package.json", Some("{ \"name\": \"shop\", \"dependencies\": { \"clock\": \"1.0.0\" } }")),
    ("node_modules/clock/package.json", Some("{ \"name\": \"clock\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./sub\": \"./lib/sub.js\" } }")),
    ("node_modules/clock/index.js", Some("import { assist } from './internal.js'\nimport { add } from 'calc'\nexport const mode = process.env.NODE_ENV\nexport function time() {\n  return assist() + add(1, 2)\n}\n")),
    ("node_modules/clock/internal.js", Some("export function assist() {\n  return 'internal'\n}\n")),
    ("node_modules/clock/lib/sub.js", Some("export const extra = 'subpath'\n")),
    ("node_modules/clock/node_modules/calc/package.json", Some("{ \"name\": \"calc\", \"type\": \"module\", \"main\": \"main.js\" }")),
    ("node_modules/clock/node_modules/calc/main.js", Some("export function add(left, right) {\n  return left + right\n}\n")),
    ("routes/index.js", Some(PAGE_WITH_PACKAGE)),
];

fn with_clock(extra: &[(&'static str, Option<&'static str>)]) -> Vec<(&'static str, Option<&'static str>)> {
    CLOCK_PACKAGE.iter().copied().chain(extra.iter().copied()).collect()
}

fn asset_with_prefix(client: &Path, prefix: &str) -> String {
    let mut files = Vec::new();
    all_files(&client.join("assets"), &mut files);
    let found: Vec<String> = files
        .iter()
        .map(|f| format!("/{}", f.strip_prefix(client).unwrap().to_string_lossy()))
        .filter(|u| u.starts_with(prefix) && u.ends_with(".js") && u.len() == prefix.len() + 11)
        .collect();
    assert_eq!(found.len(), 1, "expected one asset {prefix}*: {found:?}");
    found[0].clone()
}

fn npm_files(client: &Path) -> Vec<String> {
    let mut files = Vec::new();
    all_files(&client.join("assets/npm"), &mut files);
    let mut urls: Vec<String> = files.iter().map(|f| format!("/{}", f.strip_prefix(client).unwrap().to_string_lossy())).collect();
    urls.sort();
    urls
}

fn build_prod(t: &Tmp) -> PathBuf {
    let out = t.0.join("dist");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    out.join("client")
}

#[test]
fn npm_package_in_prod_build() {
    let t = app_with(&with_clock(&[]));
    let root = t.0.join("app");
    assert!(check(&root).is_empty(), "{:?}", check(&root));
    let client = build_prod(&t);
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    let url = asset_with_prefix(&client, "/assets/npm/clock-");
    assert_eq!(npm_files(&client), vec![url.clone()]);
    assert!(index.contains(&format!("<link rel=\"modulepreload\" href=\"{url}\">")), "no modulepreload for {url}:\n{index}");
    let page_js = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/routes/index-")[1..])).unwrap();
    assert!(page_js.contains(&url), "{page_js}");
    assert!(!page_js.contains("'clock") && !page_js.contains("\"clock"), "{page_js}");
    let clock = fs::read_to_string(client.join(&url[1..])).unwrap();
    assert!(clock.contains("internal") && clock.contains("subpath"), "{clock}");
    assert!(clock.contains("production") && !clock.contains("process.env"), "{clock}");
    assert!(!clock.contains("import"), "{clock}");
    assert!(!client.join("assets/node_modules").exists());
}

#[test]
fn npm_package_tree_shaking() {
    let t = app_with(&with_clock(&[(
        "node_modules/clock/lib/sub.js",
        Some("export const extra = 'subpath'\nexport function unused() {\n  return 'PACKAGE_LEFTOVER'\n}\n"),
    )]));
    let client = build_prod(&t);
    let clock = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/npm/clock-")[1..])).unwrap();
    assert!(clock.contains("subpath") && !clock.contains("PACKAGE_LEFTOVER"), "{clock}");
}

#[test]
fn npm_package_in_dev_build() {
    let t = app_with(&with_clock(&[]));
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let clock = fs::read_to_string(out.join("client/assets/npm/clock.js")).unwrap();
    assert!(clock.contains("\"development\"") && clock.contains("internal"), "{clock}");
    let index = fs::read_to_string(out.join("client/index.html")).unwrap();
    assert!(index.contains("href=\"/assets/npm/clock.js\""), "{index}");
    let page_js = fs::read_to_string(out.join("client/assets/routes/index.js")).unwrap();
    assert!(page_js.contains("'/assets/npm/clock.js'") && !page_js.contains("clock/sub"), "{page_js}");
}

#[test]
fn npm_package_cache() {
    let t = app_with(&with_clock(&[]));
    let client = build_prod(&t);
    let cached = t.0.join("dist/.cache/npm/out/clock.js");
    let code = fs::read_to_string(&cached).unwrap();
    fs::write(&cached, format!("{code}\nexport const cacheMark = 1\n")).unwrap();
    build_prod(&t);
    let clock = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/npm/clock-")[1..])).unwrap();
    assert!(clock.contains("cacheMark"), "{clock}");
    fs::write(t.0.join("app/node_modules/clock/package.json"), "{ \"name\": \"clock\", \"version\": \"1.0.1\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./sub\": \"./lib/sub.js\" } }").unwrap();
    build_prod(&t);
    let clock = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/npm/clock-")[1..])).unwrap();
    assert!(!clock.contains("cacheMark"), "{clock}");
}

#[test]
fn npm_package_commonjs() {
    let t = app_with(&with_clock(&[
        ("node_modules/clock/internal.js", Some("module.exports = { assist: () => 'internal-cjs' }\n")),
        ("node_modules/clock/lib/sub.js", Some("exports.extra = require('../internal.js').assist()\n")),
    ]));
    let root = t.0.join("app");
    assert!(check(&root).is_empty(), "{:?}", check(&root));
    let client = build_prod(&t);
    let clock = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/npm/clock-")[1..])).unwrap();
    assert!(clock.contains("internal-cjs") && !clock.contains("require("), "{clock}");
}

#[test]
fn npm_package_conflicting_names() {
    let t = app_with(&with_clock(&[
        ("node_modules/clock/lib/sub.js", Some("export const extra = 'subpath'\nexport const mode = 'other'\n")),
        ("routes/index.js", Some("import { mode } from 'clock'\nimport { mode as other } from 'clock/sub'\n\nexport const state = () => ({ open: false, items: [], mode, other })\nexport const actions = { toggle(ctx) {} }\n")),
    ]));
    let client = build_prod(&t);
    let url = asset_with_prefix(&client, "/assets/npm/clock-");
    assert_eq!(npm_files(&client), vec![url.clone()]);
    let clock = fs::read_to_string(client.join(&url[1..])).unwrap();
    assert!(clock.contains("other") && !clock.contains("development"), "{clock}");
    let page_js = fs::read_to_string(client.join(&asset_with_prefix(&client, "/assets/routes/index-")[1..])).unwrap();
    assert!(page_js.contains("$1$mode") && !page_js.contains("clock/sub"), "{page_js}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let page_js = fs::read_to_string(out.join("client/assets/routes/index.js")).unwrap();
    assert!(page_js.contains("import { $1$mode as other } from '/assets/npm/clock.js';\n"), "{page_js}");
    assert_eq!(page_js.lines().count(), 5, "{page_js}");
}

#[test]
fn npm_package_not_listed() {
    let d = check_with(&with_clock(&[("package.json", Some("{ \"dependencies\": {} }"))]));
    expect(&d, "js/import", "routes/index.js", 3);
    let d = check_with(&with_clock(&[("package.json", None)]));
    expect(&d, "js/import", "routes/index.js", 3);
}

#[test]
fn npm_package_not_installed() {
    let d = check_with(&[
        ("package.json", Some("{ \"dependencies\": { \"clock\": \"1.0.0\" } }")),
        ("routes/index.js", Some(PAGE_WITH_PACKAGE)),
    ]);
    expect(&d, "js/import", "routes/index.js", 3);
    expect(&d, "js/import", "routes/index.js", 4);
}

#[test]
fn npm_package_missing_export() {
    let d = check_with(&with_clock(&[(
        "routes/index.js",
        Some("import { missing } from 'clock'\n\nexport const state = () => ({ open: false, items: [], x: missing })\nexport const actions = { toggle(ctx) {} }\n"),
    )]));
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn npm_package_node_builtin() {
    let d = check_with(&with_clock(&[("node_modules/clock/internal.js", Some("import { readFileSync } from 'fs'\nexport function assist() {\n  return readFileSync('x')\n}\n"))]));
    expect(&d, "app/package", "routes/index.js", 3);
}

#[test]
fn npm_package_not_pure_js() {
    for main in ["data.json", "index.ts"] {
        let manifest = format!("{{ \"name\": \"clock\", \"main\": \"{main}\" }}");
        let manifest: &'static str = Box::leak(manifest.into_boxed_str());
        let d = check_with(&with_clock(&[
            ("node_modules/clock/package.json", Some(manifest)),
            ("node_modules/clock/data.json", Some("{}")),
            ("node_modules/clock/index.ts", Some("export const time = (): number => 1\n")),
            ("routes/index.js", Some("import { time } from 'clock'\n\nexport const state = () => ({ open: false, items: [], now: time() })\nexport const actions = { toggle(ctx) {} }\n")),
        ]));
        expect(&d, "app/package", "routes/index.js", 1);
    }
    for (file, src) in [("node_modules/clock/style.css", ".a { color: red }\n"), ("node_modules/clock/typed.ts", "export const t = (): number => 1\n")] {
        let import = format!("import './{}'\n", file.rsplit('/').next().unwrap());
        let internal: &'static str = Box::leak(format!("{import}export function assist() {{\n  return 'internal'\n}}\n").into_boxed_str());
        let d = check_with(&with_clock(&[(file, Some(src)), ("node_modules/clock/internal.js", Some(internal))]));
        expect(&d, "app/package", "routes/index.js", 3);
    }
}

#[test]
fn npm_ui_framework_denied() {
    let d = check_with(&with_clock(&[
        ("package.json", Some("{ \"dependencies\": { \"clock\": \"1.0.0\", \"react\": \"18.0.0\" } }")),
        ("node_modules/react/package.json", Some("{ \"name\": \"react\", \"type\": \"module\", \"main\": \"index.js\" }")),
        ("node_modules/react/index.js", Some("export const createElement = () => null\n")),
        ("routes/index.js", Some("import { createElement } from 'react'\n\nexport const state = () => ({ open: false, items: [], blank: createElement() })\nexport const actions = { toggle(ctx) {} }\n")),
    ]));
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn npm_package_on_server_only_needs_listing() {
    let server = "import postgres from 'postgres'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), client: typeof postgres }\n}\n";
    let t = app_with(&[
        ("package.json", Some("{ \"dependencies\": { \"postgres\": \"3.0.0\" } }")),
        ("routes/shop/[id].server.js", Some(server)),
    ]);
    let root = t.0.join("app");
    let d = check(&root);
    assert!(d.is_empty(), "{d:?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: root.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    assert_eq!(fs::read_to_string(out.join("server/package.json")).unwrap().trim(), "{ \"type\": \"module\" }");
    assert!(fs::read_to_string(out.join("server/routes/shop/[id].server.js")).unwrap().contains("from 'postgres'"));
    let d = check_with(&[("routes/shop/[id].server.js", Some(server))]);
    expect(&d, "js/import", "routes/shop/[id].server.js", 1);
}
