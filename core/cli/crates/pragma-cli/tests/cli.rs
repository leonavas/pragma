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
    let Ok(status) = Command::new("script").args(["-qec", &command, "/dev/null"]).current_dir(app).status() else {
        eprintln!("skipped: needs `script` (util-linux) to give stderr a terminal");
        return;
    };
    assert!(status.success());
    let stdout = fs::read_to_string(app.join("out.txt")).unwrap();
    assert!(stdout.contains("dist/ built in"), "{stdout}");
    assert!(!stdout.contains('\x1b'), "{stdout:?}");
}

#[cfg(unix)]
#[test]
fn a_build_that_cannot_write_its_output_fails() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let app = tmp.path();
    create(app).unwrap();
    assert_eq!(pragma(app, &["build"]).status.code(), Some(0));
    let client = app.join("dist/client");
    let index = client.join("index.html");
    let mode = |m: u32| {
        fs::set_permissions(&index, fs::Permissions::from_mode(m & 0o666)).unwrap();
        fs::set_permissions(&client, fs::Permissions::from_mode(m)).unwrap();
    };
    mode(0o555);
    if fs::write(client.join("probe"), "").is_ok() {
        mode(0o755);
        eprintln!("skipped: this user can write to a read-only folder");
        return;
    }
    let page = fs::read_to_string(app.join("routes/index.html")).unwrap();
    fs::write(app.join("routes/index.html"), page.replace("</h1>", " changed</h1>")).unwrap();
    let out = pragma(app, &["build"]);
    mode(0o755);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}\n{stderr}");
    assert!(stderr.contains("dist/client/index.html") && stderr.contains("error [app/output] cannot write"), "{stderr}");
    assert!(!stdout.contains("built in"), "{stdout}");
}
