//! Subconjunto fechado do Tailwind v4 + CSS base. Contrato: core/SPEC.md §8.

mod base;
mod doc;
mod table;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fmt::Write;
use std::sync::LazyLock;
use table::{
    BASE_COLORS, BREAKPOINTS, DARK_SEL, FRACTIONS, MARKERS, OPACITIES, PALETTE, PSEUDOS, RULES,
    SPACING, TONES, Vals,
};

#[derive(Debug, Default, Clone)]
pub struct Theme {
    pub colors: Vec<String>,
}

pub struct Generated {
    pub css: String,
    pub unknown: Vec<String>,
}

/// Lê as cores declaradas (`--color-<nome>: …`) num CSS de tema. Usos em `var()` não contam.
pub fn parse_theme(theme_css: &str) -> Theme {
    let css = strip_comments(theme_css);
    let mut found = BTreeSet::new();
    let mut rest = css.as_str();
    while let Some(i) = rest.find("--color-") {
        let after = &rest[i + "--color-".len()..];
        let len = after
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
            .unwrap_or(after.len());
        let name = &after[..len];
        let declared = after[len..].trim_start().starts_with(':');
        if declared && valid_color_name(name) {
            found.insert(name.to_string());
        }
        rest = &after[len..];
    }
    Theme {
        colors: found.into_iter().collect(),
    }
}

/// Classe com valor arbitrário (`w-[37px]`): o check reporta com regra própria.
pub fn is_arbitrary(class: &str) -> bool {
    class.contains('[') || class.contains(']')
}

pub fn is_utility(class: &str, theme: &Theme) -> bool {
    parse(class, &all_colors(theme)).is_some()
}

pub fn generate(classes: &[String], theme: &Theme) -> Generated {
    let colors = all_colors(theme);
    let mut utils = Vec::new();
    let mut unknown = BTreeSet::new();
    let unique: BTreeSet<&str> = classes.iter().map(String::as_str).collect();
    for class in unique {
        match parse(class, &colors) {
            Some(Some(u)) => utils.push(u),
            Some(None) => {}
            None => {
                unknown.insert(class.to_string());
            }
        }
    }
    utils.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| a.class.cmp(&b.class)));
    Generated {
        css: render(&utils),
        unknown: unknown.into_iter().collect(),
    }
}

pub fn base_css() -> &'static str {
    static BASE: LazyLock<String> = LazyLock::new(base::build);
    &BASE
}

pub fn classes_doc(theme: &Theme) -> String {
    doc::render(&all_colors(theme))
}

pub fn minify(css: &str) -> Result<String, String> {
    use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
    let mut sheet = StyleSheet::parse(css, ParserOptions::default())
        .map_err(|e| format!("CSS inválido: {e}"))?;
    sheet
        .minify(MinifyOptions::default())
        .map_err(|e| format!("falha ao minificar CSS: {e}"))?;
    let out = sheet
        .to_css(PrinterOptions {
            minify: true,
            ..PrinterOptions::default()
        })
        .map_err(|e| format!("falha ao gerar CSS: {e}"))?;
    Ok(out.code)
}

/// Paleta padrão (seção 16) seguida das cores novas do tema, em ordem alfabética.
fn all_colors(theme: &Theme) -> Vec<String> {
    let mut colors: Vec<String> = BASE_COLORS.iter().map(|c| c.to_string()).collect();
    for (name, _) in PALETTE {
        colors.extend(TONES.iter().map(|t| format!("{name}-{t}")));
    }
    let mut extra: Vec<&String> = theme
        .colors
        .iter()
        .filter(|c| valid_color_name(c) && !colors.contains(c))
        .collect();
    extra.sort();
    extra.dedup();
    colors.extend(extra.into_iter().cloned());
    colors
}

fn valid_color_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i + 2..]
            .find("*/")
            .map_or("", |j| &rest[i + 2 + j + 2..]);
    }
    out.push_str(rest);
    out
}

/// Ordem de saída: escuro, breakpoint, pseudo-variantes, regra da tabela, nome, valor, opacidade.
type Key = (bool, usize, u8, usize, usize, usize, usize);

struct Util {
    class: String,
    key: Key,
    selector: String,
    media: Vec<String>,
    decls: String,
}

/// `None` = desconhecida; `Some(None)` = conhecida sem CSS (marcadores).
fn parse(class: &str, colors: &[String]) -> Option<Option<Util>> {
    let mut parts: Vec<&str> = class.split(':').collect();
    let util = parts.pop()?;
    if parts.is_empty() && MARKERS.iter().any(|(m, _)| *m == util) {
        return Some(None);
    }
    let (mut dark, mut bp, mut mask) = (false, 0usize, 0u8);
    for v in parts {
        if v == "dark" && !dark {
            dark = true;
        } else if let Some(i) = BREAKPOINTS.iter().position(|(n, _)| *n == v) {
            if bp != 0 {
                return None;
            }
            bp = i + 1;
        } else {
            let i = PSEUDOS.iter().position(|(n, _)| *n == v)?;
            if mask & (1 << i) != 0 {
                return None;
            }
            mask |= 1 << i;
        }
    }
    let m = match_util(util, colors)?;

    let mut selector = format!(".{}", escape(class));
    for (i, (_, pseudo)) in PSEUDOS.iter().enumerate() {
        if mask & (1 << i) != 0 {
            selector.push_str(pseudo);
        }
    }
    if dark {
        selector.push_str(DARK_SEL);
    }
    let rule = &RULES[m.rule];
    let selector = rule.sel.replace('&', &selector);

    let mut media = Vec::new();
    if bp > 0 {
        media.push(format!("(width >= {})", BREAKPOINTS[bp - 1].1));
    }
    // Como no Tailwind v4: hover só em dispositivos com hover real (sem hover grudado no toque).
    if mask & 0b11 != 0 {
        media.push("(hover: hover)".to_string());
    }

    let decls = rule.names[m.name].1.replace('$', &m.value);
    Some(Some(Util {
        class: class.to_string(),
        key: (dark, bp, mask, m.rule, m.name, m.value_idx, m.opacity),
        selector,
        media,
        decls,
    }))
}

