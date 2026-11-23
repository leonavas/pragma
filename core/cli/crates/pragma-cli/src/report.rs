use pragma_compiler::BuildReport;
use pragma_diag::{Diagnostic, Severity};
use serde_json::Value;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Colors for one output stream: ANSI codes only when that stream is a terminal.
#[derive(Clone, Copy)]
pub struct Paint {
    color: bool,
}

pub fn out() -> Paint {
    Paint { color: std::io::stdout().is_terminal() }
}

pub fn err() -> Paint {
    Paint { color: std::io::stderr().is_terminal() }
}

impl Paint {
    fn paint(self, code: &str, text: &str) -> String {
        if self.color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_owned() }
    }

    pub fn red(self, text: &str) -> String {
        self.paint("31", text)
    }

    pub fn yellow(self, text: &str) -> String {
        self.paint("33", text)
    }

    pub fn cyan(self, text: &str) -> String {
        self.paint("36", text)
    }

    pub fn dim(self, text: &str) -> String {
        self.paint("2", text)
    }
}

fn relative(root: &Path, file: &Path) -> PathBuf {
    file.strip_prefix(root).map(Path::to_path_buf).unwrap_or_else(|_| file.to_path_buf())
}

pub fn print_diagnostics(root: &Path, diagnostics: &[Diagnostic]) {
    let paint = err();
    for d in diagnostics {
        let level = match d.severity {
            Severity::Error => paint.red(d.severity.as_str()),
            Severity::Warning => paint.yellow(d.severity.as_str()),
        };
        let rel = relative(root, &d.file);
        eprintln!("{}:{}:{} {level} {} {}", rel.display(), d.line, d.column, paint.dim(&format!("[{}]", d.rule)), d.message);
        let source = std::fs::read_to_string(root.join(&rel)).ok();
        if let Some(line) = source.as_deref().and_then(|s| s.lines().nth(d.line.saturating_sub(1) as usize))
            && d.line > 0
        {
            let number = d.line.to_string();
            let pad = " ".repeat(number.len());
            let caret = " ".repeat(line.chars().take(d.column.saturating_sub(1) as usize).map(|c| if c == '\t' { 4 } else { 1 }).sum());
            eprintln!("{}", paint.dim(&format!(" {number} | {}", line.replace('\t', "    "))));
            eprintln!("{}", paint.dim(&format!(" {pad} | {caret}^")));
        }
    }
    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diagnostics.len() - errors;
    if !diagnostics.is_empty() {
        eprintln!("{errors} error(s), {warnings} warning(s)");
    }
}

pub fn diagnostics_json(root: &Path, diagnostics: &[Diagnostic]) -> String {
    let items: Vec<_> = diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "file": relative(root, &d.file).display().to_string(),
                "line": d.line,
                "column": d.column,
                "rule": d.rule,
                "message": d.message,
                "severity": d.severity.as_str(),
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

const INLINE_CSS_MAX: u64 = 14 * 1024;

fn kb(bytes: u64) -> String {
    format!("{:.1} KB", bytes as f64 / 1024.0)
}

pub fn print_summary(report: &BuildReport) {
    let width = report.pages.iter().map(|p| p.pattern.chars().count()).max().unwrap_or(0).max(6);
    println!("{:width$}  {:>8}  {:>8}", "page", "JS gzip", "CSS gzip");
    for page in &report.pages {
        let blocking = if page.css_gzip_bytes > INLINE_CSS_MAX { "  > 14 KB: render-blocking stylesheet" } else { "" };
        println!("{:width$}  {:>8}  {:>8}{blocking}", page.pattern, kb(page.js_gzip_bytes), kb(page.css_gzip_bytes));
    }
    let server = if report.has_server { ", with server" } else { "" };
    println!("{} page(s){server}", report.pages.len());
}

/// OTel JSON record (one per line, as `.pragma/server/log.js` prints) to a readable line.
pub fn format_log(line: &str, paint: Paint) -> String {
    let Ok(Value::Object(rec)) = serde_json::from_str::<Value>(line) else { return line.to_owned() };
    let Some(level) = rec.get("severityText").and_then(Value::as_str) else { return line.to_owned() };
    let body = rec.get("body").map(text).unwrap_or_default();
    let painted = match level {
        "ERROR" | "FATAL" => paint.red(&format!("{level:5}")),
        "WARN" => paint.yellow(&format!("{level:5}")),
        "DEBUG" | "TRACE" => paint.dim(&format!("{level:5}")),
        _ => paint.cyan(&format!("{level:5}")),
    };
    let mut out = format!("{painted} {body}");
    let mut stack = None;
    for (key, value) in attributes(rec.get("attributes")) {
        if key == "exception.stacktrace" {
            stack = Some(value);
        } else {
            out.push_str(&paint.dim(&format!(" {key}={value}")));
        }
    }
    let service = match rec.get("resource") {
        Some(Value::Object(r)) => r.get("service.name").map(text),
        _ => None,
    };
    if let Some(service) = service {
        out.push_str(&paint.dim(&format!(" ({service})")));
    }
    if let Some(stack) = stack {
        out.push('\n');
        out.push_str(&paint.dim(&stack));
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
        println!("{}", format_log(&record.to_string(), out()));
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
        let formatted = format_log(line, out());
        assert!(formatted.contains("GET /shop"));
        assert!(formatted.contains("http.response.status_code=200"));
        assert!(formatted.contains("(app)"));
        assert_eq!(format_log("plain text", out()), "plain text");
    }
}
