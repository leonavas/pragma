use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

const PLAYWRIGHT: &str = "1.65.1";
const TARGETS: &str = "quick | rust | server | client | overlays | e2e | packages | all";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(cmd) = args.first().map(String::as_str)
        && cmd == "release"
    {
        return release(&Ctx::new(), &args[1..]);
    }
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
    eprintln!("usage:\n  cargo xtask test [{TARGETS}]\n  cargo xtask release <version> [--tag]");
    ExitCode::from(2)
}

fn release(ctx: &Ctx, args: &[String]) -> ExitCode {
    let mut version: Option<&str> = None;
    let mut create_tag = false;
    for arg in args {
        match arg.as_str() {
            "--tag" => create_tag = true,
            v if !v.starts_with('-') && version.is_none() => version = Some(v),
            _ => return usage(),
        }
    }
    let Some(raw) = version else { return usage() };
    let ver = raw.strip_prefix('v').unwrap_or(raw);
    let parts: Vec<&str> = ver.split('.').collect();
    let valid = parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if !valid {
        eprintln!("invalid semver '{raw}'; expected X.Y.Z (e.g. 0.2.0)");
        return ExitCode::from(2);
    }

    let cargo_toml = ctx.cli.join("Cargo.toml");
    let src = match std::fs::read_to_string(&cargo_toml) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {}: {e}", cargo_toml.display());
            return ExitCode::FAILURE;
        }
    };
    let mut replaced = false;
    let mut section = "";
    let updated: Vec<String> = src
        .lines()
        .map(|line| {
            if line.starts_with('[') {
                section = line.trim();
            }
            if !replaced && section == "[workspace.package]" && line.trim_start().starts_with("version = \"") {
                replaced = true;
                format!("version = \"{ver}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    if !replaced {
        eprintln!("could not find `version = \"...\"` under [workspace.package] in {}", cargo_toml.display());
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::write(&cargo_toml, format!("{}\n", updated.join("\n"))) {
        eprintln!("cannot write {}: {e}", cargo_toml.display());
        return ExitCode::FAILURE;
    }
    println!("› updated core/cli/Cargo.toml to {ver}");

    let repo = ctx.core.parent().unwrap();
    let bin = ctx.pragma_bin();
    let ok = build_release(ctx, "updating Cargo.lock and building release binary") && run("upgrading site/.pragma/", Command::new(&bin).arg("upgrade").current_dir(repo.join("site")))
        && run("upgrading core/example/.pragma/", Command::new(&bin).arg("upgrade").current_dir(ctx.core.join("example")))
        && run(
            "regenerating site agent docs",
            Command::new("node").arg("site/utils/agent-docs/generate.server.js").current_dir(repo),
        );
    if !ok {
        eprintln!("✗ release preparation failed");
        return ExitCode::FAILURE;
    }

    let tag = format!("v{ver}");
    if create_tag {
        let tagged = run(
            &format!("committing release {tag}"),
            Command::new("git").args(["add", "core", "site"]).current_dir(repo),
        ) && run(
            &format!("git commit -m \"Release {tag}\""),
            Command::new("git").args(["commit", "-m", &format!("Release {tag}")]).current_dir(repo),
        ) && run(
            &format!("git tag -a {tag} -m \"Release {tag}\""),
            Command::new("git").args(["tag", "-a", &tag, "-m", &format!("Release {tag}")]).current_dir(repo),
        );
        if !tagged {
            eprintln!("✗ git commit/tag failed");
            return ExitCode::FAILURE;
        }
        println!("✓ release {tag} committed and tagged");
        println!("  next: git push origin main --follow-tags");
    } else {
        println!("✓ release {tag} prepared");
        println!("  review changes, then run:");
        println!("    git add core site && git commit -m \"Release {tag}\" && git tag -a {tag} -m \"Release {tag}\" && git push origin main --follow-tags");
    }
    ExitCode::SUCCESS
}

struct Ctx {
    core: PathBuf,
    cli: PathBuf,
    target: PathBuf,
    tools: PathBuf,
}

impl Ctx {
    fn new() -> Self {
        let cli = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let core = cli.parent().unwrap().to_path_buf();
        // xtask runs from <target dir>/<profile>/xtask, wherever .cargo/config.toml or
        // CARGO_TARGET_DIR puts the target dir.
        let exe = env::current_exe().expect("cannot locate the xtask binary");
        let target = exe.parent().and_then(|p| p.parent()).expect("xtask is not in a target dir").to_path_buf();
        let tools = env::var_os("PRAGMA_TOOLS_DIR").map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(env::var_os("HOME").expect("HOME not set")).join(".cache/pragma-tools")
        });
        Self { core, cli, target, tools }
    }

    fn pragma_bin(&self) -> PathBuf {
        self.target.join("release").join(format!("pragma{}", env::consts::EXE_SUFFIX))
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
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
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

fn build_release(ctx: &Ctx, title: &str) -> bool {
    run(title, Command::new("cargo").args(["build", "--release", "--quiet", "-p", "pragma-cli"]).current_dir(&ctx.cli))
}

fn e2e(ctx: &Ctx) -> bool {
    let bin = ctx.pragma_bin();
    tools(ctx)
        && build_release(ctx, "building pragma (release)")
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
    let bin = ctx.pragma_bin();
    tools(ctx)
        && build_release(ctx, "building pragma (release)")
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
