# vendor

Forks maintained in this repo. Upstream is not tracked.

| Crate | Origin | Used by |
|---|---|---|
| `lightningcss` | lightningcss 1.0.0-alpha.72 | `pragma-css` (`minify`, `targets`) |
| `lightningcss-derive` | lightningcss-derive 1.0.0-alpha.43 | `lightningcss` |
| `parcel_selectors` | parcel_selectors 0.28.3 | `lightningcss` |

`cssparser` and `cssparser-color` stay on crates.io.

Removed from upstream: features `bundler`, `sourcemap`, `browserslist`, `nodejs`,
`serde`, `jsonschema`, `into_owned`, `visitor`, `substitute_variables`, `cli`,
`bench`, and all code behind them; `main.rs`, `bundler.rs`, `visitor.rs`,
`serialization.rs`, `test_helpers.rs`, the `Visit` derive, the upstream tests, benches, examples and
the node/wasm packaging.

Kept (still in the code, not used by pragma): CSS modules and dependency analysis.

License: MPL-2.0 (`lightningcss/LICENSE`, `parcel_selectors/LICENSE`).

## Patches

- `lightningcss/src/properties/prefix_handler.rs`: a vendor-prefixed declaration never
  replaces an earlier declaration of the same property with other prefixes.
  `backdrop-filter:x;-webkit-backdrop-filter:x` gave only `-webkit-backdrop-filter`
  (v3chat: no blur in Chrome and Firefox). Now: same value → prefixes merged and filtered
  by targets; different value → both kept in source order. Applies to `FallbackHandler`
  (`backdrop-filter`, `filter`, also with `var()`) and `PrefixHandler` (`user-select`,
  `text-size-adjust`, `appearance`, …).

## Tests

`lightningcss/tests/prefix_fallback.rs`. All tests are `#[ignore]`, so a normal
`cargo test` skips them. Run on request:

```sh
cd core/cli && CARGO_INCREMENTAL=0 cargo test -p lightningcss -- --ignored
```
