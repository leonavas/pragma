use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const PLAYWRIGHT: &str = "1.65.1";
const ALVOS: &str = "rapido | rust | servidor | cliente | overlays | e2e | tudo";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let alvo = match args.as_slice() {
        [cmd] if cmd == "test" => "rapido",
        [cmd, alvo] if cmd == "test" => alvo.as_str(),
        _ => return uso(),
    };
    let ctx = Ctx::new();
    let passos: &[fn(&Ctx) -> bool] = match alvo {
        "rapido" => &[rust, servidor],
        "rust" => &[rust],
        "servidor" => &[servidor],
        "cliente" => &[cliente],
        "overlays" => &[overlays],
        "e2e" => &[e2e],
        "tudo" => &[rust, servidor, cliente, overlays, e2e],
        _ => return uso(),
    };
    for passo in passos {
        if !passo(&ctx) {
            eprintln!("✗ falhou");
            return ExitCode::FAILURE;
        }
    }
    println!("✓ ok");
    ExitCode::SUCCESS
}

fn uso() -> ExitCode {
    eprintln!("uso: cargo xtask test [{ALVOS}]");
    ExitCode::from(2)
}

struct Ctx {
    core: PathBuf,
    cli: PathBuf,
    tools: PathBuf,
}

impl Ctx {
    fn new() -> Self {
        let cli = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let core = cli.parent().unwrap().to_path_buf();
        let tools = env::var_os("PRAGMA_TOOLS_DIR").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").expect("HOME não definido")).join(".cache/pragma-tools")
        });
        Self { core, cli, tools }
    }

    fn playwright(&self) -> PathBuf {
        self.tools.join("node_modules/playwright")
    }
}

fn run(titulo: &str, cmd: &mut Command) -> bool {
    println!("› {titulo}");
    match cmd.status() {
        Ok(status) => status.success(),
        Err(err) => {
            eprintln!("não consegui executar {:?}: {err}", cmd.get_program());
            false
        }
    }
}

fn ferramentas(ctx: &Ctx) -> bool {
    if ctx.playwright().join("index.mjs").exists() {
        return true;
    }
    let tools = ctx.tools.to_string_lossy().into_owned();
    std::fs::create_dir_all(&ctx.tools).is_ok()
        && run(
            &format!("instalando Playwright {PLAYWRIGHT} em {tools} (só na primeira vez)"),
            Command::new("npm").args(["install", "--prefix", &tools, "--no-save", "--silent"]).arg(format!("playwright@{PLAYWRIGHT}")),
        )
        && run(
            "instalando Chromium e Firefox",
            Command::new(ctx.tools.join("node_modules/.bin/playwright")).args(["install", "chromium", "firefox"]),
        )
}

fn rust(ctx: &Ctx) -> bool {
    run(
        "Rust: cargo test --workspace",
        Command::new("cargo").args(["test", "--quiet", "--workspace", "--exclude", "xtask"]).current_dir(&ctx.cli),
    )
}

fn servidor(ctx: &Ctx) -> bool {
    let dir = ctx.core.join("runtime/test/server");
    let mut arquivos: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(".test.js")).collect())
        .unwrap_or_default();
    arquivos.sort();
    run("runtime do servidor (node --test)", Command::new("node").arg("--test").args(&arquivos))
}

fn cliente(ctx: &Ctx) -> bool {
    ferramentas(ctx)
        && run(
            "runtime do cliente (Chromium)",
            Command::new("node").arg("--test").arg(ctx.core.join("runtime/test/client/")).env("PRAGMA_TOOLS", &ctx.tools),
        )
}

fn overlays(ctx: &Ctx) -> bool {
    ferramentas(ctx)
        && run(
            "overlays (Chromium e Firefox)",
            Command::new("node")
                .args(["--test", "--test-concurrency=1"])
                .arg(ctx.core.join("runtime/test/overlay/overlay.test.mjs"))
                .env("PLAYWRIGHT", ctx.playwright().join("index.mjs")),
        )
}

fn e2e(ctx: &Ctx) -> bool {
    let bin = ctx.cli.join("target/release/pragma");
    ferramentas(ctx)
        && run("compilando pragma (release)", Command::new("cargo").args(["build", "--release", "--quiet", "-p", "pragma-cli"]).current_dir(&ctx.cli))
        && run(
            "app de exemplo de ponta a ponta (Chromium e Firefox)",
            Command::new("node")
                .args(["--test", "--test-concurrency=1"])
                .current_dir(ctx.core.join("exemplo/e2e"))
                .env("PRAGMA", &bin)
                .env("PLAYWRIGHT_DIR", ctx.tools.join("node_modules")),
        )
}

