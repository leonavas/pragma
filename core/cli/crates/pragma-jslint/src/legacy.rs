//! The app layout before `.pragma/`: framework files lived in `core/`. Apps in that layout still
//! lint and build, and `pragma upgrade` moves them to `.pragma/`.

/// Files that only an app in the old layout has.
pub const MARKERS: &[&str] = &["core/boot.js", "core/base.css"];

/// Every file the old layout wrote in `core/` (core/runtime without test/, core/docs and base.css
/// of every commit before the move to `.pragma/`, 3904823). Only these are removed on upgrade.
pub const FILES: &[&str] = &[
    "core/base.css",
    "core/boot.js",
    "core/directives.js",
    "core/docs/classes.md",
    "core/docs/cli.md",
    "core/docs/componentes.md",
    "core/docs/components.md",
    "core/docs/css.md",
    "core/docs/dados.md",
    "core/docs/data.md",
    "core/docs/diretivas.md",
    "core/docs/estado.md",
    "core/docs/estrutura.md",
    "core/docs/logs.md",
    "core/docs/overlays.md",
    "core/docs/packages.md",
    "core/docs/pacotes.md",
    "core/docs/performance.md",
    "core/docs/rotas.md",
    "core/docs/routes.md",
    "core/docs/sintaxe.md",
    "core/docs/state.md",
    "core/docs/structure.md",
    "core/docs/syntax.md",
    "core/docs/webmcp.md",
    "core/loader.js",
    "core/log.js",
    "core/overlay.js",
    "core/page.js",
    "core/props.js",
    "core/router.js",
    "core/server/handler.js",
    "core/server/log.js",
    "core/server/node.mjs",
    "core/server/platform.js",
    "core/server/render.js",
    "core/state.js",
    "core/theme.js",
];

/// Server runtime folder of the old layout.
pub const SERVER_DIR: [&str; 2] = ["core", "server"];

/// Import prefix of the old runtime (`import { log } from '/core/log.js'`).
pub const IMPORT_PREFIX: &str = "/core/";

/// Store module of the old API (`store()` from `/core/state.js`).
pub const STATE_MODULE: &str = "core/state.js";
