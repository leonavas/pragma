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
    fs::write(app.join("core/docs/cli.md"), "old").unwrap();
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
    fs::write(app.join("core/docs/cli.md"), "old").unwrap();
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
    let page = fs::read(app.join("core/page.js")).unwrap();
    let base = fs::read(app.join("core/base.css")).unwrap();
    fs::write(app.join("core/page.js"), "export const stale = 1").unwrap();
    fs::remove_file(app.join("core/base.css")).unwrap();
    fs::remove_file(app.join("core/docs/cli.md")).unwrap();
    fs::create_dir_all(app.join("core/gone")).unwrap();
    fs::write(app.join("core/gone/old.js"), "export const old = 1").unwrap();
    fs::write(app.join("routes/index.html"), "<h1>mine</h1>").unwrap();
    fs::write(app.join("AGENTS.md"), "mine").unwrap();

    let warning = outdated(app).unwrap();
    assert_eq!(warning.rule, "app/core-outdated");
    assert!(warning.message.contains("core/page.js") && warning.message.contains("pragma upgrade"), "{}", warning.message);

    let changes = upgrade(app).unwrap();
    assert_eq!(changes.updated, ["core/page.js"]);
    assert_eq!(changes.added, ["core/docs/cli.md", "core/base.css"]);
    assert_eq!(changes.removed, ["core/gone/old.js"]);
    assert_eq!(fs::read(app.join("core/page.js")).unwrap(), page);
    assert_eq!(fs::read(app.join("core/base.css")).unwrap(), base);
    assert!(app.join("core/docs/cli.md").is_file());
    assert!(!app.join("core/gone").exists());
    assert_eq!(fs::read_to_string(app.join("routes/index.html")).unwrap(), "<h1>mine</h1>");
    assert_eq!(fs::read_to_string(app.join("AGENTS.md")).unwrap(), "mine");
    assert!(diff(app).is_empty());
}
