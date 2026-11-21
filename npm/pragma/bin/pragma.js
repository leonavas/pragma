#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");

const PLATFORMS = {
  "linux-x64": { pkg: "@pragma-web/cli-linux-x64", bin: "pragma" },
  "linux-arm64": { pkg: "@pragma-web/cli-linux-arm64", bin: "pragma" },
  "darwin-arm64": { pkg: "@pragma-web/cli-darwin-arm64", bin: "pragma" },
  "darwin-x64": { pkg: "@pragma-web/cli-darwin-x64", bin: "pragma" },
  "win32-x64": { pkg: "@pragma-web/cli-win32-x64", bin: "pragma.exe" }
};

const key = `${process.platform}-${process.arch}`;
const entry = PLATFORMS[key];

if (!entry) {
  process.stderr.write(`pragma: unsupported platform ${key}\n`);
  process.exit(1);
}

let binPath;
try {
  binPath = require.resolve(`${entry.pkg}/bin/${entry.bin}`);
} catch {
  process.stderr.write(
    `pragma: missing platform package ${entry.pkg}. Reinstall without --omit=optional.\n`
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: "inherit"
});

if (result.error) {
  process.stderr.write(`pragma: failed to run ${binPath}: ${result.error.message}\n`);
  process.exit(1);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(result.status ?? 1);
