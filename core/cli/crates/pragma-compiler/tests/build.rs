use pragma_compiler::{BuildOptions, Mode, build, check};
use pragma_diag::{Diagnostic, Severity};
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

/// A built page and the bodies of its template module (`compile::hoist`), as one text.
fn page_html(out: &Path, page: &str) -> String {
    let mut html = fs::read_to_string(out.join("client").join(page)).unwrap();
    let Some(url) = html.split("\"tpl\":\"").nth(1).and_then(|r| r.split('"').next()).map(str::to_string) else { return html };
    let module = fs::read_to_string(out.join("client").join(url.trim_start_matches('/'))).unwrap();
    let mut chars = module.chars();
    let mut value = false;
    while let Some(c) = chars.next() {
        match c {
            ':' => value = true,
            '\'' => {
                let mut text = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\'' => break,
                        '\\' => text.push(match chars.next() {
                            Some('n') => '\n',
                            Some('r') => '\r',
                            Some(c) => c,
                            None => break,
                        }),
                        c => text.push(c),
                    }
                }
                if value {
                    html.push('\n');
                    html.push_str(&text);
                }
                value = false;
            }
            ',' | '{' => value = false,
            _ => {}
        }
    }
    html
}

/// `html` with the tid of every block anchor written as `*` (`<!--p:if * state.on-->`).
fn anchors(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find("<!--p:") {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        let kind_end = tail.find(' ').unwrap_or(tail.len());
        out.push_str(&tail[..kind_end]);
        let after = &tail[kind_end..];
        let id_end = after[1..].find([' ', '-']).map_or(after.len(), |k| k + 1);
        out.push_str(" *");
        rest = &after[id_end..];
    }
    out.push_str(rest);
    out
}

/// The SSR ops of a route, as embedded in `server/worker.js`.
fn ssr_ops(out: &Path, pattern: &str) -> String {
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    let line = worker.lines().find(|l| l.contains(&format!("pattern: \"{pattern}\","))).unwrap_or_else(|| panic!("no route {pattern}: {worker}"));
    line.split("template: ").nth(1).and_then(|r| r.split(", shell: ").next()).unwrap().to_string()
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
        "client/assets/tpl/index.js",
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
    assert!(fs::read_to_string(out.join("server/worker.js")).unwrap().contains("pattern: \"/shop/:id\", server: s1, ssr: false, template: null"), "ops only for ssr routes");
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
        assert!(!src.contains("\n  "), "not minified: {f:?}");
        for spec in src.split("/assets/").skip(1) {
            let end = spec.find(['"', '\'', '`']).unwrap();
            assert!(client.join("assets").join(&spec[..end]).is_file(), "broken import {spec} in {f:?}");
        }
        for quote in ['"', '\'', '`'] {
            for spec in src.split(&format!("{quote}.")).skip(1) {
                let spec = format!(".{}", &spec[..spec.find(quote).unwrap()]);
                assert!(spec.ends_with(".js") && f.parent().unwrap().join(&spec).is_file(), "broken chunk import {spec} in {f:?}");
            }
        }
    }
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    assert!(worker.contains("service: \"test-shop\""));
    assert!(worker.contains("ssr: true, template: [[\"html\""));
}

