//! Regression tests: a standard declaration must never be dropped when a vendor-prefixed
//! declaration of the same property follows it (lightningcss 1.0.0-alpha.72,
//! `FallbackHandler` / `PrefixHandler` in src/properties/prefix_handler.rs).
//!
//! All tests are `#[ignore]`: run them with `cargo test -- --ignored`.

use lightningcss::stylesheet::{MinifyOptions, ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::targets::{Browsers, Features, Targets};

/// pragma targets: Chrome/Edge 111, Firefox 128, Safari/iOS 16.4, only VendorPrefixes enabled.
fn pragma() -> Targets {
  let v = |major: u32, minor: u32| Some((major << 16) | (minor << 8));
  Targets {
    browsers: Some(Browsers {
      chrome: v(111, 0),
      edge: v(111, 0),
      firefox: v(128, 0),
      safari: v(16, 4),
      ios_saf: v(16, 4),
      ..Browsers::default()
    }),
    include: Features::empty(),
    exclude: Features::all().difference(Features::VendorPrefixes),
  }
}

/// No targets (`Targets::default()`, browsers: None).
fn none() -> Targets {
  Targets::default()
}

fn minify(css: &str, targets: Targets) -> String {
  let mut sheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
  sheet
    .minify(MinifyOptions {
      targets,
      ..MinifyOptions::default()
    })
    .unwrap();
  sheet
    .to_css(PrinterOptions {
      minify: true,
      targets,
      ..PrinterOptions::default()
    })
    .unwrap()
    .code
}

#[track_caller]
fn check(css: &str, targets: Targets, expected: &str) {
  assert_eq!(minify(css, targets), expected, "input: {css}");
}

// ---------------------------------------------------------------- backdrop-filter

#[test]
#[ignore]
fn backdrop_filter_standard_then_prefixed() {
  let css = "a{backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(4px)}";
  let out = "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn backdrop_filter_prefixed_then_standard() {
  let css = "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}";
  let out = "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn backdrop_filter_only_prefixed() {
  let css = "a{-webkit-backdrop-filter:blur(4px)}";
  check(css, pragma(), "a{-webkit-backdrop-filter:blur(4px)}");
  check(css, none(), "a{-webkit-backdrop-filter:blur(4px)}");
}

#[test]
#[ignore]
fn backdrop_filter_only_standard() {
  let css = "a{backdrop-filter:blur(4px)}";
  check(css, pragma(), "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}");
  check(css, none(), "a{backdrop-filter:blur(4px)}");
}

#[test]
#[ignore]
fn backdrop_filter_differing_values_standard_first() {
  let css = "a{backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(8px)}";
  check(
    css,
    pragma(),
    "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(8px)}",
  );
  check(css, none(), "a{backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(8px)}");
}

#[test]
#[ignore]
fn backdrop_filter_differing_values_prefixed_first() {
  let css = "a{-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(4px)}";
  // With targets, the standard declaration is expanded to -webkit- too and overrides it
  // (unchanged lightningcss behaviour).
  check(css, pragma(), "a{-webkit-backdrop-filter:blur(4px);backdrop-filter:blur(4px)}");
  check(css, none(), "a{-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(4px)}");
}

#[test]
#[ignore]
fn backdrop_filter_same_prefix_still_dedupes() {
  check(
    "a{backdrop-filter:blur(4px);backdrop-filter:blur(8px)}",
    pragma(),
    "a{-webkit-backdrop-filter:blur(8px);backdrop-filter:blur(8px)}",
  );
  check("a{backdrop-filter:blur(4px);backdrop-filter:blur(8px)}", none(), "a{backdrop-filter:blur(8px)}");
  check(
    "a{-webkit-backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(8px)}",
    none(),
    "a{-webkit-backdrop-filter:blur(8px)}",
  );
}

#[test]
#[ignore]
fn backdrop_filter_var_standard_then_prefixed() {
  let css = "a{backdrop-filter:var(--b);-webkit-backdrop-filter:var(--b)}";
  let out = "a{-webkit-backdrop-filter:var(--b);backdrop-filter:var(--b)}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn backdrop_filter_var_differing_values() {
  let css = "a{backdrop-filter:var(--a);-webkit-backdrop-filter:var(--b)}";
  check(
    css,
    pragma(),
    "a{-webkit-backdrop-filter:var(--a);backdrop-filter:var(--a);-webkit-backdrop-filter:var(--b)}",
  );
  check(css, none(), "a{backdrop-filter:var(--a);-webkit-backdrop-filter:var(--b)}");
}

// ---------------------------------------------------------------- filter (same handler)

#[test]
#[ignore]
fn filter_standard_then_prefixed() {
  let css = "a{filter:blur(4px);-webkit-filter:blur(4px)}";
  // pragma targets do not need -webkit-filter, so the merged declaration drops the prefix.
  check(css, pragma(), "a{filter:blur(4px)}");
  check(css, none(), "a{-webkit-filter:blur(4px);filter:blur(4px)}");
}

#[test]
#[ignore]
fn filter_differing_values() {
  let css = "a{filter:blur(4px);-webkit-filter:blur(8px)}";
  check(css, pragma(), "a{filter:blur(4px);-webkit-filter:blur(8px)}");
  check(css, none(), "a{filter:blur(4px);-webkit-filter:blur(8px)}");
}

// ---------------------------------------------------------------- PrefixHandler properties

#[test]
#[ignore]
fn user_select_same_value() {
  let css = "a{user-select:none;-webkit-user-select:none}";
  let out = "a{-webkit-user-select:none;user-select:none}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn user_select_differing_values_keeps_standard() {
  let css = "a{user-select:none;-webkit-user-select:text}";
  check(css, pragma(), "a{-webkit-user-select:none;user-select:none;-webkit-user-select:text}");
  check(css, none(), "a{user-select:none;-webkit-user-select:text}");
}

#[test]
#[ignore]
fn user_select_same_prefix_still_dedupes() {
  check(
    "a{-webkit-user-select:none;-webkit-user-select:text}",
    none(),
    "a{-webkit-user-select:text}",
  );
  check(
    "a{-webkit-user-select:none;user-select:text}",
    pragma(),
    "a{-webkit-user-select:text;user-select:text}",
  );
}

#[test]
#[ignore]
fn text_size_adjust_standard_then_prefixed() {
  let css = "a{text-size-adjust:100%;-webkit-text-size-adjust:100%}";
  check(
    css,
    pragma(),
    "a{-webkit-text-size-adjust:100%;-moz-text-size-adjust:100%;text-size-adjust:100%}",
  );
  check(css, none(), "a{-webkit-text-size-adjust:100%;text-size-adjust:100%}");
}

#[test]
#[ignore]
fn text_size_adjust_differing_values_keeps_standard() {
  let css = "a{text-size-adjust:100%;-webkit-text-size-adjust:none}";
  check(
    css,
    pragma(),
    "a{-webkit-text-size-adjust:100%;-moz-text-size-adjust:100%;text-size-adjust:100%;-webkit-text-size-adjust:none}",
  );
  check(css, none(), "a{text-size-adjust:100%;-webkit-text-size-adjust:none}");
}

#[test]
#[ignore]
fn appearance_standard_then_prefixed() {
  let css = "a{appearance:none;-webkit-appearance:none}";
  check(css, pragma(), "a{appearance:none}");
  check(css, none(), "a{-webkit-appearance:none;appearance:none}");
}

// ---------------------------------------------------------------- other handlers (not affected, guard)

#[test]
#[ignore]
fn mask_image_standard_then_prefixed() {
  let css = "a{mask-image:url(a.svg);-webkit-mask-image:url(a.svg)}";
  let out = "a{-webkit-mask-image:url(a.svg);mask-image:url(a.svg)}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn mask_shorthand_standard_then_prefixed() {
  let css = "a{mask:url(a.svg) no-repeat;-webkit-mask:url(a.svg) no-repeat}";
  let out = "a{-webkit-mask:url(a.svg) no-repeat;mask:url(a.svg) no-repeat}";
  check(css, pragma(), out);
  check(css, none(), out);
}

#[test]
#[ignore]
fn background_clip_text_both_orders() {
  let out = "a{-webkit-background-clip:text;background-clip:text}";
  for css in [
    "a{background-clip:text;-webkit-background-clip:text}",
    "a{-webkit-background-clip:text;background-clip:text}",
  ] {
    check(css, pragma(), out);
    check(css, none(), out);
  }
}

#[test]
#[ignore]
fn position_sticky_both_orders() {
  for css in [
    "a{position:sticky;position:-webkit-sticky}",
    "a{position:-webkit-sticky;position:sticky}",
  ] {
    check(css, pragma(), "a{position:sticky}");
    check(css, none(), "a{position:-webkit-sticky;position:sticky}");
  }
}

#[test]
#[ignore]
fn unprefixed_fallback_property_still_dedupes() {
  check("a{color:red;color:blue}", pragma(), "a{color:#00f}");
  check("a{color:red;color:blue}", none(), "a{color:#00f}");
}
