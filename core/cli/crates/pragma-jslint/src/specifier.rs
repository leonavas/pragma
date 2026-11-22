pub fn is_node_builtin(specifier: &str) -> bool {
    specifier.starts_with("node:") || nodejs_built_in_modules::BUILTINS.binary_search(&specifier).is_ok()
}

pub fn node_builtin_message(specifier: &str) -> String {
    format!("`{specifier}` is a Node module and doesn't run in the browser; use it in a .server.js")
}

pub fn is_workers_module(specifier: &str) -> bool {
    specifier.starts_with("cloudflare:")
}

pub fn workers_module_message(specifier: &str) -> String {
    format!("`{specifier}` is a Cloudflare Workers module and doesn't run in the browser; use it in a .server.js")
}

pub fn is_package(specifier: &str) -> bool {
    let name = package_name(specifier);
    !name.is_empty()
        && !specifier.starts_with('.')
        && !specifier.starts_with('/')
        && !name.contains(':')
        && name
            .trim_start_matches('@')
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
}

pub fn package_name(specifier: &str) -> &str {
    let mut parts = specifier.splitn(3, '/');
    let first = parts.next().unwrap_or_default();
    if first.starts_with('@') {
        match parts.next() {
            Some(second) if !second.is_empty() => &specifier[..first.len() + 1 + second.len()],
            _ => "",
        }
    } else {
        first
    }
}
