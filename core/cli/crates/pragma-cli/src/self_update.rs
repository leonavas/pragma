use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_REPO: &str = "leonavas/pragma";

pub fn run(requested: Option<&str>) -> Result<(), String> {
    let current = env!("CARGO_PKG_VERSION");
    let repo = env::var("PRAGMA_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_owned());
    let target = target_triple()?;
    let tag = match requested {
        Some(v) => normalize_tag(v)?,
        None => latest_tag(&repo)?,
    };
    let next = tag.strip_prefix('v').unwrap_or(&tag);
    if next == current {
        println!("pragma {current} is already up to date");
        return Ok(());
    }

    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    let archive = format!("pragma-{target}.{ext}");
    let base = format!("https://github.com/{repo}/releases/download/{tag}");

    let tmp = TempDir::new()?;
    println!("downloading pragma {tag} ({target})...");
    let extracted = fetch_verified(&base, &archive, &tmp.0)?;
    let exe = env::current_exe().map_err(|e| format!("cannot locate current pragma executable: {e}"))?;
    replace_exe(&extracted, &exe)?;

    println!("updated pragma from {current} to {next}");
    println!("run `pragma upgrade` inside your apps to update .pragma/");
    Ok(())
}

/// Release targets: (`std::env::consts::OS`, `ARCH`, Rust target). release.yml, install.sh and
/// npm/scripts/stage.mjs list the same targets (tests/contracts.rs).
pub const TARGETS: &[(&str, &str, &str)] = &[
    ("linux", "x86_64", "x86_64-unknown-linux-gnu"),
    ("linux", "aarch64", "aarch64-unknown-linux-gnu"),
    ("macos", "x86_64", "x86_64-apple-darwin"),
    ("macos", "aarch64", "aarch64-apple-darwin"),
    ("windows", "x86_64", "x86_64-pc-windows-msvc"),
];

fn target_triple() -> Result<&'static str, String> {
    TARGETS
        .iter()
        .find(|(os, arch, _)| *os == env::consts::OS && *arch == env::consts::ARCH)
        .map(|(_, _, target)| *target)
        .ok_or_else(|| "self-update is not supported on this platform; rebuild from source with Cargo".into())
}

fn normalize_tag(input: &str) -> Result<String, String> {
    let clean = input.trim();
    let ver = clean.strip_prefix('v').unwrap_or(clean);
    let valid = !ver.is_empty() && ver.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+'));
    if !valid {
        return Err(format!("invalid version '{input}'; expected e.g. 0.2.0 or v0.2.0"));
    }
    Ok(format!("v{ver}"))
}

fn latest_tag(repo: &str) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let out = Command::new("curl")
        .args(["-fsSL", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: pragma-cli", &url])
        .output()
        .map_err(|e| format!("cannot run curl to check latest release: {e}"))?;
    if !out.status.success() {
        return Err(format!("failed to check latest release at {url} (no GitHub Release published yet?)"));
    }
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid GitHub API response: {e}"))?;
    body.get("tag_name")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "GitHub release response is missing tag_name".into())
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("cannot run curl: {e}"))?;
    if !status.success() {
        return Err(format!("failed to download {url}"));
    }
    Ok(())
}

/// Downloads `archive` and `SHA256SUMS.txt` from `base` into `dir`, checks the archive against its
/// line in the sums and extracts the binary.
fn fetch_verified(base: &str, archive: &str, dir: &Path) -> Result<PathBuf, String> {
    let archive_path = dir.join(archive);
    let sums_path = dir.join("SHA256SUMS.txt");
    download(&format!("{base}/{archive}"), &archive_path)?;
    download(&format!("{base}/SHA256SUMS.txt"), &sums_path)
        .map_err(|_| format!("cannot download SHA256SUMS.txt from {base}; pragma was not updated, because the archive cannot be verified"))?;
    verify_checksum(&archive_path, &sums_path, archive)?;
    extract(&archive_path, dir)
}

