use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const PLAYWRIGHT: &str = "1.65.1";
const TARGETS: &str = "quick | rust | server | client | overlays | e2e | packages | all";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let target = match args.as_slice() {
        [cmd] if cmd == "test" => "quick",
        [cmd, target] if cmd == "test" => target.as_str(),
        _ => return usage(),
    };
    let ctx = Ctx::new();
    let steps: &[fn(&Ctx) -> bool] = match target {
        "quick" => &[rust, server],
        "rust" => &[rust],
        "server" => &[server],
        "client" => &[client],
        "overlays" => &[overlays],
        "e2e" => &[e2e],
        "packages" => &[packages],
        "all" => &[rust, server, client, overlays, e2e],
        _ => return usage(),
    };
    for step in steps {
        if !step(&ctx) {
            eprintln!("✗ failed");
            return ExitCode::FAILURE;
        }
    }
    println!("✓ ok");
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    eprintln!("usage: cargo xtask test [{TARGETS}]");
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
            PathBuf::from(env::var_os("HOME").expect("HOME not set")).join(".cache/pragma-tools")
        });
        Self { core, cli, tools }
    }

    fn playwright(&self) -> PathBuf {
        self.tools.join("node_modules/playwright")
    }
}

fn run(title: &str, cmd: &mut Command) -> bool {
    println!("› {title}");
    match cmd.status() {
        Ok(status) => status.success(),
        Err(err) => {
            eprintln!("could not run {:?}: {err}", cmd.get_program());
            false
        }
    }
}

fn tools(ctx: &Ctx) -> bool {
    if ctx.playwright().join("index.mjs").exists() {
        return true;
    }
    let tools = ctx.tools.to_string_lossy().into_owned();
    std::fs::create_dir_all(&ctx.tools).is_ok()
        && run(
            &format!("installing Playwright {PLAYWRIGHT} in {tools} (first run only)"),
            Command::new("npm").args(["install", "--prefix", &tools, "--no-save", "--silent"]).arg(format!("playwright@{PLAYWRIGHT}")),
        )
        && run(
            "installing Chromium and Firefox",
            Command::new(ctx.tools.join("node_modules/.bin/playwright")).args(["install", "chromium", "firefox"]),
        )
}

fn rust(ctx: &Ctx) -> bool {
    run(
        "Rust: cargo test --workspace",
        Command::new("cargo").args(["test", "--quiet", "--workspace", "--exclude", "xtask"]).current_dir(&ctx.cli),
    )
}

fn server(ctx: &Ctx) -> bool {
    let dir = ctx.core.join("runtime/test/server");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|it| it.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.to_string_lossy().ends_with(".test.js")).collect())
        .unwrap_or_default();
    files.sort();
    run("server runtime (node --test)", Command::new("node").arg("--test").args(&files))
}

fn client(ctx: &Ctx) -> bool {
    tools(ctx)
        && run(
            "client runtime (Chromium)",
            Command::new("node").arg("--test").arg(ctx.core.join("runtime/test/client/")).env("PRAGMA_TOOLS", &ctx.tools),
        )
}

fn overlays(ctx: &Ctx) -> bool {
    tools(ctx)
        && run(
            "overlays (Chromium and Firefox)",
            Command::new("node")
                .args(["--test", "--test-concurrency=1"])
                .arg(ctx.core.join("runtime/test/overlay/overlay.test.mjs"))
                .env("PLAYWRIGHT", ctx.playwright().join("index.mjs")),
        )
}

fn pragma_release(ctx: &Ctx) -> bool {
    run("building pragma (release)", Command::new("cargo").args(["build", "--release", "--quiet", "-p", "pragma-cli"]).current_dir(&ctx.cli))
}

fn e2e(ctx: &Ctx) -> bool {
    let bin = ctx.cli.join("target/release/pragma");
    tools(ctx)
        && pragma_release(ctx)
        && run(
            "example app end to end (Chromium and Firefox)",
            Command::new("node")
                .args(["--test", "--test-concurrency=1"])
                .current_dir(ctx.core.join("example/e2e"))
                .env("PRAGMA", &bin)
                .env("PLAYWRIGHT_DIR", ctx.tools.join("node_modules")),
        )
}

fn packages(ctx: &Ctx) -> bool {
    let bin = ctx.cli.join("target/release/pragma");
    tools(ctx)
        && pragma_release(ctx)
        && run(
            "real npm packages: check, build, serve, dev, live reload (Chromium and Firefox)",
            Command::new("node")
                .args(["--test", "--test-concurrency=1"])
                .arg(ctx.cli.join("xtask/packages/packages.test.mjs"))
                .env("PRAGMA", &bin)
                .env("PRAGMA_TOOLS", &ctx.tools)
                .env("PLAYWRIGHT_DIR", ctx.tools.join("node_modules")),
        )
}
