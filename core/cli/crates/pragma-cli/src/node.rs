use crate::report::{dim, paint, red};
use crate::server::Site;
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// `node <entry>` process on an internal port, with the `Site` proxy pointing at it.
pub struct Node {
    entry: PathBuf,
    cwd: PathBuf,
    child: Option<Child>,
    fingerprint: u64,
    dev: bool,
    cf_config: Option<PathBuf>,
}

impl Node {
    /// `dev`: shows `log.debug` (PRAGMA_LOG_LEVEL=debug) unless the environment sets a level.
    pub fn new(entry: PathBuf, cwd: PathBuf, dev: bool) -> Node {
        Node { entry, cwd, child: None, fingerprint: 0, dev, cf_config: None }
    }

    /// `pragma dev --cf`: `env`, `ctx` and `caches` come from wrangler `getPlatformProxy` over `config`.
    pub fn with_cloudflare(mut self, config: PathBuf) -> Node {
        self.cf_config = Some(config);
        self
    }

    /// Starts, restarts (if server code changed) or stops node according to the current build.
    pub async fn sync(&mut self, site: &Site) {
        if !self.entry.is_file() {
            self.stop().await;
            site.set_backend(None);
            return;
        }
        let mut fingerprint = fingerprint(self.entry.parent().unwrap());
        if let Some(config) = &self.cf_config {
            fingerprint ^= cloudflare_fingerprint(config);
        }
        if self.child.is_some() && fingerprint == self.fingerprint {
            return;
        }
        let restarting = self.child.is_some();
        self.stop().await;
        self.fingerprint = fingerprint;
        let port = match std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()) {
            Ok(addr) => addr.port(),
            Err(e) => return eprintln!("{} no free port for node: {e}", red("error")),
        };
        let mut cmd = Command::new("node");
        if self.dev && std::env::var_os("PRAGMA_LOG_LEVEL").is_none() {
            cmd.env("PRAGMA_LOG_LEVEL", "debug");
        }
        if let Some(config) = &self.cf_config {
            cmd.env("PRAGMA_CF_CONFIG", config);
        }
        let spawned = cmd
            .arg(&self.entry)
            .env("PORT", port.to_string())
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => return eprintln!("{} cannot start `node {}`: {e} (is Node >= 22 installed?)", red("error"), self.entry.display()),
        };
        let pretty = std::io::stdout().is_terminal();
        if let Some(out) = child.stdout.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(out).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    println!("{}", if pretty { format_log(&line) } else { line });
                }
            });
        }
        if let Some(err) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(err).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    eprintln!("{} {line}", dim("node"));
                }
            });
        }
        let started = Instant::now();
        // remote bindings open a session with Cloudflare first
        let limit = Duration::from_secs(if self.cf_config.is_some() { 90 } else { 10 });
        let ready = loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                break true;
            }
            if matches!(child.try_wait(), Ok(Some(_))) || started.elapsed() > limit {
                break false;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        };
        if ready {
            println!("{}", dim(&format!("node {} on port {port}", if restarting { "restarted" } else { "running" })));
        } else {
            eprintln!("{} node server did not start; see the output above", red("error"));
        }
        site.set_backend(Some(port));
        self.child = Some(child);
    }

    pub async fn stop(&mut self) {
        let Some(mut child) = self.child.take() else { return };
        // getPlatformProxy runs workerd: SIGTERM lets node dispose it before exiting
        if self.cf_config.is_some()
            && let Some(pid) = child.id()
            && Command::new("kill").arg("-TERM").arg(pid.to_string()).status().await.is_ok_and(|s| s.success())
            && tokio::time::timeout(Duration::from_secs(5), child.wait()).await.is_ok()
        {
            return;
        }
        let _ = child.kill().await;
    }
}

fn cloudflare_fingerprint(config: &Path) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::fs::read(config).unwrap_or_default().hash(&mut hasher);
    std::fs::read(config.with_file_name(".dev.vars")).unwrap_or_default().hash(&mut hasher);
    hasher.finish()
}

