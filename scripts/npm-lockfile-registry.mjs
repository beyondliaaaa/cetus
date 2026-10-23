#!/usr/bin/env node
// Keeps package-lock.json pointing at the official npm registry.
//
// Why: collaborators in mainland China install through npmmirror (.npmrc).
// npm records the registry it actually fetched from in `resolved`, so a lockfile
// written there points every other machine — CI, contributors abroad — at the
// mirror too. Canonical URLs on registry.npmjs.org avoid that, and npm (9+, with
// the default `replace-registry-host=npmjs`) transparently rewrites them to the
// registry configured in .npmrc, so installs in China still use the mirror.
// The mirror serves byte-identical tarballs, so `integrity` hashes are unchanged.
//
// Usage:
//   node scripts/npm-lockfile-registry.mjs --check   # CI: fail on non-canonical URLs
//   node scripts/npm-lockfile-registry.mjs --fix     # rewrite known mirrors in place
//   (or: npm run lockfile:check / npm run lockfile:fix)

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const CANONICAL = "https://registry.npmjs.org/";
const KNOWN_MIRRORS = ["https://registry.npmmirror.com/", "https://registry.npm.taobao.org/"];

const mode = process.argv[2];
if (mode !== "--check" && mode !== "--fix") {
  console.error("usage: node scripts/npm-lockfile-registry.mjs --check | --fix");
  process.exit(2);
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const lockPath = join(root, "package-lock.json");
const text = readFileSync(lockPath, "utf8");
const lock = JSON.parse(text);

// Remote entries only: workspace packages resolve to relative paths.
const remote = Object.entries(lock.packages ?? {})
  .map(([key, entry]) => [key, entry?.resolved])
  .filter(([, resolved]) => typeof resolved === "string" && /^https?:\/\//.test(resolved));

const offending = remote.filter(([, resolved]) => !resolved.startsWith(CANONICAL));

if (mode === "--check") {
  if (offending.length === 0) {
    console.log(`ok: all ${remote.length} resolved URLs use ${CANONICAL}`);
    process.exit(0);
  }
  console.error(`package-lock.json has ${offending.length} resolved URL(s) outside ${CANONICAL}:`);
  for (const [key, resolved] of offending.slice(0, 10)) {
    console.error(`  ${key}: ${resolved}`);
  }
  if (offending.length > 10) console.error(`  ... and ${offending.length - 10} more`);
  console.error("Fix with: npm run lockfile:fix   (installs through the mirror keep working)");
  process.exit(1);
}

// --fix: plain string replacement keeps npm's formatting and key order intact.
let fixed = text;
for (const mirror of KNOWN_MIRRORS) {
  fixed = fixed.split(`"resolved": "${mirror}`).join(`"resolved": "${CANONICAL}`);
}
writeFileSync(lockPath, fixed);

const unknown = Object.values(JSON.parse(fixed).packages ?? {})
  .map((entry) => entry?.resolved)
  .filter((r) => typeof r === "string" && /^https?:\/\//.test(r) && !r.startsWith(CANONICAL));
if (unknown.length > 0) {
  console.error(`rewrote known mirrors, but ${unknown.length} URL(s) use an unknown host, e.g.:`);
  console.error(`  ${unknown[0]}`);
  console.error("Add that host to KNOWN_MIRRORS in this script if it is a plain npm mirror.");
  process.exit(1);
}
console.log(`ok: package-lock.json now uses ${CANONICAL} for all ${remote.length} remote packages`);
