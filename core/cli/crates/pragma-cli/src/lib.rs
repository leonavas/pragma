pub mod dev;
pub mod new;
pub mod node;
pub mod report;
pub mod self_update;
pub mod server;
pub mod upgrade;

use report::{dim, print_diagnostics, print_summary, red};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

const HELP: &str = "pragma — HTML, CSS and JS without a framework

usage:
  pragma new <dir>                              create an app in <dir> (warns and keeps existing files if not empty)
  pragma dev [--port 5180] [--host 127.0.0.1]   dev server with live reload
             [--cf]                             (--cf: .server.js gets the root wrangler.jsonc bindings)
  pragma build [--allow-outdated-core]          build dist/ for production (stops if .pragma/ is outdated)
  pragma check [--strict]                       check HTML, CSS and JS (--strict: warnings become errors)
  pragma serve [--port 3000] [--host 127.0.0.1] serve dist/ as in production
  pragma upgrade                                update .pragma/ (runtime, docs, base.css) to this version
  pragma self-update [version]                  update the pragma CLI binary from GitHub Releases

options:
  -h, --help          show this help
  -v, -V, --version   show the version";

pub fn run(args: Vec<String>) -> i32 {
    let Some((command, rest)) = args.split_first() else {
        println!("{HELP}");
        return 0;
    };
    if rest.iter().any(|a| a == "-h" || a == "--help") {
        println!("{HELP}");
        return 0;
    }
    let result = match command.as_str() {
        "-h" | "--help" | "help" => {
            println!("{HELP}");
            Ok(0)
        }
        "-v" | "-V" | "--version" => {
            println!("pragma {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        "new" => cmd_new(rest),
        "dev" => {
            let cf = rest.iter().any(|a| a == "--cf");
            let rest: Vec<String> = rest.iter().filter(|a| *a != "--cf").cloned().collect();
            cmd_dev(&rest, cf)
        }
        "build" => {
            let allow = rest.iter().any(|a| a == "--allow-outdated-core");
            let rest: Vec<String> = rest.iter().filter(|a| *a != "--allow-outdated-core").cloned().collect();
            parse(&rest, &[]).and_then(|a| no_positional(&a)).and_then(|_| cmd_build(allow))
        }
        "check" => {
            let strict = rest.iter().any(|a| a == "--strict");
            let rest: Vec<String> = rest.iter().filter(|a| *a != "--strict").cloned().collect();
            parse(&rest, &[]).and_then(|a| no_positional(&a)).and_then(|_| cmd_check(strict))
        }
        "serve" => cmd_serve(rest),
        "upgrade" => parse(rest, &[]).and_then(|a| no_positional(&a)).and_then(|_| cmd_upgrade()),
        "self-update" => cmd_self_update(rest),
        other => Err(format!("unknown command '{other}'; see `pragma --help`")),
    };
    result.unwrap_or_else(|message| {
        eprintln!("{} {message}", red("error:"));
        1
    })
}

struct Args {
    positional: Vec<String>,
    options: HashMap<String, String>,
}

impl Args {
    fn port(&self, default: u16) -> Result<u16, String> {
        self.options.get("port").map_or(Ok(default), |p| p.parse().map_err(|_| format!("invalid port: '{p}'")))
    }

    fn host(&self) -> String {
        self.options.get("host").cloned().unwrap_or_else(|| "127.0.0.1".into())
    }
}

fn parse(args: &[String], allowed: &[&str]) -> Result<Args, String> {
    let mut out = Args { positional: Vec::new(), options: HashMap::new() };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let Some(option) = arg.strip_prefix("--") else {
            if arg.starts_with('-') && arg.len() > 1 {
                return Err(format!("unknown option '{arg}'"));
            }
            out.positional.push(arg.clone());
            continue;
        };
        let (name, value) = match option.split_once('=') {
            Some((n, v)) => (n.to_owned(), v.to_owned()),
            None => (option.to_owned(), it.next().cloned().ok_or_else(|| format!("missing value for '--{option}'"))?),
        };
        if !allowed.contains(&name.as_str()) {
            return Err(format!("unknown option '--{name}'"));
        }
        out.options.insert(name, value);
    }
    Ok(out)
}

fn app_root() -> Result<PathBuf, String> {
    let root = std::env::current_dir().map_err(|e| format!("cannot access current directory: {e}"))?;
    if !root.join("pragma.json").is_file() {
        return Err("pragma.json not found; run the command from the root of a pragma app".into());
    }
    Ok(root)
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Runtime::new().map_err(|e| format!("failed to start the async runtime: {e}"))
}

fn cmd_new(args: &[String]) -> Result<i32, String> {
    let args = parse(args, &[])?;
    let [dir] = args.positional.as_slice() else {
        return Err("usage: pragma new <dir>".into());
    };
    let created = new::create(Path::new(dir))?;
    if created.non_empty {
        let kept = if created.skipped > 0 { format!(" ({} kept)", created.skipped) } else { String::new() };
        eprintln!("{} directory '{dir}' is not empty; existing files were not overwritten{kept}", report::paint("33", "warning:"));
    }
    if dir == "." {
        println!("app created in {dir} ({} files)\n\n  pragma dev", created.written);
    } else {
        println!("app created in {dir} ({} files)\n\n  cd {dir}\n  pragma dev", created.written);
    }
    Ok(0)
}

fn cmd_dev(args: &[String], cf: bool) -> Result<i32, String> {
    let args = parse(args, &["port", "host"])?;
    no_positional(&args)?;
    let port = args.port(5180)?;
    let root = app_root()?;
    if let Some(warning) = upgrade::outdated(&root) {
        print_diagnostics(&root, &[warning]);
        eprintln!("{} pages may fail to render with an old .pragma/ (e.g. SSR \"unknown op\"); run `pragma upgrade`\n", report::paint("33", "warning:"));
    }
    let cf_config = if cf { Some(node::cloudflare_config(&root)?) } else { None };
    Ok(runtime()?.block_on(dev::run_with(root, &args.host(), port, cf_config)))
}

fn cmd_build(allow_outdated_core: bool) -> Result<i32, String> {
    let root = app_root()?;
    match upgrade::build_gate(&root, allow_outdated_core) {
        Ok(warning) => print_diagnostics(&root, warning.as_slice()),
        Err((error, message)) => {
            print_diagnostics(&root, &[error]);
            return Err(message);
        }
    }
    let started = Instant::now();
    let opts = pragma_compiler::BuildOptions { root: root.clone(), out_dir: root.join("dist"), mode: pragma_compiler::Mode::Prod };
    match pragma_compiler::build(&opts) {
        Ok(report) => {
            print_diagnostics(&root, &report.warnings);
            print_summary(&report);
            println!("{}", dim(&format!("dist/ built in {} ms", started.elapsed().as_millis())));
            Ok(0)
        }
        Err(diagnostics) => {
            print_diagnostics(&root, &diagnostics);
            Ok(1)
        }
    }
}

fn cmd_check(strict: bool) -> Result<i32, String> {
    let root = app_root()?;
    let mut diagnostics: Vec<_> = upgrade::outdated(&root).into_iter().collect();
    diagnostics.extend(pragma_compiler::check(&root));
    let status = check_status(&mut diagnostics, strict);
    print_diagnostics(&root, &diagnostics);
    if diagnostics.is_empty() {
        println!("no problems found");
    }
    Ok(status)
}

fn cmd_upgrade() -> Result<i32, String> {
    let root = app_root()?;
    let version = env!("CARGO_PKG_VERSION");
    let prev = upgrade::installed_version(&root);
    let changes = upgrade::upgrade(&root)?;
    if changes.is_empty() {
        println!(".pragma/ is already at pragma {version}");
        return Ok(0);
    }
    for (label, paths) in [("updated", &changes.updated), ("added", &changes.added), ("removed", &changes.removed)] {
        for path in paths {
            println!("  {label:<8} {path}");
        }
    }
    match prev {
        Some(from) if from != version => println!(
            ".pragma/ upgraded from {from} to {version} ({} file(s) changed); files outside .pragma/ were not touched",
            changes.len()
        ),
        _ => println!(
            ".pragma/ upgraded to pragma {version} ({} file(s) changed); files outside .pragma/ were not touched",
            changes.len()
        ),
    }
    Ok(0)
}

fn cmd_self_update(args: &[String]) -> Result<i32, String> {
    let args = parse(args, &[])?;
    let requested = match args.positional.as_slice() {
        [] => None,
        [v] => Some(v.as_str()),
        _ => return Err("usage: pragma self-update [version]".into()),
    };
    self_update::run(requested)?;
    Ok(0)
}

/// Warnings do not fail the check, except with `--strict`.
fn check_status(diagnostics: &mut [pragma_diag::Diagnostic], strict: bool) -> i32 {
    if strict {
        for d in diagnostics.iter_mut() {
            d.severity = pragma_diag::Severity::Error;
        }
    }
    diagnostics.iter().any(|d| d.severity == pragma_diag::Severity::Error) as i32
}

/// Ctrl+C or SIGTERM: both must stop the child node process.
pub async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::check_status;
    use pragma_diag::Diagnostic;

    #[test]
    fn strict_turns_warnings_into_errors() {
        let mut d = vec![Diagnostic::warning("a.html", 1, 1, "a11y/img-alt", "x")];
        assert_eq!(check_status(&mut d, false), 0);
        assert_eq!(check_status(&mut d, true), 1);
        assert_eq!(check_status(&mut [], true), 0);
    }
}

