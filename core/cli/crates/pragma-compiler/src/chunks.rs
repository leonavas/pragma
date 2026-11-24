use crate::Module;
use pragma_diag::Diagnostic;
use rolldown::{
    Bundler, BundlerOptions, ChunkFilenamesOutputOption, CodeSplittingMode, HashCharacters, InputItem, IsExternal, LegalComments, LogLevel,
    MakeAbsoluteExternalsRelative, ManualCodeSplittingOptions, MatchGroup, MatchGroupName, OutputFormat, Platform, PreserveEntrySignatures, RawMinifyOptions,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const TINY_GROUP_BYTES: usize = 4096;

/// A route of the bundle: its entry module (`None`: a static route, no JS).
pub struct Route {
    pub name: String,
    pub entry: Option<crate::kernel::Entry>,
}

pub struct Chunks {
    pub files: Vec<(String, String)>,
    pub imports: HashMap<String, Vec<String>>,
    /// URL of each route's entry chunk.
    pub routes: Vec<Option<String>>,
}

impl Chunks {
    /// Chunks a page loads before it runs, in load order (static imports only).
    pub fn closure(&self, starts: &[&str]) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut queue: VecDeque<String> = starts.iter().map(|s| s.to_string()).collect();
        while let Some(url) = queue.pop_front() {
            if !seen.insert(url.clone()) {
                continue;
            }
            queue.extend(self.imports.get(&url).into_iter().flatten().cloned());
            order.push(url);
        }
        order
    }
}

fn url_of(file: &str) -> String {
    format!("/assets/{file}")
}

fn spec(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"")
}

/// App modules used by 2+ routes, grouped by the exact set of routes that use them. A group under
/// `TINY_GROUP_BYTES` joins the closest group used by a wider set of routes, so no page loads a
/// chunk it would not load anyway; wider groups sort first (the bundler gives a shared dependency
/// to the first group that claims it).
fn shared_groups(mods: &BTreeMap<PathBuf, Module>, routes: &[Route], core: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let mut used_by: BTreeMap<PathBuf, BTreeSet<usize>> = BTreeMap::new();
    for (i, r) in routes.iter().enumerate() {
        let Some(entry) = &r.entry else { continue };
        for m in crate::closure(mods, &entry.paths(), false) {
            if !m.starts_with("node_modules") && !core.contains(&m) {
                used_by.entry(m).or_default().insert(i);
            }
        }
    }
    let mut groups: BTreeMap<BTreeSet<usize>, (Vec<PathBuf>, usize)> = BTreeMap::new();
    for (m, set) in used_by.into_iter().filter(|(_, s)| s.len() > 1) {
        let g = groups.entry(set).or_default();
        g.1 += mods[&m].src.len();
        g.0.push(m);
    }
    let mut order: Vec<BTreeSet<usize>> = groups.keys().cloned().collect();
    order.sort_by_key(|k| (k.len(), groups[k].1));
    for set in order {
        if groups[&set].1 >= TINY_GROUP_BYTES {
            continue;
        }
        let wider = groups.iter().filter(|(t, _)| t.len() > set.len() && set.is_subset(t)).min_by_key(|(t, g)| (t.len(), usize::MAX - g.1)).map(|(t, _)| t.clone());
        if let Some(wider) = wider {
            let (moved, size) = groups.remove(&set).unwrap_or_default();
            let g = groups.get_mut(&wider).expect("wider group");
            g.0.extend(moved);
            g.1 += size;
        }
    }
    let mut ordered: Vec<(BTreeSet<usize>, Vec<PathBuf>)> = groups.into_iter().map(|(k, (m, _))| (k, m)).collect();
    ordered.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    ordered.into_iter().enumerate().flat_map(|(i, (_, ms))| ms.into_iter().map(move |m| (m, format!("shared{i:03}")))).collect()
}

