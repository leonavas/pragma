use pragma_compiler::{BuildOptions, BuildReport, Mode, build, check};
use pragma_diag::{Diagnostic, Severity};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::SystemTime;

static N: AtomicU32 = AtomicU32::new(0);

struct App {
    base: PathBuf,
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
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

fn scratch(kind: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("pragma-npm-{kind}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

impl App {
    fn new() -> App {
        let base = scratch("test");
        copy_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/app"), &base.join("app"));
        App { base }
    }

    fn root(&self) -> PathBuf {
        self.base.join("app")
    }

    fn write(&self, rel: &str, content: &str) -> &App {
        let path = self.root().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        self
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.base.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }

    fn deps(&self, names: &[&str]) -> &App {
        let list: Vec<String> = names.iter().map(|n| format!("\"{n}\": \"1.0.0\"")).collect();
        self.write("package.json", &format!("{{ \"name\": \"shop\", \"dependencies\": {{ {} }} }}", list.join(", ")))
    }

    fn pkg(&self, dir: &str, manifest: &str, files: &[(&str, &str)]) -> &App {
        self.write(&format!("node_modules/{dir}/package.json"), manifest);
        for (f, src) in files {
            self.write(&format!("node_modules/{dir}/{f}"), src);
        }
        self
    }

    fn esm(&self, name: &str, index: &str) -> &App {
        self.pkg(name, &format!("{{ \"name\": \"{name}\", \"version\": \"1.0.0\", \"type\": \"module\", \"main\": \"index.js\" }}"), &[("index.js", index)])
    }

    fn page(&self, imports: &str, fields: &str) -> &App {
        self.write("routes/index.js", &page_src(imports, fields))
    }

    fn check(&self) -> Vec<Diagnostic> {
        check(&self.root())
    }

    fn clean(&self) {
        let d = self.check();
        assert!(d.is_empty(), "{}", show(&d));
    }

    fn try_build(&self, out: &str, mode: Mode) -> Result<BuildReport, Vec<Diagnostic>> {
        build(&BuildOptions { root: self.root(), out_dir: self.base.join(out), mode })
    }

    fn build(&self, out: &str, mode: Mode) -> Out {
        if let Err(d) = self.try_build(out, mode) {
            panic!("build failed:\n{}", show(&d));
        }
        Out { client: self.base.join(out).join("client"), prod: mode == Mode::Prod }
    }

    fn prod(&self) -> Out {
        self.build("dist", Mode::Prod)
    }

    fn dev(&self) -> Out {
        self.build("dev", Mode::Dev)
    }
}

/// Page script that uses each imported name. A direct store import is an error now
/// (js/store-import), so the extra import line some tests need comes from utils/fmt.js.
fn page_src(imports: &str, fields: &str) -> String {
    let imports = imports.replace("import { cart } from '../lib/state/cart.js'", "import { format } from '../utils/fmt.js'");
    let fields = fields.replace("cart.total", "format(0)");
    format!("{imports}\n\nlet open = false\nlet items = []\nconst used = {{ {fields} }}\n\nfunction toggle() {{\n  open = !open\n}}\n")
}

const CARD: &str = "export let heading = ''\nexport let featured = false\n";

struct Out {
    client: PathBuf,
    prod: bool,
}

fn all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if e.file_type().unwrap().is_dir() {
            all_files(&e.path(), out);
        } else {
            out.push(e.path());
        }
    }
}

impl Out {
    fn npm(&self) -> Vec<String> {
        let mut files = Vec::new();
        all_files(&self.client.join("assets/npm"), &mut files);
        let mut urls: Vec<String> = files.iter().map(|f| format!("/{}", f.strip_prefix(&self.client).unwrap().to_string_lossy())).collect();
        urls.sort();
        urls
    }

    fn url(&self, stem: &str) -> String {
        let prefix = format!("/assets/{stem}");
        let found: Vec<String> = self
            .assets()
            .into_iter()
            .filter(|u| if self.prod { u.starts_with(&format!("{prefix}-")) && u.len() == prefix.len() + 12 } else { *u == format!("{prefix}.js") })
            .collect();
        assert_eq!(found.len(), 1, "expected one asset for {stem}: {found:?} in {:?}", self.assets());
        found[0].clone()
    }

    fn assets(&self) -> Vec<String> {
        let mut files = Vec::new();
        all_files(&self.client.join("assets"), &mut files);
        let mut urls: Vec<String> = files.iter().map(|f| format!("/{}", f.strip_prefix(&self.client).unwrap().to_string_lossy())).collect();
        urls.sort();
        urls.dedup();
        urls
    }

    fn code(&self, stem: &str) -> String {
        self.at(&self.url(stem))
    }

    fn at(&self, url: &str) -> String {
        fs::read_to_string(self.client.join(&url[1..])).unwrap()
    }

    fn html(&self, page: &str) -> String {
        fs::read_to_string(self.client.join(format!("{page}.html"))).unwrap()
    }

    fn preloads(&self, page: &str) -> Vec<String> {
        let html = self.html(page);
        html.split("<link rel=\"modulepreload\" href=\"").skip(1).map(|s| s.split('"').next().unwrap().to_string()).collect()
    }

