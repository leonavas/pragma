use pragma_cli::new::create;
use pragma_cli::upgrade::{diff, outdated, upgrade};
use std::fs;

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
