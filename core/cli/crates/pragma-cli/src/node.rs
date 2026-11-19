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
}

impl Node {
    pub fn new(entry: PathBuf, cwd: PathBuf) -> Node {
        Node { entry, cwd, child: None, fingerprint: 0 }
    }

    /// Starts, restarts (if server code changed) or stops node according to the current build.
    pub async fn sync(&mut self, site: &Site) {
        if !self.entry.is_file() {
            self.stop().await;
            site.set_backend(None);
            return;
        }
        let fingerprint = fingerprint(self.entry.parent().unwrap());
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
        let spawned = Command::new("node")
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
        let ready = loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
                break true;
            }
            if matches!(child.try_wait(), Ok(Some(_))) || started.elapsed() > Duration::from_secs(10) {
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
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
    }
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
    fn formats_otel_record() {
        let line = r#"{"timeUnixNano":"1","severityNumber":9,"severityText":"INFO","body":"GET /shop","attributes":{"http.response.status_code":200},"resource":{"service.name":"app"}}"#;
        let out = format_log(line);
        assert!(out.contains("GET /shop"));
        assert!(out.contains("http.response.status_code=200"));
        assert!(out.contains("(app)"));
        assert_eq!(format_log("plain text"), "plain text");
    }
}
