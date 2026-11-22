use pragma_compiler::BuildReport;
use pragma_diag::{Diagnostic, Severity};
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
            Severity::Error => paint.red("error"),
            Severity::Warning => paint.yellow("warning"),
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
                "severity": match d.severity { Severity::Error => "error", Severity::Warning => "warning" },
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

pub fn kb(bytes: u64) -> String {
    format!("{:.1} KB", bytes as f64 / 1024.0)
}

pub fn print_summary(report: &BuildReport) {
    let width = report.pages.iter().map(|p| p.pattern.chars().count()).max().unwrap_or(0).max(6);
    println!("{:width$}  {:>8}", "page", "JS gzip");
    for page in &report.pages {
        println!("{:width$}  {:>8}", page.pattern, kb(page.js_gzip_bytes));
    }
    println!("{:width$}  {:>8}", "CSS", kb(report.css_gzip_bytes));
    let server = if report.has_server { ", with server" } else { "" };
    println!("{} page(s){server}", report.pages.len());
}
