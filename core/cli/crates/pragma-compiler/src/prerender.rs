//! Renders each prebuilt page once at build with the SSR renderer (`.pragma/server/render.js`) in
//! QuickJS, so initial `let`s, props, derived values and the blocks they decide are in the HTML.
use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::{Context, Ctx, Error, Module, Result, Runtime};
use serde_json::Value;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

const RENDER: &str = "server/render.js";
const PAGE_TIME: Duration = Duration::from_secs(2);
const MEMORY: usize = 256 * 1024 * 1024;
const BROWSER_ONLY: &str = r#"
const later = (name) => () => { throw new Error(name + ' is known only when the page opens') }
const RealDate = Date
globalThis.Date = new Proxy(RealDate, {
  apply: later('the date'),
  construct: (target, args) => (args.length ? Reflect.construct(target, args) : later('the date')()),
  get: (target, key) => (key === 'now' ? later('the date') : Reflect.get(target, key)),
})
Math.random = later('Math.random')
for (const proto of [RealDate.prototype, Number.prototype, Array.prototype, Object.prototype]) proto.toLocaleString = later('the locale')
for (const name of ['toLocaleDateString', 'toLocaleTimeString']) RealDate.prototype[name] = later('the locale')
String.prototype.localeCompare = later('the locale')
for (const name of ['toLocaleUpperCase', 'toLocaleLowerCase']) String.prototype[name] = later('the locale')
"#;

pub struct Prerender {
    _rt: Runtime,
    ctx: Context,
    deadline: Rc<Cell<Instant>>,
    render: String,
    pages: Cell<usize>,
}

struct Paths;

impl Resolver for Paths {
    fn resolve<'js>(&mut self, _: &Ctx<'js>, base: &str, name: &str, _: Option<ImportAttributes<'js>>) -> Result<String> {
        if !name.starts_with("./") && !name.starts_with("../") {
            return Err(Error::new_resolving(base, name));
        }
        let mut out = PathBuf::new();
        for c in Path::new(base).parent().unwrap_or(Path::new("")).join(name).components() {
            match c {
                Component::ParentDir => {
                    if !out.pop() {
                        return Err(Error::new_resolving(base, name));
                    }
                }
                Component::Normal(s) => out.push(s),
                _ => {}
            }
        }
        Ok(out.to_string_lossy().replace('\\', "/"))
    }
}

struct Sources(HashMap<String, String>);

impl Loader for Sources {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str, _: Option<ImportAttributes<'js>>) -> Result<Module<'js, rquickjs::module::Declared>> {
        let src = self.0.get(name).ok_or_else(|| Error::new_loading(name))?;
        Module::declare(ctx.clone(), name, src.as_str())
    }
}

fn module_name(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

impl Prerender {
    /// `None` when the app has no `.pragma/server/render.js` (an app older than the renderer).
    pub fn new(root: &Path, framework: &Path, modules: impl Iterator<Item = (PathBuf, String)>) -> Option<Self> {
        let mut sources: HashMap<String, String> = modules.map(|(p, src)| (module_name(&p), src)).collect();
        for file in [RENDER, "contract.js"] {
            let path = framework.join(file);
            let src = std::fs::read_to_string(root.join(&path)).ok()?;
            sources.insert(module_name(&path), src);
        }
        let rt = Runtime::new().ok()?;
        rt.set_memory_limit(MEMORY);
        let deadline = Rc::new(Cell::new(Instant::now() + PAGE_TIME));
        let until = deadline.clone();
        rt.set_interrupt_handler(Some(Box::new(move || Instant::now() > until.get())));
        rt.set_loader(Paths, Sources(sources));
        let ctx = Context::full(&rt).ok()?;
        ctx.with(|ctx| ctx.eval::<(), _>(BROWSER_ONLY)).ok()?;
        Some(Prerender { _rt: rt, ctx, deadline, render: module_name(&framework.join(RENDER)), pages: Cell::new(0) })
    }

    /// The page body (`<body>` content) rendered from `ops`, or `None` when the render fails: the
    /// page then keeps its unrendered HTML and the browser fills it at boot.
    pub fn body(&self, ops: &Value, page: Option<&Path>, components: &[(String, PathBuf)]) -> Option<String> {
        let n = self.pages.get();
        self.pages.set(n + 1);
        let entry = format!("__pragma_prerender_{n}.js");
        let mut src = format!("import {{ prerender }} from './{}'\n", self.render);
        if let Some(m) = page {
            src.push_str(&format!("import * as page from './{}'\n", module_name(m)));
        }
        for (i, (_, m)) in components.iter().enumerate() {
            src.push_str(&format!("import * as c{i} from './{}'\n", module_name(m)));
        }
        let comps: Vec<String> = components.iter().enumerate().map(|(i, (id, _))| format!("{}: c{i}", Value::String(id.clone()))).collect();
        src.push_str(&format!(
            "globalThis.__pragmaBody = prerender({ops}, {{ page: {}, components: {{ {} }} }})\n",
            if page.is_some() { "page" } else { "undefined" },
            comps.join(", ")
        ));
        self.deadline.set(Instant::now() + PAGE_TIME);
        self.ctx.with(|ctx| {
            let done = Module::evaluate(ctx.clone(), entry, src).ok()?;
            done.finish::<()>().ok()?;
            let body: Option<String> = ctx.globals().get("__pragmaBody").ok()?;
            let _ = ctx.globals().remove("__pragmaBody");
            body
        })
    }
}
