//! Lightning CSS is a CSS parser, transformer, and minifier based on the
//! [cssparser](https://github.com/servo/rust-cssparser) crate used in Firefox.
//! It supports fully parsing all CSS rules, properties, and values into normalized
//! structures exactly how a browser would. Once parsed, the CSS can be transformed
//! to add or remove vendor prefixes, or lower syntax for older browsers as appropriate.
//! The style sheet can also be minified to merge longhand properties into shorthands,
//! merge adjacent rules, reduce `calc()` expressions, and more. Finally, the style sheet
//! can be printed back to CSS syntax, either minified to remove whitespace and compress
//! the output as much as possible, or pretty printed.
//!
//! The [StyleSheet](stylesheet::StyleSheet) struct is the main entrypoint for Lightning CSS,
//! and supports parsing and transforming entire CSS files. You can also parse and manipulate
//! individual CSS [rules](rules), [properties](properties), or [values](values).
//!
//! Vendored and trimmed for pragma: no bundler, sourcemap, serde, visitor or CLI.

#![deny(missing_docs)]

mod compat;
mod context;
pub mod css_modules;
pub mod declaration;
pub mod dependencies;
pub mod error;
mod logical;
mod macros;
pub mod media_query;
mod parser;
mod prefixes;
pub mod printer;
pub mod properties;
pub mod rules;
pub mod selector;
pub mod stylesheet;
pub mod targets;
pub mod traits;
pub mod values;
pub mod vendor_prefix;