/// Bundles app and core modules into one entry chunk per reactive route, shared chunks and the
/// kernel. npm modules stay as they are (`external`, imported by their hashed URL).
pub fn bundle(
    dir: &Path,
    mods: &BTreeMap<PathBuf, Module>,
    urls: &HashMap<PathBuf, String>,
    kernel: &Path,
    routes: &[Route],
    diags: &mut Vec<Diagnostic>,
) -> Option<Chunks> {
    let _ = fs::remove_dir_all(dir);
    if routes.iter().all(|r| r.entry.is_none()) {
        return Some(Chunks { files: Vec::new(), imports: HashMap::new(), routes: routes.iter().map(|_| None).collect() });
    }
    let src_dir = dir.join("src");
    let mut externals: BTreeSet<String> = BTreeSet::new();
    for (path, m) in mods.iter().filter(|(p, _)| !p.starts_with("node_modules")) {
        let specs: Vec<crate::js::Import> = m.imports.iter().map(|(i, _)| i.clone()).collect();
        let mut targets = m.imports.iter().map(|(imp, t)| match t {
            Some(t) if t.starts_with("node_modules") => {
                externals.insert(urls[t].clone());
                Some(urls[t].clone())
            }
            Some(t) => Some(spec(&src_dir.join(t))),
            None => {
                externals.insert(imp.spec.clone());
                None
            }
        });
        let code = crate::js::rewrite(&m.src, &specs, |_| targets.next().flatten());
        let file = src_dir.join(path);
        let _ = fs::create_dir_all(file.parent().unwrap_or(&src_dir));
        if let Err(e) = fs::write(&file, code) {
            diags.push(Diagnostic::error(path, 1, 1, "js/syntax", format!("cannot prepare the JS bundle in {}: {e}", dir.display())));
            return None;
        }
    }
    let entry_dir = dir.join("entries");
    let _ = fs::create_dir_all(&entry_dir);
    let mut input = Vec::new();
    let mut route_entries: Vec<Option<String>> = Vec::new();
    for (i, r) in routes.iter().enumerate() {
        let Some(entry) = &r.entry else {
            route_entries.push(None);
            continue;
        };
        let file = entry_dir.join(format!("{i}.js"));
        let _ = fs::write(&file, entry.source(|m| src_dir.join(m).to_string_lossy().into_owned()));
        let import = file.to_string_lossy().into_owned();
        input.push(InputItem { name: Some(r.name.clone()), import: import.clone() });
        route_entries.push(Some(import));
    }
    let core = crate::closure(mods, &[kernel.to_path_buf()], false);
    let group_of: HashMap<String, String> = shared_groups(mods, routes, &core).into_iter().map(|(m, g)| (src_dir.join(m).to_string_lossy().into_owned(), g)).collect();
    let name_of: Arc<_> = Arc::new(move |ids: Vec<String>, _: &rolldown::ChunkingContext| {
        let names: Vec<Option<String>> = ids.iter().map(|id| group_of.get(id).cloned()).collect();
        Box::pin(async move { Ok::<_, anyhow::Error>(names) }) as std::pin::Pin<Box<dyn Future<Output = anyhow::Result<Vec<Option<String>>>> + Send>>
    });
    let groups = MatchGroup { name: MatchGroupName::Dynamic(name_of), ..MatchGroup::default() };
    let options = BundlerOptions {
        input: Some(input),
        cwd: Some(src_dir.clone()),
        platform: Some(Platform::Browser),
        format: Some(OutputFormat::Esm),
        external: Some(IsExternal::from(externals.into_iter().collect::<Vec<_>>())),
        make_absolute_externals_relative: Some(MakeAbsoluteExternalsRelative::Bool(false)),
        entry_filenames: Some(ChunkFilenamesOutputOption::String("[name]-[hash].js".into())),
        chunk_filenames: Some(ChunkFilenamesOutputOption::String("chunks/[name]-[hash].js".into())),
        hash_characters: Some(HashCharacters::Hex),
        preserve_entry_signatures: Some(PreserveEntrySignatures::AllowExtension),
        code_splitting: Some(CodeSplittingMode::Advanced(ManualCodeSplittingOptions { groups: Some(vec![groups]), ..ManualCodeSplittingOptions::default() })),
        minify: Some(RawMinifyOptions::Bool(true)),
        // Only `pragma dev` sets this flag, so a production bundle drops the dev-only checks.
        define: Some(["self.__pragmaDev", "globalThis.__pragmaDev"].map(|k| (k.to_string(), "false".to_string())).into_iter().collect()),
        legal_comments: Some(LegalComments::None),
        log_level: Some(LogLevel::Silent),
        ..BundlerOptions::default()
    };
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
            rt.block_on(async {
                let mut bundler = Bundler::new(options).map_err(|e| e.to_string())?;
                let out = bundler.generate().await;
                let _ = bundler.close().await;
                Ok::<_, String>(out)
            })
        })
        .join()
        .unwrap_or_else(|_| Err("the JS bundler crashed".into()))
    });
    let out = match result {
        Ok(Ok(out)) => out,
        Ok(Err(errs)) => {
            for e in errs.into_vec() {
                let id = e.id().unwrap_or_default().to_string();
                let file = id.strip_prefix(&format!("{}/", src_dir.display())).map(PathBuf::from).unwrap_or_else(|| kernel.to_path_buf());
                diags.push(Diagnostic::error(file, 1, 1, "js/syntax", format!("cannot bundle: {}", e.to_string().trim())));
            }
            return None;
        }
        Err(e) => {
            diags.push(Diagnostic::error(kernel, 1, 1, "js/syntax", e));
            return None;
        }
    };
    let mut chunks = Chunks { files: Vec::new(), imports: HashMap::new(), routes: Vec::new() };
    let mut entry_urls: HashMap<String, String> = HashMap::new();
    for o in &out.assets {
        let rolldown_common::Output::Chunk(c) = o else { continue };
        let url = url_of(&c.filename);
        if c.is_entry
            && let Some(facade) = &c.facade_module_id
        {
            entry_urls.insert(facade.to_string(), url.clone());
        }
        let imports = c.imports.iter().map(|i| if i.starts_with('/') || i.contains("://") { i.to_string() } else { url_of(i) }).collect();
        chunks.imports.insert(url, imports);
        chunks.files.push((c.filename.to_string(), c.code.clone()));
    }
    chunks.routes = route_entries.into_iter().map(|e| e.and_then(|e| entry_urls.get(&e).cloned())).collect();
    Some(chunks)
}