    fn npm_preloads(&self, page: &str) -> Vec<String> {
        self.preloads(page).into_iter().filter(|u| u.starts_with("/assets/npm/")).collect()
    }
}

fn show(d: &[Diagnostic]) -> String {
    d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")
}

#[track_caller]
fn expect(d: &[Diagnostic], rule: &str, file: &str, line: u32) {
    let found = d.iter().any(|x| x.rule == rule && x.file == Path::new(file) && x.line == line && x.severity == Severity::Error);
    assert!(found, "expected {rule} at {file}:{line}; got:\n{}", show(d));
}

#[track_caller]
fn expect_only(d: &[Diagnostic], rule: &str, file: &str, line: u32) {
    expect(d, rule, file, line);
    let others: Vec<&Diagnostic> = d.iter().filter(|x| x.rule != rule || x.file != Path::new(file) || x.line != line).collect();
    assert!(others.is_empty(), "unexpected extra diagnostics:\n{}", show(&others.into_iter().cloned().collect::<Vec<_>>()));
}

const PAGE: &str = "routes/index.js";

fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

#[test]
fn scoped_package_name_and_subpath() {
    let app = App::new();
    app.deps(&["@acme/dates"])
        .pkg(
            "@acme/dates",
            "{ \"name\": \"@acme/dates\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./tz\": \"./tz/index.js\" } }",
            &[("index.js", "export const today = 'SCOPED_ROOT'\n"), ("tz/index.js", "export const zone = 'SCOPED_TZ'\n")],
        )
        .page("import { today } from '@acme/dates'\nimport { zone } from '@acme/dates/tz'", "today, zone");
    app.clean();
    let out = app.prod();
    let url = out.url("npm/acme__dates");
    assert_eq!(out.npm(), vec![url.clone()]);
    let code = out.at(&url);
    assert!(code.contains("SCOPED_ROOT") && code.contains("SCOPED_TZ"), "{code}");
    assert!(out.npm_preloads("index").contains(&url));
    let dev = app.dev();
    assert_eq!(dev.npm(), vec!["/assets/npm/acme__dates.js".to_string()]);
}

fn conditions_app(exports: &str) -> String {
    let app = App::new();
    let files = [
        ("browser.js", "export const pick = 'COND_BROWSER'\n"),
        ("import.js", "export const pick = 'COND_IMPORT'\n"),
        ("require.cjs", "exports.pick = 'COND_REQUIRE'\n"),
        ("default.js", "export const pick = 'COND_DEFAULT'\n"),
        ("node.js", "export const pick = 'COND_NODE'\n"),
    ];
    app.deps(&["cond"])
        .pkg("cond", &format!("{{ \"name\": \"cond\", \"type\": \"module\", \"exports\": {exports} }}"), &files)
        .page("import { pick } from 'cond'", "pick");
    let out = app.prod();
    let code = out.code("npm/cond");
    let picked: Vec<&str> = ["COND_BROWSER", "COND_IMPORT", "COND_REQUIRE", "COND_DEFAULT", "COND_NODE"].into_iter().filter(|m| code.contains(m)).collect();
    assert_eq!(picked.len(), 1, "{exports}: {code}");
    picked[0].to_string()
}

#[test]
fn export_conditions_follow_key_order() {
    let all = "\"node\": \"./node.js\", \"require\": \"./require.cjs\", \"browser\": \"./browser.js\", \"import\": \"./import.js\", \"default\": \"./default.js\"";
    assert_eq!(conditions_app(&format!("{{ {all} }}")), "COND_BROWSER");
    assert_eq!(conditions_app("{ \"require\": \"./require.cjs\", \"import\": \"./import.js\", \"browser\": \"./browser.js\" }"), "COND_IMPORT");
    assert_eq!(conditions_app("{ \"node\": \"./node.js\", \"require\": \"./require.cjs\", \"default\": \"./default.js\" }"), "COND_DEFAULT");
    assert_eq!(conditions_app("{ \".\": { \"browser\": { \"require\": \"./require.cjs\", \"import\": \"./import.js\" }, \"default\": \"./default.js\" } }"), "COND_IMPORT");
    assert_eq!(conditions_app("{ \".\": { \"node\": \"./node.js\", \"default\": { \"require\": \"./require.cjs\", \"default\": \"./default.js\" } } }"), "COND_DEFAULT");
}

#[test]
fn subpath_export_conditions() {
    let app = App::new();
    app.deps(&["kit"])
        .pkg(
            "kit",
            "{ \"name\": \"kit\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./fmt\": { \"node\": \"./fmt-node.js\", \"browser\": \"./fmt-browser.js\", \"default\": \"./fmt-node.js\" }, \"./feature/*\": \"./features/*.js\" } }",
            &[
                ("index.js", "export const root = 'KIT_ROOT'\n"),
                ("fmt-node.js", "export const fmt = 'KIT_FMT_NODE'\n"),
                ("fmt-browser.js", "export const fmt = 'KIT_FMT_BROWSER'\n"),
                ("features/zoom.js", "export const zoom = 'KIT_ZOOM'\n"),
            ],
        )
        .page("import { fmt } from 'kit/fmt'\nimport { zoom } from 'kit/feature/zoom'", "fmt, zoom");
    app.clean();
    let out = app.prod();
    let code = out.code("npm/kit");
    assert!(code.contains("KIT_FMT_BROWSER") && code.contains("KIT_ZOOM"), "{code}");
    assert!(!code.contains("KIT_FMT_NODE") && !code.contains("KIT_ROOT"), "{code}");
    app.page("import { fmt } from 'kit/missing'", "fmt");
    expect_only(&app.check(), "app/package", PAGE, 1);
}

#[test]
fn browser_field_string_replaces_main() {
    let app = App::new();
    app.deps(&["dual"])
        .pkg(
            "dual",
            "{ \"name\": \"dual\", \"type\": \"module\", \"main\": \"./node.js\", \"browser\": \"./browser.js\" }",
            &[("node.js", "export const where = 'DUAL_NODE'\n"), ("browser.js", "export const where = 'DUAL_BROWSER'\n")],
        )
        .page("import { where } from 'dual'", "where");
    app.clean();
    let code = app.prod().code("npm/dual");
    assert!(code.contains("DUAL_BROWSER") && !code.contains("DUAL_NODE"), "{code}");
}

#[test]
fn browser_field_object_mapping() {
    let app = App::new();
    app.deps(&["mapped"])
        .pkg(
            "mapped",
            "{ \"name\": \"mapped\", \"type\": \"module\", \"main\": \"./index.js\", \"browser\": { \"./lib/node.js\": \"./lib/browser.js\", \"./lib/optional.js\": false } }",
            &[
                ("index.js", "import { impl } from './lib/node.js'\nimport './lib/optional.js'\nexport const engine = impl\n"),
                ("lib/node.js", "export const impl = 'MAP_NODE'\n"),
                ("lib/browser.js", "export const impl = 'MAP_BROWSER'\n"),
                ("lib/optional.js", "globalThis.optional = 'MAP_OPTIONAL'\n"),
            ],
        )
        .page("import { engine } from 'mapped'", "engine");
    app.clean();
    let code = app.prod().code("npm/mapped");
    assert!(code.contains("MAP_BROWSER") && !code.contains("MAP_NODE"), "{code}");
    assert!(!code.contains("MAP_OPTIONAL"), "{code}");
}

#[test]
fn browser_field_false_for_entry_is_package_error() {
    let app = App::new();
    app.deps(&["nodeonly"])
        .pkg("nodeonly", "{ \"name\": \"nodeonly\", \"type\": \"module\", \"main\": \"./index.js\", \"browser\": { \"./index.js\": false } }", &[("index.js", "export const disk = 'DISK'\n")])
        .page("import { cart } from '../lib/state/cart.js'\nimport { disk } from 'nodeonly'", "disk, total: cart.total");
    expect_only(&app.check(), "app/package", PAGE, 2);
}

#[test]
fn module_field_wins_over_main() {
    let app = App::new();
    app.deps(&["legacy"])
        .pkg(
            "legacy",
            "{ \"name\": \"legacy\", \"main\": \"./main.cjs\", \"module\": \"./module.mjs\" }",
            &[("main.cjs", "exports.flavor = 'FLAVOR_MAIN'\n"), ("module.mjs", "export const flavor = 'FLAVOR_MODULE'\n")],
        )
        .page("import { flavor } from 'legacy'", "flavor");
    app.clean();
    let code = app.prod().code("npm/legacy");
    assert!(code.contains("FLAVOR_MODULE") && !code.contains("FLAVOR_MAIN"), "{code}");
}

#[test]
fn main_without_extension_and_directory_index() {
    let app = App::new();
    app.deps(&["bare", "folder"])
        .pkg("bare", "{ \"name\": \"bare\", \"type\": \"module\", \"main\": \"lib/entry\" }", &[("lib/entry.js", "export const bareMark = 'BARE_MAIN'\n")])
        .pkg("folder", "{ \"name\": \"folder\", \"type\": \"module\" }", &[("index.js", "export const folderMark = 'FOLDER_INDEX'\n")])
        .page("import { bareMark } from 'bare'\nimport { folderMark } from 'folder'", "bareMark, folderMark");
    app.clean();
    let out = app.prod();
    assert!(out.code("npm/bare").contains("BARE_MAIN"));
    assert!(out.code("npm/folder").contains("FOLDER_INDEX"));
}

const RICH: &str = "export const used = 'RICH_USED'\n\
export const other = 'RICH_OTHER'\n\
export function unusedExport() {\n  return 'RICH_UNUSED'\n}\n\
export default function richDefault() {\n  return 'RICH_DEFAULT'\n}\n";

#[test]
fn named_default_and_namespace_imports() {
    let app = App::new();
    app.deps(&["rich", "whole"]).esm("rich", RICH).esm("whole", &RICH.replace("RICH_", "WHOLE_")).page(
        "import richDefault, { used } from 'rich'\nimport * as wholeModule from 'whole'",
        "used, label: richDefault(), whole: wholeModule.used",
    );
    app.clean();
    let out = app.prod();
    let rich = out.code("npm/rich");
    assert!(rich.contains("RICH_USED") && rich.contains("RICH_DEFAULT"), "{rich}");
    assert!(!rich.contains("RICH_UNUSED") && !rich.contains("RICH_OTHER"), "{rich}");
    let whole = out.code("npm/whole");
    for m in ["WHOLE_USED", "WHOLE_OTHER", "WHOLE_UNUSED", "WHOLE_DEFAULT"] {
        assert!(whole.contains(m), "{m} missing: {whole}");
    }
    let page = out.code("routes/index");
    assert!(page.contains(&out.url("npm/rich")) && page.contains(&out.url("npm/whole")), "{page}");
}

#[test]
fn side_effect_import_is_kept_and_preloaded() {
    let app = App::new();
    app.deps(&["polyfill"]).esm("polyfill", "globalThis.polyfilled = 'EFFECT_RAN'\nexport const unusedThing = 'EFFECT_UNUSED'\n").page("import 'polyfill'", "ready: true");
    app.clean();
    let out = app.prod();
    let url = out.url("npm/polyfill");
    let code = out.at(&url);
    assert!(code.contains("EFFECT_RAN") && !code.contains("EFFECT_UNUSED"), "{code}");
    assert!(out.npm_preloads("index").contains(&url));
    assert!(out.code("routes/index").contains(&url));
}

#[test]
fn dynamic_import_ships_whole_package_without_preload() {
    let app = App::new();
    app.deps(&["lazy"]).esm("lazy", RICH.replace("RICH_", "LAZY_").as_str()).write(
        PAGE,
        "let open = false\nlet items = []\n\nasync function toggle() {\n  const lazyModule = await import('lazy')\n  open = lazyModule.used\n}\n",
    );
    app.clean();
    let out = app.prod();
    let url = out.url("npm/lazy");
    let code = out.at(&url);
    assert!(code.contains("LAZY_USED") && code.contains("LAZY_UNUSED") && code.contains("LAZY_DEFAULT"), "{code}");
    assert!(out.code("routes/index").contains(&url));
    assert!(out.npm_preloads("index").is_empty(), "{:?}", out.preloads("index"));
}

#[test]
fn dynamic_and_static_imports_of_same_package() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).write(
        PAGE,
        "import { used } from 'rich'\n\nlet open = false\nlet items = []\nconst label = used\n\nasync function toggle() {\n  const richModule = await import('rich')\n  open = richModule.other\n}\n",
    );
    app.clean();
    let out = app.prod();
    let url = out.url("npm/rich");
    assert_eq!(out.npm(), vec![url.clone()]);
    let code = out.at(&url);
    assert!(code.contains("RICH_USED") && code.contains("RICH_OTHER"), "{code}");
    assert!(out.npm_preloads("index").contains(&url));
}

