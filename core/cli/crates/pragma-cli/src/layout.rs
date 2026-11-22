/// Names never copied into or read from `.pragma/`.
pub const SKIPPED_NAMES: [&str; 3] = ["node_modules", ".git", ".DS_Store"];

/// Output of `pragma dev`, inside `.pragma/` but not part of it.
pub const DEV_OUT: &str = ".pragma/dev";
