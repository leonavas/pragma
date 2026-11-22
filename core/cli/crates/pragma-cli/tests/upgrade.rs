use pragma_cli::new::create;
use pragma_cli::upgrade::{build_gate, diff, outdated, upgrade};
use std::fs;
use std::process::Command;

#[test]
fn build_gate_stops_an_outdated_core_unless_allowed() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    assert_eq!(build_gate(app, false), Ok(None));
    fs::write(app.join(".pragma/docs/cli.md"), "old").unwrap();
    let (error, message) = build_gate(app, false).unwrap_err();
    assert_eq!((error.rule, error.severity), ("app/core-outdated", pragma_diag::Severity::Error));
    assert!(message.contains("pragma upgrade") && message.contains("--allow-outdated-core"), "{message}");
    let warning = build_gate(app, true).unwrap().unwrap();
    assert_eq!(warning.severity, pragma_diag::Severity::Warning);
}

#[test]
fn pragma_build_stops_on_an_outdated_core() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path().join("shop");
    create(&app).unwrap();
    fs::write(app.join(".pragma/docs/cli.md"), "old").unwrap();
    let pragma = |args: &[&str]| Command::new(env!("CARGO_BIN_EXE_pragma")).args(args).current_dir(&app).output().unwrap();
    let out = pragma(&["build"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("error [app/core-outdated]") && stderr.contains("run `pragma upgrade` first"), "{stderr}");
    assert!(!app.join("dist").exists(), "nothing is built with an outdated core/");
    let out = pragma(&["build", "--allow-outdated-core"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("warning [app/core-outdated]"), "{stderr}");
    assert!(app.join("dist/client/index.html").is_file());
    pragma(&["upgrade"]);
    let out = pragma(&["build"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("core-outdated"), "{stderr}");
}

#[test]
fn fresh_app_is_up_to_date() {
    let tmp = tempfile::tempdir().unwrap();
    create(tmp.path()).unwrap();
    assert!(diff(tmp.path()).is_empty());
    assert_eq!(outdated(tmp.path()), None);
    assert!(upgrade(tmp.path()).unwrap().is_empty());
}

#[test]
fn upgrade_restores_core_and_keeps_user_files() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    let page = fs::read(app.join(".pragma/page.js")).unwrap();
    let base = fs::read(app.join(".pragma/base.css")).unwrap();
    fs::write(app.join(".pragma/page.js"), "export const stale = 1").unwrap();
    fs::remove_file(app.join(".pragma/base.css")).unwrap();
    fs::remove_file(app.join(".pragma/docs/cli.md")).unwrap();
    fs::create_dir_all(app.join(".pragma/gone")).unwrap();
    fs::write(app.join(".pragma/gone/old.js"), "export const old = 1").unwrap();
    fs::write(app.join("routes/index.html"), "<h1>mine</h1>").unwrap();
    fs::write(app.join("AGENTS.md"), "mine").unwrap();

    let warning = outdated(app).unwrap();
    assert_eq!(warning.rule, "app/core-outdated");
    assert!(warning.message.contains(".pragma/page.js") && warning.message.contains("pragma upgrade"), "{}", warning.message);

    fs::write(app.join(".pragma/version"), "0.0.1\n").unwrap();
    let versioned = outdated(app).unwrap();
    assert!(versioned.message.contains("0.0.1") && versioned.message.contains(env!("CARGO_PKG_VERSION")), "{}", versioned.message);

    let changes = upgrade(app).unwrap();
    let sorted = |paths: &[String]| paths.iter().cloned().collect::<std::collections::BTreeSet<String>>();
    assert_eq!(sorted(&changes.updated), [".pragma/page.js", ".pragma/version"].map(String::from).into());
    assert_eq!(sorted(&changes.added), [".pragma/base.css", ".pragma/docs/cli.md"].map(String::from).into());
    assert_eq!(changes.removed, [".pragma/gone/old.js"]);
    assert_eq!(fs::read(app.join(".pragma/page.js")).unwrap(), page);
    assert_eq!(fs::read(app.join(".pragma/base.css")).unwrap(), base);
    assert!(app.join(".pragma/docs/cli.md").is_file());
    assert!(!app.join(".pragma/gone").exists());
    assert_eq!(fs::read_to_string(app.join("routes/index.html")).unwrap(), "<h1>mine</h1>");
    assert_eq!(fs::read_to_string(app.join("AGENTS.md")).unwrap(), "mine");
    assert!(diff(app).is_empty());
}

#[test]
fn upgrade_from_the_core_layout_removes_only_its_files() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    for (rel, text) in [("core/base.css", "old"), ("core/boot.js", "old"), ("core/docs/cli.md", "old"), ("core/mine/notes.txt", "keep"), ("core/server/db.js", "keep")] {
        fs::create_dir_all(app.join(rel).parent().unwrap()).unwrap();
        fs::write(app.join(rel), text).unwrap();
    }

    let warning = outdated(app).unwrap();
    assert!(warning.message.contains("core/base.css") && !warning.message.contains("core/mine"), "{}", warning.message);

    let out = Command::new(env!("CARGO_BIN_EXE_pragma")).arg("upgrade").current_dir(app).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("removed  core/base.css") && stdout.contains("removed  core/docs/cli.md"), "{stdout}");
    assert!(!stdout.contains("core/mine") && !stdout.contains("core/server/db.js"), "{stdout}");
    assert!(stdout.contains("the old core/ files listed above were removed"), "{stdout}");
    assert_eq!(fs::read_to_string(app.join("core/mine/notes.txt")).unwrap(), "keep");
    assert_eq!(fs::read_to_string(app.join("core/server/db.js")).unwrap(), "keep");
    assert!(!app.join("core/docs").exists() && !app.join("core/boot.js").exists());
    assert!(diff(app).is_empty());
}

#[test]
fn upgrade_removes_an_emptied_core_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    fs::create_dir_all(app.join("core/server")).unwrap();
    fs::write(app.join("core/boot.js"), "old").unwrap();
    fs::write(app.join("core/server/handler.js"), "old").unwrap();
    assert_eq!(upgrade(app).unwrap().removed, ["core/boot.js", "core/server/handler.js"]);
    assert!(!app.join("core").exists());
}