fn cmd_serve(args: &[String]) -> Result<i32, String> {
    let args = parse(args, &["port", "host"])?;
    no_positional(&args)?;
    let port = args.port(3000)?;
    let host = args.host();
    let root = std::env::current_dir().map_err(|e| format!("cannot access current directory: {e}"))?;
    let client = root.join("dist/client");
    if !client.is_dir() {
        return Err("dist/client does not exist; run `pragma build` first".into());
    }
    runtime()?.block_on(async move {
        let started = Instant::now();
        let site = tokio::task::spawn_blocking(move || server::Site::new(client, server::Mode::Serve)).await.map_err(|e| e.to_string())?;
        println!("{}", dim(&format!("files compressed in {} ms", started.elapsed().as_millis())));
        let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await.map_err(|e| format!("cannot listen on {host}:{port}: {e}"))?;
        let mut node = node::Node::new(root.join("dist/server/node.mjs"), root.clone(), false);
        node.sync(&site).await;
        tokio::spawn(server::serve(listener, site));
        println!("pragma serve at http://{}:{port}", dev::display_host(&host));
        shutdown().await;
        node.stop().await;
        Ok(0)
    })
}

fn no_positional(args: &Args) -> Result<(), String> {
    match args.positional.first() {
        Some(extra) => Err(format!("unexpected argument '{extra}'")),
        None => Ok(()),
    }
}
