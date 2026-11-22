use crate::node::Node;
use crate::report::{self, diagnostics_json, print_diagnostics};
use crate::server::{self, Event, Mode, Site};
use notify::{EventKind, RecursiveMode, Watcher};
use pragma_compiler::{BuildOptions, BuildReport};
use pragma_diag::{Diagnostic, Severity};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

const DEBOUNCE: Duration = Duration::from_millis(30);
const IGNORED_DIRS: [&str; 5] = [".pragma", "dist", "node_modules", ".git", ".wrangler"];

pub async fn run(root: PathBuf, host: &str, port: u16) -> i32 {
    run_with(root, host, port, None).await
}

/// `cf_config`: `pragma dev --cf`, Cloudflare bindings from that wrangler config.
pub async fn run_with(root: PathBuf, host: &str, port: u16, cf_config: Option<PathBuf>) -> i32 {
    let out = root.join(".pragma/dev");
    let site = Site::new(out.join("client"), Mode::Dev);
    let mut failed = !rebuild(&root, &out, &site).await;
    let listener = match TcpListener::bind((host, port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{} cannot listen on {host}:{port}: {e}", report::err().red("error"));
            return 1;
        }
    };
    let mut node = Node::new(out.join("server/node.mjs"), root.clone(), true);
    if let Some(config) = cf_config {
        println!("{}", report::out().dim(&format!("Cloudflare bindings from {}", config.file_name().unwrap_or_default().to_string_lossy())));
        node = node.with_cloudflare(config);
    }
    node.sync(&site).await;
    tokio::spawn(server::serve(listener, site.clone()));
    println!("pragma dev at http://{}:{port}", display_host(host));

    let ctrl_c = crate::shutdown();
    tokio::pin!(ctrl_c);
    let (tx, mut changes) = mpsc::unbounded_channel();
    let _watcher = match watch(&root, tx) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{} cannot watch '{}': {e}", report::err().red("error"), root.display());
            node.stop().await;
            return 1;
        }
    };
    loop {
        tokio::select! {
            batch = changes.recv() => {
                let Some(batch) = batch else { break };
                let css_only = batch.iter().all(|p| p.extension().is_some_and(|e| e == "css"));
                let names: Vec<_> = batch.iter().map(|p| p.strip_prefix(&root).unwrap_or(p).display().to_string()).collect();
                println!("{}", report::out().dim(&format!("changed: {}", names.join(", "))));
                let ok = rebuild(&root, &out, &site).await;
                // the build writes its outputs even with errors elsewhere: node must serve the same
                // build as the client files, or a new page's `load` answers 404 (BUG-S-8)
                node.sync(&site).await;
                if !ok {
                    failed = true;
                    continue;
                }
                // after an error the page may be stale beyond the CSS
                if css_only && !failed {
                    site.send(Event::Css(newest_css(&out.join("client/assets"))));
                } else {
                    site.send(Event::Reload);
                }
                failed = false;
            }
            _ = &mut ctrl_c => break,
        }
    }
    node.stop().await;
    0
}

pub fn display_host(host: &str) -> &str {
    if host == "0.0.0.0" || host == "::" { "localhost" } else { host }
}

async fn rebuild(root: &Path, out: &Path, site: &Site) -> bool {
    let started = Instant::now();
    let opts = BuildOptions { root: root.to_path_buf(), out_dir: out.to_path_buf(), mode: pragma_compiler::Mode::Dev };
    let result = tokio::task::spawn_blocking(move || pragma_compiler::build(&opts)).await;
    let result: Result<BuildReport, Vec<Diagnostic>> = match result {
        Ok(r) => r,
        Err(e) => Err(vec![Diagnostic::error(root, 0, 0, "internal", format!("compiler crashed: {e}"))]),
    };
    // routes follow the files on disk, also after a build with errors
    site.reload_manifest();
    match result {
        Ok(report) => {
            print_diagnostics(root, &report.warnings);
            println!("{}", report::out().dim(&format!("compiled in {} ms", started.elapsed().as_millis())));
            true
        }
        Err(diagnostics) => {
            print_diagnostics(root, &diagnostics);
            let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == Severity::Error).cloned().collect();
            site.send(Event::Error(diagnostics_json(root, if errors.is_empty() { &diagnostics } else { &errors })));
            false
        }
    }
}

fn newest_css(assets: &Path) -> String {
    std::fs::read_dir(assets)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "css"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.file_name())))
        .max()
        .map(|(_, name)| format!("/assets/{}", name.to_string_lossy()))
        .unwrap_or_default()
}

fn watch(root: &Path, tx: mpsc::UnboundedSender<Vec<PathBuf>>) -> notify::Result<notify::RecommendedWatcher> {
    let (events_tx, events) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(events_tx)?;
    watcher.watch(root, RecursiveMode::Recursive)?;
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let relevant = |event: notify::Result<notify::Event>, batch: &mut BTreeSet<PathBuf>| {
            let Ok(event) = event else { return };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            batch.extend(event.paths.into_iter().filter(|p| !ignored(&root, p)));
        };
        while let Ok(first) = events.recv() {
            let mut batch = BTreeSet::new();
            relevant(first, &mut batch);
            loop {
                match events.recv_timeout(DEBOUNCE) {
                    Ok(event) => relevant(event, &mut batch),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            if !batch.is_empty() && tx.send(batch.into_iter().collect()).is_err() {
                return;
            }
        }
    });
    Ok(watcher)
}

fn ignored(root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    if rel.components().next().is_some_and(|c| IGNORED_DIRS.iter().any(|d| c.as_os_str() == *d)) {
        return true;
    }
    let name = rel.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    // editor temp files
    name.ends_with('~') || name.ends_with(".swp") || name.ends_with(".swx") || name.starts_with(".#") || name == "4913"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_output_and_editor_files() {
        let root = Path::new("/app");
        assert!(ignored(root, Path::new("/app/.pragma/dev/client/index.html")));
        assert!(ignored(root, Path::new("/app/dist/client/x.js")));
        assert!(ignored(root, Path::new("/app/routes/.index.html.swp")));
        assert!(!ignored(root, Path::new("/app/routes/index.html")));
        assert!(!ignored(root, Path::new("/app/lib/theme.css")));
    }
}