fn verify_checksum(archive_path: &Path, sums_path: &Path, archive_name: &str) -> Result<(), String> {
    let sums = fs::read_to_string(sums_path).map_err(|e| format!("cannot read checksums: {e}"))?;
    let expected = sums
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?.trim_start_matches('*');
            (file == archive_name).then(|| hash.to_ascii_lowercase())
        })
        .next();
    let Some(expected) = expected else {
        return Err(format!("SHA256SUMS.txt has no checksum for {archive_name}; pragma was not updated, because the archive cannot be verified"));
    };
    let actual = sha256_file(archive_path)?;
    if actual != expected {
        return Err(format!("checksum mismatch for {archive_name}: expected {expected}, got {actual}"));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let candidates: &[(&str, &[&str])] = if cfg!(windows) {
        &[("certutil", &["-hashfile"])]
    } else {
        &[("sha256sum", &[]), ("shasum", &["-a", "256"])]
    };
    for (bin, args) in candidates {
        let mut cmd = Command::new(bin);
        cmd.args(*args).arg(path);
        if cfg!(windows) {
            cmd.arg("SHA256");
        }
        if let Ok(out) = cmd.output()
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if cfg!(windows) {
                if let Some(line) = text.lines().nth(1) {
                    return Ok(line.trim().to_ascii_lowercase());
                }
            } else if let Some(hash) = text.split_whitespace().next() {
                return Ok(hash.to_ascii_lowercase());
            }
        }
    }
    Err("could not compute SHA-256 checksum (sha256sum/shasum not found)".into())
}

fn extract(archive: &Path, dir: &Path) -> Result<PathBuf, String> {
    let bin = if cfg!(windows) { "pragma.exe" } else { "pragma" };
    if cfg!(windows) {
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force", archive.display(), dir.display())])
            .status()
            .map_err(|e| format!("cannot run powershell to extract archive: {e}"))?;
        if !status.success() {
            return Err("failed to extract zip archive".into());
        }
    } else {
        let status = Command::new("tar")
            .args(["-xzf"])
            .arg(archive)
            .arg("-C")
            .arg(dir)
            .status()
            .map_err(|e| format!("cannot run tar: {e}"))?;
        if !status.success() {
            return Err("failed to extract tar.gz archive".into());
        }
    }
    let path = dir.join(bin);
    if !path.is_file() {
        return Err(format!("archive did not contain {bin}"));
    }
    Ok(path)
}

fn replace_exe(new_bin: &Path, target: &Path) -> Result<(), String> {
    let parent = target.parent().ok_or_else(|| "cannot determine executable directory".to_owned())?;
    let staged = parent.join(format!(".pragma-update-{}", std::process::id()));
    fs::copy(new_bin, &staged).map_err(|e| format!("cannot write to '{}': {e}", parent.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&staged, fs::Permissions::from_mode(0o755));
    }
    if cfg!(windows) {
        let backup = target.with_extension("old.exe");
        let _ = fs::remove_file(&backup);
        fs::rename(target, &backup).map_err(|e| format!("cannot move current binary: {e}"))?;
        if let Err(e) = fs::rename(&staged, target) {
            let _ = fs::rename(&backup, target);
            return Err(format!("cannot install new binary: {e}"));
        }
    } else {
        fs::rename(&staged, target).map_err(|e| {
            let _ = fs::remove_file(&staged);
            format!("cannot replace '{}': {e}", target.display())
        })?;
    }
    Ok(())
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<TempDir, String> {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let dir = env::temp_dir().join(format!("pragma-self-update-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("cannot create temp dir: {e}"))?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn installs_only_an_archive_that_matches_its_checksum() {
        let release = tempfile::tempdir().unwrap();
        let archive = "pragma-test.tar.gz";
        fs::write(release.path().join("pragma"), "binary").unwrap();
        assert!(Command::new("tar").args(["-czf", archive, "pragma"]).current_dir(release.path()).status().unwrap().success());
        let base = format!("file://{}", release.path().display());
        let fetch = || fetch_verified(&base, archive, TempDir::new().unwrap().0.as_path());

        assert!(fetch().unwrap_err().contains("cannot download SHA256SUMS.txt"));
        fs::write(release.path().join("SHA256SUMS.txt"), "abc  pragma-other.tar.gz\n").unwrap();
        assert!(fetch().unwrap_err().contains("no checksum for pragma-test.tar.gz"));
        fs::write(release.path().join("SHA256SUMS.txt"), format!("abc  {archive}\n")).unwrap();
        assert!(fetch().unwrap_err().contains("checksum mismatch"));
        let sum = sha256_file(&release.path().join(archive)).unwrap();
        fs::write(release.path().join("SHA256SUMS.txt"), format!("{sum}  {archive}\n")).unwrap();
        let dir = TempDir::new().unwrap();
        let installed = fetch_verified(&base, archive, &dir.0).unwrap();
        assert_eq!(fs::read_to_string(installed).unwrap(), "binary");
    }

    #[test]
    fn normalizes_version_tags() {
        assert_eq!(normalize_tag("0.2.0").unwrap(), "v0.2.0");
        assert_eq!(normalize_tag("v0.2.0-rc.1").unwrap(), "v0.2.0-rc.1");
        assert!(normalize_tag("bad/version").is_err());
        assert!(normalize_tag("").is_err());
    }
}
