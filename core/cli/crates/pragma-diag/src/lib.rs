use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub rule: &'static str,
    pub message: String,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn error(file: impl Into<PathBuf>, line: u32, column: u32, rule: &'static str, message: impl Into<String>) -> Self {
        Self { file: file.into(), line, column, rule, message: message.into(), severity: Severity::Error }
    }

    pub fn warning(file: impl Into<PathBuf>, line: u32, column: u32, rule: &'static str, message: impl Into<String>) -> Self {
        Self { file: file.into(), line, column, rule, message: message.into(), severity: Severity::Warning }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Error => "erro",
            Severity::Warning => "aviso",
        };
        write!(f, "{}:{}:{} {} [{}] {}", self.file.display(), self.line, self.column, level, self.rule, self.message)
    }
}
