import { chmodSync, copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const CARGO_TOML = join(ROOT, "core/cli/Cargo.toml");
const ROOT_PKG_JSON = join(ROOT, "npm/pragma/package.json");

const TARGETS = [
  {
    target: "x86_64-unknown-linux-gnu",
    dir: "cli-linux-x64",
    pkg: "@pragma-web/cli-linux-x64",
    bin: "pragma"
  },
  {
    target: "aarch64-unknown-linux-gnu",
    dir: "cli-linux-arm64",
    pkg: "@pragma-web/cli-linux-arm64",
    bin: "pragma"
  },
  {
    target: "aarch64-apple-darwin",
    dir: "cli-darwin-arm64",
    pkg: "@pragma-web/cli-darwin-arm64",
    bin: "pragma"
  },
  {
    target: "x86_64-apple-darwin",
    dir: "cli-darwin-x64",
    pkg: "@pragma-web/cli-darwin-x64",
    bin: "pragma"
  },
  {
    target: "x86_64-pc-windows-msvc",
    dir: "cli-win32-x64",
    pkg: "@pragma-web/cli-win32-x64",
    bin: "pragma.exe"
  }
];

function parseArgs(argv) {
  const options = { artifacts: join(ROOT, "artifacts"), version: "" };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--artifacts" && argv[i + 1]) {
      options.artifacts = resolve(argv[++i]);
    } else if (arg === "--version" && argv[i + 1]) {
      options.version = argv[++i].replace(/^v/, "");
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return options;
}

function readCargoVersion() {
  const toml = readFileSync(CARGO_TOML, "utf8");
  const section = toml.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/);
  const match = section?.[1].match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`Could not find [workspace.package] version in ${CARGO_TOML}`);
  }
  return match[1];
}

function resolveBinary(artifactsDir, target, bin) {
  const candidates = [
    join(artifactsDir, `bin-${target}`, bin),
    join(artifactsDir, target, bin)
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  throw new Error(`Missing binary for ${target}: checked ${candidates.join(", ")}`);
}

function writeJson(filePath, data) {
  writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const version = options.version || readCargoVersion();

  const rootPkg = JSON.parse(readFileSync(ROOT_PKG_JSON, "utf8"));
  rootPkg.version = version;
  rootPkg.optionalDependencies = {};

  for (const entry of TARGETS) {
    rootPkg.optionalDependencies[entry.pkg] = version;

    const platformDir = join(ROOT, "npm/platforms", entry.dir);
    const platformPkgPath = join(platformDir, "package.json");
    const platformPkg = JSON.parse(readFileSync(platformPkgPath, "utf8"));
    platformPkg.version = version;
    writeJson(platformPkgPath, platformPkg);

    const srcBin = resolveBinary(options.artifacts, entry.target, entry.bin);
    const destDir = join(platformDir, "bin");
    const destBin = join(destDir, entry.bin);
    mkdirSync(destDir, { recursive: true });
    copyFileSync(srcBin, destBin);
    if (!entry.bin.endsWith(".exe")) {
      chmodSync(destBin, 0o755);
    }
  }

  writeJson(ROOT_PKG_JSON, rootPkg);
  process.stdout.write(`Staged pragma@${version} and ${TARGETS.length} platform packages.\n`);
}

main();