#[test]
fn reexports_from_app_modules() {
    let app = App::new();
    app.deps(&["rich"])
        .esm("rich", RICH)
        .write("utils/everything.js", "export * from 'rich'\n")
        .write("utils/renamed.js", "export { used as richUsed } from 'rich'\n")
        .page("import { other } from '../utils/everything.js'\nimport { richUsed } from '../utils/renamed.js'", "other, richUsed");
    app.clean();
    let out = app.prod();
    let url = out.url("npm/rich");
    let code = out.at(&url);
    for m in ["RICH_USED", "RICH_OTHER", "RICH_UNUSED", "RICH_DEFAULT"] {
        assert!(code.contains(m), "{m} missing: {code}");
    }
    assert!(out.npm_preloads("index").contains(&url));
    assert!(out.code("routes/index").contains(&url), "utils/ modules used by one page join its route chunk");
    let dev = app.dev();
    let renamed = dev.code("utils/renamed");
    assert!(renamed.contains("used as richUsed") && renamed.contains("/assets/npm/rich.js"), "{renamed}");
}

#[test]
fn renamed_reexport_alone_is_tree_shaken() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).write("utils/renamed.js", "export { used as richUsed } from 'rich'\n").page("import { richUsed } from '../utils/renamed.js'", "richUsed");
    app.clean();
    let code = app.prod().code("npm/rich");
    assert!(code.contains("RICH_USED") && !code.contains("RICH_UNUSED") && !code.contains("RICH_OTHER"), "{code}");
}

#[test]
fn star_reexport_beside_subpath_import_of_same_package() {
    let app = App::new();
    app.deps(&["kit"])
        .pkg(
            "kit",
            "{ \"name\": \"kit\", \"type\": \"module\", \"exports\": { \".\": \"./index.js\", \"./extra\": \"./extra.js\" } }",
            &[("index.js", "export const root = 'KIT_ROOT'\n"), ("extra.js", "export const root = 'KIT_EXTRA_ROOT'\n")],
        )
        .write("utils/everything.js", "export * from 'kit'\n")
        .page("import { root } from '../utils/everything.js'\nimport { root as extraRoot } from 'kit/extra'", "root, extraRoot");
    app.clean();
    let code = app.prod().code("npm/kit");
    assert!(code.contains("KIT_ROOT") && code.contains("KIT_EXTRA_ROOT"), "{code}");
}

