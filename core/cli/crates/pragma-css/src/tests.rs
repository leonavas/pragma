use super::*;

fn run(classes: &[&str]) -> Generated {
    run_with(classes, &Theme::default())
}

fn run_with(classes: &[&str], theme: &Theme) -> Generated {
    let classes: Vec<String> = classes.iter().map(|c| c.to_string()).collect();
    generate(&classes, theme)
}

/// Every class the table accepts (without variants).
fn every_class(colors: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for rule in RULES {
        let mut values: Vec<String> = Vec::new();
        for v in rule.vals {
            match v {
                Vals::Spacing => values.extend(SPACING.iter().map(|s| s.to_string())),
                Vals::Fractions => values.extend(FRACTIONS.iter().map(|(f, _)| f.to_string())),
                Vals::Colors => {
                    values.extend(colors.iter().cloned());
                    values.push("primary-500/50".into());
                }
                Vals::Int(lo, hi, _) => values.extend((*lo..=*hi).map(|n| n.to_string())),
                Vals::List(l) => values.extend(l.iter().map(|(k, _)| k.to_string())),
            }
        }
        for (name, _) in rule.names {
            for v in &values {
                out.push(match (name.is_empty(), v.is_empty()) {
                    (true, _) => v.clone(),
                    (false, true) => name.to_string(),
                    (false, false) => format!("{name}-{v}"),
                });
            }
        }
    }
    out
}

#[test]
fn known_classes_generate_expected_css() {
    let g = run(&[
        "p-4",
        "flex",
        "w-1/2",
        "p-0.5",
        "bg-primary-500",
        "text-sm",
        "mx-auto",
        "hidden",
    ]);
    assert!(g.unknown.is_empty(), "{:?}", g.unknown);
    let css = &g.css;
    assert!(css.starts_with("@layer utilities {\n"));
    assert!(
        css.contains(".p-4 { padding: calc(var(--spacing) * 4); }"),
        "{css}"
    );
    assert!(css.contains(".flex { display: flex; }"));
    assert!(css.contains(".hidden { display: none; }"));
    assert!(css.contains(r".w-1\/2 { width: 50%; }"));
    assert!(css.contains(r".p-0\.5 { padding: calc(var(--spacing) * 0.5); }"));
    assert!(css.contains(".bg-primary-500 { background-color: var(--color-primary-500); }"));
    assert!(css.contains(".mx-auto { margin-inline: auto; }"));
    assert!(css.contains(
        ".text-sm { font-size: var(--text-sm); line-height: var(--tw-leading, var(--text-sm--line-height)); }"
    ));
}

#[test]
fn opacity_and_special_colors() {
    let g = run(&[
        "bg-black/50",
        "text-current",
        "border-transparent",
        "ring-primary-500/30",
    ]);
    assert!(g.unknown.is_empty(), "{:?}", g.unknown);
    assert!(g.css.contains(r".bg-black\/50 { background-color: color-mix(in oklab, var(--color-black) 50%, transparent); }"));
    assert!(g.css.contains(".text-current { color: currentColor; }"));
    assert!(
        g.css
            .contains(".border-transparent { border-color: transparent; }")
    );
    assert!(g.css.contains(
        "--tw-ring-color: color-mix(in oklab, var(--color-primary-500) 30%, transparent)"
    ));
}

#[test]
fn unknown_and_arbitrary_are_rejected() {
    let bad = [
        "w-[37px]",
        "[mask-type:luminance]",
        "bg-[#fff]",
        "-mt-2",
        "-translate-x-1",
        "p-13",
        "p-",
        "foo",
        "bg-primary-500/15",
        "bg-primary-500/100",
        "bg-transparent/50",
        "w-1/2/50",
        "p-4/50",
        "hover:hover:p-4",
        "md:lg:p-4",
        "dark:dark:p-4",
        "print:p-4",
        "md:group",
        "hover:",
        "text-primary-1000",
        "grid-cols-13",
        "grid-cols-01",
        "z-60",
        "duration-500",
        "max-w-8xl",
        "bg-purple-500",
        "Flex",
        "p-4!",
    ];
    let g = run(&bad);
    assert_eq!(g.css, "");
    let mut expected: Vec<String> = bad.iter().map(|c| c.to_string()).collect();
    expected.sort();
    assert_eq!(g.unknown, expected);
    for c in bad {
        assert!(!is_utility(c, &Theme::default()), "{c}");
    }
    assert!(is_arbitrary("w-[37px]"));
    assert!(!is_arbitrary("w-1/2"));
}

#[test]
fn markers_are_known_without_css() {
    let g = run(&["group", "skeleton", "group-hover:underline"]);
    assert!(g.unknown.is_empty());
    assert!(!g.css.contains(".group {"));
    assert!(!g.css.contains(".skeleton"));
    assert!(
        g.css
            .contains(r".group-hover\:underline:is(:where(.group):hover *)")
    );
}

