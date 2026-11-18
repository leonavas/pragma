use crate::table::{
    BREAKPOINTS, CATS, FRACTIONS, MARKERS, OPACITIES, PSEUDOS, RULES, Rule, SPACING, Vals,
};
use std::fmt::Write;

const HEADER: &str = r#"# Classes utilitárias

Gerado por `pragma-css` (`cargo run -p pragma-css --example classes_doc`); não edite à mão.

Subconjunto fechado do Tailwind v4: mesmos nomes, mesmo efeito. Classe fora desta lista é erro
no `pragma check`.

## Regras

- Proibido: valor arbitrário (`w-[37px]`), negativo (`-mt-2`) e qualquer classe fora da lista.
- Combinação que se repete em 3+ lugares, ou que precisa de algo fora da lista: crie uma classe
  custom. Global em `lib/theme.css` (`@layer components`); de componente em `<nome>.css`, com
  escopo `[data-c="nome"]`.
- Cores novas: declare `--color-<nome>` ou `--color-<nome>-<tom>` em `lib/theme.css`; elas
  entram em `C` (`bg-<nome>`, `text-<nome>-<tom>`…).
- Tokens redefiníveis em `lib/theme.css`: `--color-*`, `--font-{sans,serif,mono}`, `--radius-*`,
  `--shadow-*`, `--spacing`.
- Escuro: `[data-theme="dark"]` no `<html>`. A paleta não muda; troque o tom com `dark:`
  (`bg-white dark:bg-neutral-900`).

## Notação

`{a,b}` = alternativas; `{,-a}` = sem sufixo ou `-a`; `{1..12}` = inteiros.

"#;

pub fn render(colors: &[String]) -> String {
    let mut s = String::from(HEADER);
    let _ = writeln!(
        s,
        "- `S` espaçamento (n × 0.25rem; `px` = 1px): `{}`",
        SPACING.join(" ")
    );
    let fr: Vec<&str> = FRACTIONS.iter().map(|(f, _)| *f).collect();
    let _ = writeln!(s, "- `F` frações: `{}`", fr.join(" "));
    let _ = writeln!(s, "- `C` cores: `{}`", compress_colors(colors));
    let _ = writeln!(
        s,
        "- Opacidade em qualquer cor: `C/{{{}}}` (ex.: `bg-primary-500/50`)",
        OPACITIES.join(",")
    );

    s.push_str("\n## Variantes\n\n");
    let pseudos: Vec<String> = PSEUDOS.iter().map(|(p, _)| format!("`{p}:`")).collect();
    let _ = writeln!(
        s,
        "- Estado: {} (`group-hover:` reage ao pai com `group`)",
        pseudos.join(" ")
    );
    s.push_str("- Tema: `dark:`\n");
    let bps: Vec<String> = BREAKPOINTS
        .iter()
        .map(|(n, w)| format!("`{n}:` ≥{w}"))
        .collect();
    let _ = writeln!(s, "- Responsivo, mobile first: {}", bps.join(", "));
    s.push_str(
        "- Combináveis, cada uma no máximo uma vez e um só breakpoint: `md:hover:bg-primary-600`\n",
    );

    for (ci, cat) in CATS.iter().enumerate() {
        let _ = write!(s, "\n## {cat}\n\n");
        // Regras vizinhas com os mesmos valores viram uma linha: `{text,bg,border}-C`.
        let mut lines: Vec<(Vec<&str>, String, &str)> = Vec::new();
        for rule in RULES.iter().filter(|r| r.cat == ci) {
            let (names, values) = pattern(rule);
            match lines.last_mut() {
                Some((n, v, note))
                    if !names.is_empty()
                        && !n.is_empty()
                        && *v == values
                        && note.is_empty()
                        && rule.note.is_empty() =>
                {
                    n.extend(names)
                }
                _ => lines.push((names, values, rule.note)),
            }
        }
        for (names, values, note) in lines {
            let _ = write!(
                s,
                "- `{}{values}`",
                if names.is_empty() {
                    String::new()
                } else {
                    alternatives(&names)
                }
            );
            if !note.is_empty() {
                let _ = write!(s, " — {note}");
            }
            s.push('\n');
        }
    }

    s.push_str("\n## Outras classes reconhecidas\n\n");
    for (m, what) in MARKERS {
        let _ = writeln!(s, "- `{m}` — {what}");
    }
    s
}