#[test]
fn same_package_from_pages_and_components_is_one_entry() {
    let app = App::new();
    app.deps(&["rich"])
        .esm("rich", RICH)
        .page("import { used } from 'rich'", "used")
        .write("routes/shop/index.js", "import { other } from 'rich'\n\nlet label = other\n")
        .write("lib/components/card/card.js", &format!("import richDefault from 'rich'\nimport {{ shared }} from 'pragma'\n\n{CARD}\nfunction buy() {{\n  shared.cart.total += richDefault().length\n}}\n"));
    app.clean();
    let out = app.prod();
    let url = out.url("npm/rich");
    assert_eq!(out.npm(), vec![url.clone()]);
    let code = out.at(&url);
    assert!(code.contains("RICH_USED") && code.contains("RICH_OTHER") && code.contains("RICH_DEFAULT") && !code.contains("RICH_UNUSED"), "{code}");
    assert_eq!(out.npm_preloads("index"), vec![url.clone()]);
    assert_eq!(out.npm_preloads("shop/index"), vec![url.clone()]);
    assert!(out.npm_preloads("blog/[slug]").is_empty());
    assert!(out.code("routes/shop/index").contains(&url));
}

#[test]
fn shared_dependency_becomes_shared_chunk() {
    let app = App::new();
    app.deps(&["left", "right"])
        .esm("shared", "export function shout(word) {\n  return word + ' SHARED_HELPER'\n}\nexport const sharedUnused = 'SHARED_UNUSED'\n")
        .esm("left", "import { shout } from 'shared'\nexport const leftWord = shout('LEFT_MARK')\n")
        .esm("right", "import { shout } from 'shared'\nexport const rightWord = shout('RIGHT_MARK')\n")
        .page("import { leftWord } from 'left'", "leftWord")
        .write("routes/shop/index.js", "import { rightWord } from 'right'\n\nlet label = rightWord\n");
    app.clean();
    let out = app.prod();
    let left = out.url("npm/left");
    let right = out.url("npm/right");
    let chunks: Vec<String> = out.npm().into_iter().filter(|u| u.starts_with("/assets/npm/_")).collect();
    assert_eq!(chunks.len(), 1, "{:?}", out.npm());
    let chunk = &chunks[0];
    let shared = out.at(chunk);
    assert!(shared.contains("SHARED_HELPER") && !shared.contains("SHARED_UNUSED"), "{shared}");
    assert!(!out.at(&left).contains("SHARED_HELPER") && !out.at(&right).contains("SHARED_HELPER"));
    assert!(out.at(&left).contains(chunk.as_str()), "{}", out.at(&left));
    let index = out.npm_preloads("index");
    assert!(index.contains(&left) && index.contains(chunk) && !index.contains(&right), "{index:?}");
    let shop = out.npm_preloads("shop/index");
    assert!(shop.contains(&right) && shop.contains(chunk) && !shop.contains(&left), "{shop:?}");
    assert!(out.npm_preloads("blog/[slug]").is_empty());
    let dev = app.dev();
    assert!(dev.npm().iter().any(|u| u.starts_with("/assets/npm/_") && !u[13..].contains('-')), "{:?}", dev.npm());
}

#[test]
fn side_effects_false_drops_unused_modules() {
    let app = App::new();
    let index = "import './effect.js'\nexport const pure = 'PURE_EXPORT'\n";
    app.deps(&["clean", "dirty"])
        .pkg("clean", "{ \"name\": \"clean\", \"type\": \"module\", \"main\": \"index.js\", \"sideEffects\": false }", &[("index.js", index), ("effect.js", "globalThis.cleanEffect = 'CLEAN_EFFECT'\n")])
        .pkg("dirty", "{ \"name\": \"dirty\", \"type\": \"module\", \"main\": \"index.js\" }", &[("index.js", index), ("effect.js", "globalThis.dirtyEffect = 'DIRTY_EFFECT'\n")])
        .page("import { pure } from 'clean'\nimport { pure as dirtyPure } from 'dirty'", "pure, dirtyPure");
    app.clean();
    let out = app.prod();
    let clean = out.code("npm/clean");
    assert!(clean.contains("PURE_EXPORT") && !clean.contains("CLEAN_EFFECT"), "{clean}");
    let dirty = out.code("npm/dirty");
    assert!(dirty.contains("PURE_EXPORT") && dirty.contains("DIRTY_EFFECT"), "{dirty}");
}

#[test]
fn side_effects_list_keeps_listed_files() {
    let app = App::new();
    app.deps(&["partial"])
        .pkg(
            "partial",
            "{ \"name\": \"partial\", \"type\": \"module\", \"main\": \"index.js\", \"sideEffects\": [\"./keep.js\"] }",
            &[("index.js", "import './keep.js'\nimport './drop.js'\nexport const value = 'PARTIAL_VALUE'\n"), ("keep.js", "globalThis.kept = 'PARTIAL_KEEP'\n"), ("drop.js", "globalThis.dropped = 'PARTIAL_DROP'\n")],
        )
        .page("import { value } from 'partial'", "value");
    let code = app.prod().code("npm/partial");
    assert!(code.contains("PARTIAL_KEEP") && !code.contains("PARTIAL_DROP"), "{code}");
}

#[test]
fn commonjs_default_and_named_interop() {
    let app = App::new();
    app.deps(&["counter-lib"])
        .pkg(
            "counter-lib",
            "{ \"name\": \"counter-lib\", \"main\": \"index.js\" }",
            &[
                ("index.js", "const meta = require('./meta.json')\nmodule.exports = {\n  increment: (n) => n + 1,\n  version: meta.version,\n  label: 'CJS_OBJECT',\n}\n"),
                ("meta.json", "{ \"version\": \"CJS_JSON_VERSION\" }"),
            ],
        )
        .page("import counterLib, { increment } from 'counter-lib'", "count: increment(1), version: counterLib.version");
    app.clean();
    let out = app.prod();
    let code = out.code("npm/counter-lib");
    assert!(code.contains("CJS_OBJECT") && code.contains("CJS_JSON_VERSION"), "{code}");
    assert!(!code.contains("require("), "{code}");
    assert!(code.contains("export"), "{code}");
}

#[test]
fn commonjs_namespace_import() {
    let app = App::new();
    app.deps(&["counter-lib"])
        .pkg("counter-lib", "{ \"name\": \"counter-lib\", \"main\": \"index.js\" }", &[("index.js", "exports.label = 'CJS_NS'\n")])
        .page("import * as counterLib from 'counter-lib'", "label: counterLib.label");
    app.clean();
    assert!(app.prod().code("npm/counter-lib").contains("CJS_NS"));
}