#[test]
fn prod_bundles_one_chunk_per_route() {
    let t = tmp();
    let out = t.0.join("dist");
    build(&BuildOptions { root: fixture(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let client = out.join("client");
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    let preloads: Vec<String> = urls(&index, "href").into_iter().filter(|u| u.ends_with(".js")).collect();
    let route = preloads.iter().find(|u| u.starts_with("/assets/routes/index-")).expect("route chunk");
    assert!(index.contains(&format!("\"page\":\"{route}#p\"")) && index.contains(&format!("\"cart\":\"{route}#s0\"")), "{index}");
    assert!(preloads.iter().any(|u| u.starts_with("/assets/chunks/shared")), "the cart store, used by every route, is a shared chunk: {preloads:?}");
    assert!(!preloads.iter().any(|u| u.contains("lazy")), "import() stays a separate chunk, not preloaded: {preloads:?}");
    let mut files = Vec::new();
    all_files(&client.join("assets/routes"), &mut files);
    assert_eq!(files.iter().filter(|f| f.extension().is_some_and(|e| e == "js")).count(), 5, "one chunk per route with JS: {files:?}");
    let mut assets = Vec::new();
    all_files(&client.join("assets"), &mut assets);
    assert!(assets.iter().any(|f| f.to_string_lossy().contains("assets/css/shop/_id_-")), "{assets:?}");
    assert!(!assets.iter().any(|f| f.strip_prefix(&client).unwrap().to_string_lossy().contains(['[', ']'])), "asset names are URL-safe: {assets:?}");
    let product = fs::read_to_string(client.join("shop/[id].html")).unwrap();
    assert!(!urls(&product, "href").iter().chain(&urls(&product, "src")).any(|u| u.contains(['[', ']'])), "{product}");
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
    assert!(prefix("index-"), "the route chunk holding fmt changes: {changed:?}");
    assert!(!prefix("boot-") && !prefix("shared") && !prefix("_id_-"), "the rest does not change: {changed:?}");
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

#[test]
fn same_named_route_components_are_scoped_by_folder() {
    let t = app_with(&[
        ("routes/one/index.html", Some("<title>One</title>\n<p-badge></p-badge>")),
        ("routes/one/_components/badge/badge.html", Some("<span><span class=\"badge-icon\"></span></span>")),
        ("routes/one/_components/badge/badge.css", Some("[data-c=\"badge\"] .badge-icon { width: 1px; }\n")),
        ("routes/two/index.html", Some("<title>Two</title>\n<p-badge></p-badge>")),
        ("routes/two/_components/badge/badge.html", Some("<button class=\"badge-icon\" @click=\"ping\">b</button>")),
        ("routes/two/_components/badge/badge.css", Some("[data-c=badge] .badge-icon { width: 2px; }\n")),
        ("routes/two/_components/badge/badge.js", Some("function ping() {}\n")),
    ]);
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap_or_else(|d| panic!("{d:#?}"));
    let page = |p: &str| fs::read_to_string(out.join("client").join(p).join("index.html")).unwrap();
    let ids = |html: &str| -> Vec<String> { html.split("data-c=\"").skip(1).map(|s| s[..s.find('"').unwrap()].to_string()).collect() };
    let (one, two) = (ids(&page("one")), ids(&page("two")));
    // lib/components stay global: the layout's <c-counter> keeps data-c="counter"
    assert!(one.contains(&"counter".to_string()) && two.contains(&"counter".to_string()), "{one:?} {two:?}");
    let badge = |ids: &[String]| ids.iter().find(|id| id.starts_with("badge-")).cloned().unwrap_or_else(|| panic!("{ids:?}"));
    let (one_badge, two_badge) = (badge(&one), badge(&two));
    assert_ne!(one_badge, two_badge, "same-named components in different folders share a data-c");
    let one_css = fs::read_to_string(out.join("client/assets/css/one/index.css")).unwrap();
    let two_css = fs::read_to_string(out.join("client/assets/css/two/index.css")).unwrap();
    assert!(one_css.contains(&format!("[data-c=\"{one_badge}\"] .badge-icon {{ width: 1px; }}")), "{one_css}");
    assert!(!one_css.contains(&two_badge), "{one_css}");
    assert!(two_css.contains(&format!("[data-c=\"{two_badge}\"] .badge-icon {{ width: 2px; }}")), "{two_css}");
    assert!(!two_css.contains(&one_badge), "{two_css}");
    assert!(!one_css.contains("[data-c=\"badge\"]") && !two_css.contains("[data-c=badge]"));
    // the runtime finds the component module by the same id
    assert!(page("two").contains(&format!("\"{two_badge}\":\"/assets/routes/two/_components/badge/badge.js\"")), "{}", page("two"));
}

#[test]
fn optional_prop_of_script_component() {
    let t = app_with(&[
        ("lib/components/flagged/flagged.html", Some("<p><span>{$label}</span><if $flag><strong>flag on</strong></if><span>{$clicks}</span></p>")),
        ("lib/components/flagged/flagged.js", Some("export let label = ''\nexport let flag = false\n\nlet clicks = 0\n")),
        ("routes/flag.html", Some("<title>Flag</title>\n<c-flagged label=\"no flag passed\"></c-flagged>")),
    ]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "flag.html");
    // props are live: the instance reads them (`state.flag`, default false when not passed)
    assert!(anchors(&html).contains("<!--p:if * state.flag-->") && html.contains("state.clicks"), "{html}");
    assert!(html.contains("data-prop-label=\"no flag passed\"") && !html.contains("data-prop-flag"), "{html}");
}

#[test]
fn literal_values_in_bindings_and_props() {
    let t = app_with(&[
        ("lib/components/hours/hours.html", Some("<p><if $holiday><b>holiday</b></if><span>{$label} {$count}</span></p>")),
        ("lib/components/hours/hours.js", Some("export let holiday = true\nexport let label = ''\nexport let count = 0\n")),
        ("lib/components/plain/plain.html", Some("<p><if $holiday><b>holiday</b></if><i :title=\"$label\">x</i></p>")),
        (
            "routes/hours.html",
            Some("<title>Hours</title>\n<c-hours :holiday=\"false\" :label=\"'a b'\" :count=\"-2.5\"></c-hours>\n<c-plain :holiday=\"false\" :label=\"'$5 off'\"></c-plain>\n<input aria-label=\"Name\" :disabled=\"true\" :aria-busy=\"false\" :maxlength=\"20\" :placeholder=\"'Your name'\">\n<div :--left=\"0.5\" :--tone=\"'red'\" :--gone=\"null\"></div>"),
        ),
    ]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "hours.html");
    assert!(html.contains("data-with-holiday=\"false\"") && html.contains("data-with-label=\"&quot;a b&quot;\"") && html.contains("data-with-count=\"-2.5\""), "{html}");
    assert!(html.contains("<p data-c=\"plain\"><i title=\"$5 off\">x</i></p>"), "{html}");
    assert!(html.contains("<input aria-label=\"Name\" disabled maxlength=\"20\" placeholder=\"Your name\">"), "{html}");
    assert!(html.contains("style=\"--left:0.5;--tone:red\"") && !html.contains("data-var-"), "{html}");
}

#[test]
fn script_rules_report_at_the_html_line_in_one_pass() {
    // page with an embedded script: script rules, HTML name checks and const-state in one run
    let src = "<title>X</title>\n<script>\n  import { data } from 'pragma'\n  let items = data.items\n  let count = 0\n  let doubled = count * 2\n  const picked = []\n  function pick() { picked.push(1); if (empty) return }\n  const empty = () => count === 0\n  function bump() { $count += 1 }\n</script>\n<p @click=\"pick\">{$doubled} {$picked.length} {$cuont}</p>\n";
    let d = check_with(&[("routes/shop/index.html", Some(src)), ("routes/shop/index.server.js", Some("export function load() { return { items: [] } }\n"))]);
    let at = |rule: &str| -> Vec<u32> { d.iter().filter(|x| x.rule == rule && x.file == Path::new("routes/shop/index.html")).map(|x| x.line).collect() };
    assert_eq!(at("js/top-level-data"), [4], "{d:#?}");
    assert_eq!(at("js/stale-derived"), [6], "{d:#?}");
    assert_eq!(at("js/derived-call"), [8], "{d:#?}");
    assert_eq!(at("js/dollar-in-script"), [10], "{d:#?}");
    assert_eq!(at("js/const-state"), [7], "{d:#?}");
    assert_eq!(at("html/unknown-name"), [12], "{d:#?}");
    // the old API hides the names, and its `pragma` import is not reported a second time
    let old = "<title>X</title>\n<script>\n  import { data } from 'pragma'\n  export function mount(ctx) {}\n</script>\n<p>{$total}</p>\n";
    let d = check_with(&[("routes/shop/index.html", Some(old)), ("routes/shop/index.server.js", Some("export function load() { return {} }\n"))]);
    let rules: Vec<&str> = d.iter().map(|x| x.rule).collect();
    assert_eq!(rules, ["js/legacy-api"], "{d:#?}");
}

#[test]
fn unknown_prop_suggests_the_declared_one() {
    let d = check_with(&[
        ("lib/components/tag/tag.html", Some("<script>\n  export let label = ''\n</script>\n<span>{$label}</span>")),
        (PAGE, Some("<c-tag lable=\"x\"></c-tag>\n<c-tag colour=\"red\"></c-tag>")),
    ]);
    let msgs: Vec<&str> = d.iter().filter(|x| x.rule == "html/unknown-prop").map(|x| x.message.as_str()).collect();
    assert_eq!(msgs.len(), 2, "{d:#?}");
    assert!(msgs[0].contains("did you mean `label`?") && !msgs[0].contains("export let lable"), "{}", msgs[0]);
    assert!(msgs[1].contains("declare it with `export let colour = …`"), "{}", msgs[1]);
}

#[test]
fn a_const_with_any_value_is_a_handler() {
    let script = "<script>\n  function wrap(action) { return action }\n  let saves = 0\n  function save() { saves += 1 }\n  const saveLater = wrap(save)\n</script>\n";
    let shop = "routes/shop/index.html";
    let ok = check_with(&[
        ("lib/state/cart.js", Some("let total = 0\nfunction add() { total += 1 }\nconst addLater = ((action) => action)(add)\n")),
        (shop, Some(&format!("{script}<input aria-label=\"Draft\" @input=\"saveLater\">\n<button @click=\"saveLater(1)\">{{$saves}}</button>\n<button @click=\"shared.cart.addLater\">+</button>"))),
    ]);
    assert!(ok.is_empty(), "{ok:#?}");
    let d = check_with(&[(shop, Some(&format!("{script}<button @click=\"saves\">a</button>\n<button @click=\"saveLatr\">b</button>")))]);
    assert_eq!(d.iter().filter(|x| x.rule == "html/unknown-name").count(), 2, "{d:#?}");
    assert!(d.iter().any(|x| x.line == 7 && x.message.contains("\"saves\" is not a function")), "{d:#?}");
    assert!(d.iter().any(|x| x.line == 8 && x.message.contains("did you mean \"saveLater\"?")), "{d:#?}");
}

#[test]
fn script_component_props_and_names() {
    let comp = |js: &str, html: &str, call: &str| {
        check_with(&[
            ("lib/components/stepper/stepper.js", Some(js)),
            ("lib/components/stepper/stepper.html", Some(html)),
            ("routes/shop/index.html", Some(call)),
        ])
    };
    let js = "export let step = 1\nlet count = 0\nconst atZero = () => count === 0\nfunction add() { count += step }\n";
    let ok = comp(js, "<button @click=\"add\" :disabled=\"$atZero\">{$count}</button>", "<c-stepper step=\"2\"></c-stepper>");
    assert!(ok.is_empty(), "{ok:#?}");
    let d = comp(js, "<button @click=\"add\">{$count}</button>", "<c-stepper stepp=\"2\" :step=\"query.n\"></c-stepper>");
    expect(&d, "html/unknown-prop", "routes/shop/index.html", 1);
    assert!(message(&d, "html/unknown-prop").contains("`stepp`") && message(&d, "html/unknown-prop").contains("its props: step"), "{d:#?}");
    assert_eq!(d.iter().filter(|x| x.rule == "html/unknown-prop").count(), 1, "{d:#?}");
    let d = comp(js, "<button @click=\"reset\">{$total}</button>", "<c-stepper></c-stepper>");
    expect(&d, "html/unknown-name", "lib/components/stepper/stepper.html", 1);
    assert!(d.iter().any(|x| x.message.contains("\"reset\" is not declared")), "{d:#?}");
    assert!(d.iter().any(|x| x.message.contains("$total is not declared")), "{d:#?}");
    let d = comp(js, "<div><button @click=\"atZero\">x</button><input aria-label=\"s\" :model=\"$step\"></div>", "<c-stepper></c-stepper>");
    assert!(d.iter().any(|x| x.rule == "js/prop-write"), "{d:#?}");
    let d = comp("const items = []\nfunction add() { items.push(1) }\n", "<p @click=\"add\">{$items.length}</p>", "<c-stepper></c-stepper>");
    expect(&d, "js/const-state", "lib/components/stepper/stepper.js", 1);
}

#[test]
fn bound_style_and_handler_rules() {
    let d = page("<p :style=\"$css\">x</p>\n<button :onclick=\"$go\">b</button>");
    expect(&d, "html/style-inline", PAGE, 1);
    expect(&d, "html/on-inline", PAGE, 2);
    assert!(message(&d, "html/style-inline").contains(".name=\"path\""), "{d:#?}");
    assert!(message(&d, "html/on-inline").contains("@click=\"action\""), "{d:#?}");
}

#[test]
fn foreign_framework_attributes() {
    let d = page("<ul>\n  <li v-if=\"done\">Task</li>\n  <li x-show=\"open\">b</li>\n  <li on:click=\"go\">c</li>\n</ul>");
    expect(&d, "html/unknown-directive", PAGE, 2);
    expect(&d, "html/unknown-directive", PAGE, 3);
    expect(&d, "html/unknown-directive", PAGE, 4);
    assert!(message(&d, "html/unknown-directive").contains("`v-if` is Vue syntax"), "{d:#?}");
    assert!(!d.iter().any(|x| x.rule == "html/on-inline"), "{d:#?}");
}

#[track_caller]
fn message(d: &[Diagnostic], rule: &str) -> String {
    let list: Vec<String> = d.iter().map(|x| x.to_string()).collect();
    d.iter().find(|x| x.rule == rule).map(|x| x.message.clone()).unwrap_or_else(|| panic!("expected {rule}; got:\n{}", list.join("\n")))
}

#[test]
fn reserved_data_attribute_names_short_form() {
    let cases = [
        ("<p data-text=\"$heading\"></p>", "`data-text` is internal; write `{path} inside the element`"),
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
    expect(&page("<p>{$items[0]}</p>"), "html/invalid-path", PAGE, 1);
}

#[test]
fn negation_only_where_allowed() {
    expect(&page("<p>{!$open}</p>"), "html/invalid-path", PAGE, 1);
}

#[test]
fn unknown_root() {
    expect(&page("<p>{user.name}</p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn for_variable_out_of_scope() {
    let src = "<ul><for x in $items key=x.id><li>{x.name}</li></for></ul>\n<p>{x.name}</p>";
    expect(&page(src), "html/invalid-root", PAGE, 2);
}

#[test]
fn data_without_loader() {
    expect(&page("<p>{data.x}</p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn state_without_module() {
    let d = check_with(&[("routes/shop/index.html", Some("<p>{$x}</p>"))]);
    expect(&d, "html/unknown-name", "routes/shop/index.html", 1);
    assert!(message(&d, "html/unknown-name").contains("has no <script>"), "{d:#?}");
}

#[test]
fn unknown_store() {
    expect(&page("<p>{shared.bag.total}</p>"), "html/invalid-root", PAGE, 1);
}

#[test]
fn unknown_param() {
    let d = check_with(&[("routes/shop/[id].html", Some("<p>{params.slug}</p>"))]);
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
    let d = page("<button @click=\"vanish\">x</button>\n<button @click=\"open\">y</button>");
    expect(&d, "html/unknown-name", PAGE, 1);
    expect(&d, "html/unknown-name", PAGE, 2);
    assert!(d.iter().any(|x| x.message.contains("\"open\" is not a function")), "{d:#?}");
}

const BOX: [(&str, Option<&str>); 2] = [
    ("lib/components/box/box.html", Some("<section><button @click=\"dismiss\">x</button><slot></slot></section>")),
    ("lib/components/box/box.js", Some("export let step = 1\n\nfunction dismiss() {}\n")),
];

#[test]
fn slot_content_belongs_to_writer() {
    let src = "<c-box><button @click=\"dismiss\">x</button></c-box>";
    expect(&check_with(&[BOX[0], BOX[1], (PAGE, Some(src))]), "html/unknown-name", PAGE, 1);
    let ok = "<c-card heading=\"x\"><c-box><button @click=\"toggle\">{$open}</button></c-box></c-card>";
    let d = check_with(&[BOX[0], BOX[1], (PAGE, Some(ok))]);
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn slot_marks_lexical_owner_and_module_props() {
    let src = "<c-card heading=\"x\"><p>fixed</p><div><c-box step=\"2\" class=\"p-4\"><b>{$open}</b> and {$open}</c-box></div></c-card>";
    let t = app_with(&[BOX[0], BOX[1], (PAGE, Some(src))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    assert!(html.contains("<p>fixed</p>"), "{html}");
    assert!(html.contains("<div data-slot=\"1\"><section data-c=\"box\" data-prop-step=\"2\" class=\"p-4\">"), "{html}");
    assert!(html.contains("<b data-text=\"{state.open}\" data-slot=\"1\"></b> and <!--1{state.open}--><!--}-->"), "{html}");
    assert!(html.contains("<div data-c=\"card\" class=\"card p-4\" data-class-featured=\"state.featured\" data-prop-heading=\"x\">"), "{html}");
    assert!(!html.contains("data-s="), "card's CSS names [data-c=\"card\"] itself, so nothing needs a token: {html}");
}

#[test]
fn component_and_page_css_match_only_their_own_markup() {
    let t = scratch(&[
        ("lib/theme.css", "@layer components {\n  .panel { padding: 1rem; }\n}\n"),
        ("lib/components/panel/panel.html", "<section class=\"panel\"><h2 class=\"title\">Panel</h2><p>text</p><slot></slot></section>\n<style>\n  .title { font-size: 2rem }\n  .panel { display: grid }\n</style>\n"),
        ("lib/components/item/item.html", "<div><h3 class=\"title\">Item</h3></div>"),
        ("routes/index.html", "<title>Home</title>\n<c-panel class=\"wide flex\"><c-item></c-item><b class=\"title wide\">slot</b></c-panel>\n<style>\n  .wide { margin: 0 }\n  .flex { gap: 1rem }\n</style>\n"),
    ]);
    let app = t.0.join("app");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    let panel = token_after(&html, "<h2 class=\"title\"");
    let page = token_after(&html, "<b class=\"title wide\"");
    assert_ne!(panel, page);
    assert!(html.contains(&format!("<b class=\"title wide\" data-s=\"{page}\">")), "slot content belongs to the page, not to the panel: {html}");
    assert!(html.contains(&format!("class=\"panel wide flex\" data-s=\"{panel} {page}\"")), "the root belongs to the component and, with a class from the caller, to the caller too: {html}");
    assert!(html.contains("<p>text</p>"), "an element no selector can match gets no token: {html}");
    assert!(html.contains("<h3 class=\"title\">Item</h3>"), "a child component's markup is not the panel's: {html}");
    let css = fs::read_to_string(out.join("client/assets/css/index.css")).unwrap();
    assert!(css.contains(&format!(".title:where([data-s~=\"{panel}\"]) {{ font-size: 2rem }}")), "{css}");
    let theme_at = css.find(".panel { padding").unwrap();
    let panel_at = css.find(&format!(".panel:where([data-s~=\"{panel}\"]) {{ display: grid }}")).unwrap();
    let utilities_at = css.find("@layer utilities").unwrap();
    assert!(theme_at < panel_at && panel_at < utilities_at, "{css}");
    assert!(!css[panel_at..utilities_at].contains("@layer"), "component and page CSS stay outside layers, so they beat utilities: {css}");
    assert!(css.contains(".flex {") && css.contains(&format!(".flex:where([data-s~=\"{page}\"]) {{ gap: 1rem }}")), "a project selector named like a utility keeps the utility: {css}");
}

#[test]
fn scoped_token_only_where_ancestors_allow_a_match() {
    let t = scratch(&[
        ("lib/components/box/box.html", "<div class=\"box\"><slot></slot></div>"),
        ("routes/index.html", "<title>Home</title>\n<script>\n  let showTable = true\n</script>\n<table class=\"tabela\"><tr><td>in</td></tr></table>\n<table><tr><td>out</td></tr></table>\n<if $showTable><div class=\"tabela\"><span>block</span></div></if>\n<span>loose</span>\n<c-box><em>slot</em></c-box><em>bare</em>\n<p>dark</p>\n<style>\n  .tabela td, .tabela span { color: red }\n  .box em { margin: 0 }\n  :global(.dark) p { color: white }\n</style>\n"),
    ]);
    let app = t.0.join("app");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    let page = token_after(&html, "<td");
    for kept in ["<td data-s", "<span data-s=\"PAGE\">block", "<em data-s=\"PAGE\">slot", "<p data-s=\"PAGE\">dark"] {
        assert!(html.contains(&kept.replace("PAGE", &page)), "{kept}: {html}");
    }
    for bare in ["<td>out</td>", "<span>loose</span>", "<em>bare</em>", "<table class=\"tabela\">", "class=\"box\"><em"] {
        assert!(html.contains(bare), "no selector of the page can match {bare}: {html}");
    }
}

#[test]
fn theme_class_named_like_a_utility_is_an_error() {
    let d = check_with(&[("lib/theme.css", Some("@layer components {\n  .btn { padding: 0.5rem 1rem; border-radius: var(--radius-md); }\n  .tag, .active { font-size: .75rem; }\n  .hero > .contents, .x.mr-auto { margin: 0 }\n  .hidden:hover { color: red }\n}\n"))]);
    expect(&d, "css/utility-name", "lib/theme.css", 5);
    assert_eq!(d.iter().filter(|x| x.rule == "css/utility-name").count(), 1, "{d:#?}");
    assert!(message(&d, "css/utility-name").contains(".hidden"), "{d:#?}");
}

#[test]
fn scoped_selector_for_slot_content_is_unmatched() {
    let switch = "<script>\n  export let open = false\n</script>\n<div class=\"switch-row\">\n  <nav class=\"switch\" .is-on=\"$open\"><slot></slot></nav>\n  <if $open><span class=\"hint\">open</span></if>\n  <c-pill class=\"pill\"></c-pill>\n  <slot name=\"note\"></slot>\n</div>\n<style>\n  .switch a { padding: 0 1rem }\n  .switch a[aria-current=\"page\"], .switch-row p { color: red }\n  .switch > :global(a) { display: flex }\n  [data-c=\"switch\"] p { margin: 0 }\n  .switch-row .hint, .pill, .switch.is-on, :is(nav, em).switch, :root { color: blue }\n</style>\n";
    let page = "<title>Switch</title>\n<c-switch>\n  <a href=\"/\" aria-current=\"page\">Human</a>\n  <a href=\"/agents\">Agent</a>\n  <p slot=\"note\">Note</p>\n</c-switch>\n<style>\n  a, p { color: green }\n  .ghost { color: red }\n</style>\n";
    let d = check_with(&[("lib/components/switch/switch.html", Some(switch)), ("lib/components/pill/pill.html", Some("<b>pill</b>")), ("routes/switcher.html", Some(page))]);
    let unmatched: Vec<(String, u32)> = d.iter().filter(|x| x.rule == "css/unmatched-selector").map(|x| (x.file.to_string_lossy().into_owned(), x.line)).collect();
    let comp = "lib/components/switch/switch.html".to_string();
    assert_eq!(unmatched, [(comp.clone(), 11), (comp.clone(), 12), (comp, 12), ("routes/switcher.html".into(), 9)], "{d:#?}");
    let first = d.iter().find(|x| x.rule == "css/unmatched-selector").unwrap();
    assert!(first.severity == Severity::Warning && first.message.contains("`.switch a`") && first.message.contains("`.switch :global(a)`"), "{}", first.message);
}

#[test]
fn route_css_keeps_only_matching_theme_rules_and_preloads_used_fonts() {
    let theme = "@font-face { font-family: 'Body'; font-weight: 100 900; src: url('/fonts/body-latin.woff2') format('woff2'); unicode-range: U+0000-00FF; }\n@font-face { font-family: 'Body'; font-weight: 100 900; src: url('/fonts/body-ext.woff2') format('woff2'); unicode-range: U+0100-024F; }\n@font-face { font-family: 'Title'; src: url('/fonts/title.woff2') format('woff2'); }\n@layer tokens {\n  :root { --font-sans: 'Body', sans-serif; --font-title: 'Title', serif; }\n}\n@layer components {\n  .hero { font-family: var(--font-title); }\n  .note { color: red; }\n}\n";
    let t = scratch(&[
        ("lib/theme.css", theme),
        ("routes/_layout.html", "<head>\n  <link rel=\"preload\" href=\"/fonts/body-latin.woff2\" as=\"font\" type=\"font/woff2\" crossorigin>\n</head>\n<main data-page><slot></slot></main>"),
        ("routes/index.html", "<title>Home</title><h1 class=\"hero\">Hi</h1>"),
        ("routes/plain.html", "<title>Plain</title><p class=\"note\">Plain</p>"),
    ]);
    let app = t.0.join("app");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let index = page_html(&out, "index.html");
    let plain = page_html(&out, "plain.html");
    for html in [&index, &plain] {
        assert!(!html.contains("rel=\"stylesheet\""), "small pages inline their CSS: {html}");
        assert_eq!(html.matches("href=\"/fonts/body-latin.woff2\"").count(), 1, "the layout's own preload is not repeated: {html}");
        assert!(!html.contains("body-ext.woff2\" as"), "{html}");
    }
    assert!(index.contains("<link rel=\"preload\" href=\"/fonts/title.woff2\" as=\"font\" type=\"font/woff2\" crossorigin>"), "{index}");
    assert!(!plain.contains("<link rel=\"preload\" href=\"/fonts/title.woff2\""), "{plain}");
    assert!(index.contains(".hero{") && !index.contains(".note{"), "{index}");
    assert!(plain.contains(".note{") && !plain.contains(".hero{"), "{plain}");
}

fn css_url_app() -> Tmp {
    let theme = "@font-face { font-family: 'Body'; src: url('/fonts/body.woff2') format('woff2'); }\n@font-face { font-family: 'Cdn'; src: url(https://cdn.example/cdn.woff2) format('woff2'); }\n@layer tokens {\n  :root { --font-sans: 'Body', 'Cdn', sans-serif; }\n}\n@layer components {\n  .hero { background: url(\"/img/bg.svg#top\"), url(/missing.png), url(//cdn.example/a.png), url(img/rel.png); }\n  .mask { mask: url(\"data:image/svg+xml,%3Csvg/%3E\"); fill: url(#grad); }\n}\n";
    let mut seed: u64 = 1;
    let noise: String = (0..1500)
        .map(|i| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            format!("--n{i}: \"{seed:016x}\"; ")
        })
        .collect();
    let big = format!("<title>Big</title><p class=\"hero\">Big</p>\n<style>\n  p {{ background-image: url('/img/bg.svg'); {noise}}}\n</style>\n");
    let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\">{}</svg>", "<rect width=\"1\" height=\"1\"/>".repeat(40));
    let t = scratch(&[
        ("lib/theme.css", theme),
        ("routes/index.html", "<title>Home</title><h1 class=\"hero mask\">Hi</h1>"),
        ("routes/big.html", &big),
        ("public/img/bg.svg", &svg),
        ("public/img/rel.png", "png"),
    ]);
    fs::create_dir_all(t.0.join("app/public/fonts")).unwrap();
    fs::write(t.0.join("app/public/fonts/body.woff2"), b"wOF2 body").unwrap();
    t
}

fn hashed_url(text: &str, prefix: &str, ext: &str) -> String {
    let start = text.find(prefix).unwrap_or_else(|| panic!("no {prefix} in {text}"));
    let rest = &text[start..];
    let url = &rest[..rest.find(ext).unwrap() + ext.len()];
    let hash = url[prefix.len()..url.len() - ext.len()].trim_start_matches('-');
    assert!(hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()), "{url} has no hash");
    url.to_string()
}

#[test]
fn css_urls_to_public_files_become_hashed_assets() {
    let t = css_url_app();
    let app = t.0.join("app");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let client = out.join("client");
    let index = fs::read_to_string(client.join("index.html")).unwrap();
    assert!(!index.contains("rel=\"stylesheet\""), "inline CSS: {index}");
    let font = hashed_url(&index, "/assets/fonts/body", ".woff2");
    let svg = hashed_url(&index, "/assets/img/bg", ".svg");
    assert_eq!(fs::read(client.join(&font[1..])).unwrap(), b"wOF2 body");
    assert_eq!(fs::read(client.join(&svg[1..])).unwrap(), fs::read(app.join("public/img/bg.svg")).unwrap());
    assert!(client.join(format!("{}.br", &svg[1..])).is_file() && client.join(format!("{}.gz", &svg[1..])).is_file(), "SVG is precompressed like other assets");
    assert!(!client.join(format!("{}.br", &font[1..])).exists(), "woff2 is not precompressed");
    assert!(client.join("fonts/body.woff2").is_file() && client.join("img/bg.svg").is_file(), "public/ files are still copied");
    assert!(index.contains(&format!("src:url({font})")) || index.contains(&format!("src:url(\"{font}\")")), "{index}");
    assert!(index.contains(&format!("{svg}#top")), "the fragment is kept: {index}");
    assert!(index.contains(&format!("<link rel=\"preload\" href=\"{font}\" as=\"font\" type=\"font/woff2\" crossorigin>")), "{index}");
    assert!(!index.contains("href=\"/fonts/body.woff2\"") && !index.contains("url(/fonts/") && !index.contains("url(\"/fonts/"), "{index}");
    assert!(index.contains("https://cdn.example/cdn.woff2") && index.contains("//cdn.example/a.png"), "external URLs stay: {index}");
    assert!(index.contains("/missing.png") && index.contains("img/rel.png") && index.contains("data:image/svg+xml") && index.contains("url(#grad)"), "missing, relative, data and fragment URLs stay: {index}");
    assert!(!index.contains("/assets/img/rel"), "{index}");

    let big = fs::read_to_string(client.join("big.html")).unwrap();
    let route_css = urls(&big, "href").into_iter().find(|u| u.starts_with("/assets/css/big-")).unwrap_or_else(|| panic!("external route CSS: {big}"));
    assert!(big.contains(&format!("<link rel=\"preload\" href=\"{font}\" as=\"font\"")), "{big}");
    let css = fs::read_to_string(client.join(&route_css[1..])).unwrap();
    assert!(css.contains(&font) && css.contains(&svg), "{css}");
    assert!(!css.contains("url(/img/bg.svg") && !css.contains("url(\"/img/bg.svg") && !css.contains("url('/img/bg.svg"), "{css}");

    let dev = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: dev.clone(), mode: Mode::Dev }).unwrap();
    let index = fs::read_to_string(dev.join("client/index.html")).unwrap();
    let css = fs::read_to_string(dev.join("client/assets/css/index.css")).unwrap();
    assert!(css.contains("url('/fonts/body.woff2')") && index.contains("href=\"/fonts/body.woff2\""), "dev keeps public URLs: {css}");
    assert!(!dev.join("client/assets/fonts").exists());
}

#[test]
fn data_slot_is_generated() {
    expect(&page("<p data-slot=\"1\">x</p>"), "html/data-attribute", PAGE, 1);
}

#[test]
fn layout_action_without_owner() {
    let layout = "<button @click=\"expand\">x</button>\n<main data-page><slot></slot></main>";
    let d = check_with(&[("routes/_layout.html", Some(layout))]);
    expect(&d, "html/unknown-name", "routes/_layout.html", 1);
    assert!(message(&d, "html/unknown-name").contains("the layout has no script"), "{d:#?}");
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
    // routes/index.js exists: a top-level <script> would be a second module
    expect(&page("<script>alert(1)</script>"), "html/duplicate-module", PAGE, 1);
    let shop = |src: &str| check_with(&[("routes/shop/index.html", Some(src))]);
    expect(&shop("<p>\n  <script>let n = 1</script>\n</p>"), "html/script", "routes/shop/index.html", 2);
    expect(&shop("<script src=\"/x.js\"></script>"), "html/script", "routes/shop/index.html", 1);
    expect(&shop("<script type=\"text/javascript\">let total = 1</script>"), "html/script", "routes/shop/index.html", 1);
    expect(&shop("<script>let total = 1</script>\n<script>let other = 2</script>"), "html/script", "routes/shop/index.html", 2);
    expect(&shop("<style>p { color: red }</style>\n<style>b {}</style>"), "html/style-inline", "routes/shop/index.html", 2);
    expect(&check_with(&[("routes/_layout.html", Some("<script>let n = 1</script>\n<main data-page><slot></slot></main>"))]), "html/script", "routes/_layout.html", 1);
}

/// The `data-s` token written on the element whose opening tag contains `marker`.
fn token_after(html: &str, marker: &str) -> String {
    let at = html.find(marker).unwrap_or_else(|| panic!("{marker} not in {html}"));
    let tag_end = at + html[at..].find('>').unwrap();
    let tag = &html[html[..=at].rfind('<').unwrap()..tag_end];
    let start = tag.find("data-s=\"").unwrap_or_else(|| panic!("no data-s in {tag}")) + "data-s=\"".len();
    tag[start..start + tag[start..].find('"').unwrap()].to_string()
}

#[test]
fn script_and_style_inside_the_html() {
    let page = "<title>Shop</title>\n<script>\n  import { log } from 'pragma'\n\n  let count = 0\n  function add() {\n    count += 1\n    log.info('added')\n  }\n</script>\n\n<button class=\"shop-btn\" @click=\"add\">{$count}</button>\n\n<style>\n  .shop-btn { padding: 1rem }\n</style>\n";
    let comp_html = "<script type=\"module\">\n  export let step = 1\n  let total = 0\n  function bump() { total += step }\n</script>\n<p class=\"meter\" @click=\"bump\">{$total}</p>\n<style>\n  [data-c=\"meter\"] .x, .meter { margin: 0 }\n  :global(html[data-theme=\"dark\"]) .meter { color: blue }\n</style>\n";
    let t = app_with(&[("routes/shop/index.html", Some(page)), ("lib/components/meter/meter.html", Some(comp_html)), (PAGE, Some("<c-meter step=\"3\"></c-meter>"))]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.iter().all(|x| x.severity == pragma_diag::Severity::Warning), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "shop/index.html");
    assert!(!html.contains("<script>\n") && !html.contains("<style>"), "{html}");
    assert!(html.contains("\"page\":\"/assets/routes/shop/index.js\""), "{html}");
    let js = fs::read_to_string(out.join("client/assets/routes/shop/index.js")).unwrap();
    assert!(js.contains("export default function ($, $pragma) {") && js.contains("$.count += 1") && js.contains("$pragma.log.info('added')"), "{js}");
    let meter = fs::read_to_string(out.join("client/assets/lib/components/meter/meter.js")).unwrap();
    assert!(meter.contains("$pragma.prop(\"step\", 1)"), "{meter}");
    let shop_css = fs::read_to_string(out.join("client/assets/css/shop/index.css")).unwrap();
    let index_css = fs::read_to_string(out.join("client/assets/css/index.css")).unwrap();
    // styles are scoped to the markup their owner wrote (data-s token), outside any layer
    let shop_token = token_after(&html, "class=\"shop-btn\"");
    assert!(shop_css.contains("/* routes/shop/index.html */") && shop_css.contains(&format!(".shop-btn:where([data-s~=\"{shop_token}\"]) {{ padding: 1rem }}")), "{shop_css}");
    assert!(!shop_css.split("/* routes/shop/index.html */").nth(1).unwrap().contains("@layer"), "{shop_css}");
    assert!(shop_css.contains(".btn { padding") && !shop_css.contains(".tag"), "theme rules the route cannot match are dropped: {shop_css}");
    let index_html = page_html(&out, "index.html");
    let meter_token = token_after(&index_html, "class=\"meter\"");
    let meter_scope = format!(":where([data-s~=\"{meter_token}\"])");
    assert!(index_css.contains("/* lib/components/meter/meter.html */") && index_css.contains(&format!("[data-c=\"meter\"] .x, .meter{meter_scope} {{ margin: 0 }}")), "{index_css}");
    // :global(…) opts out only what is inside it: `.meter` stays scoped (BUG-S-21)
    assert!(index_css.contains(&format!("html[data-theme=\"dark\"] .meter{meter_scope} {{ color: blue }}")), "{index_css}");
    assert!(html.contains("<main data-page=\"shop/index\">"), "{html}");
    // diagnostics in an embedded script point at the .html line
    let bad = page.replace("count += 1", "count += 1\n    console.log(count)");
    let d = check_with(&[("routes/shop/index.html", Some(&bad))]);
    expect(&d, "js/console", "routes/shop/index.html", 8);
    // a .js next to a <script> defines the module twice
    let d = check_with(&[("routes/shop/index.html", Some(page)), ("routes/shop/index.js", Some("let count = 0\n"))]);
    expect(&d, "html/duplicate-module", "routes/shop/index.html", 2);
    let d = check_with(&[("lib/components/meter/meter.html", Some(comp_html)), ("lib/components/meter/meter.css", Some(".meter {}\n")), (PAGE, Some("<c-meter></c-meter>"))]);
    expect(&d, "html/duplicate-module", "lib/components/meter/meter.html", 7);
}

#[test]
fn stores_are_reached_through_shared() {
    let d = page("<p>{shared.cart.total}</p>");
    assert!(d.is_empty(), "{d:#?}");
    let d = check_with(&[("routes/index.js", Some("import { format } from '../utils/fmt.js'\nimport { total } from '../lib/state/cart.js'\n\nlet open = false\nlet items = []\nfunction toggle() { open = format(total) }\n"))]);
    expect(&d, "js/store-import", "routes/index.js", 2);
    let d = check_with(&[("lib/state/cart.js", Some("import { store } from '../../core/state.js'\nexport default store({ total: 0 })\n"))]);
    expect(&d, "js/legacy-api", "lib/state/cart.js", 2);
    let d = check_with(&[("utils/fmt.js", Some("import { log } from 'pragma'\nexport const format = (n) => { log.info('x'); return String(n) }\n"))]);
    expect(&d, "js/import", "utils/fmt.js", 1);
    // a store used only by another store is loaded with it
    let t = app_with(&[
        ("lib/state/prefs.js", Some("let currency = 'BRL'\n")),
        ("lib/state/cart.js", Some("import { shared } from 'pragma'\n\nlet total = 0\nconst label = () => shared.prefs.currency + total\n")),
    ]);
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    assert!(html.contains("\"prefs\":\"/assets/lib/state/prefs.js\""), "{html}");
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
fn legacy_budgets_key_is_ignored() {
    let cfg = "{\n  \"name\": \"x\",\n  \"budgets\": {\n    \"initialJsKb\": 0.1,\n    \"cssKb\": 0.1,\n    \"routes\": { \"/nope\": { \"jsKb\": 1 } }\n  }\n}\n";
    let d = check_with(&[("pragma.json", Some(cfg))]);
    assert!(d.is_empty(), "{d:#?}");
}

#[test]
fn js_lint_runs_on_app_code() {
    let d = check_with(&[("utils/fmt.js", Some("export function format(n) {\n  console.log(n)\n  return n\n}\n"))]);
    expect(&d, "js/console", "utils/fmt.js", 2);
}

#[test]
fn missing_import() {
    let d = check_with(&[("routes/index.js", Some("import { x } from './missing.js'\nlet open = false\nlet items = []\nfunction toggle() { return x }\n"))]);
    expect(&d, "js/import", "routes/index.js", 1);
}

#[test]
fn client_cannot_import_server_module() {
    let src = "import { load } from './shop/[id].server.js'\nlet open = false\nlet items = []\nfunction toggle() { return load }\n";
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
    let r = build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let index = page_html(&out, "index.html");
    assert!(index.contains("data-c=\"modal\"") && !index.contains("props."));
    // no *.server.js: static-only, no server bundle (so no node in dev/serve)
    assert!(!r.has_server && !out.join("server").exists(), "server bundle without *.server.js");
}

#[test]
fn preload_only_what_first_paint_uses() {
    let t = app_with(&[
        ("core/overlay.js", Some("export const open = () => {}")),
        ("routes/about.html", Some("<dialog id=\"d\"><p>x</p></dialog>")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let index = page_html(&out, "index.html");
    let about = page_html(&out, "about.html");
    assert!(out.join("client/assets/core/lazy.js").is_file());
    assert!(!index.contains("lazy.js"), "dynamic import() is not modulepreloaded");
    assert!(!index.contains("overlay.js"), "page without overlay does not preload overlay.js");
    assert!(about.contains("<link rel=\"modulepreload\" href=\"/assets/core/overlay.js\">"), "{about}");
    assert!(about.contains("\"overlays\":true") && !index.contains("\"overlays\""), "the route says when the page has overlays");
}

#[test]
fn theme_script_only_when_used() {
    let t = app_with(&[
        ("core/theme.js", Some("export const setTheme = () => {}")),
        ("routes/index.js", Some("import { setTheme } from '../core/theme.js'\n\nlet open = false\nlet items = []\nfunction toggle() { setTheme('dark') }\n")),
    ]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "shop/[id].html");
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
    let html = page_html(&out, "index.html");
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
    let ok = "<button>{$open}</button><button aria-label=\"Close\">×</button><a href=\"/\"><span class=\"sr-only\">Home</span></a><label><input type=\"checkbox\"> {$open}</label>";
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
fn state_passed_to_script_component_prop() {
    let tally = [("lib/components/tally/tally.html", Some("<p>{$total}</p>")), ("lib/components/tally/tally.js", Some("export let total = 0\n"))];
    let t = app_with(&[tally[0], tally[1], (PAGE, Some("<c-tally :total=\"$items.length\"></c-tally>\n<c-tally total=\"3\"></c-tally>"))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap_or_else(|d| panic!("{d:#?}"));
    let html = page_html(&out, "index.html");
    // the caller's state goes in live as data-with-<prop>; a literal as data-prop-<prop>
    assert!(html.contains("data-with-total=\"state.items.length\""), "{html}");
    assert!(html.contains("data-prop-total=\"3\""), "{html}");
}

#[test]
fn ssr_runs_page_and_component_scripts_on_the_server() {
    let list = "<script>\n  import { format } from '../../../utils/fmt.js'\n  export let items = []\n  let open = true\n  const total = () => format(items.length)\n</script>\n<ul .hidden=\"!$open\"><for item in $items key=item.id><li>{item.name}</li></for><b>{$total}</b></ul>";
    let clocky = "<script>\n  import { time } from 'clock'\n  let now = time()\n</script>\n<p>{$now}</p>";
    let page = "<script>\n  import { data } from 'pragma'\n  let heading = 'Isle'\n</script>\n<title>Isle</title>\n<h1>{$heading}</h1>\n<c-list :items=\"data.items\"></c-list>\n<c-clocky></c-clocky>";
    let mut files = with_clock(&[
        ("lib/components/list/list.html", Some(list)),
        ("lib/components/clocky/clocky.html", Some(clocky)),
        ("routes/isle.html", Some(page)),
        ("routes/isle.server.js", Some("export const ssr = true\nexport function load() {\n  return { items: [{ id: 1, name: 'one' }] }\n}\n")),
    ]);
    files.retain(|(f, _)| *f != "routes/index.js");
    let t = app_with(&files);
    let out = t.0.join("dev");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap_or_else(|d| panic!("{d:#?}"));
    let ops = ssr_ops(&out, "/isle");
    assert!(ops.contains("[\"comp\",\"list\",[[\"items\",\"path\",\"data.items\"]],"), "{ops}");
    assert!(ops.contains("[\"text\",\"{state.heading}\"]") && ops.contains("[\"for\","), "{ops}");
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    // the list and the page run on the server; clocky imports an npm package, so it stays on the client
    assert!(worker.contains("from \"./lib/components/list/list.js\"") && worker.contains("from \"./routes/isle.js\""), "{worker}");
    assert!(!worker.contains("from \"./lib/components/clocky/clocky.js\"") && !worker.contains("\"clocky\": m"), "{worker}");
    let isle = worker.lines().find(|l| l.contains("pattern: \"/isle\"")).unwrap();
    assert!(isle.contains(", page: m") && isle.contains("\"list\": m"), "{isle}");
    let server_list = fs::read_to_string(out.join("server/lib/components/list/list.js")).unwrap();
    assert!(server_list.contains("export default function ($, $pragma) {") && server_list.contains("$pragma.prop(\"items\", [])"), "{server_list}");
    assert!(out.join("server/utils/fmt.js").is_file(), "the server bundle has the script's imports");
}

const PAGE_WITH_PACKAGE: &str = "import { format } from '../utils/fmt.js'\n\
import { shared, log } from 'pragma'\n\
import { time, mode } from 'clock'\n\
import { extra } from 'clock/sub'\n\n\
let open = false\nlet items = []\nconst used = { now: time(), mode, extra }\n\n\
function toggle() {\n  open = !open\n  log.info(format(shared.cart.total))\n}\n";

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
    let index = page_html(&out, "index.html");
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
        ("routes/index.js", Some("import { mode } from 'clock'\nimport { mode as other } from 'clock/sub'\n\nlet open = false\nlet items = []\nconst used = { mode, other }\nfunction toggle() {}\n")),
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
    // the aliased import keeps its line (the script factory opens after the last import)
    assert!(page_js.lines().nth(1).is_some_and(|l| l.starts_with("import { $1$mode as other } from '/assets/npm/clock.js'")), "{page_js}");
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
        Some("import { missing } from 'clock'\n\nlet open = false\nlet items = []\nconst used = missing\nfunction toggle() {}\n"),
    )]));
    expect(&d, "js/import", "routes/index.js", 1);
    assert!(d.iter().any(|x| x.message == "`missing` is not exported by clock"), "{d:?}");
    let d = check_with(&with_clock(&[("node_modules/clock/index.js", Some("import { subtract } from 'calc'\nexport const mode = 1\nexport function time() {\n  return subtract(1, 2)\n}\n"))]));
    assert!(d.iter().any(|x| x.rule == "app/package" && x.message == "package \"clock\" imports `subtract` from \"calc\", which does not export it"), "{d:?}");
    assert!(!d.iter().any(|x| x.message.contains("entries/") || x.message.contains("pragma-check-")), "{d:?}");
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
            ("routes/index.js", Some("import { time } from 'clock'\n\nlet open = false\nlet items = []\nconst now = time()\nfunction toggle() {}\n")),
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
        ("routes/index.js", Some("import { createElement } from 'react'\n\nlet open = false\nlet items = []\nconst blank = createElement()\nfunction toggle() {}\n")),
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

/// App with the real runtime in `core/`, a bare layout and `files` (independent of the fixture app).
fn scratch(files: &[(&str, &str)]) -> Tmp {
    let t = tmp();
    let app = t.0.join("app");
    copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../runtime"), &app.join("core"));
    fs::remove_dir_all(app.join("core/test")).unwrap();
    let base = [("pragma.json", "{ \"name\": \"tpl\" }"), ("routes/_layout.html", "<main data-page><slot></slot></main>")];
    for (f, c) in base.iter().chain(files) {
        let path = app.join(f);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, c).unwrap();
    }
    t
}

const TPL_PAGE: &str = "<title>{data.product.name} | Shop</title>
<link rel=\"canonical\" href=\"https://shop.example/tpl/{params.id}\">
<h1>{data.product.name}</h1>
<p><b>Price:</b> {data.product.price} USD \\{net}</p>
<a href=\"/tpl/{params.id}/reviews\" title=\"Reviews of {data.product.name}\">Reviews</a>
<c-tag label=\"New\"></c-tag>
<c-tag :label=\"data.product.name\"></c-tag>
<code>{literal}</code>
<ul><for p in data.product.parts key=p.id><li>{p.name}</li></for></ul>";

const TPL_SERVER: &str = "export const ssr = true\n\nexport async function load({ params }) {\n  return { product: { id: params.id, name: 'Mug', price: 5, parts: [{ id: 1, name: 'Lid' }] } }\n}\n";

#[test]
fn text_interpolation_builds_markers_templates_and_ops() {
    let t = scratch(&[("routes/tpl/[id].html", TPL_PAGE), ("routes/tpl/[id].server.js", TPL_SERVER), ("lib/components/tag/tag.html", "<span class=\"text-sm\">{$label} item</span>")]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "tpl/[id].html");
    for want in [
        "<h1 data-text=\"{data.product.name}\"></h1>",
        "<p><b>Price:</b> <!--{data.product.price}--><!--}--> USD {net}</p>",
        "<a data-attr-href=\"/tpl/{params.id}/reviews\" data-attr-title=\"Reviews of {data.product.name}\">Reviews</a>",
        "<span data-c=\"tag\" class=\"text-sm\">New item</span>",
        "<span data-c=\"tag\" class=\"text-sm\" data-text=\"{data.product.name} item\"></span>",
        "<code>{literal}</code>",
        "<li data-text=\"{p.name}\"></li>",
        "<title data-text=\"{data.product.name} | Shop\"></title>",
        "<link rel=\"canonical\" data-attr-href=\"https://shop.example/tpl/{params.id}\">",
    ] {
        assert!(html.contains(want), "missing {want}:\n{html}");
    }
    assert!(!html.contains("{$") && !html.contains(":text"), "{html}");
    let ops = ssr_ops(&out, "/tpl/:id");
    for want in [
        "[\"text\",\"{data.product.name}\"]",
        "<!--{data.product.price}-->\"],[\"text\",\"{data.product.price}\"],[\"html\",\"<!--}-->",
        "[\"attr\",\"href\",\"/tpl/{params.id}/reviews\"]",
        "[\"text\",\"{data.product.name} item\"]",
        "[\"text\",\"{p.name}\"]",
    ] {
        assert!(ops.contains(want), "missing {want}:\n{ops}");
    }
}

#[test]
fn text_interpolation_errors() {
    let page = "<p>{a + b}</p>\n<p>{$open</p>\n<p :text=\"$x\"></p>\n<p class=\"text-sm {data.c}\">x</p>\n<p>{{ data.x }}</p>\n<c-tag title=\"Hi {data.x}\"></c-tag>\n<p>{!data.x}</p>\n<p>{data.x[0]}</p>";
    let t = scratch(&[(PAGE, page), ("lib/components/tag/tag.html", "<span>{$title}</span>")]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/invalid-path", PAGE, 1);
    expect(&d, "html/invalid-path", PAGE, 2);
    expect(&d, "html/text-removed", PAGE, 3);
    expect(&d, "html/class", PAGE, 4);
    expect(&d, "html/interpolation", PAGE, 5);
    expect(&d, "html/component", PAGE, 6);
    expect(&d, "html/invalid-path", PAGE, 7);
    expect(&d, "html/invalid-path", PAGE, 8);
    assert!(message(&d, "html/text-removed").contains("write {$x} inside the element"), "{d:#?}");
    assert!(message(&d, "html/interpolation").contains("write {path}"), "{d:#?}");
    let t = scratch(&[(PAGE, "<ul><for x in params.all key=x><li>a</li>{x}</for></ul>\n<c-solo></c-solo>"), ("lib/components/solo/solo.html", "{$label}")]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/block-single-root", PAGE, 1);
    expect(&d, "html/component", "lib/components/solo/solo.html", 1);
    let t = scratch(&[(PAGE, "<p>Price: ${data.price}</p>")]);
    assert!(message(&check(&t.0.join("app")), "html/interpolation").contains("&#36;{path}"));
    let t = scratch(&[(PAGE, "<p>Price: &#36;{params.x} and {query.y}</p>")]);
    let d = check(&t.0.join("app"));
    assert!(!d.iter().any(|x| x.rule == "html/interpolation" || x.rule == "html/invalid-path"), "{d:#?}");
}

#[test]
fn head_templates_need_ssr() {
    let server = "export async function load() {\n  return { product: { name: 'Mug' } }\n}\n";
    let t = scratch(&[("routes/tpl/[id].html", "<title>{data.product.name} | Shop</title>\n<h1>Shop</h1>"), ("routes/tpl/[id].server.js", server)]);
    expect(&check(&t.0.join("app")), "html/head", "routes/tpl/[id].html", 1);
    let t = scratch(&[("routes/tpl/[id].html", "<title>{$name} | Shop</title>\n<h1>Shop</h1>"), ("routes/tpl/[id].server.js", TPL_SERVER)]);
    expect(&check(&t.0.join("app")), "html/head", "routes/tpl/[id].html", 1);
}

#[test]
fn event_call_arguments_are_checked_and_resolved() {
    let row = "<button class=\"text-sm\" @click=\"pick($item, $label, event, 2, 'x')\">{$label}</button>";
    let page = "<ul><for p in data.items key=p.id><li><c-row :item=\"p\" label=\"One\"></c-row></li></for></ul>\n<button @click=\"pick(a + 1)\">bad</button>\n<button @click=\"pick(nope.x)\">root</button>";
    let server = "export async function load() {\n  return { items: [{ id: 1 }] }\n}\n";
    let t = scratch(&[(PAGE, page), ("routes/index.server.js", server), ("lib/components/row/row.html", row)]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/invalid-path", PAGE, 2);
    expect(&d, "html/invalid-root", PAGE, 3);
    let ok = format!("<script>\n  function pick(item, label) {{}}\n</script>\n{}", page.lines().next().unwrap());
    let t = scratch(&[(PAGE, &ok), ("routes/index.server.js", server), ("lib/components/row/row.html", row)]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    assert!(html.contains("data-on-click=\"pick(p, 'One', event, 2, 'x')\">One</button>"), "{html}");
}

#[test]
fn directives_and_blocks_work_inside_pre_and_code() {
    let page = "<script>\n  let lines = [{ n: 1, text: 'one' }]\n  let path = '/x'\n</script>\n<title>Pre</title>\n<h1>Pre</h1>\n<pre :aria-label=\"$path\"><code><for line in $lines key=line.n><span :data-n=\"line.n\">{line.text}</span></for></code></pre>\n<pre>$ curl {url} <span>{$path}</span><c-tag label=\"A\"></c-tag></pre>\n<p><code>{literal}</code></p>";
    let t = scratch(&[(PAGE, page), ("lib/components/tag/tag.html", "<b>{$label}</b>")]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    for want in [
        "<pre data-attr-aria-label=\"state.path\"><code><!--p:for * line state.lines line.n--></code></pre>",
        "\n<span data-attr-data-n=\"line.n\" data-text=\"{line.text}\"></span>",
        "<pre>$ curl {url} <span data-text=\"{state.path}\"></span><b data-c=\"tag\">A</b></pre>",
        "<code>{literal}</code>",
    ] {
        assert!(anchors(&html).contains(want), "missing {want}:\n{html}");
    }
    assert!(!html.contains("<for") && !html.contains(":data-n") && !html.contains("<c-tag"), "{html}");
    let bad = scratch(&[(PAGE, "<pre><span @click=\"nope\">{$missing}</span></pre>")]);
    let d = check(&bad.0.join("app"));
    assert!(d.iter().any(|x| x.rule == "html/unknown-name"), "{d:#?}");
}

const DX_SCRIPT: &str = "<script>\n  let count = 0\n  let tasks = []\n  function increase() { count += 1 }\n</script>\n";
const DX_CART: &str = "let items = []\n\nfunction remove(id) {\n  items = items.filter(item => item.id !== id)\n}\n";

#[test]
fn template_messages_name_the_fix() {
    let lines = [
        "<p>{count}</p>",
        "<p>{$cuont}</p>",
        "<button @click=\"incrase\">a</button>",
        "<button @click=\"$increase\">b</button>",
        "<button :disabled=\"{$count}\">c</button>",
        "<if $tasks.length > 0><p>x</p></if>",
        "<ul><for task in $tasks><li>{task.title}</li></for></ul>",
        "<p>{$price(task)}</p>",
        "<p>{shared.crat.items}</p>",
        "<button @click=\"shared.cart.remvoe(1)\">d</button>",
    ];
    let page = format!("{DX_SCRIPT}{}", lines.join("\n"));
    let t = scratch(&[(PAGE, &page), ("lib/state/cart.js", DX_CART)]);
    let d = check(&t.0.join("app"));
    let at = |n: usize| -> Vec<(&str, &str)> { d.iter().filter(|x| x.line as usize == n + 6).map(|x| (x.rule, x.message.as_str())).collect() };
    let one = |n: usize, rule: &str, want: &str| {
        let found = at(n);
        assert_eq!(found.len(), 1, "line {}: {found:#?}", n + 6);
        assert!(found[0].0 == rule && found[0].1.contains(want), "line {}: {found:#?}", n + 6);
    };
    one(0, "html/invalid-root", "\"count\" is a name of the script: in HTML write $count");
    one(1, "html/unknown-name", "did you mean $count?");
    one(2, "html/unknown-name", "did you mean \"increase\"?");
    one(3, "html/invalid-path", "no `$` in events, write `increase`");
    one(4, "html/invalid-path", "`:disabled` takes a bare path: `:disabled=\"$count\"`");
    one(5, "html/invalid-path", "no operators or quotes in a block tag");
    one(6, "html/for-without-key", "add key=task.id");
    one(7, "html/invalid-path", "a value per item is a field");
    one(8, "html/invalid-root", "did you mean shared.cart?");
    one(9, "html/unknown-name", "Did you mean shared.cart.remove?");
}

#[test]
fn events_pass_the_event_el_and_call_store_functions() {
    let page = format!(
        "{DX_SCRIPT}<input aria-label=\"Search\" @input=\"increase\">\n<button @click=\"increase()\">a</button>\n<ul><for item in shared.cart.items key=item.id><li><button @click=\"shared.cart.remove(item.id)\" @focus=\"increase(el, event)\">x</button></li></for></ul>"
    );
    let t = scratch(&[(PAGE, &page), ("lib/state/cart.js", DX_CART)]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    for want in ["data-on-input=\"increase\"", "data-on-click=\"increase()\"", "data-on-click=\"shared.cart.remove(item.id)\"", "data-on-focus=\"increase(el, event)\"", "\"stores\":{\"cart\""] {
        assert!(html.contains(want), "missing {want}:\n{html}");
    }
    let bad = format!("{DX_SCRIPT}<button @click=\"shared.nope.remove()\">a</button>\n<button @click=\"shared.cart\">b</button>\n<ul><for el in $tasks key=el.id><li>x</li></for></ul>");
    let t = scratch(&[(PAGE, &bad), ("lib/state/cart.js", DX_CART)]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/invalid-root", PAGE, 6);
    expect(&d, "html/invalid-path", PAGE, 7);
    expect(&d, "html/invalid-root", PAGE, 8);
}

#[test]
fn invalid_path_quotes_the_source_and_suggests_a_function() {
    let page = "<script>\n  let items = []\n  let name = ''\n</script>\n<title>X</title>\n<h1>X</h1>\n<p>{$items.length > 0}</p>\n<p>{$name.toUpperCase()}</p>\n<p :title=\"$items[0]\">x</p>";
    let t = scratch(&[(PAGE, page)]);
    let d = check(&t.0.join("app"));
    let messages: Vec<&str> = d.iter().filter(|x| x.rule == "html/invalid-path").map(|x| x.message.as_str()).collect();
    assert_eq!(messages.len(), 3, "{d:#?}");
    for (m, src) in messages.iter().zip(["$items.length > 0", "$name.toUpperCase()", "$items[0]"]) {
        assert!(m.starts_with(&format!("invalid path \"{src}\"")), "{m}");
        assert!(m.contains("function in <script>") && !m.contains("$."), "{m}");
    }
}

const LD_PAGE: &str ="<script>\n  let likes = 0\n</script>\n<title>{data.post.title}</title>\n<script type=\"application/ld+json\">\n{\"@context\": \"https://schema.org\", \"@type\": \"Article\",\n \"headline\": \"{data.post.title}\", \"author\": {\"@type\": \"Person\", \"name\": \"{data.post.author}\"},\n \"url\": \"https://shop.example/blog/{params.slug}\"}\n</script>\n<h1>{data.post.title}</h1>\n<p>{$likes}</p>";
const LD_SERVER: &str = "export const ssr = true\n\nexport async function load({ params }) {\n  return { post: { title: 'Hi', author: 'Ana', slug: params.slug } }\n}\n";

#[test]
fn json_ld_goes_to_head_with_server_values() {
    let t = scratch(&[("routes/blog/[slug].html", LD_PAGE), ("routes/blog/[slug].server.js", LD_SERVER)]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "blog/[slug].html");
    let head = &html[..html.find("</head>").unwrap()];
    assert!(head.contains("<script type=\"application/ld+json\">\n{\"@context\": \"https://schema.org\""), "{html}");
    assert!(head.contains("\"headline\": \"{data.post.title}\""), "{html}");
    assert_eq!(html.matches("application/ld+json").count(), 1, "{html}");
    let static_ld = "<title>About</title>\n<script type=\"application/ld+json\">{\"@type\": \"Organization\", \"name\": \"Shop\"}</script>\n<h1>About</h1>";
    let t = scratch(&[(PAGE, static_ld)]);
    assert!(check(&t.0.join("app")).is_empty(), "static JSON-LD needs no ssr");
}

#[test]
fn json_ld_errors() {
    let bad_json = "<title>X</title>\n<script type=\"application/ld+json\">\n{\"a\": 1,\n \"b\": }\n</script>\n<h1>X</h1>";
    let t = scratch(&[(PAGE, bad_json)]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/json-ld", PAGE, 4);
    assert!(!d.iter().any(|x| x.rule == "js/syntax" || x.rule == "html/script"), "{d:#?}");
    let roots = "<title>X</title>\n<script type=\"application/ld+json\">{\"a\": \"{$x}\", \"b\": \"{a + b}\", \"c\": \"{data.c\"}</script>\n<h1>X</h1>";
    let t = scratch(&[(PAGE, roots)]);
    expect(&check(&t.0.join("app")), "html/json-ld", PAGE, 2);
    let paths = "<title>X</title>\n<script type=\"application/ld+json\">{\"a\": \"{$x}\", \"b\": \"{a + b}\"}</script>\n<h1>X</h1>";
    let t = scratch(&[(PAGE, paths)]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/head", PAGE, 2);
    expect(&d, "html/invalid-path", PAGE, 2);
    let no_ssr = LD_SERVER.replace("export const ssr = true\n\n", "");
    let page = LD_PAGE.replace("<title>{data.post.title}</title>", "<title>Blog</title>");
    let t = scratch(&[("routes/blog/[slug].html", &page), ("routes/blog/[slug].server.js", &no_ssr)]);
    let d = check(&t.0.join("app"));
    assert!(d.iter().any(|x| x.rule == "html/head" && x.message.contains("JSON-LD")), "{d:#?}");
    let placed = "<title>X</title>\n<div><script type=\"application/ld+json\">{}</script></div>\n<script type=\"text/plain\">x</script>\n<c-ld></c-ld>";
    let t = scratch(&[(PAGE, placed), ("lib/components/ld/ld.html", "<script type=\"application/ld+json\">{}</script>\n<p>x</p>")]);
    let d = check(&t.0.join("app"));
    expect(&d, "html/script", PAGE, 2);
    expect(&d, "html/script", PAGE, 3);
    expect(&d, "html/script", "lib/components/ld/ld.html", 1);
    assert!(!d.iter().any(|x| x.rule == "js/syntax"), "{d:#?}");
}

#[test]
fn webmcp_valid_forms_and_notool_stripped() {
    let html = "<script>\n  let open = true\n  function run() {}\n</script>\n<title>Tools</title>\n<h1>Tools</h1>\n<form toolname=\"search_products\" tooldescription=\"Search the catalog by product name\" toolautosubmit @submit=\"run\">\n  <label for=\"query\">Query</label>\n  <input id=\"query\" name=\"query\" type=\"search\">\n  <input name=\"token\" type=\"hidden\" value=\"abc\">\n  <input type=\"submit\" value=\"Go\">\n</form>\n<form notool @submit=\"run\">\n  <label for=\"pwd\">Password</label>\n  <input id=\"pwd\" type=\"password\">\n  <button>Sign in</button>\n</form>\n<if $open>\n  <form toolname=\"save_draft\" tooldescription=\"Save the current draft text\" @submit=\"run\">\n    <textarea name=\"body\" aria-label=\"Body\" toolparamdescription=\"Draft body text\"></textarea>\n    <button>Save</button>\n  </form>\n</if>\n<else>\n  <form toolname=\"save_draft\" tooldescription=\"Save the alternate draft text\" @submit=\"run\">\n    <input name=\"body\" aria-label=\"Body\" aria-description=\"Alternate draft body\">\n    <button>Save</button>\n  </form>\n</else>";
    let t = scratch(&[(PAGE, html)]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let built = page_html(&out, "index.html");
    assert!(built.contains("toolname=\"search_products\""), "{built}");
    assert!(!built.contains("notool"), "notool must be stripped from built HTML: {built}");
}

#[test]
fn webmcp_rules_errors_and_warnings() {
    let html = "<script>\n  let dyn = 'x'\n  function run() {}\n</script>\n<title>Bad</title>\n<h1>Bad</h1>\n<form @submit=\"run\"><label>A <input name=\"a\"></label></form>\n<form toolname=\"only_name\" @submit=\"run\"><button>Go</button></form>\n<form tooldescription=\"Only a description without a name\" @submit=\"run\"><button>Go</button></form>\n<form toolname=\"bad name!\" tooldescription=\"Invalid characters in tool name\" @submit=\"run\"><button>Go</button></form>\n<form toolname=\"searchProducts\" tooldescription=\"Short\" @submit=\"run\"><input aria-label=\"Unnamed\"><input name=\"no_desc\" aria-label=\"X\"></form>\n<form toolname=\"delete_item\" tooldescription=\"Delete an item permanently from store\" toolautosubmit @submit=\"run\"><button>Del</button></form>\n<form toolname=\"delete_item\" tooldescription=\"Duplicate tool name on same page\" @submit=\"run\"><button>Del</button></form>\n<form notool toolname=\"conflict_tool\" @submit=\"run\"><button>No</button></form>\n<form notool tooldescription=\"Conflict description with notool\" @submit=\"run\"><button>No</button></form>\n<div toolname=\"wrong_tag\" tooldescription=\"Wrong tag\" toolautosubmit notool>x</div>\n<form toolname=\"ok_tool\" tooldescription=\"Tool with misplaced param description\" toolparamdescription=\"wrong\" @submit=\"run\"><button>Ok</button></form>\n<form :toolname=\"$dyn\" tooldescription=\"Dynamic {$dyn} text here\" @submit=\"run\"><button>Dyn</button></form>";
    let t = scratch(&[(PAGE, html)]);
    let d = check(&t.0.join("app"));
    expect(&d, "webmcp/form-tool", PAGE, 7);
    expect(&d, "webmcp/pair", PAGE, 8);
    expect(&d, "webmcp/pair", PAGE, 9);
    expect(&d, "webmcp/name", PAGE, 10);
    expect(&d, "webmcp/name-style", PAGE, 11);
    expect(&d, "webmcp/description", PAGE, 11);
    expect(&d, "webmcp/param-name", PAGE, 11);
    expect(&d, "webmcp/param-description", PAGE, 11);
    expect(&d, "webmcp/autosubmit", PAGE, 12);
    expect(&d, "webmcp/duplicate", PAGE, 13);
    expect(&d, "webmcp/notool", PAGE, 14);
    expect(&d, "webmcp/notool", PAGE, 15);
    expect(&d, "webmcp/attribute", PAGE, 16);
    expect(&d, "webmcp/notool", PAGE, 16);
    expect(&d, "webmcp/attribute", PAGE, 17);
    expect(&d, "webmcp/dynamic", PAGE, 18);
    assert!(d.iter().any(|x| x.rule == "webmcp/name-style" && x.message.contains("search_products")), "{d:#?}");
    assert_eq!(d.iter().filter(|x| x.rule == "webmcp/dynamic").count(), 2, "{d:#?}");
}

#[test]
fn webmcp_disabled_in_pragma_json() {
    let cfg = "{\"name\":\"app\",\"webmcp\":false}";
    let plain = "<script>\n  function run() {}\n</script>\n<title>Off</title>\n<h1>Off</h1>\n<form @submit=\"run\"><label>Q <input name=\"q\"></label></form>";
    let t = scratch(&[("pragma.json", cfg), (PAGE, plain)]);
    assert!(check(&t.0.join("app")).is_empty());

    let with_attrs = "<script>\n  function run() {}\n</script>\n<title>Off</title>\n<h1>Off</h1>\n<form toolname=\"search_items\" tooldescription=\"Search items in the list\" @submit=\"run\"><button>Go</button></form>\n<form notool @submit=\"run\"><button>No</button></form>";
    let t = scratch(&[("pragma.json", cfg), (PAGE, with_attrs)]);
    let d = check(&t.0.join("app"));
    expect(&d, "webmcp/disabled", PAGE, 6);
    expect(&d, "webmcp/disabled", PAGE, 7);
    assert_eq!(d.iter().filter(|x| x.rule == "webmcp/disabled").count(), 3, "{d:#?}");
}

#[test]
fn per_route_css_splitting() {
    let t = scratch(&[
        ("routes/_layout.html", "<header class=\"p-4\">Nav</header><main data-page><slot></slot></main>"),
        ("routes/index.html", "<title>Home</title><h1 class=\"text-2xl font-bold\">Home</h1>"),
        ("routes/plain.html", "<title>Plain</title><p class=\"p-4\">Only layout classes</p>"),
    ]);
    let app = t.0.join("app");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let index = page_html(&out, "index.html");
    let plain = page_html(&out, "plain.html");
    assert!(index.contains("data-pragma-route-css>"), "route with page-only classes gets route CSS: {index}");
    assert!(!plain.contains("data-pragma-route-css"), "route with no extra CSS skips route CSS: {plain}");
    assert!(!out.join("client/assets/css/plain.css").exists());
}

#[test]
fn prod_precompression_and_speculation_rules() {
    let t = scratch(&[
        ("routes/index.html", "<title>Home</title><h1 class=\"text-2xl\">Home</h1><p>Hello world</p>"),
        ("public/tiny.txt", "small"),
        ("public/large.txt", &"hello pragma compression ".repeat(40)),
    ]);
    let app = t.0.join("app");
    let prod_out = t.0.join("prod");
    build(&BuildOptions { root: app.clone(), out_dir: prod_out.clone(), mode: Mode::Prod }).unwrap();
    let prod_html = page_html(&prod_out, "index.html");
    assert!(prod_html.contains("<script type=\"speculationrules\">{\"prefetch\":"), "{prod_html}");
    assert!(prod_out.join("client/large.txt.br").is_file() && prod_out.join("client/large.txt.gz").is_file());
    assert!(!prod_out.join("client/tiny.txt.br").exists() && !prod_out.join("client/tiny.txt.gz").exists());
    assert!(prod_out.join("client/index.html.br").is_file() && prod_out.join("client/index.html.gz").is_file());
    let dev_out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: dev_out.clone(), mode: Mode::Dev }).unwrap();
    let dev_html = page_html(&dev_out, "index.html");
    assert!(!dev_html.contains("speculationrules"), "{dev_html}");
    assert!(!dev_out.join("client/large.txt.br").exists() && !dev_out.join("client/large.txt.gz").exists());
}

#[test]
fn static_app_with_public_markdown_emits_cloudflare_worker() {
    let t = scratch(&[
        ("routes/index.html", "<title>Docs</title><h1>Docs</h1>"),
        ("public/index.md", "# Docs\n\nHello markdown.\n"),
    ]);
    let out = t.0.join("dist");
    let r = build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    assert!(!r.has_server, "static app with public/*.md should not spawn Node in dev/serve");
    assert!(out.join("server/worker.js").is_file());
    assert!(out.join("server/wrangler.jsonc").is_file());
    assert!(!out.join("server/node.mjs").exists());
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    assert!(worker.contains("createHandler") && worker.contains("routes: [\n  ]"), "{worker}");
}

#[test]
fn nested_layouts() {
    let t = app_with(&[
        ("routes/docs/_components/nav/nav.html", Some("<nav aria-label=\"Docs\"><a href=\"/docs\">Docs</a></nav>")),
        ("routes/docs/_layout.html", Some("<div class=\"flex\"><p-nav></p-nav><section class=\"p-4\"><slot></slot></section></div>")),
        ("routes/docs/index.html", Some("<title>Docs</title><h1>Overview</h1>")),
        ("routes/docs/api/_layout.html", Some("<article class=\"p-2\"><header>API</header><slot></slot></article>")),
        ("routes/docs/api/index.html", Some("<title>API</title><h1>{data.title}</h1>")),
        ("routes/docs/api/index.server.js", Some("export const ssr = true\nexport function load() { return { title: 'Ref' } }\n")),
    ]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();

    // 1 level deep: wrapped by routes/docs/_layout.html (with <p-nav> resolved relative to routes/docs)
    let docs = page_html(&out, "docs/index.html");
    assert!(docs.contains("<main data-page><div class=\"flex\"><nav data-c=\"nav-"), "{docs}");
    assert!(docs.contains("<h1>Overview</h1></section></div></main>"), "{docs}");

    // 2 levels deep: wrapped outermost-to-innermost (docs -> api -> page) + SSR template includes both
    let api = page_html(&out, "docs/api/index.html");
    let docs_pos = api.find("<div class=\"flex\">").expect("outer nested layout");
    let api_pos = api.find("<article class=\"p-2\"><header>API</header>").expect("inner nested layout");
    let h1_pos = api.find("<h1 data-text=\"{data.title}\"></h1>").expect("page content");
    assert!(docs_pos < api_pos && api_pos < h1_pos, "{api}");
    let tpl = ssr_ops(&out, "/docs/api");
    assert!(tpl.contains("API") && tpl.contains("data.title"), "SSR template should include nested layout and page ops: {tpl}");

    // Sibling route is not wrapped by nested layouts
    let home = page_html(&out, "index.html");
    assert!(!home.contains("aria-label=\"Docs\"") && !home.contains("<header>API</header>"), "{home}");

    // Validation: missing <slot></slot>
    let d = check_with(&[("routes/admin/_layout.html", Some("<div>no slot</div>"))]);
    expect(&d, "html/layout", "routes/admin/_layout.html", 1);

    // Validation: <head> not allowed in nested layout
    let d = check_with(&[("routes/admin/_layout.html", Some("<head><title>Bad</title></head><div><slot></slot></div>"))]);
    expect(&d, "html/layout", "routes/admin/_layout.html", 1);

    // Validation: <script> and <main data-page> and _layout.server.js not allowed
    let d = check_with(&[("routes/admin/_layout.html", Some("<script>let x = 1</script><div><slot></slot></div>"))]);
    expect(&d, "html/script", "routes/admin/_layout.html", 1);
    let d = check_with(&[("routes/admin/_layout.html", Some("<main data-page><slot></slot></main>"))]);
    expect(&d, "html/layout", "routes/admin/_layout.html", 1);
    let d = check_with(&[
        ("routes/admin/_layout.html", Some("<div><slot></slot></div>")),
        ("routes/admin/_layout.server.js", Some("export function load() { return {} }\n")),
    ]);
    expect(&d, "app/structure", "routes/admin/_layout.server.js", 1);
}

fn starter_component(name: &str) -> (String, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../starter/lib/components").join(name);
    (format!("lib/components/{name}/{name}.html"), fs::read_to_string(dir.join(format!("{name}.html"))).unwrap())
}

#[test]
fn webmcp_component_passes_caller_text_to_tool_attribute() {
    let (select_path, select) = starter_component("select");
    let (dropdown_path, dropdown) = starter_component("dropdown");
    let page = "<script>\n  const columns = () => [{ value: 'todo', label: 'To do' }]\n  function add() {}\n</script>\n<title>Board</title>\n<h1>Board</h1>\n<form toolname=\"add_card\" tooldescription=\"Add a card to a column\" @submit=\"add\">\n  <c-select name=\"column\" label=\"Column\" toolparamdescription=\"Column id: todo\" :options=\"$columns\"></c-select>\n  <button>Add</button>\n</form>";
    let theme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../starter/lib/theme.css")).unwrap();
    let t = scratch(&[(PAGE, page), (&select_path, &select), (&dropdown_path, &dropdown), ("lib/theme.css", &theme)]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root: app.clone(), out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let built = page_html(&out, "index.html");
    assert!(built.contains("toolparamdescription=\"Column id: todo\""), "{built}");

    let missing = page.replace(" toolparamdescription=\"Column id: todo\"", "");
    fs::write(app.join(PAGE), &missing).unwrap();
    expect(&check(&app), "webmcp/param-description", &select_path, 32);

    let bound = page.replace("toolparamdescription=\"Column id: todo\"", ":toolparamdescription=\"$columns\"");
    fs::write(app.join(PAGE), &bound).unwrap();
    expect(&check(&app), "webmcp/dynamic", &select_path, 32);

    fs::write(app.join("pragma.json"), "{ \"name\": \"tpl\", \"webmcp\": false }").unwrap();
    fs::write(app.join(PAGE), "<title>Board</title>\n<h1>Board</h1>").unwrap();
    let d = check(&app);
    assert!(!d.iter().any(|x| x.rule == "webmcp/disabled"), "{d:#?}");
}

const ENDPOINT: &str = "import { error } from 'pragma/server'\n\nexport async function GET({ params, env }) {\n  const file = await env.FILES.get(params.name)\n  if (!file) error(404, 'No file')\n  return new Response(file.body)\n}\n";
const MIDDLEWARE: &str = "import { redirect } from 'pragma/server'\n\nexport async function handle({ url, cookies, next }) {\n  if (url.pathname.startsWith('/files') && !cookies.get('sid')) redirect(303, '/')\n  return next()\n}\n";

#[test]
fn endpoints_and_middleware_reach_the_worker() {
    let t = app_with(&[("routes/files/[name].server.js", Some(ENDPOINT)), ("routes/_middleware.server.js", Some(MIDDLEWARE))]);
    let out = t.0.join("dev");
    let r = build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    assert!(r.has_server);
    let worker = fs::read_to_string(out.join("server/worker.js")).unwrap();
    assert!(worker.contains("import * as e0 from \"./routes/files/[name].server.js\""), "{worker}");
    assert!(worker.contains("import { handle as middleware } from \"./routes/_middleware.server.js\""), "{worker}");
    assert!(worker.contains("  middleware,\n"), "{worker}");
    assert!(worker.contains("{ pattern: \"/files/:name\", endpoint: e0, file: \"routes/files/[name].server.js\" }"), "{worker}");
    let routing: serde_json::Value = serde_json::from_str(&fs::read_to_string(out.join("server/routes.json")).unwrap()).unwrap();
    assert_eq!(routing, serde_json::json!({ "middleware": true, "endpoints": ["/files/:name"] }));
    let endpoint = fs::read_to_string(out.join("server/routes/files/[name].server.js")).unwrap();
    assert!(endpoint.starts_with("import { error } from '../../core/server/http.js'"), "{endpoint}");
    let middleware = fs::read_to_string(out.join("server/routes/_middleware.server.js")).unwrap();
    assert!(middleware.starts_with("import { redirect } from '../core/server/http.js'"), "{middleware}");
    assert!(out.join("server/core/server/http.js").is_file());
    let manifest = fs::read_to_string(out.join("client/_pragma/manifest.json")).unwrap();
    assert!(!manifest.contains("/files"), "endpoints stay out of the client manifest");
}

#[test]
fn endpoint_rules() {
    let d = check_with(&[("routes/files/[name].server.js", Some(ENDPOINT))]);
    assert!(d.is_empty(), "{d:#?}");
    let d = check_with(&[("routes/feed.server.js", Some("export function load() {\n  return {}\n}\n"))]);
    expect(&d, "app/endpoint", "routes/feed.server.js", 1);
    assert!(message(&d, "app/endpoint").contains("add feed.html"));
    let d = check_with(&[("routes/feed.server.js", Some("export const limit = 3\n\nexport function GET() {\n  return new Response('x')\n}\n"))]);
    assert!(message(&d, "app/endpoint").contains("move `limit` to a private helper such as _feed.server.js"));
    let d = check_with(&[("routes/products.server.js", Some("export function GET() {\n  return Response.json({ products: [] })\n}\n"))]);
    expect(&d, "app/endpoint-json", "routes/products.server.js", 2);
    let d = check_with(&[("routes/shop.server.js", Some("export function GET() {\n  return new Response('x')\n}\n"))]);
    expect(&d, "app/route-conflict", "routes/shop.server.js", 1);
}

#[test]
fn middleware_rules() {
    expect(&check_with(&[("routes/_middleware.server.js", Some("export const x = 1\n"))]), "app/middleware", "routes/_middleware.server.js", 1);
    expect(&check_with(&[("routes/shop/_middleware.server.js", Some(MIDDLEWARE))]), "app/middleware", "routes/shop/_middleware.server.js", 1);
    assert!(check_with(&[("routes/_middleware.server.js", Some(MIDDLEWARE))]).is_empty());
}

#[test]
fn pragma_server_is_server_only() {
    let d = check_with(&[("routes/index.js", Some("import { redirect } from 'pragma/server'\n\nexport const go = () => redirect(303, '/')\n"))]);
    assert!(message(&d, "js/import").contains("only for .server.js files"));
}

#[test]
fn css_variable_binding() {
    let t = app_with(&[
        ("lib/components/bar/bar.html", Some("<div class=\"bar\" :--left=\"$left\" :--tone=\"$tone\"></div>")),
        ("lib/components/meter/meter.html", Some("<script>\n  export let share = 0\n</script>\n<span class=\"meter\" :--share=\"$share\"></span>")),
        ("lib/theme.css", Some("@layer components {\n  .btn { padding: 0.5rem 1rem; }\n  .tag, .active { font-size: .75rem; }\n  .bar, .meter, .row { left: calc(var(--left) * 1%); width: calc(var(--share) * 1%); }\n}\n")),
        ("routes/bars.html", Some("<title>Bars</title>\n<script>\n  let rows = [{ id: 1, left: 10 }]\n</script>\n<ul>\n  <for row in $rows key=row.id><li class=\"row\" :--left=\"row.left\"><c-meter :share=\"row.left\" :--left=\"row.left\"></c-meter></li></for>\n</ul>\n<c-bar left=\"30\" tone=\"false\"></c-bar>")),
    ]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "bars.html");
    assert!(html.contains("<li class=\"row\" data-var-left=\"row.left\">"), "{html}");
    assert!(html.contains("data-var-share=\"state.share\"") && html.contains("data-with-share=\"row.left\" data-var-left=\"row.left\""), "{html}");
    assert!(html.contains("<div data-c=\"bar\" class=\"bar\" style=\"--left:30\"></div>"), "{html}");
}

#[test]
fn css_variable_rules() {
    let d = page("<p :--barLeft=\"query.x\">x</p>");
    expect(&d, "html/css-var", PAGE, 1);
    assert!(message(&d, "html/css-var").contains("`:--bar-left`"), "{d:#?}");
    let d = check_with(&[("lib/components/bar/bar.html", Some("<div :--left=\"$left\"></div>")), (PAGE, Some("<c-bar left=\"1;color:red\"></c-bar>"))]);
    expect(&d, "html/css-var", "lib/components/bar/bar.html", 1);
    let d = page("<p style=\"left: 3%\">x</p>\n<p :style=\"$css\">y</p>\n<p style:--x=\"a\">z</p>");
    let styles: Vec<&Diagnostic> = d.iter().filter(|x| x.rule == "html/style-inline").collect();
    assert_eq!(styles.len(), 2, "{d:#?}");
    assert!(styles.iter().all(|x| x.message.contains(":--name=\"path\"") && x.message.contains("var(--x)")), "{d:#?}");
    assert!(message(&d, "html/unknown-directive").contains(":--x=\"path\""), "{d:#?}");
    expect(&page("<p :--x=\"!data.x\">x</p>"), "html/invalid-path", PAGE, 1);
}

const TREE_JS: &str = "export let node = null\n";
const TREE_HTML: &str = "<li class=\"tag\"><span>{$node.name}</span><ul><for child in $node.children key=child.id><c-tree :node=\"child\" :--depth=\"child.depth\"></c-tree></for></ul></li>";

#[test]
fn component_renders_itself() {
    let t = app_with(&[
        ("lib/components/tree/tree.html", Some(TREE_HTML)),
        ("lib/components/tree/tree.js", Some(TREE_JS)),
        ("routes/blog/[slug].html", Some("<title>Post</title>\n<ul><c-tree :node=\"data.post.root\"></c-tree></ul>")),
        ("routes/blog/[slug].js", None),
    ]);
    let app = t.0.join("app");
    let d = check(&app);
    assert!(d.is_empty(), "{d:#?}");
    let out = t.0.join("dev");
    build(&BuildOptions { root: app, out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "blog/[slug].html");
    let stub = "<li data-c=\"tree\" data-self=\"c-tree\" class=\"tag\" data-with-node=\"child\" data-var-depth=\"child.depth\"></li>";
    assert_eq!(html.matches(stub).count(), 1, "the page and the definition share the body of the <for>: {html}");
    assert!(anchors(&html).contains("\n<li data-c=\"tree\" class=\"tag\"><span data-text=\"{state.node.name}\"></span><ul><!--p:for * child state.node.children child.id--></ul></li>"), "the definition goes to the template module: {html}");
    let page = fs::read_to_string(out.join("client/blog/[slug].html")).unwrap();
    assert!(!page.contains("<template") && !page.contains("data-def"), "{page}");
    let ops = ssr_ops(&out, "/blog/:slug");
    assert_eq!(ops.matches("[\"node\",\"path\",\"child\"]").count(), 12, "the server renders 12 levels: {ops}");
}

#[test]
fn component_renders_itself_errors() {
    let tree = |html: &str, js: Option<&str>| {
        let mut files = vec![("lib/components/tree/tree.html", Some(html)), (PAGE, Some("<c-tree></c-tree>"))];
        files.extend(js.map(|js| ("lib/components/tree/tree.js", Some(js))));
        check_with(&files)
    };
    let d = tree("<div><c-tree></c-tree></div>", Some(TREE_JS));
    expect(&d, "html/component", "lib/components/tree/tree.html", 1);
    assert!(message(&d, "html/component").contains("never stops"), "{d:#?}");
    let d = tree("<div><if query.open><c-tree></c-tree></if></div>", None);
    assert!(message(&d, "html/component").contains("add a <script>"), "{d:#?}");
    let d = tree("<div><if $node><c-tree><p>x</p></c-tree></if></div>", Some(TREE_JS));
    assert!(message(&d, "html/component").contains("cannot fill slots"), "{d:#?}");
    let d = check_with(&[
        ("lib/components/aa/aa.html", Some("<div><if $node><c-bb></c-bb></if></div>")),
        ("lib/components/aa/aa.js", Some(TREE_JS)),
        ("lib/components/bb/bb.html", Some("<div><c-aa></c-aa></div>")),
        (PAGE, Some("<c-aa></c-aa>")),
    ]);
    expect(&d, "html/component", "lib/components/bb/bb.html", 1);
    assert!(message(&d, "html/component").contains("aa → bb → aa"), "{d:#?}");
}

#[test]
fn markdown_tree_example_from_the_docs() {
    let docs = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/components.md")).unwrap();
    let html = docs.split("`lib/components/md-node/md-node.html`:\n```html\n").nth(1).and_then(|s| s.split("```").next()).unwrap();
    let d = check_with(&[
        ("lib/components/md-node/md-node.html", Some(html)),
        ("routes/blog/[slug].html", Some("<title>Post</title>\n<c-md-node :node=\"data.post.body\"></c-md-node>")),
        ("routes/blog/[slug].js", None),
    ]);
    assert!(d.is_empty(), "{d:#?}");
}

const WASM: &str = "\0asm\u{1}\0\0\0";

#[test]
fn server_wasm_is_copied_with_a_compiled_wasm_rule() {
    let server = "import loadSolver from 'solver'\nimport solverWasm from 'solver/runtime'\nimport addWasm from './_add.wasm'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), kinds: [typeof loadSolver, typeof solverWasm, typeof addWasm] }\n}\n";
    let t = app_with(&[
        ("package.json", Some("{ \"dependencies\": { \"solver\": \"1.0.0\" } }")),
        ("node_modules/solver/package.json", Some("{ \"name\": \"solver\", \"exports\": { \".\": \"./build/solver.mjs\", \"./runtime\": \"./build/solver.wasm\" } }")),
        ("node_modules/solver/build/solver.mjs", Some("export default function loadSolver() {}\n")),
        ("node_modules/solver/build/solver.wasm", Some(WASM)),
        ("routes/shop/_add.wasm", Some(WASM)),
        ("routes/shop/[id].server.js", Some(server)),
    ]);
    let root = t.0.join("app");
    let d = check(&root);
    assert!(d.is_empty(), "{d:?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root, out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    let code = fs::read_to_string(out.join("server/routes/shop/[id].server.js")).unwrap();
    assert!(code.contains("from 'solver'\n"), "{code}");
    assert!(code.contains("from '../../npm/solver/build/solver.wasm'"), "{code}");
    assert!(code.contains("from './_add.wasm'"), "{code}");
    assert_eq!(fs::read(out.join("server/npm/solver/build/solver.wasm")).unwrap(), WASM.as_bytes());
    assert_eq!(fs::read(out.join("server/routes/shop/_add.wasm")).unwrap(), WASM.as_bytes());
    let wrangler: serde_json::Value = serde_json::from_str(&fs::read_to_string(out.join("server/wrangler.jsonc")).unwrap()).unwrap();
    assert_eq!(wrangler["rules"][0]["type"], "CompiledWasm");
    assert_eq!(wrangler["rules"][0]["globs"][0], "**/*.wasm");
    let node = fs::read_to_string(out.join("server/node.mjs")).unwrap();
    assert!(node.contains("registerNodeHooks()\nconst { default: h } = await import('./worker.js')"), "{node}");
}

#[test]
fn wasm_import_in_client_code_is_an_error() {
    let src = "import addWasm from './_add.wasm'\nlet open = false\nlet items = []\nfunction toggle() { return addWasm }\n";
    expect(&check_with(&[("routes/_add.wasm", Some(WASM)), ("routes/index.js", Some(src))]), "js/import", "routes/index.js", 1);
}

#[test]
fn workers_modules_stay_external_in_server_code() {
    let server = "import { env } from 'cloudflare:workers'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), name: env.APP_NAME }\n}\n";
    let t = app_with(&[("routes/shop/[id].server.js", Some(server))]);
    let root = t.0.join("app");
    let d = check(&root);
    assert!(d.is_empty(), "{d:?}");
    let out = t.0.join("dist");
    build(&BuildOptions { root, out_dir: out.clone(), mode: Mode::Prod }).unwrap();
    assert!(fs::read_to_string(out.join("server/routes/shop/[id].server.js")).unwrap().contains("from 'cloudflare:workers'"));
    let src = "import { env } from 'cloudflare:workers'\nlet open = false\nlet items = []\nfunction toggle() { return env }\n";
    expect(&check_with(&[("routes/index.js", Some(src))]), "app/package", "routes/index.js", 1);
}

#[test]
fn layout_block_bodies_carry_no_aria_current_of_the_first_page() {
    let layout = fs::read_to_string(fixture().join("routes/_layout.html")).unwrap().replace("<main data-page>", "<if shared.cart.isEmpty><nav><a href=\"/\">Menu home</a></nav></if>\n<main data-page>");
    let t = app_with(&[("routes/_layout.html", Some(&layout)), (PAGE, Some("<if shared.cart.isEmpty><a href=\"/\">Page home</a></if>"))]);
    let out = t.0.join("out");
    build(&BuildOptions { root: t.0.join("app"), out_dir: out.clone(), mode: Mode::Dev }).unwrap();
    let html = page_html(&out, "index.html");
    assert!(html.contains("<a href=\"/\" aria-current=\"page\">Home</a>"), "a layout link outside blocks is marked: {html}");
    assert!(html.contains("<a href=\"/\">Menu home</a>"), "a layout block body is shared by every page: {html}");
    assert!(html.contains("<a href=\"/\" aria-current=\"page\">Page home</a>"), "a page block body belongs to its page: {html}");
}

#[test]
fn model_with_a_wrong_root_is_one_error() {
    let d = page("<input :model=\"data.x\" aria-label=\"x\">");
    assert_eq!(d.iter().filter(|x| x.rule == "html/invalid-root").count(), 1, "{d:#?}");
    expect(&d, "html/invalid-root", PAGE, 1);
}

#[test]
fn columns_count_characters_after_non_ascii_text() {
    let col = |d: &[Diagnostic], rule: &str| d.iter().find(|x| x.rule == rule).map(|x| (x.line, x.column)).unwrap_or_else(|| panic!("{rule}: {d:#?}"));
    let theme = fs::read_to_string(fixture().join("lib/theme.css")).unwrap();
    let line = theme.lines().count() as u32 + 1;
    let d = check_with(&[("lib/theme.css", Some(&format!("{theme}\n.é{{}} /* x */\n")))]);
    assert_eq!(col(&d, "css/comment"), (line + 1, 6));
    assert_eq!(col(&page("<p>é</p><!-- x -->"), "html/comment"), (1, 9));
    assert_eq!(col(&page("<p>é {{x}}</p>"), "html/interpolation"), (1, 6));
    let d = check_with(&[("routes/products.server.js", Some("export function GET() {\n  return 'é' ? Response.json({}) : null\n}\n"))]);
    assert_eq!(col(&d, "app/endpoint-json"), (2, 16));
}

#[test]
fn attributes_the_build_writes_are_errors_in_source() {
    for attr in ["data-prop-step", "data-self", "data-def", "data-tpl", "data-tid"] {
        let d = page(&format!("<p {attr}=\"x\">a</p>"));
        expect(&d, "html/data-attribute", PAGE, 1);
        assert!(message(&d, "html/data-attribute").contains("is internal"), "{attr}: {d:#?}");
    }
}

#[test]
fn an_imported_module_that_is_not_utf8_is_an_error() {
    let t = app_with(&[]);
    let app = t.0.join("app");
    fs::write(app.join("utils/fmt.js"), b"export const format = (value) => \"\xe7\" + value\n").unwrap();
    let d = check(&app);
    expect(&d, "js/syntax", "utils/fmt.js", 1);
    let out = t.0.join("out");
    let errors = build(&BuildOptions { root: app, out_dir: out, mode: Mode::Dev }).unwrap_err();
    expect(&errors, "js/syntax", "utils/fmt.js", 1);
}
