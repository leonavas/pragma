use crate::table::{PALETTE, TONES};
use std::fmt::Write;

const PREFLIGHT: &str = include_str!("../assets/preflight.css");

const LICENSE: &str = "/*! pragma base.css | includes Tailwind CSS v4 Preflight (MIT, Copyright (c) Tailwind Labs, Inc.) */\n";

const TOKENS_ROOT: &str = r#"    --spacing: 0.25rem;
    --font-sans: ui-sans-serif, system-ui, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol", "Noto Color Emoji";
    --font-serif: ui-serif, Georgia, Cambria, "Times New Roman", Times, serif;
    --font-mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    --default-font-family: var(--font-sans);
    --default-mono-font-family: var(--font-mono);
    --text-xs: 0.75rem;
    --text-xs--line-height: calc(1 / 0.75);
    --text-sm: 0.875rem;
    --text-sm--line-height: calc(1.25 / 0.875);
    --text-base: 1rem;
    --text-base--line-height: calc(1.5 / 1);
    --text-lg: 1.125rem;
    --text-lg--line-height: calc(1.75 / 1.125);
    --text-xl: 1.25rem;
    --text-xl--line-height: calc(1.75 / 1.25);
    --text-2xl: 1.5rem;
    --text-2xl--line-height: calc(2 / 1.5);
    --text-3xl: 1.875rem;
    --text-3xl--line-height: calc(2.25 / 1.875);
    --text-4xl: 2.25rem;
    --text-4xl--line-height: calc(2.5 / 2.25);
    --text-5xl: 3rem;
    --text-5xl--line-height: 1;
    --radius-xs: 0.125rem;
    --radius-sm: 0.25rem;
    --radius-md: 0.375rem;
    --radius-lg: 0.5rem;
    --radius-xl: 0.75rem;
    --radius-2xl: 1rem;
    --radius-3xl: 1.5rem;
    --shadow-2xs: 0 1px rgb(0 0 0 / 0.05);
    --shadow-xs: 0 1px 2px 0 rgb(0 0 0 / 0.05);
    --shadow-sm: 0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1);
    --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
    --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
    --shadow-xl: 0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1);
    --shadow-2xl: 0 25px 50px -12px rgb(0 0 0 / 0.25);
    --container-xs: 20rem;
    --container-sm: 24rem;
    --container-md: 28rem;
    --container-lg: 32rem;
    --container-xl: 36rem;
    --container-2xl: 42rem;
    --container-3xl: 48rem;
    --container-4xl: 56rem;
    --container-5xl: 64rem;
    --container-6xl: 72rem;
    --container-7xl: 80rem;
    --page-bg: var(--color-white);
    --page-fg: var(--color-neutral-900);
    --skeleton-bg: var(--color-neutral-200);
  }
"#;

// The palette does not change in dark mode (Tailwind semantics: `dark:` picks another tone); page
// surfaces and shadow strength do.
const TOKENS_DARK: &str = r#"  [data-theme="dark"] {
    color-scheme: dark;
    --page-bg: var(--color-neutral-950);
    --page-fg: var(--color-neutral-100);
    --skeleton-bg: var(--color-neutral-800);
    --shadow-2xs: 0 1px rgb(0 0 0 / 0.3);
    --shadow-xs: 0 1px 2px 0 rgb(0 0 0 / 0.3);
    --shadow-sm: 0 1px 3px 0 rgb(0 0 0 / 0.4), 0 1px 2px -1px rgb(0 0 0 / 0.4);
    --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.4), 0 2px 4px -2px rgb(0 0 0 / 0.4);
    --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.4), 0 4px 6px -4px rgb(0 0 0 / 0.4);
    --shadow-xl: 0 20px 25px -5px rgb(0 0 0 / 0.4), 0 8px 10px -6px rgb(0 0 0 / 0.4);
    --shadow-2xl: 0 25px 50px -12px rgb(0 0 0 / 0.6);
  }
}
"#;

const BASE: &str = r#"@layer base {
  html {
    background-color: var(--page-bg);
    color: var(--page-fg);
  }
  html[data-scroll-lock] {
    overflow: hidden;
  }
  html[data-scroll-lock="gutter"] {
    scrollbar-gutter: stable;
  }
  [hidden] {
    display: none !important;
  }
  [data-page]:focus {
    outline: none;
  }
  dialog::backdrop {
    background-color: rgb(0 0 0 / 0.5);
  }
  /* Chromium (GPU) can capture an empty old snapshot and paint it white: fade the new page in over the background */
  ::view-transition-old(root) {
    display: none;
  }
  ::view-transition-new(root) {
    mix-blend-mode: normal;
  }
  :tool-form-active {
    outline: 2px dashed var(--color-primary-500);
    outline-offset: 4px;
  }
  :tool-submit-active {
    outline: 2px dashed var(--color-primary-500);
    outline-offset: 2px;
  }
  .skeleton {
    display: block;
    min-height: 1em;
    border-radius: var(--radius-md);
    background-color: var(--skeleton-bg);
    animation: pragma-skeleton 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
  }
  @keyframes pragma-skeleton {
    50% {
      opacity: 0.5;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton {
      animation: none;
    }
  }
}
"#;

// No inheritance: the parent ring/shadow must not leak into children.
const PROPERTIES: &str = r#"@property --tw-ring-color {
  syntax: "*";
  inherits: false;
}
@property --tw-ring-shadow {
  syntax: "*";
  inherits: false;
  initial-value: 0 0 #0000;
}
@property --tw-shadow {
  syntax: "*";
  inherits: false;
  initial-value: 0 0 #0000;
}
@property --tw-leading {
  syntax: "*";
  inherits: false;
}
@property --tw-duration {
  syntax: "*";
  inherits: false;
}
"#;

pub fn build() -> String {
    let mut s = String::with_capacity(PREFLIGHT.len() + 12_000);
    s.push_str(LICENSE);
    s.push_str("@layer reset, tokens, base, components, utilities;\n\n@layer reset {\n");
    s.push_str(PREFLIGHT.trim_end());
    s.push_str("\n}\n\n@layer tokens {\n  :root {\n    color-scheme: light;\n");
    s.push_str("    --color-white: #fff;\n    --color-black: #000;\n");
    for (name, tones) in PALETTE {
        for (tone, value) in TONES.iter().zip(tones) {
            let _ = writeln!(s, "    --color-{name}-{tone}: {value};");
        }
    }
    s.push_str(TOKENS_ROOT);
    s.push_str(TOKENS_DARK);
    s.push('\n');
    s.push_str(BASE);
    s.push('\n');
    s.push_str(PROPERTIES);
    s
}
