use pragma_jslint::script::{Script, ScriptKind, transform};
use std::path::Path;

fn component(src: &str) -> Script {
    transform(Path::new("lib/components/x/x.js"), src, ScriptKind::Component)
}

fn rules(s: &Script) -> Vec<(u32, &'static str)> {
    s.diags.iter().map(|d| (d.line, d.rule)).collect()
}

#[track_caller]
fn has(code: &str, want: &str) {
    assert!(code.contains(want), "missing {want:?} in:\n{code}");
}

/// The output must be a valid module.
#[track_caller]
fn parses(code: &str) {
    let diags = pragma_jslint::lint(Path::new("lib/x.js"), code);
    assert!(!diags.iter().any(|d| d.rule == "js/syntax"), "{diags:#?}\n{code}");
}

#[test]
fn counter_from_the_proposal() {
    let src = "export let start = 0\nexport let step = 1\n\nlet count = start\n\nconst atMinimum = () => count - step < 0\nconst atStart = () => count === start\n\nfunction decrease() { count -= step }\nfunction increase() { count += step }\nfunction reset() { count = start }\n";
    let s = component(src);
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    let code = &s.code;
    assert!(code.starts_with("export default function ($, $pragma) {"), "{code}");
    has(code, "$pragma.prop(\"start\", 0)");
    has(code, "$pragma.prop(\"step\", 1)");
    has(code, "$.count = $.start");
    has(code, "const atMinimum = () => $.count - $.step < 0");
    has(code, "function decrease() { $.count -= $.step }");
    has(code, "function reset() { $.count = $.start }");
    has(code, "return { atMinimum, atStart, decrease, increase, reset }");
    parses(code);
    // line numbers are kept: header on line 1, return after the last line
    assert_eq!(code.lines().position(|l| l.contains("$.count = $.start")), Some(3));
    assert_eq!(s.info.props, ["start", "step"]);
    assert_eq!(s.info.lets, ["count"]);
    assert_eq!(s.info.functions, ["atMinimum", "atStart", "decrease", "increase", "reset"]);
    assert!(s.info.declares("count") && s.info.is_function("reset") && !s.info.declares("total"));
}

#[test]
fn shorthand_destructuring_and_shadowing() {
    let src = "let items = []\nlet draft = ''\nconst snapshot = () => ({ items, draft })\nfunction take(source) { ({ draft } = source); [draft] = [draft] }\nfunction local(items) { return items.length }\nfunction add() { items.push(draft); draft = '' }\n";
    let s = component(src);
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    let code = &s.code;
    has(code, "({ items: $.items, draft: $.draft })");
    has(code, "({ draft: $.draft } = source)");
    has(code, "[$.draft] = [$.draft]");
    has(code, "function local(items) { return items.length }");
    has(code, "$.items.push($.draft); $.draft = ''");
    parses(code);
}

#[test]
fn pragma_imports_become_services_and_other_imports_are_hoisted() {
    let src = "import { formatPrice } from '../../utils/money.js'\nimport { data, log as logger, call } from 'pragma'\n\nlet sending = false\n\nasync function like() {\n  sending = true\n  const { likes } = await call('like')\n  data.product.likes = likes\n  logger.info('liked', { data })\n  sending = false\n}\nconst price = () => formatPrice(data.product.price)\n";
    let s = component(src);
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    let code = &s.code;
    // imports stay on their lines; the factory opens after the last one
    assert!(code.starts_with("import { formatPrice } from '../../utils/money.js'\n; export default function ($, $pragma) {"), "{code}");
    assert!(!code.contains("from 'pragma'"), "{code}");
    has(code, "await $pragma.call('like')");
    has(code, "$pragma.data.product.likes = likes");
    has(code, "$pragma.log.info('liked', { data: $pragma.data })");
    has(code, "formatPrice($pragma.data.product.price)");
    parses(code);
}

#[test]
fn let_without_value_and_several_declarators() {
    let s = component("let a = 1, b, c = a + 1\nexport let label, size = 2\nfunction all() { return [a, b, c, label, size] }\nfunction bump() { c = 0 }\n");
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    has(&s.code, "$.a = 1, $.b = undefined, $.c = $.a + 1");
    has(&s.code, "$pragma.prop(\"label\", undefined), $pragma.prop(\"size\", 2)");
    has(&s.code, "return [$.a, $.b, $.c, $.label, $.size]");
    parses(&s.code);
}

