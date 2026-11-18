use pragma_cli::new::create;

#[test]
fn creates_app_from_embedded_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("loja");
    let count = create(&dir).unwrap();
    assert!(count > 1);
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
    assert!(!dir.join("core/test").exists(), "testes do runtime não vão para o app");
    if let Ok(json) = std::fs::read_to_string(dir.join("pragma.json")) {
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["name"], "loja");
    }
}

#[test]
fn accepts_empty_dir_and_refuses_non_empty() {
    let tmp = tempfile::tempdir().unwrap();
    create(tmp.path()).unwrap();
    let err = create(tmp.path()).unwrap_err();
    assert!(err.contains("não está vazio"), "{err}");

    let file = tmp.path().join("arquivo.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(create(&file).is_err());
}