#[test]
fn variants() {
    let g = run(&[
        "hover:bg-primary-600",
        "focus-visible:ring-2",
        "disabled:opacity-50",
        "active:bg-primary-700",
        "focus:outline-none",
        "dark:bg-neutral-900",
        "dark:hover:text-white",
    ]);
    assert!(g.unknown.is_empty(), "{:?}", g.unknown);
    let css = &g.css;
    assert!(css.contains("  @media (hover: hover) {\n    .hover\\:bg-primary-600:hover { background-color: var(--color-primary-600); }"), "{css}");
    assert!(css.contains(r".focus-visible\:ring-2:focus-visible {"));
    assert!(css.contains(r".disabled\:opacity-50:disabled { opacity: 0.5; }"));
    assert!(css.contains(r".active\:bg-primary-700:active {"));
    assert!(css.contains(r".focus\:outline-none:focus { outline-style: none; }"));
    assert!(
        css.contains(
            r#".dark\:bg-neutral-900:where([data-theme="dark"], [data-theme="dark"] *) {"#
        )
    );
    assert!(css.contains(
        r#".dark\:hover\:text-white:hover:where([data-theme="dark"], [data-theme="dark"] *) {"#
    ));
}

#[test]
fn responsive_and_order() {
    let g = run(&[
        "md:p-4",
        "2xl:flex",
        "px-2",
        "p-2",
        "sm:p-1",
        "dark:p-8",
        "lg:hover:p-3",
        "pt-1",
    ]);
    assert!(g.unknown.is_empty(), "{:?}", g.unknown);
    let css = &g.css;
    let pos = |s: &str| css.find(s).unwrap_or_else(|| panic!("missing {s} in\n{css}"));
    assert!(css.contains("@media (width >= 48rem) {\n    .md\\:p-4 {"));
    assert!(
        css.contains("@media (width >= 64rem) and (hover: hover) {\n    .lg\\:hover\\:p-3:hover {")
    );
    assert!(css.contains(r".\32 xl\:flex { display: flex; }"));
    // shorthand before specific; base before increasing breakpoints; dark last
    assert!(pos(".p-2 ") < pos(".px-2 "));
    assert!(pos(".px-2 ") < pos(".pt-1 "));
    assert!(pos(".pt-1 ") < pos(r".sm\:p-1"));
    assert!(pos(r".sm\:p-1") < pos(r".md\:p-4"));
    assert!(pos(r".md\:p-4") < pos(r".lg\:hover\:p-3"));
    assert!(pos(r".lg\:hover\:p-3") < pos(r".\32 xl\:flex"));
    assert!(pos(r".\32 xl\:flex") < pos(r".dark\:p-8"));
}

#[test]
fn skip_link_not_sr_only_before_layout() {
    let g = run(&[
        "sr-only",
        "focus:not-sr-only",
        "focus:fixed",
        "focus:left-4",
        "focus:top-3",
        "focus:z-50",
        "focus:p-2",
    ]);
    assert!(g.unknown.is_empty(), "{:?}", g.unknown);
    let css = &g.css;
    let pos = |s: &str| css.find(s).unwrap_or_else(|| panic!("missing {s} in\n{css}"));
    assert!(pos(".sr-only ") < pos(r".focus\:not-sr-only"));
    for later in [r".focus\:fixed", r".focus\:left-4", r".focus\:top-3", r".focus\:z-50", r".focus\:p-2"] {
        assert!(pos(r".focus\:not-sr-only") < pos(later), "{later} must come after not-sr-only:\n{css}");
    }
}

#[test]
fn children_selectors() {
    let g = run(&[
        "space-y-4",
        "divide-y",
        "divide-neutral-200",
        "md:space-x-2",
    ]);
    assert!(g.unknown.is_empty());
    assert!(g.css.contains(
        ".space-y-4 > :not(:last-child)) { margin-block-end: calc(var(--spacing) * 4); }"
    ));
    assert!(g.css.contains(
        ":where(.divide-y > :not(:last-child)) { border-top-width: 0; border-bottom-width: 1px; }"
    ));
    assert!(
        g.css
            .contains(r":where(.md\:space-x-2 > :not(:last-child))")
    );
}

