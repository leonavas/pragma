pub mod dev;
pub mod new;
pub mod node;
pub mod report;
pub mod server;

use report::{dim, print_diagnostics, print_summary, red};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

const HELP: &str = "pragma — framework de HTML, CSS e JS sem framework

uso:
  pragma new <dir>                              cria um app em <dir> (vazio ou inexistente)
  pragma dev [--port 5180] [--host 127.0.0.1]   servidor de desenvolvimento com live reload
  pragma build                                  gera dist/ para produção
  pragma check [--strict]                       valida HTML, CSS, JS e orçamentos (--strict: avisos viram erro)
  pragma serve [--port 3000] [--host 127.0.0.1] serve dist/ como em produção

opções:
  -h, --help      mostra esta ajuda
  -V, --version   mostra a versão";

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
        "-V" | "--version" => {
            println!("pragma {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
        "new" => cmd_new(rest),
        "dev" => cmd_dev(rest),
        "build" => parse(rest, &[]).and_then(|_| cmd_build()),
        "check" => {
            let strict = rest.iter().any(|a| a == "--strict");
            let rest: Vec<String> = rest.iter().filter(|a| *a != "--strict").cloned().collect();
            parse(&rest, &[]).and_then(|a| no_positional(&a)).and_then(|_| cmd_check(strict))
        }
        "serve" => cmd_serve(rest),
        other => Err(format!("comando desconhecido '{other}'; veja `pragma --help`")),
    };
    result.unwrap_or_else(|message| {
        eprintln!("{} {message}", red("erro:"));
        1
    })
}

struct Args {
    positional: Vec<String>,
    options: HashMap<String, String>,
}

impl Args {
    fn port(&self, default: u16) -> Result<u16, String> {
        self.options.get("port").map_or(Ok(default), |p| p.parse().map_err(|_| format!("porta inválida: '{p}'")))
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
                return Err(format!("opção desconhecida '{arg}'"));
            }
            out.positional.push(arg.clone());
            continue;
        };
        let (name, value) = match option.split_once('=') {
            Some((n, v)) => (n.to_owned(), v.to_owned()),
            None => (option.to_owned(), it.next().cloned().ok_or_else(|| format!("falta o valor de '--{option}'"))?),
        };
        if !allowed.contains(&name.as_str()) {
            return Err(format!("opção desconhecida '--{name}'"));
        }
        out.options.insert(name, value);
    }
    Ok(out)
}

fn app_root() -> Result<PathBuf, String> {
    let root = std::env::current_dir().map_err(|e| format!("diretório atual inacessível: {e}"))?;
    if !root.join("pragma.json").is_file() {
        return Err("pragma.json não encontrado; rode o comando na raiz de um app pragma".into());
    }
    Ok(root)
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Runtime::new().map_err(|e| format!("falha ao iniciar o runtime: {e}"))
}

fn cmd_new(args: &[String]) -> Result<i32, String> {
    let args = parse(args, &[])?;
    let [dir] = args.positional.as_slice() else {
        return Err("uso: pragma new <dir>".into());
    };
    let count = new::create(Path::new(dir))?;
    println!("app criado em {dir} ({count} arquivos)\n\n  cd {dir}\n  pragma dev");
    Ok(0)
}

fn cmd_dev(args: &[String]) -> Result<i32, String> {
    let args = parse(args, &["port", "host"])?;
    no_positional(&args)?;
    let port = args.port(5180)?;
    let root = app_root()?;
    Ok(runtime()?.block_on(dev::run(root, &args.host(), port)))
}

fn cmd_build() -> Result<i32, String> {
    let root = app_root()?;
    let started = Instant::now();
    let opts = pragma_compiler::BuildOptions { root: root.clone(), out_dir: root.join("dist"), mode: pragma_compiler::Mode::Prod };
    match pragma_compiler::build(&opts) {
        Ok(report) => {
            print_diagnostics(&root, &report.warnings);
            print_summary(&report);
            println!("{}", dim(&format!("dist/ gerado em {} ms", started.elapsed().as_millis())));
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
    let mut diagnostics = pragma_compiler::check(&root);
    let status = check_status(&mut diagnostics, strict);
    print_diagnostics(&root, &diagnostics);
    if diagnostics.is_empty() {
        println!("nenhum problema encontrado");
    }
    Ok(status)
}

/// Avisos não falham o check, exceto com `--strict`.
fn check_status(diagnostics: &mut [pragma_diag::Diagnostic], strict: bool) -> i32 {
    if strict {
        for d in diagnostics.iter_mut() {
            d.severity = pragma_diag::Severity::Error;
        }
    }
    diagnostics.iter().any(|d| d.severity == pragma_diag::Severity::Error) as i32
}

/// Ctrl+C ou SIGTERM: em ambos o processo node filho precisa ser encerrado.
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
    let root = std::env::current_dir().map_err(|e| format!("diretório atual inacessível: {e}"))?;
    let client = root.join("dist/client");
    if !client.is_dir() {
        return Err("dist/client não existe; rode `pragma build` antes".into());
    }
    runtime()?.block_on(async move {
        let started = Instant::now();
        let site = tokio::task::spawn_blocking(move || server::Site::new(client, server::Mode::Serve)).await.map_err(|e| e.to_string())?;
        println!("{}", dim(&format!("arquivos comprimidos em {} ms", started.elapsed().as_millis())));
        let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await.map_err(|e| format!("não foi possível abrir {host}:{port}: {e}"))?;
        let mut node = node::Node::new(root.join("dist/server/node.mjs"), root.clone());
        node.sync(&site).await;
        tokio::spawn(server::serve(listener, site));
        println!("pragma serve em http://{}:{port}", dev::display_host(&host));
        shutdown().await;
        node.stop().await;
        Ok(0)
    })
}

fn no_positional(args: &Args) -> Result<(), String> {
    match args.positional.first() {
        Some(extra) => Err(format!("argumento inesperado '{extra}'")),
        None => Ok(()),
    }
}