#[test]
fn esm_package_importing_commonjs_dependency() {
    let app = App::new();
    app.deps(&["modern"])
        .pkg("old-dep", "{ \"name\": \"old-dep\", \"main\": \"index.js\" }", &[("index.js", "module.exports = function legacy() {\n  return 'OLD_DEP_MARK'\n}\nmodule.exports.extra = 'OLD_DEP_EXTRA'\n")])
        .esm("modern", "import legacy from 'old-dep'\nimport { extra } from 'old-dep'\nexport const combined = legacy() + extra\n")
        .page("import { combined } from 'modern'", "combined");
    app.clean();
    let code = app.prod().code("npm/modern");
    assert!(code.contains("OLD_DEP_MARK") && code.contains("OLD_DEP_EXTRA") && !code.contains("require("), "{code}");
}

#[test]
fn circular_imports_inside_package() {
    let app = App::new();
    app.deps(&["loop"])
        .pkg(
            "loop",
            "{ \"name\": \"loop\", \"type\": \"module\", \"main\": \"a.js\" }",
            &[
                ("a.js", "import { fromB } from './b.js'\nexport function fromA() {\n  return 'LOOP_A'\n}\nexport const both = () => fromB() + fromA()\n"),
                ("b.js", "import { fromA } from './a.js'\nexport function fromB() {\n  return 'LOOP_B' + typeof fromA\n}\n"),
            ],
        )
        .page("import { both } from 'loop'", "both: both()");
    app.clean();
    let code = app.prod().code("npm/loop");
    assert!(code.contains("LOOP_A") && code.contains("LOOP_B"), "{code}");
}

#[test]
fn nested_versions_are_kept_apart() {
    let app = App::new();
    app.deps(&["alpha", "beta"])
        .pkg("alpha", "{ \"name\": \"alpha\", \"version\": \"1.0.0\", \"type\": \"module\", \"main\": \"index.js\" }", &[("index.js", "export const version = 'ALPHA_ONE'\n")])
        .esm("beta", "import { version } from 'alpha'\nexport const betaSees = version\n")
        .pkg("beta/node_modules/alpha", "{ \"name\": \"alpha\", \"version\": \"2.0.0\", \"type\": \"module\", \"main\": \"index.js\" }", &[("index.js", "export const version = 'ALPHA_TWO'\n")])
        .page("import { version } from 'alpha'\nimport { betaSees } from 'beta'", "version, betaSees");
    app.clean();
    let out = app.prod();
    let alpha = out.code("npm/alpha");
    assert!(alpha.contains("ALPHA_ONE") && !alpha.contains("ALPHA_TWO"), "{alpha}");
    let beta = out.code("npm/beta");
    assert!(beta.contains("ALPHA_TWO") && !beta.contains("ALPHA_ONE"), "{beta}");
    assert!(out.npm().iter().all(|u| !u.starts_with("/assets/npm/_")), "{:?}", out.npm());
}

#[test]
fn error_not_listed_points_at_import() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).esm("stray", "export const stray = 1\n").page("import { used } from 'rich'\nimport { stray } from 'stray'", "used, stray");
    let d = app.check();
    expect_only(&d, "js/import", PAGE, 2);
    assert!(d[0].message.contains("not in package.json"), "{}", d[0].message);
    app.write("package.json", "{ \"name\": \"shop\", \"devDependencies\": { \"rich\": \"1.0.0\", \"stray\": \"1.0.0\" } }");
    let d = app.check();
    expect(&d, "js/import", PAGE, 1);
    expect(&d, "js/import", PAGE, 2);
}

#[test]
fn error_not_installed_points_at_import() {
    let app = App::new();
    app.deps(&["rich", "ghost"]).esm("rich", RICH).page("import { used } from 'rich'\nimport { phantom } from 'ghost'", "used, phantom");
    let d = app.check();
    expect_only(&d, "js/import", PAGE, 2);
    assert!(d[0].message.contains("not found in node_modules"), "{}", d[0].message);
}

#[test]
fn error_missing_export_points_at_import() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).write("lib/components/card/card.js", &format!("import {{ missingName }} from 'rich'\nimport {{ shared }} from 'pragma'\n\n{CARD}\nfunction buy() {{\n  shared.cart.total += missingName\n}}\n"));
    let d = app.check();
    expect_only(&d, "js/import", "lib/components/card/card.js", 1);
    assert!(d[0].message.contains("missingName"), "{}", d[0].message);
}

#[test]
fn error_missing_export_on_second_line() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).page("import { cart } from '../lib/state/cart.js'\nimport { missingName } from 'rich'", "missingName, total: cart.total");
    expect_only(&app.check(), "js/import", PAGE, 2);
}

#[test]
fn error_missing_default_export() {
    let app = App::new();
    app.deps(&["named"]).esm("named", "export const only = 1\n").page("import namedDefault from 'named'", "label: namedDefault");
    expect_only(&app.check(), "js/import", PAGE, 1);
}

#[test]
fn error_non_js_inside_package() {
    let cases: [(&str, &str, &str); 6] = [
        ("typed.ts", "export const typed = (): number => 1\n", "import { typed } from './typed.ts'\nexport const value = typed\n"),
        ("view.jsx", "export const view = () => <b>x</b>\n", "import { view } from './view.jsx'\nexport const value = view\n"),
        ("inline.js", "export const view = () => <b>x</b>\n", "import { view } from './inline.js'\nexport const value = view\n"),
        ("style.css", ".box { color: red }\n", "import './style.css'\nexport const value = 1\n"),
        ("logo.png", "PNG", "import logo from './logo.png'\nexport const value = logo\n"),
        ("widget.vue", "<template><b>x</b></template>\n", "import widget from './widget.vue'\nexport const value = widget\n"),
    ];
    for (file, src, index) in cases {
        let app = App::new();
        app.deps(&["mixed"])
            .pkg("mixed", "{ \"name\": \"mixed\", \"type\": \"module\", \"main\": \"index.js\" }", &[(file, src), ("index.js", index)])
            .page("import { cart } from '../lib/state/cart.js'\nimport { value } from 'mixed'", "value, total: cart.total");
        let d = app.check();
        expect_only(&d, "app/package", PAGE, 2);
        assert_eq!(d.len(), 1, "{file}:\n{}", show(&d));
        if file != "inline.js" {
            assert!(d[0].message.starts_with(&format!("package \"mixed\" imports {file}, which is not JavaScript")), "{file}: {}", d[0].message);
        }
    }
}

#[test]
fn error_entry_is_not_js() {
    for (main, file, src) in [("index.ts", "index.ts", "export const value = 1\n"), ("index.jsx", "index.jsx", "export const value = 1\n"), ("data.json", "data.json", "{}")] {
        let app = App::new();
        app.deps(&["typed"])
            .pkg("typed", &format!("{{ \"name\": \"typed\", \"main\": \"{main}\" }}"), &[(file, src)])
            .page("import { cart } from '../lib/state/cart.js'\nimport { value } from 'typed'", "value, total: cart.total");
        expect_only(&app.check(), "app/package", PAGE, 2);
    }
}