/// Root wrangler config `pragma dev --cf` reads bindings from.
pub fn cloudflare_config(root: &Path) -> Result<PathBuf, String> {
    let config = ["wrangler.jsonc", "wrangler.json", "wrangler.toml"]
        .iter()
        .map(|name| root.join(name))
        .find(|path| path.is_file())
        .ok_or("pragma dev --cf reads bindings from wrangler.jsonc in the app root; create it (cli.md, \"Secrets, variables, bindings\")")?;
    if !root.ancestors().any(|dir| dir.join("node_modules/wrangler/package.json").is_file()) {
        return Err("pragma dev --cf needs wrangler in the app: run `npm i -D wrangler@4`".into());
    }
    Ok(config)
}

fn fingerprint(dir: &Path) -> u64 {
    let mut files = Vec::new();
    collect(dir, &mut files);
    files.sort();
    let mut hasher = std::hash::DefaultHasher::new();
    for path in files {
        path.hash(&mut hasher);
        std::fs::read(&path).unwrap_or_default().hash(&mut hasher);
    }
    hasher.finish()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() { collect(&path, out) } else { out.push(path) }
    }
}

/// OTel JSON record (one per line, as `core/server/log.js` prints) to a readable line.
pub fn format_log(line: &str) -> String {
    let Ok(Value::Object(rec)) = serde_json::from_str::<Value>(line) else { return line.to_owned() };
    let Some(level) = rec.get("severityText").and_then(Value::as_str) else { return line.to_owned() };
    let body = rec.get("body").map(text).unwrap_or_default();
    let painted = match level {
        "ERROR" | "FATAL" => red(&format!("{level:5}")),
        "WARN" => paint("33", &format!("{level:5}")),
        "DEBUG" | "TRACE" => dim(&format!("{level:5}")),
        _ => paint("36", &format!("{level:5}")),
    };
    let mut out = format!("{painted} {body}");
    let mut stack = None;
    for (key, value) in attributes(rec.get("attributes")) {
        if key == "exception.stacktrace" {
            stack = Some(value);
        } else {
            out.push_str(&dim(&format!(" {key}={value}")));
        }
    }
    let service = match rec.get("resource") {
        Some(Value::Object(r)) => r.get("service.name").map(text),
        _ => None,
    };
    if let Some(service) = service {
        out.push_str(&dim(&format!(" ({service})")));
    }
    if let Some(stack) = stack {
        out.push('\n');
        out.push_str(&dim(&stack));
    }
    out
}

/// Client batch (`{ resource, logs }`) when dev has no node server to receive it.
pub fn print_client_logs(body: &[u8]) {
    let Ok(Value::Object(batch)) = serde_json::from_slice::<Value>(body) else { return };
    let resource = batch.get("resource").cloned().unwrap_or_else(|| serde_json::json!({ "service.name": "pragma-client" }));
    for record in batch.get("logs").and_then(Value::as_array).into_iter().flatten() {
        let mut record = record.clone();
        if let Value::Object(map) = &mut record {
            map.insert("resource".into(), resource.clone());
        }
        println!("{}", format_log(&record.to_string()));
    }
}

fn attributes(value: Option<&Value>) -> Vec<(String, String)> {
    match value {
        Some(Value::Object(map)) => map.iter().map(|(k, v)| (k.clone(), text(v))).collect(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|kv| Some((kv.get("key")?.as_str()?.to_owned(), text(kv.get("value")?))))
            .collect(),
        _ => Vec::new(),
    }
}

fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Object(map) if map.len() == 1 => {
            let (_, inner) = map.iter().next().unwrap();
            text(inner)
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflare_needs_a_root_config_and_wrangler() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert!(cloudflare_config(root).unwrap_err().contains("wrangler.jsonc"));
        std::fs::write(root.join("wrangler.jsonc"), "{}").unwrap();
        assert!(cloudflare_config(root).unwrap_err().contains("npm i -D wrangler@4"));
        std::fs::create_dir_all(root.join("node_modules/wrangler")).unwrap();
        std::fs::write(root.join("node_modules/wrangler/package.json"), "{}").unwrap();
        assert_eq!(cloudflare_config(root).unwrap(), root.join("wrangler.jsonc"));
    }

    #[test]
    fn formats_otel_record() {
        let line = r#"{"timeUnixNano":"1","severityNumber":9,"severityText":"INFO","body":"GET /shop","attributes":{"http.response.status_code":200},"resource":{"service.name":"app"}}"#;
        let out = format_log(line);
        assert!(out.contains("GET /shop"));
        assert!(out.contains("http.response.status_code=200"));
        assert!(out.contains("(app)"));
        assert_eq!(format_log("plain text"), "plain text");
    }
}