/// (nomes, padrão dos valores); estáticos (prefixo vazio) vêm sem nomes.
fn pattern(rule: &Rule) -> (Vec<&'static str>, String) {
    let names: Vec<&'static str> = rule.names.iter().map(|(n, _)| *n).collect();
    if names == [""] {
        let mut items = Vec::new();
        for v in rule.vals {
            if let Vals::List(list) = v {
                items.extend(list.iter().map(|(k, _)| *k));
            }
        }
        return (Vec::new(), items.join(" "));
    }

    let mut bare = false;
    let mut tokens = Vec::new();
    for v in rule.vals {
        match v {
            Vals::Spacing => tokens.push("S".to_string()),
            Vals::Fractions => tokens.push("F".to_string()),
            Vals::Colors => tokens.push("C".to_string()),
            Vals::Int(lo, hi, _) => tokens.push(format!("{lo}..{hi}")),
            Vals::List(list) => {
                for (k, _) in *list {
                    if k.is_empty() {
                        bare = true;
                    } else {
                        tokens.push(k.to_string());
                    }
                }
            }
        }
    }
    let values = if bare {
        let t: Vec<String> = tokens.iter().map(|t| format!("-{t}")).collect();
        format!("{{,{}}}", t.join(","))
    } else if tokens.len() == 1 && !tokens[0].contains("..") {
        format!("-{}", tokens[0])
    } else {
        format!("-{{{}}}", tokens.join(","))
    };
    (names, values)
}

fn alternatives(items: &[&str]) -> String {
    if items.len() == 1 {
        return items[0].to_string();
    }
    let first = items[0];
    let lcp = items.iter().fold(first.len(), |n, it| {
        first
            .bytes()
            .zip(it.bytes())
            .take(n)
            .take_while(|(a, b)| a == b)
            .count()
    });
    let rest: Vec<&str> = items.iter().map(|it| &it[lcp..]).collect();
    format!("{}{{{}}}", &first[..lcp], rest.join(","))
}

/// `white black … {neutral,primary}-{50,…,950} brand`: agrupa famílias com os mesmos tons.
fn compress_colors(colors: &[String]) -> String {
    let mut families: Vec<(&str, Vec<&str>)> = Vec::new();
    let mut plain: Vec<&str> = Vec::new();
    for c in colors {
        match c.rsplit_once('-') {
            Some((base, tone)) if tone.bytes().all(|b| b.is_ascii_digit()) => {
                match families.iter_mut().find(|(b, _)| *b == base) {
                    Some((_, tones)) => tones.push(tone),
                    None => families.push((base, vec![tone])),
                }
            }
            _ => plain.push(c),
        }
    }
    let mut groups: Vec<(Vec<&str>, Vec<&str>)> = Vec::new();
    for (base, tones) in families {
        match groups.iter_mut().find(|(t, _)| *t == tones) {
            Some((_, bases)) => bases.push(base),
            None => groups.push((tones, vec![base])),
        }
    }
    let mut out: Vec<String> = plain.iter().map(|p| p.to_string()).collect();
    for (tones, bases) in groups {
        let tones = if tones.len() == 1 {
            tones[0].to_string()
        } else {
            format!("{{{}}}", tones.join(","))
        };
        let bases = if bases.len() == 1 {
            bases[0].to_string()
        } else {
            format!("{{{}}}", bases.join(","))
        };
        out.push(format!("{bases}-{tones}"));
    }
    out.join(" ")
}