#[test]
fn error_node_builtin_inside_package() {
    for builtin in ["fs", "node:fs", "path", "node:crypto"] {
        let app = App::new();
        app.deps(&["disk"])
            .esm("disk", &format!("import {{ readFileSync }} from '{builtin}'\nexport const load = () => readFileSync('x')\n"))
            .page("import { cart } from '../lib/state/cart.js'\nimport { load } from 'disk'", "load, total: cart.total");
        let d = app.check();
        expect_only(&d, "app/package", PAGE, 2);
        assert!(d[0].message.contains(&format!("package \"disk\" imports the Node module `{builtin}`")), "{}", d[0].message);
        assert!(!d[0].message.contains("tmp") && !d[0].message.contains(".."), "{}", d[0].message);
    }
}

#[test]
fn error_node_builtin_in_nested_dependency_points_at_top_package() {
    let app = App::new();
    app.deps(&["rich", "outer"])
        .esm("rich", RICH)
        .esm("outer", "import { digest } from 'inner'\nexport const hashed = digest\n")
        .esm("outer/node_modules/inner", "import { createHash } from 'node:crypto'\nexport const digest = createHash\n")
        .page("import { used } from 'rich'\nimport { hashed } from 'outer'", "used, hashed");
    let d = app.check();
    expect_only(&d, "app/package", PAGE, 2);
    assert!(d[0].message.contains("package \"inner\" imports the Node module `node:crypto`"), "{}", d[0].message);
}

#[test]
fn error_unresolved_dependency_inside_package() {
    let app = App::new();
    app.deps(&["outer"]).esm("outer", "import { gone } from 'vanished'\nexport const value = gone\n").page("import { value } from 'outer'", "value");
    let d = app.check();
    expect_only(&d, "app/package", PAGE, 1);
    assert!(d[0].message.contains("package \"outer\" imports \"vanished\"") && !d[0].message.contains("tmp"), "{}", d[0].message);
}

#[test]
fn error_node_builtin_in_app_client_code() {
    let app = App::new();
    app.deps(&[]).page("import { cart } from '../lib/state/cart.js'\nimport { readFileSync } from 'node:fs'\nimport { join } from 'path'", "reader: readFileSync, join, total: cart.total");
    let d = app.check();
    assert_eq!(d.len(), 2, "{}", show(&d));
    expect(&d, "app/package", PAGE, 2);
    expect(&d, "app/package", PAGE, 3);
    assert!(d.iter().any(|x| x.line == 2 && x.message.starts_with("`node:fs` is a Node module and doesn't run in the browser; use it in a .server.js")), "{}", show(&d));
    assert!(d.iter().any(|x| x.line == 3 && x.message.starts_with("`path` is a Node module")), "{}", show(&d));
    assert!(d.iter().all(|x| !x.message.contains("npm i")), "{}", show(&d));
    app.page("import { readFile } from 'fs/promises'", "reader: readFile");
    expect_only(&app.check(), "app/package", PAGE, 1);
}

#[test]
fn node_builtins_allowed_in_server_without_listing() {
    let app = App::new();
    app.deps(&[]).write(
        "routes/shop/[id].server.js",
        "import { join } from 'path'\nimport { readFile } from 'node:fs/promises'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), file: join('a', 'b'), reader: typeof readFile }\n}\n",
    );
    app.clean();
}

#[test]
fn listed_builtin_polyfill_is_a_package() {
    let app = App::new();
    app.deps(&["buffer"]).esm("buffer", "export const Buffer = 'BUFFER_POLYFILL'\n").page("import { Buffer } from 'buffer'", "kind: Buffer");
    app.clean();
    assert!(app.prod().code("npm/buffer").contains("BUFFER_POLYFILL"));
}

#[test]
fn error_ui_frameworks_on_client() {
    for (name, export) in [("react", "createElement"), ("vue", "createApp"), ("@angular/core", "Component"), ("preact", "h"), ("lit", "html"), ("jquery", "ajax")] {
        let app = App::new();
        app.deps(&[name]).esm(name, &format!("export const {export} = () => null\n")).page(&format!("import {{ cart }} from '../lib/state/cart.js'\nimport {{ {export} }} from '{name}'"), &format!("ui: {export}, total: cart.total"));
        let d = app.check();
        expect_only(&d, "js/import", PAGE, 2);
        assert!(d[0].message.contains("UI framework"), "{name}: {}", d[0].message);
    }
    let app = App::new();
    app.deps(&["react-dom"]).esm("react-dom", "export const render = () => null\n").page("import { render } from 'react-dom/client'", "render");
    expect_only(&app.check(), "js/import", PAGE, 1);
}

#[test]
fn error_template_literal_dynamic_import() {
    let app = App::new();
    app.deps(&["lazy"]).esm("lazy", RICH).write(
        PAGE,
        "export const state = () => ({ open: false, items: [] })\nexport const actions = {\n  async toggle(ctx) {\n    const lazyModule = await import(`lazy`)\n    ctx.$.open = lazyModule.used\n  },\n}\n",
    );
    let d = app.check();
    expect(&d, "js/import", PAGE, 4);
    assert!(d.iter().any(|x| x.message.contains("template literal")), "{}", show(&d));
}

#[test]
fn error_package_imported_in_several_files_points_at_first() {
    let app = App::new();
    app.deps(&["rich"])
        .esm("rich", RICH)
        .page("import { missingName } from 'rich'", "missingName")
        .write("routes/shop/index.js", "import { used } from 'rich'\n\nlet label = used\n");
    let d = app.check();
    expect(&d, "js/import", PAGE, 1);
    assert!(d.iter().all(|x| x.rule == "js/import"), "{}", show(&d));
}

const SERVER: &str = "import postgres from 'postgres'\nimport { readFile } from 'node:fs/promises'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), kind: typeof postgres, reader: typeof readFile }\n}\n";

#[test]
fn server_listed_package_needs_no_node_modules() {
    let app = App::new();
    app.deps(&["postgres"]).write("routes/shop/[id].server.js", SERVER);
    app.clean();
    let out = app.prod();
    assert!(out.npm().is_empty(), "{:?}", out.npm());
    let server = app.read("dist/server/routes/shop/[id].server.js");
    assert!(server.contains("from 'postgres'") && server.contains("from 'node:fs/promises'"), "{server}");
    assert!(!app.base.join("dist/.cache/npm/key").exists());
}

#[test]
fn server_unlisted_package_is_error() {
    let app = App::new();
    app.deps(&[]).write("routes/shop/[id].server.js", SERVER);
    expect_only(&app.check(), "js/import", "routes/shop/[id].server.js", 1);
}

