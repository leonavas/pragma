use pragma_cli::new::create;

#[test]
fn creates_app_from_embedded_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("shop");
    let created = create(&dir).unwrap();
    assert!(created.written > 1);
    assert_eq!(created.skipped, 0);
    assert!(!created.non_empty);
    let base = std::fs::read_to_string(dir.join("core/base.css")).unwrap();
    assert_eq!(base, pragma_css::base_css());

    let core = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    for (src, dest) in [("starter", ""), ("runtime", "core"), ("docs", "core/docs")] {
        for entry in std::fs::read_dir(core.join(src)).unwrap().flatten() {
            let name = entry.file_name();
            if entry.path().is_file() && name != "pragma.json" {
                let copied = std::fs::read(dir.join(dest).join(&name)).ok();
                assert_eq!(copied, std::fs::read(entry.path()).ok(), "{src}/{name:?}");
            }
        }
    }
    assert!(!dir.join("core/test").exists(), "runtime tests are not copied to the app");
    if let Ok(json) = std::fs::read_to_string(dir.join("pragma.json")) {
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["name"], "shop");
    }
}

#[test]
fn warns_and_keeps_existing_files_in_non_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("README.md"), "# Existing\n").unwrap();
    std::fs::write(tmp.path().join(".gitignore"), "custom/\n").unwrap();

    let first = create(tmp.path()).unwrap();
    assert!(first.non_empty);
    assert!(first.written > 1);
    assert_eq!(first.skipped, 1);
    assert_eq!(std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap(), "custom/\n");
    assert_eq!(std::fs::read_to_string(tmp.path().join("README.md")).unwrap(), "# Existing\n");
    assert!(tmp.path().join("pragma.json").is_file());

    let second = create(tmp.path()).unwrap();
    assert!(second.non_empty);
    assert_eq!(second.written, 0);
    assert_eq!(second.skipped, first.written + first.skipped);

    let file = tmp.path().join("file.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(create(&file).is_err());
}
