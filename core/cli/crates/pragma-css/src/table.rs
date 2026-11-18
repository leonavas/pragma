//! Tabela única de utilitários: o gerador e a doc leem daqui.
//!
//! Cada regra tem nomes (prefixo, declarações com `$` no lugar do valor) e conjuntos de
//! valores. Classe = `prefixo-valor`; prefixo vazio = o valor é a classe; valor vazio = o
//! prefixo sozinho. A ordem da tabela é a ordem de saída do CSS: dentro de uma regra, os
//! atalhos (`p`) vêm antes dos específicos (`px`, `pt`) para que estes vençam.

pub enum Vals {
    Spacing,
    Fractions,
    Colors,
    Int(u32, u32, &'static str),
    List(&'static [(&'static str, &'static str)]),
}

pub struct Rule {
    pub cat: usize,
    pub names: &'static [(&'static str, &'static str)],
    pub vals: &'static [Vals],
    pub sel: &'static str,
    pub note: &'static str,
}

pub const CATS: &[&str] = &[
    "Layout",
    "Flex e grid",
    "Espaçamento",
    "Tamanho",
    "Tipografia",
    "Cor",
    "Borda e efeitos",
    "Interação e acessibilidade",
    "Transição e mídia",
];

#[rustfmt::skip]
pub const SPACING: &[&str] = &[
    "0", "px", "0.5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "5", "6", "7", "8", "9", "10",
    "11", "12", "14", "16", "20", "24", "28", "32", "36", "40", "44", "48", "52", "56", "60",
    "64", "72", "80", "96",
];

pub const FRACTIONS: &[(&str, &str)] = &[
    ("1/2", "50%"),
    ("1/3", "33.333333%"),
    ("2/3", "66.666667%"),
    ("1/4", "25%"),
    ("3/4", "75%"),
];

pub const OPACITIES: &[&str] = &["10", "20", "30", "40", "50", "60", "70", "80", "90"];

pub const BASE_COLORS: &[&str] = &["white", "black", "transparent", "current"];
pub const TONES: &[&str] = &[
    "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950",
];

// Tons do Tailwind v4: primary = blue, danger = red, success = green, warning = amber.
#[rustfmt::skip]
pub const PALETTE: &[(&str, [&str; 11])] = &[
    (
        "neutral",
        [
            "oklch(98.5% 0 0)", "oklch(97% 0 0)", "oklch(92.2% 0 0)", "oklch(87% 0 0)",
            "oklch(70.8% 0 0)", "oklch(55.6% 0 0)", "oklch(43.9% 0 0)", "oklch(37.1% 0 0)",
            "oklch(26.9% 0 0)", "oklch(20.5% 0 0)", "oklch(14.5% 0 0)",
        ],
    ),
    (
        "primary",
        [
            "oklch(97% 0.014 254.604)", "oklch(93.2% 0.032 255.585)", "oklch(88.2% 0.059 254.128)",
            "oklch(80.9% 0.105 251.813)", "oklch(70.7% 0.165 254.624)", "oklch(62.3% 0.214 259.815)",
            "oklch(54.6% 0.245 262.881)", "oklch(48.8% 0.243 264.376)", "oklch(42.4% 0.199 265.638)",
            "oklch(37.9% 0.146 265.522)", "oklch(28.2% 0.091 267.935)",
        ],
    ),
    (
        "danger",
        [
            "oklch(97.1% 0.013 17.38)", "oklch(93.6% 0.032 17.717)", "oklch(88.5% 0.062 18.334)",
            "oklch(80.8% 0.114 19.571)", "oklch(70.4% 0.191 22.216)", "oklch(63.7% 0.237 25.331)",
            "oklch(57.7% 0.245 27.325)", "oklch(50.5% 0.213 27.518)", "oklch(44.4% 0.177 26.899)",
            "oklch(39.6% 0.141 25.723)", "oklch(25.8% 0.092 26.042)",
        ],
    ),
    (
        "success",
        [
            "oklch(98.2% 0.018 155.826)", "oklch(96.2% 0.044 156.743)", "oklch(92.5% 0.084 155.995)",
            "oklch(87.1% 0.15 154.449)", "oklch(79.2% 0.209 151.711)", "oklch(72.3% 0.219 149.579)",
            "oklch(62.7% 0.194 149.214)", "oklch(52.7% 0.154 150.069)", "oklch(44.8% 0.119 151.328)",
            "oklch(39.3% 0.095 152.535)", "oklch(26.6% 0.065 152.934)",
        ],
    ),
    (
        "warning",
        [
            "oklch(98.7% 0.022 95.277)", "oklch(96.2% 0.059 95.617)", "oklch(92.4% 0.12 95.746)",
            "oklch(87.9% 0.169 91.605)", "oklch(82.8% 0.189 84.429)", "oklch(76.9% 0.188 70.08)",
            "oklch(66.6% 0.179 58.318)", "oklch(55.5% 0.163 48.998)", "oklch(47.3% 0.137 46.201)",
            "oklch(41.4% 0.112 45.904)", "oklch(27.9% 0.077 45.635)",
        ],
    ),
];

pub const BREAKPOINTS: &[(&str, &str)] = &[
    ("sm", "40rem"),
    ("md", "48rem"),
    ("lg", "64rem"),
    ("xl", "80rem"),
    ("2xl", "96rem"),
];

/// Pseudo-variantes na ordem de saída. `group-hover` usa o seletor do pai `.group`.
pub const PSEUDOS: &[(&str, &str)] = &[
    ("group-hover", ":is(:where(.group):hover *)"),
    ("hover", ":hover"),
    ("focus", ":focus"),
    ("focus-visible", ":focus-visible"),
    ("active", ":active"),
    ("disabled", ":disabled"),
];

pub const DARK_SEL: &str = r#":where([data-theme="dark"], [data-theme="dark"] *)"#;

/// Classes reconhecidas que não geram CSS de utilitário.
pub const MARKERS: &[(&str, &str)] = &[
    ("group", "marca o pai para `group-hover:`"),
    (
        "skeleton",
        "bloco pulsante de carregamento (vem do `base.css`)",
    ),
];

const CHILDREN: &str = ":where(& > :not(:last-child))";

const fn r(
    cat: usize,
    names: &'static [(&'static str, &'static str)],
    vals: &'static [Vals],
) -> Rule {
    Rule {
        cat,
        names,
        vals,
        sel: "&",
        note: "",
    }
}

const fn rn(
    cat: usize,
    names: &'static [(&'static str, &'static str)],
    vals: &'static [Vals],
    note: &'static str,
) -> Rule {
    Rule {
        cat,
        names,
        vals,
        sel: "&",
        note,
    }
}

use Vals::*;

const AUTO_FULL: Vals = List(&[("auto", "auto"), ("full", "100%")]);
const BORDER_W: Vals = List(&[
    ("", "1px"),
    ("0", "0"),
    ("2", "2px"),
    ("4", "4px"),
    ("8", "8px"),
]);
const GRID_TPL: &str = "repeat(#, minmax(0, 1fr))";

#[rustfmt::skip]
pub static RULES: &[Rule] = &[
    // Layout
    r(0, &[("", "display:$")], &[List(&[
        ("block", "block"), ("inline-block", "inline-block"), ("inline", "inline"),
        ("flex", "flex"), ("inline-flex", "inline-flex"), ("grid", "grid"),
        ("inline-grid", "inline-grid"), ("contents", "contents"), ("hidden", "none"),
    ])]),
    r(0, &[("", "position:$")], &[List(&[
        ("static", "static"), ("fixed", "fixed"), ("absolute", "absolute"),
        ("relative", "relative"), ("sticky", "sticky"),
    ])]),
    r(0, &[
        ("inset", "inset:$"), ("inset-x", "inset-inline:$"), ("inset-y", "inset-block:$"),
        ("top", "top:$"), ("right", "right:$"), ("bottom", "bottom:$"), ("left", "left:$"),
    ], &[Spacing, Fractions, AUTO_FULL]),
    r(0, &[("z", "z-index:$")], &[List(&[
        ("0", "0"), ("10", "10"), ("20", "20"), ("30", "30"), ("40", "40"), ("50", "50"), ("auto", "auto"),
    ])]),
    r(0, &[("overflow", "overflow:$"), ("overflow-x", "overflow-x:$"), ("overflow-y", "overflow-y:$")], &[List(&[
        ("auto", "auto"), ("hidden", "hidden"), ("clip", "clip"), ("visible", "visible"), ("scroll", "scroll"),
    ])]),
    r(0, &[("", "visibility:$")], &[List(&[("visible", "visible"), ("invisible", "hidden")])]),
    // Flex e grid
    r(1, &[("flex", "flex-direction:$")], &[List(&[
        ("row", "row"), ("row-reverse", "row-reverse"), ("col", "column"), ("col-reverse", "column-reverse"),
    ])]),
    r(1, &[("flex", "flex-wrap:$")], &[List(&[("wrap", "wrap"), ("nowrap", "nowrap"), ("wrap-reverse", "wrap-reverse")])]),
    r(1, &[("flex", "flex:$")], &[List(&[("1", "1"), ("auto", "auto"), ("initial", "0 auto"), ("none", "none")])]),
    r(1, &[("grow", "flex-grow:$"), ("shrink", "flex-shrink:$")], &[List(&[("", "1"), ("0", "0")])]),
    r(1, &[("basis", "flex-basis:$")], &[Spacing, Fractions, AUTO_FULL]),
    r(1, &[("grid-cols", "grid-template-columns:$")], &[Int(1, 12, GRID_TPL), List(&[("none", "none")])]),
    r(1, &[("grid-rows", "grid-template-rows:$")], &[Int(1, 6, GRID_TPL), List(&[("none", "none")])]),
    r(1, &[("grid-flow", "grid-auto-flow:$")], &[List(&[("row", "row"), ("col", "column"), ("dense", "dense")])]),
    r(1, &[("col-span", "grid-column:$")], &[Int(1, 12, "span # / span #"), List(&[("full", "1 / -1")])]),
    r(1, &[("row-span", "grid-row:$")], &[Int(1, 6, "span # / span #"), List(&[("full", "1 / -1")])]),
    r(1, &[("gap", "gap:$"), ("gap-x", "column-gap:$"), ("gap-y", "row-gap:$")], &[Spacing]),
    r(1, &[("items", "align-items:$")], &[List(&[
        ("start", "flex-start"), ("end", "flex-end"), ("center", "center"), ("baseline", "baseline"), ("stretch", "stretch"),
    ])]),
    r(1, &[("justify", "justify-content:$")], &[List(&[
        ("start", "flex-start"), ("end", "flex-end"), ("center", "center"),
        ("between", "space-between"), ("around", "space-around"), ("evenly", "space-evenly"),
    ])]),
    r(1, &[("justify-items", "justify-items:$")], &[List(&[
        ("start", "start"), ("end", "end"), ("center", "center"), ("stretch", "stretch"),
    ])]),
    r(1, &[("self", "align-self:$")], &[List(&[
        ("auto", "auto"), ("start", "flex-start"), ("end", "flex-end"), ("center", "center"),
        ("stretch", "stretch"), ("baseline", "baseline"),
    ])]),
    r(1, &[("place-items", "place-items:$")], &[List(&[
        ("start", "start"), ("end", "end"), ("center", "center"), ("stretch", "stretch"),
    ])]),
    // Espaçamento
    r(2, &[
        ("p", "padding:$"), ("px", "padding-inline:$"), ("py", "padding-block:$"), ("pt", "padding-top:$"),
        ("pr", "padding-right:$"), ("pb", "padding-bottom:$"), ("pl", "padding-left:$"),
    ], &[Spacing]),
    r(2, &[
        ("m", "margin:$"), ("mx", "margin-inline:$"), ("my", "margin-block:$"), ("mt", "margin-top:$"),
        ("mr", "margin-right:$"), ("mb", "margin-bottom:$"), ("ml", "margin-left:$"),
    ], &[Spacing, List(&[("auto", "auto")])]),
    Rule {
        cat: 2,
        names: &[("space-x", "margin-inline-end:$"), ("space-y", "margin-block-end:$")],
        vals: &[Spacing],
        sel: CHILDREN,
        note: "espaço entre filhos",
    },
    // Tamanho
    r(3, &[("size", "width:$;height:$")], &[Spacing, Fractions, AUTO_FULL, List(&[("fit", "fit-content")])]),
    r(3, &[("w", "width:$")], &[Spacing, Fractions, AUTO_FULL, List(&[
        ("screen", "100vw"), ("fit", "fit-content"), ("min", "min-content"), ("max", "max-content"),
    ])]),
    r(3, &[("min-w", "min-width:$")], &[Spacing, List(&[
        ("full", "100%"), ("fit", "fit-content"), ("min", "min-content"), ("max", "max-content"),
    ])]),
    r(3, &[("max-w", "max-width:$")], &[List(&[
        ("xs", "var(--container-xs)"), ("sm", "var(--container-sm)"), ("md", "var(--container-md)"),
        ("lg", "var(--container-lg)"), ("xl", "var(--container-xl)"), ("2xl", "var(--container-2xl)"),
        ("3xl", "var(--container-3xl)"), ("4xl", "var(--container-4xl)"), ("5xl", "var(--container-5xl)"),
        ("6xl", "var(--container-6xl)"), ("7xl", "var(--container-7xl)"), ("prose", "65ch"),
        ("full", "100%"), ("screen", "100vw"), ("fit", "fit-content"), ("none", "none"),
    ])]),
    r(3, &[("h", "height:$")], &[Spacing, Fractions, AUTO_FULL, List(&[
        ("screen", "100vh"), ("dvh", "100dvh"), ("svh", "100svh"), ("fit", "fit-content"),
        ("min", "min-content"), ("max", "max-content"),
    ])]),
    r(3, &[("min-h", "min-height:$")], &[Spacing, List(&[
        ("full", "100%"), ("screen", "100vh"), ("dvh", "100dvh"), ("svh", "100svh"), ("fit", "fit-content"),
    ])]),
    r(3, &[("max-h", "max-height:$")], &[Spacing, List(&[
        ("full", "100%"), ("screen", "100vh"), ("dvh", "100dvh"), ("fit", "fit-content"), ("none", "none"),
    ])]),
    // Tipografia
    r(4, &[("font", "font-family:$")], &[List(&[
        ("sans", "var(--font-sans)"), ("serif", "var(--font-serif)"), ("mono", "var(--font-mono)"),
    ])]),
    rn(4, &[("text", "$")], &[List(&[
        ("xs", "font-size:var(--text-xs);line-height:var(--tw-leading, var(--text-xs--line-height))"),
        ("sm", "font-size:var(--text-sm);line-height:var(--tw-leading, var(--text-sm--line-height))"),
        ("base", "font-size:var(--text-base);line-height:var(--tw-leading, var(--text-base--line-height))"),
        ("lg", "font-size:var(--text-lg);line-height:var(--tw-leading, var(--text-lg--line-height))"),
        ("xl", "font-size:var(--text-xl);line-height:var(--tw-leading, var(--text-xl--line-height))"),
        ("2xl", "font-size:var(--text-2xl);line-height:var(--tw-leading, var(--text-2xl--line-height))"),
        ("3xl", "font-size:var(--text-3xl);line-height:var(--tw-leading, var(--text-3xl--line-height))"),
        ("4xl", "font-size:var(--text-4xl);line-height:var(--tw-leading, var(--text-4xl--line-height))"),
        ("5xl", "font-size:var(--text-5xl);line-height:var(--tw-leading, var(--text-5xl--line-height))"),
    ])], "tamanho com altura de linha"),
    r(4, &[("font", "font-weight:$")], &[List(&[
        ("thin", "100"), ("extralight", "200"), ("light", "300"), ("normal", "400"), ("medium", "500"),
        ("semibold", "600"), ("bold", "700"), ("extrabold", "800"), ("black", "900"),
    ])]),
    r(4, &[("", "font-style:$")], &[List(&[("italic", "italic"), ("not-italic", "normal")])]),
    r(4, &[("leading", "--tw-leading:$;line-height:$")], &[List(&[
        ("none", "1"), ("tight", "1.25"), ("snug", "1.375"), ("normal", "1.5"), ("relaxed", "1.625"), ("loose", "2"),
    ])]),
    r(4, &[("tracking", "letter-spacing:$")], &[List(&[
        ("tighter", "-0.05em"), ("tight", "-0.025em"), ("normal", "0em"), ("wide", "0.025em"),
        ("wider", "0.05em"), ("widest", "0.1em"),
    ])]),
    r(4, &[("text", "text-align:$")], &[List(&[
        ("left", "left"), ("center", "center"), ("right", "right"), ("justify", "justify"),
        ("start", "start"), ("end", "end"),
    ])]),
    r(4, &[("", "text-transform:$")], &[List(&[
        ("uppercase", "uppercase"), ("lowercase", "lowercase"), ("capitalize", "capitalize"), ("normal-case", "none"),
    ])]),
    r(4, &[("", "text-decoration-line:$")], &[List(&[
        ("underline", "underline"), ("line-through", "line-through"), ("no-underline", "none"),
    ])]),
    r(4, &[("", "font-variant-numeric:$")], &[List(&[("tabular-nums", "tabular-nums")])]),
    r(4, &[("whitespace", "white-space:$")], &[List(&[
        ("normal", "normal"), ("nowrap", "nowrap"), ("pre", "pre"), ("pre-line", "pre-line"), ("pre-wrap", "pre-wrap"),
    ])]),
    r(4, &[("break", "$")], &[List(&[("words", "overflow-wrap:break-word"), ("all", "word-break:break-all")])]),
    r(4, &[("", "$")], &[List(&[("truncate", "overflow:hidden;text-overflow:ellipsis;white-space:nowrap")])]),
    r(4, &[("line-clamp", "overflow:hidden;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:$")], &[Int(1, 3, "#")]),
    r(4, &[("list", "list-style-type:$")], &[List(&[("none", "none"), ("disc", "disc"), ("decimal", "decimal")])]),
    // Cor
    r(5, &[("text", "color:$")], &[Colors]),
    r(5, &[("bg", "background-color:$")], &[Colors]),
    r(5, &[("border", "border-color:$")], &[Colors]),
    r(5, &[("ring", "--tw-ring-color:$")], &[Colors]),
    Rule { cat: 5, names: &[("divide", "border-color:$")], vals: &[Colors], sel: CHILDREN, note: "" },
    r(5, &[("fill", "fill:$"), ("stroke", "stroke:$")], &[Colors]),
    // Borda e efeitos
    r(6, &[
        ("border", "border-width:$"), ("border-x", "border-inline-width:$"), ("border-y", "border-block-width:$"),
        ("border-t", "border-top-width:$"), ("border-r", "border-right-width:$"),
        ("border-b", "border-bottom-width:$"), ("border-l", "border-left-width:$"),
    ], &[BORDER_W]),
    r(6, &[("border", "border-style:$")], &[List(&[
        ("solid", "solid"), ("dashed", "dashed"), ("dotted", "dotted"), ("none", "none"),
    ])]),
    Rule {
        cat: 6,
        names: &[
            ("divide-x", "border-inline-start-width:0;border-inline-end-width:$"),
            ("divide-y", "border-top-width:0;border-bottom-width:$"),
        ],
        vals: &[BORDER_W],
        sel: CHILDREN,
        note: "borda entre filhos",
    },
    r(6, &[
        ("rounded", "border-radius:$"),
        ("rounded-t", "border-top-left-radius:$;border-top-right-radius:$"),
        ("rounded-r", "border-top-right-radius:$;border-bottom-right-radius:$"),
        ("rounded-b", "border-bottom-right-radius:$;border-bottom-left-radius:$"),
        ("rounded-l", "border-top-left-radius:$;border-bottom-left-radius:$"),
    ], &[List(&[
        ("", "var(--radius-sm)"), ("none", "0"), ("xs", "var(--radius-xs)"), ("sm", "var(--radius-sm)"),
        ("md", "var(--radius-md)"), ("lg", "var(--radius-lg)"), ("xl", "var(--radius-xl)"),
        ("2xl", "var(--radius-2xl)"), ("3xl", "var(--radius-3xl)"), ("full", "9999px"),
    ])]),
    rn(6, &[("ring", "--tw-ring-shadow:0 0 0 $ var(--tw-ring-color, currentColor);box-shadow:var(--tw-ring-shadow, 0 0 #0000), var(--tw-shadow, 0 0 #0000)")],
        &[List(&[("", "1px"), ("0", "0px"), ("1", "1px"), ("2", "2px"), ("4", "4px"), ("8", "8px")])],
        "anel via box-shadow; cor com `ring-C`"),
    r(6, &[("shadow", "--tw-shadow:$;box-shadow:var(--tw-ring-shadow, 0 0 #0000), var(--tw-shadow, 0 0 #0000)")], &[List(&[
        ("", "var(--shadow-sm)"), ("2xs", "var(--shadow-2xs)"), ("xs", "var(--shadow-xs)"),
        ("sm", "var(--shadow-sm)"), ("md", "var(--shadow-md)"), ("lg", "var(--shadow-lg)"),
        ("xl", "var(--shadow-xl)"), ("2xl", "var(--shadow-2xl)"), ("none", "0 0 #0000"),
    ])]),
    r(6, &[("opacity", "opacity:$")], &[List(&[
        ("0", "0"), ("5", "0.05"), ("10", "0.1"), ("20", "0.2"), ("25", "0.25"), ("30", "0.3"),
        ("40", "0.4"), ("50", "0.5"), ("60", "0.6"), ("70", "0.7"), ("75", "0.75"), ("80", "0.8"),
        ("90", "0.9"), ("95", "0.95"), ("100", "1"),
    ])]),
    r(6, &[("outline", "$")], &[List(&[
        ("none", "outline-style:none"), ("hidden", "outline:2px solid transparent;outline-offset:2px"),
    ])]),
    // Interação e acessibilidade
    r(7, &[("cursor", "cursor:$")], &[List(&[
        ("auto", "auto"), ("default", "default"), ("pointer", "pointer"), ("wait", "wait"), ("text", "text"),
        ("move", "move"), ("not-allowed", "not-allowed"), ("grab", "grab"), ("grabbing", "grabbing"),
    ])]),
    r(7, &[("pointer-events", "pointer-events:$")], &[List(&[("none", "none"), ("auto", "auto")])]),
    r(7, &[("select", "-webkit-user-select:$;user-select:$")], &[List(&[
        ("none", "none"), ("text", "text"), ("all", "all"), ("auto", "auto"),
    ])]),
    r(7, &[("", "$")], &[List(&[
        ("sr-only", "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap;border-width:0"),
        ("not-sr-only", "position:static;width:auto;height:auto;padding:0;margin:0;overflow:visible;clip-path:none;white-space:normal"),
    ])]),
    // Transição e mídia
    r(8, &[("transition", "transition-property:$;transition-timing-function:cubic-bezier(0.4, 0, 0.2, 1);transition-duration:var(--tw-duration, 150ms)")], &[List(&[
        ("", "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, opacity, box-shadow, transform, translate, scale, rotate, filter, backdrop-filter"),
        ("all", "all"),
        ("colors", "color, background-color, border-color, outline-color, text-decoration-color, fill, stroke"),
        ("opacity", "opacity"),
        ("shadow", "box-shadow"),
        ("transform", "transform, translate, scale, rotate"),
        ("none", "none"),
    ])]),
    r(8, &[("duration", "--tw-duration:$;transition-duration:$")], &[List(&[
        ("150", "150ms"), ("200", "200ms"), ("300", "300ms"),
    ])]),
    r(8, &[("aspect", "aspect-ratio:$")], &[List(&[("auto", "auto"), ("square", "1 / 1"), ("video", "16 / 9")])]),
    r(8, &[("object", "object-fit:$")], &[List(&[
        ("contain", "contain"), ("cover", "cover"), ("fill", "fill"), ("none", "none"), ("scale-down", "scale-down"),
    ])]),
    r(8, &[("object", "object-position:$")], &[List(&[
        ("center", "center"), ("top", "top"), ("bottom", "bottom"), ("left", "left"), ("right", "right"),
    ])]),
];