#[test]
fn server_may_import_ui_framework() {
    let app = App::new();
    app.deps(&["react"]).write("routes/shop/[id].server.js", "import react from 'react'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), kind: typeof react }\n}\n");
    app.clean();
}

#[test]
fn client_importing_node_only_package() {
    let app = App::new();
    let manifest = "{ \"name\": \"pg-lite\", \"type\": \"module\", \"exports\": { \"node\": \"./index.js\" } }";
    app.deps(&["pg-lite"])
        .pkg("pg-lite", manifest, &[("index.js", "export const connect = () => 'PG'\n")])
        .write("routes/shop/[id].server.js", "import { connect } from 'pg-lite'\nimport { find } from '../../utils/db.js'\n\nexport async function load({ params }) {\n  return { product: await find(params.id), conn: connect() }\n}\n");
    app.clean();
    app.page("import { cart } from '../lib/state/cart.js'\nimport { connect } from 'pg-lite'", "connect, total: cart.total");
    let d = app.check();
    expect_only(&d, "app/package", PAGE, 2);
    assert!(d[0].message.contains("not exported under the conditions"), "{}", d[0].message);
}

#[test]
fn client_cannot_import_package_installed_only_for_server_listing() {
    let app = App::new();
    app.deps(&["postgres"]).write("routes/shop/[id].server.js", SERVER).page("import postgres from 'postgres'", "kind: typeof postgres");
    let d = app.check();
    expect_only(&d, "js/import", PAGE, 1);
    assert!(d[0].message.contains("not found in node_modules"), "{}", d[0].message);
}

fn cached_app() -> App {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH).page("import { used } from 'rich'", "used");
    app
}

fn key(app: &App) -> String {
    app.read("dist/.cache/npm/key")
}

fn cached_file(app: &App) -> PathBuf {
    app.base.join("dist/.cache/npm/out/rich.js")
}

#[test]
fn cache_hit_skips_rebundle() {
    let app = cached_app();
    let first = app.prod().npm();
    let k = key(&app);
    let stamp = mtime(&cached_file(&app));
    let key_stamp = mtime(&app.base.join("dist/.cache/npm/key"));
    let out = app.prod();
    assert_eq!(out.npm(), first);
    assert_eq!(key(&app), k);
    assert_eq!(mtime(&cached_file(&app)), stamp);
    assert_eq!(mtime(&app.base.join("dist/.cache/npm/key")), key_stamp);
    app.write("utils/fmt.js", "export function format(value) {\n  return `R$${value.toFixed(2)}`\n}\n");
    app.prod();
    assert_eq!(key(&app), k);
    assert_eq!(mtime(&cached_file(&app)), stamp);
}

#[track_caller]
fn assert_rebundled(app: &App, old_key: &str, change: &str) {
    let now = key(app);
    assert_ne!(now, old_key, "cache not invalidated by {change}");
}

#[test]
fn cache_invalidated_by_app_manifest_version() {
    let app = cached_app();
    app.prod();
    let k = key(&app);
    app.write("package.json", "{ \"name\": \"shop\", \"dependencies\": { \"rich\": \"1.1.0\" } }");
    app.prod();
    assert_rebundled(&app, &k, "package.json version");
}

#[test]
fn cache_invalidated_by_package_version() {
    let app = cached_app();
    app.prod();
    let k = key(&app);
    app.write("node_modules/rich/package.json", "{ \"name\": \"rich\", \"version\": \"1.0.10\", \"type\": \"module\", \"main\": \"index.js\" }");
    app.write("node_modules/rich/index.js", &RICH.replace("RICH_USED", "RICH_USED_V2"));
    let out = app.prod();
    assert_rebundled(&app, &k, "node_modules package.json");
    assert!(out.code("npm/rich").contains("RICH_USED_V2"));
}

#[test]
fn cache_invalidated_by_lockfiles() {
    for lock in ["package-lock.json", "bun.lock", "pnpm-lock.yaml", "yarn.lock", "node_modules/.package-lock.json"] {
        let app = cached_app();
        app.write(lock, "lockfileVersion: 1\n");
        app.prod();
        let k = key(&app);
        app.write(lock, "lockfileVersion: 2 updated\n");
        app.prod();
        assert_rebundled(&app, &k, lock);
    }
}

#[test]
fn cache_invalidated_by_new_imported_name() {
    let app = cached_app();
    let out = app.prod();
    assert!(!out.code("npm/rich").contains("RICH_OTHER"));
    let k = key(&app);
    app.page("import { used, other } from 'rich'", "used, other");
    let out = app.prod();
    assert_rebundled(&app, &k, "a new imported name");
    assert!(out.code("npm/rich").contains("RICH_OTHER"));
}

#[test]
fn cache_invalidated_by_mode_change() {
    let app = cached_app();
    app.build("dist", Mode::Dev);
    let k = key(&app);
    let dev_code = fs::read_to_string(cached_file(&app)).unwrap();
    app.build("dist", Mode::Prod);
    assert_rebundled(&app, &k, "dev -> prod");
    let prod_code = fs::read_to_string(cached_file(&app)).unwrap();
    assert_ne!(dev_code, prod_code);
    let prod_key = key(&app);
    app.build("dist", Mode::Dev);
    assert_eq!(key(&app), k);
    assert_ne!(key(&app), prod_key);
}

#[test]
fn cache_ignores_corrupt_metadata() {
    let app = cached_app();
    app.prod();
    fs::write(app.base.join("dist/.cache/npm/bundle.json"), "{ broken").unwrap();
    let out = app.prod();
    assert!(out.code("npm/rich").contains("RICH_USED"));
    fs::remove_file(cached_file(&app)).unwrap();
    let out = app.prod();
    assert!(out.code("npm/rich").contains("RICH_USED"));
}

#[test]
fn dev_stable_names_prod_hashed() {
    let app = App::new();
    app.deps(&["rich", "@acme/dates"])
        .esm("rich", RICH)
        .pkg("@acme/dates", "{ \"name\": \"@acme/dates\", \"type\": \"module\", \"main\": \"index.js\" }", &[("index.js", "export const today = 'SCOPED_ROOT'\n")])
        .page("import { used } from 'rich'\nimport { today } from '@acme/dates'", "used, today");
    let dev = app.dev();
    assert_eq!(dev.npm(), vec!["/assets/npm/acme__dates.js".to_string(), "/assets/npm/rich.js".to_string()]);
    let prod = app.prod();
    let files = prod.npm();
    assert_eq!(files.len(), 2);
    for f in &files {
        let name = f.trim_start_matches("/assets/npm/").trim_end_matches(".js");
        let (stem, hash) = name.rsplit_once('-').unwrap();
        assert!(stem == "rich" || stem == "acme__dates", "{f}");
        assert!(hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()), "{f}");
    }
}