#[test]
fn theme_colors_are_recognized() {
    let theme = parse_theme(
        r#"/* --color-ignored: red; */
        :root {
          --color-brand: oklch(60% 0.2 30);
          --color-brand-500: #f50;
          --color-primary-500: #00f;
          --font-sans: "Inter";
          color: var(--color-used-only);
        }
        [data-theme="dark"] { --color-surface-2 : #111; }"#,
    );
    assert_eq!(
        theme.colors,
        ["brand", "brand-500", "primary-500", "surface-2"]
    );
    let g = run_with(
        &[
            "bg-brand",
            "text-brand-500/40",
            "border-surface-2",
            "dark:bg-brand",
            "bg-used-only",
            "bg-ignored",
        ],
        &theme,
    );
    assert_eq!(g.unknown, ["bg-ignored", "bg-used-only"]);
    assert!(
        g.css
            .contains(".bg-brand { background-color: var(--color-brand); }")
    );
    assert!(
        g.css
            .contains("color: color-mix(in oklab, var(--color-brand-500) 40%, transparent)")
    );
    assert!(is_utility("fill-brand", &theme));
    assert!(!is_utility("fill-brand", &Theme::default()));
    let doc = classes_doc(&theme);
    assert!(doc.contains("brand"), "the doc lists the theme colors");
    assert!(doc.contains("surface-2"));
}

#[test]
fn output_is_deterministic() {
    let a = [
        "md:flex",
        "p-4",
        "bg-primary-500",
        "hover:bg-primary-600",
        "dark:text-white",
        "w-full",
        "p-4",
        "x-y",
    ];
    let mut b = a;
    b.reverse();
    let ga = run(&a);
    let gb = run(&b);
    assert_eq!(ga.css, gb.css);
    assert_eq!(ga.unknown, gb.unknown);
    assert_eq!(ga.css.matches(".p-4 ").count(), 1);
}

#[test]
fn every_table_class_is_valid_and_minifies() {
    let colors = all_colors(&Theme::default());
    let all = every_class(&colors);
    assert!(all.len() > 1000);
    let mut with_variants = all.clone();
    with_variants.extend(all.iter().map(|c| format!("md:hover:{c}")));
    with_variants.extend(all.iter().map(|c| format!("dark:{c}")));
    let g = generate(&with_variants, &Theme::default());
    assert!(
        g.unknown.is_empty(),
        "{:?}",
        &g.unknown[..g.unknown.len().min(20)]
    );
    let min = minify(&g.css).expect("lightningcss accepts the output");
    assert!(min.contains("@layer utilities{"));
    // unique names: no class matches two rules ambiguously for the generator
    let unique: BTreeSet<&String> = all.iter().collect();
    assert_eq!(unique.len(), all.len(), "duplicate class in the table");
}

#[test]
fn base_css_has_layers_tokens_and_runtime_rules() {
    let css = base_css();
    let first_rule = css.lines().find(|l| !l.starts_with("/*")).unwrap();
    assert_eq!(
        first_rule,
        "@layer reset, tokens, base, components, utilities;"
    );
    assert!(css.contains("@layer reset {\n/*\n  Tailwind CSS v4.3.5 Preflight"));
    assert!(css.contains("MIT"));
    for needle in [
        "--color-white: #fff;",
        "--color-neutral-50:",
        "--color-primary-950:",
        "--color-danger-500:",
        "--color-success-500:",
        "--color-warning-500:",
        "--spacing: 0.25rem;",
        "--font-sans:",
        "--radius-md:",
        "--shadow-md:",
        "[data-theme=\"dark\"] {",
        "html[data-scroll-lock] {\n    overflow: hidden;\n  }\n  html[data-scroll-lock=\"gutter\"] {\n    scrollbar-gutter: stable;",
        "[hidden] {\n    display: none !important;",
        "dialog::backdrop {",
        ".skeleton {",
        "@media (prefers-reduced-motion: reduce) {\n    .skeleton {\n      animation: none;",
    ] {
        assert!(css.contains(needle), "missing {needle}");
    }
    let min = minify(css).expect("valid base.css");
    assert!(min.starts_with("/*!"), "license comment preserved");
    assert!(
        min.contains("html[data-scroll-lock]{overflow:hidden}html[data-scroll-lock=gutter]{scrollbar-gutter:stable}"),
        "{min}"
    );
    assert!(min.contains("[hidden]{display:none!important}"));
}

#[test]
fn every_used_token_is_defined() {
    let base = base_css();
    let colors = all_colors(&Theme::default());
    let g = generate(&every_class(&colors), &Theme::default());
    for name in crate::scan::var_names(&g.css) {
        if !name.starts_with("--tw-") {
            assert!(base.contains(&format!("{name}:")), "token {name} not defined in base.css");
        }
    }
}

#[test]
fn doc_is_compact_and_complete() {
    let doc = classes_doc(&Theme::default());
    for needle in [
        "`p{,x,y,t,r,b,l}-S`",
        "`m{,x,y,t,r,b,l}-{S,auto}`",
        "`space-{x,y}-S`",
        "`grid-cols-{1..12,none}`",
        "`border{,-x,-y,-t,-r,-b,-l}{,-0,-2,-4,-8}`",
        "`{text,bg,border,ring,divide,fill,stroke}-C`",
        "{neutral,primary,danger,success,warning}-{50,100,200,300,400,500,600,700,800,900,950}",
        "`md:` ≥48rem",
        "css.md",
        "w-[37px]",
        "`group`",
    ] {
        assert!(doc.contains(needle), "missing {needle} in\n{doc}");
    }
    assert!(doc.len() < 9000, "compact doc: {} bytes", doc.len());
}

#[test]
fn docs_classes_md_is_up_to_date() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../docs/classes.md");
    let on_disk = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        on_disk == classes_doc(&Theme::default()),
        "core/docs/classes.md is stale: run `cargo run -p pragma-css --example classes_doc`"
    );
}
