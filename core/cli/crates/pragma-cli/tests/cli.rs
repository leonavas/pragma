use pragma_cli::new::create;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn pragma(app: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pragma")).args(args).current_dir(app).output().unwrap()
}

#[test]
fn template_literal_import_builds_like_a_quoted_one() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    fs::write(app.join("utils/lazy.js"), "export const value = 2\n").unwrap();
    fs::create_dir_all(app.join("routes/lazy")).unwrap();
    let page = "<script>\n  let total = 0\n\n  async function loadLazy() {\n    const lazy = await import(`../../utils/lazy.js`)\n    total = lazy.value\n  }\n</script>\n\n<title>Lazy</title>\n\n<h1>Lazy {$total}</h1>\n<button type=\"button\" @click=\"loadLazy\">Load</button>\n";
    fs::write(app.join("routes/lazy/index.html"), page).unwrap();

    let check = pragma(app, &["check", "--strict"]);
    assert_eq!(check.status.code(), Some(0), "{}", String::from_utf8_lossy(&check.stderr));
    let build = pragma(app, &["build"]);
    assert_eq!(build.status.code(), Some(0), "{}", String::from_utf8_lossy(&build.stderr));
    let chunks: Vec<String> = fs::read_dir(app.join("dist/client/assets/chunks")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert!(chunks.iter().any(|name| name.starts_with("lazy-") && name.ends_with(".js")), "{chunks:?}");
}

#[cfg(target_os = "linux")]
#[test]
fn redirected_stdout_has_no_color_when_stderr_is_a_terminal() {
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    let command = format!("'{}' build > out.txt", env!("CARGO_BIN_EXE_pragma"));
    let Ok(status) = Command::new("script").args(["-qec", &command, "/dev/null"]).current_dir(app).status() else { return };
    assert!(status.success());
    let stdout = fs::read_to_string(app.join("out.txt")).unwrap();
    assert!(stdout.contains("dist/ built in"), "{stdout}");
    assert!(!stdout.contains('\x1b'), "{stdout:?}");
}