#[test]
fn prop_write_is_an_error() {
    let s = component("export let step = 1\nfunction bump() {\n  step += 1\n}\n");
    assert_eq!(rules(&s), [(3, "js/prop-write")]);
}

#[test]
fn old_api_is_reported_with_a_hint() {
    let s = component("export const state = () => ({ n: 0 })\nexport const actions = {\n  inc(ctx) { ctx.$.n++; ctx.update(() => {}) },\n}\nexport function mount(ctx) {}\n");
    let r = rules(&s);
    assert!(r.iter().all(|(_, rule)| *rule == "js/legacy-api"), "{r:?}");
    assert_eq!(r.iter().map(|x| x.0).collect::<Vec<_>>(), [1, 2, 3, 3, 5]);
    assert!(s.diags[0].message.contains("top-level `let`"), "{}", s.diags[0].message);
    let store = transform(Path::new("lib/state/cart.js"), "import { store } from '/core/state.js'\nexport default store({ items: [] }, { persist: 'cart' })\n", ScriptKind::Store);
    assert_eq!(rules(&store), [(1, "js/legacy-api"), (2, "js/legacy-api")]);
}

#[test]
fn script_structure_errors() {
    let cases = [
        ("var n = 1\n", "`var`"),
        ("let { a, b } = source\n", "one state value per name"),
        ("export const size = 1\n", "only declares props"),
        ("export function go() {}\n", "only declares props"),
        ("export { go }\nfunction go() {}\n", "only declares props"),
        ("export default {}\n", "no `export default`"),
        ("const rows = await fetch('/x')\n", "top-level `await`"),
        ("import { router } from 'pragma'\n", "'pragma' has no `router`"),
        ("import pragma from 'pragma'\n", "import names from 'pragma'"),
        ("const $ = 1\n", "reserved"),
    ];
    for (src, want) in cases {
        let s = component(src);
        assert!(s.diags.iter().any(|d| d.rule == "js/component-script" && d.message.contains(want)), "{src}: {:#?}", s.diags);
        // not rewritten, except that `pragma` imports go (no second error from the module graph)
        let expected: String = src.lines().map(|l| if l.contains("from 'pragma'") { "\n".to_string() } else { format!("{l}\n") }).collect();
        assert_eq!(s.code, expected, "a script with errors is not rewritten");
    }
    let page = transform(Path::new("routes/x.js"), "export let id = ''\n", ScriptKind::Page);
    assert!(page.diags[0].message.contains("pages have no props"), "{:#?}", page.diags);
}

#[test]
fn mutated_const_is_recorded() {
    let s = component("const items = []\nconst totals = { n: 0 }\nconst fixed = [1, 2]\nconst add = () => items.push(1)\nfunction bump() { totals.n++ }\nconst first = () => fixed[0]\n");
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    let names: Vec<&str> = s.info.mutated_consts.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, ["items", "totals"]);
    assert_eq!((s.info.mutated_consts[1].1, s.info.mutated_consts[1].2), (2, 7));
    assert_eq!(s.info.consts, ["items", "totals", "fixed"]);
}

#[test]
fn store_with_persist() {
    let src = "import { log } from 'pragma'\n\nlet items = []\nlet open = false\nexport const persist = ['items']\n\nconst count = () => items.length\nfunction add(product) {\n  items.push(product)\n  log.info('added', { 'product.id': product.id })\n}\n";
    let s = transform(Path::new("lib/state/cart.js"), src, ScriptKind::Store);
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    assert!(s.code.starts_with("; export const persist = [\"items\"]; export default function ($, $pragma) {"), "{}", s.code);
    has(&s.code, "$.items.push(product)");
    has(&s.code, "return { count, add }");
    assert_eq!(s.info.persist, ["items"]);
    assert_eq!(s.info.lets, ["items", "open"]);
    parses(&s.code);
    let bad = transform(Path::new("lib/state/cart.js"), "let items = []\nexport const persist = ['missing']\n", ScriptKind::Store);
    assert!(bad.diags[0].message.contains("\"missing\""), "{:#?}", bad.diags);
    let props = transform(Path::new("lib/state/cart.js"), "export let x = 1\n", ScriptKind::Store);
    assert_eq!(rules(&props), [(1, "js/component-script")]);
}

