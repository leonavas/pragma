//! Where a build writes, relative to its output folder; `pragma dev` and `pragma serve` read it there.

/// Output folder of `pragma build`, in the app root.
pub const DIST: &str = "dist";
pub const CLIENT: &str = "client";
pub const SERVER: &str = "server";
/// Page routes for dev/serve, relative to [`CLIENT`].
pub const MANIFEST: &str = "_pragma/manifest.json";
/// Endpoints and middleware for dev/serve.
pub const SERVER_ROUTES: &str = "server/routes.json";
pub const NODE_ENTRY: &str = "server/node.mjs";