struct Match {
    rule: usize,
    name: usize,
    value_idx: usize,
    opacity: usize,
    value: String,
}

fn match_util(util: &str, colors: &[String]) -> Option<Match> {
    if let Some(m) = find(util, colors, None) {
        return Some(m);
    }
    let (base, op) = util.rsplit_once('/')?;
    let oi = OPACITIES.iter().position(|o| *o == op)?;
    find(base, colors, Some(oi))
}

fn find(util: &str, colors: &[String], opacity: Option<usize>) -> Option<Match> {
    for (ri, rule) in RULES.iter().enumerate() {
        for (ni, (name, _)) in rule.names.iter().enumerate() {
            let suffix = if name.is_empty() {
                util
            } else if util == *name {
                ""
            } else {
                match util.strip_prefix(name).and_then(|s| s.strip_prefix('-')) {
                    Some(s) => s,
                    None => continue,
                }
            };
            if let Some((value_idx, value)) = lookup(rule.vals, suffix, colors, opacity) {
                let opacity = opacity.map_or(0, |o| o + 1);
                return Some(Match {
                    rule: ri,
                    name: ni,
                    value_idx,
                    opacity,
                    value,
                });
            }
        }
    }
    None
}

fn lookup(
    vals: &[Vals],
    suffix: &str,
    colors: &[String],
    opacity: Option<usize>,
) -> Option<(usize, String)> {
    let mut base = 0;
    for v in vals {
        match v {
            Vals::Colors => {
                if let Some(i) = colors.iter().position(|c| c == suffix) {
                    return color_value(suffix, opacity).map(|val| (base + i, val));
                }
                base += colors.len();
            }
            _ if opacity.is_some() => {}
            Vals::Spacing => {
                if let Some(i) = SPACING.iter().position(|s| *s == suffix) {
                    return Some((base + i, spacing_value(suffix)));
                }
                base += SPACING.len();
            }
            Vals::Fractions => {
                if let Some(i) = FRACTIONS.iter().position(|(f, _)| *f == suffix) {
                    return Some((base + i, FRACTIONS[i].1.to_string()));
                }
                base += FRACTIONS.len();
            }
            Vals::Int(lo, hi, tpl) => {
                if let Ok(n) = suffix.parse::<u32>()
                    && (lo..=hi).contains(&&n)
                    && n.to_string() == suffix
                {
                    return Some((base + n as usize, tpl.replace('#', suffix)));
                }
                base += *hi as usize + 1;
            }
            Vals::List(list) => {
                if let Some(i) = list.iter().position(|(k, _)| *k == suffix) {
                    return Some((base + i, list[i].1.to_string()));
                }
                base += list.len();
            }
        }
    }
    None
}

fn spacing_value(s: &str) -> String {
    match s {
        "0" => "0".into(),
        "px" => "1px".into(),
        n => format!("calc(var(--spacing) * {n})"),
    }
}

fn color_value(name: &str, opacity: Option<usize>) -> Option<String> {
    let base = match name {
        "transparent" if opacity.is_some() => return None,
        "transparent" => "transparent".to_string(),
        "current" => "currentColor".to_string(),
        n => format!("var(--color-{n})"),
    };
    Some(match opacity {
        Some(o) => format!("color-mix(in oklab, {base} {}%, transparent)", OPACITIES[o]),
        None => base,
    })
}

/// Escapa um nome de classe para seletor CSS (`md:p-0.5` → `md\:p-0\.5`, `2xl:` → `\32 xl\:`).
fn escape(class: &str) -> String {
    let mut out = String::with_capacity(class.len() + 4);
    for (i, c) in class.chars().enumerate() {
        if i == 0 && c.is_ascii_digit() {
            let _ = write!(out, "\\{:x} ", c as u32);
        } else if c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii() {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
    out
}

fn render(utils: &[Util]) -> String {
    if utils.is_empty() {
        return String::new();
    }
    let mut out = String::from("@layer utilities {\n");
    let mut i = 0;
    while i < utils.len() {
        let media = &utils[i].media;
        let mut j = i;
        while j < utils.len() && utils[j].media == *media {
            j += 1;
        }
        let indent = if media.is_empty() {
            "  "
        } else {
            let _ = writeln!(out, "  @media {} {{", media.join(" and "));
            "    "
        };
        for u in &utils[i..j] {
            let _ = write!(out, "{indent}{} {{", u.selector);
            for d in u.decls.split(';') {
                let (prop, val) = d.split_once(':').unwrap_or((d, ""));
                let _ = write!(out, " {prop}: {val};");
            }
            out.push_str(" }\n");
        }
        if !media.is_empty() {
            out.push_str("  }\n");
        }
        i = j;
    }
    out.push_str("}\n");
    out
}