#[test]
fn data_read_at_the_top_level() {
    let page = |src: &str| transform(Path::new("routes/x.js"), src, ScriptKind::Page);
    let s = page("import { data } from 'pragma'\nlet products = data.products\nconst total = () => data.total\nfunction like() { data.likes += 1 }\n");
    assert_eq!(rules(&s), [(2, "js/top-level-data")]);
    assert!(s.diags[0].message.contains("const products = () => data.products"), "{}", s.diags[0].message);
    let ok = page("import { data, params, query } from 'pragma'\nlet id = params.id\nlet tab = query.tab\nconst rows = () => data.rows\n");
    assert!(ok.diags.is_empty(), "{:#?}", ok.diags);
}

#[test]
fn stale_derived_values() {
    let s = component("export let step = 1\nlet count = 0\nlet doubled = count * 2\nconst title = 'Count ' + count\nconst big = step * 10\nlet start = step\nlet max = 10\nconst limit = max\nconst label = () => 'n ' + count\nlet seed = count\nfunction reset() { seed = 0 }\nlet names = [1].map(n => n + count)\n");
    let got: Vec<u32> = s.diags.iter().filter(|d| d.rule == "js/stale-derived").map(|d| d.line).collect();
    // doubled, title, big, start, limit (max is a let: state); not label (a function), seed
    // (assigned later) nor names (reads count only inside a callback: not flagged)
    assert_eq!(got, [3, 4, 5, 6, 8], "{:#?}", s.diags);
    let doubled = &s.diags[0].message;
    assert!(doubled.contains("`doubled` reads `count` once") && doubled.contains("const doubled = () => count * 2"), "{doubled}");
}

#[test]
fn derived_function_used_without_call() {
    let s = component("let items = []\nconst empty = () => items.length === 0\nconst total = () => items.length\nfunction tick() {}\nfunction check() {\n  if (empty) return\n  const label = `${total} items`\n  const none = !empty\n  const many = total > 1 && empty\n  setInterval(tick, 1000)\n  return [items.filter(Boolean), typeof total, empty() ? 1 : 2]\n}\n");
    let got: Vec<(u32, String)> = s.diags.iter().map(|d| (d.line, d.message.split('`').nth(1).unwrap_or("").to_string())).collect();
    assert!(s.diags.iter().all(|d| d.rule == "js/derived-call"), "{:#?}", s.diags);
    assert_eq!(got, [(6, "empty".into()), (7, "total".into()), (8, "empty".into()), (9, "total".into()), (9, "empty".into())], "{:#?}", s.diags);
    assert!(s.diags[0].message.contains("call it in JS: `empty()`"));
}

#[test]
fn dollar_names_in_a_script() {
    let s = component("let count = 0\nfunction add() { $count += 1 }\nfunction other() { return $unknown }\nconst $ok = 1\n");
    let dollar: Vec<(u32, &str)> = s.diags.iter().filter(|d| d.rule == "js/dollar-in-script").map(|d| (d.line, d.message.as_str())).collect();
    assert_eq!(dollar.len(), 2, "{:#?}", s.diags);
    assert!(dollar[0].0 == 2 && dollar[0].1.contains("in <script> write `count`"), "{dollar:?}");
    assert_eq!(dollar[1].0, 3);
}

#[test]
fn several_errors_in_one_pass() {
    let s = component("import { data } from 'pragma'\nexport let step = 1\nlet products = data.list\nlet twice = step * 2\nfunction bump() { step += 1; if (show) return }\nconst show = () => true\nvar old = 1\n");
    let mut found: Vec<&str> = s.diags.iter().map(|d| d.rule).collect();
    found.sort();
    found.dedup();
    assert_eq!(found, ["js/component-script", "js/derived-call", "js/prop-write", "js/stale-derived", "js/top-level-data"], "{:#?}", s.diags);
    // the names stay known, so the HTML checks still run (only the old API hides them)
    assert!(!s.info.incomplete && s.info.declares("old") && s.info.declares("products"));
    assert!(component("export const state = () => ({})\n").info.incomplete);
}

#[test]
fn import_after_code_is_hoisted() {
    let s = component("let count = 0\nimport { format } from './fmt.js'\nconst label = () => format(count)\n");
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    assert!(s.code.starts_with("import { format } from './fmt.js'; export default function ($, $pragma) {"), "{}", s.code);
    parses(&s.code);
}

#[test]
fn mount_and_timers_reassign_state() {
    let src = "let now = new Date()\nfunction mount() {\n  const timer = setInterval(() => { now = new Date() }, 1000)\n  return () => clearInterval(timer)\n}\n";
    let s = component(src);
    assert!(s.diags.is_empty(), "{:#?}", s.diags);
    has(&s.code, "setInterval(() => { $.now = new Date() }, 1000)");
    has(&s.code, "return { mount }");
    parses(&s.code);
}