#[test]
fn hashes_are_deterministic_across_clean_builds() {
    let make = || {
        let app = App::new();
        app.deps(&["left", "right"])
            .esm("shared", "export const shout = (word) => word + ' SHARED_HELPER'\n")
            .esm("left", "import { shout } from 'shared'\nexport const leftWord = shout('LEFT_MARK')\n")
            .esm("right", "import { shout } from 'shared'\nexport const rightWord = shout('RIGHT_MARK')\n")
            .page("import { leftWord } from 'left'\nimport { rightWord } from 'right'", "leftWord, rightWord");
        app
    };
    let first = make();
    let second = make();
    let a = first.prod();
    let b = second.prod();
    assert_eq!(a.npm(), b.npm());
    assert_eq!(a.assets(), b.assets());
    for u in a.npm() {
        assert_eq!(a.at(&u), b.at(&u), "{u}");
    }
    first.build("again", Mode::Prod);
    let again = Out { client: first.base.join("again/client"), prod: true };
    assert_eq!(again.assets(), a.assets());
    let dev_a = first.dev();
    let dev_b = second.dev();
    for u in dev_a.npm() {
        assert_eq!(dev_a.at(&u), dev_b.at(&u), "{u}");
    }
}

#[test]
fn node_env_replaced_per_mode() {
    let app = App::new();
    app.deps(&["envy"])
        .esm("envy", "export const mode = process.env.NODE_ENV\nexport const debug = process.env.NODE_ENV !== 'production' ? 'ENV_DEBUG_ON' : 'ENV_DEBUG_OFF'\n")
        .page("import { mode, debug } from 'envy'", "mode, debug");
    let prod = app.prod().code("npm/envy");
    assert!(prod.contains("production") && !prod.contains("process.env") && !prod.contains("ENV_DEBUG_ON"), "{prod}");
    let dev = app.dev().code("npm/envy");
    assert!(dev.contains("development") && !dev.contains("process.env") && !dev.contains("ENV_DEBUG_OFF"), "{dev}");
}

fn noise(len: usize) -> String {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            char::from(b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[(seed % 62) as usize])
        })
        .collect()
}

fn heavy_app() -> App {
    let app = App::new();
    app.deps(&["heavy"]).esm("heavy", &format!("export const blob = '{}'\n", noise(40_000))).page("import { blob } from 'heavy'", "size: blob.length");
    app
}

#[test]
fn npm_bytes_count_toward_page_size() {
    let plain = App::new();
    let base = plain.try_build("dist", Mode::Prod).unwrap().pages.iter().find(|p| p.pattern == "/").unwrap().js_gzip_bytes;
    let report = heavy_app().try_build("dist", Mode::Prod).unwrap();
    let with = report.pages.iter().find(|p| p.pattern == "/").unwrap().js_gzip_bytes;
    assert!(with > base + 25_000, "{base} -> {with}");
}

#[test]
fn invalid_app_package_json_is_diagnostic() {
    let app = App::new();
    app.write("package.json", "{\n  \"dependencies\": {\n    \"rich\": \"1.0.0\",\n  }\n}\n").esm("rich", RICH).page("import { used } from 'rich'", "used");
    let d = app.check();
    expect(&d, "app/package", "package.json", 4);
    assert!(app.try_build("dist", Mode::Prod).is_err());
}

#[test]
fn non_object_dependencies_are_ignored() {
    let app = App::new();
    app.write("package.json", "{ \"dependencies\": [\"rich\"] }").esm("rich", RICH).page("import { used } from 'rich'", "used");
    expect_only(&app.check(), "js/import", PAGE, 1);
    app.write("package.json", "[]");
    expect_only(&app.check(), "js/import", PAGE, 1);
}

#[test]
fn invalid_package_manifest_in_node_modules_is_diagnostic() {
    let app = App::new();
    app.deps(&["broken"]).pkg("broken", "{ \"name\": \"broken\", ", &[("index.js", "export const value = 1\n")]).page("import { value } from 'broken'", "value");
    let d = app.check();
    expect_only(&d, "app/package", PAGE, 1);
    assert!(d[0].message.starts_with("invalid package.json in broken: EOF while parsing"), "{}", d[0].message);
    assert!(app.try_build("dist", Mode::Prod).is_err());
}

#[cfg(unix)]
#[test]
fn symlink_outside_project_is_package_error() {
    let app = App::new();
    let outside = scratch("outside");
    fs::create_dir_all(outside.join("ext")).unwrap();
    fs::write(outside.join("ext/package.json"), "{ \"name\": \"ext\", \"type\": \"module\", \"main\": \"index.js\" }").unwrap();
    fs::write(outside.join("ext/index.js"), "export const ext = 'EXT_OUTSIDE'\n").unwrap();
    fs::create_dir_all(app.root().join("node_modules")).unwrap();
    std::os::unix::fs::symlink(outside.join("ext"), app.root().join("node_modules/ext")).unwrap();
    app.deps(&["ext"]).page("import { cart } from '../lib/state/cart.js'\nimport { ext } from 'ext'", "ext, total: cart.total");
    let d = app.check();
    let _ = fs::remove_dir_all(&outside);
    expect_only(&d, "app/package", PAGE, 2);
    assert!(d[0].message.contains("outside the project"), "{}", d[0].message);
}

#[cfg(unix)]
#[test]
fn symlink_inside_project_works() {
    let app = App::new();
    app.deps(&["linked"]).pkg(".pnpm/linked@1.0.0/node_modules/linked", "{ \"name\": \"linked\", \"type\": \"module\", \"main\": \"index.js\" }", &[("index.js", "export const linked = 'PNPM_LINKED'\n")]);
    std::os::unix::fs::symlink(app.root().join("node_modules/.pnpm/linked@1.0.0/node_modules/linked"), app.root().join("node_modules/linked")).unwrap();
    app.page("import { linked } from 'linked'", "linked");
    app.clean();
    assert!(app.prod().code("npm/linked").contains("PNPM_LINKED"));
}

#[test]
fn package_without_client_imports_has_no_npm_output() {
    let app = App::new();
    app.deps(&["rich"]).esm("rich", RICH);
    app.clean();
    let out = app.prod();
    assert!(out.npm().is_empty());
}

#[test]
fn page_module_url_changes_when_package_changes() {
    let app = cached_app();
    let out = app.prod();
    let page = out.url("routes/index");
    app.write("node_modules/rich/index.js", &RICH.replace("RICH_USED", "RICH_USED_CHANGED"));
    app.write("node_modules/rich/package.json", "{ \"name\": \"rich\", \"version\": \"1.0.1\", \"type\": \"module\", \"main\": \"index.js\" }");
    let out = app.prod();
    assert_ne!(out.url("routes/index"), page);
}

