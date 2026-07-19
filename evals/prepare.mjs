#!/usr/bin/env node
// Substitute the fixture date tokens — %%TODAY%%, %%DAYS_AGO_N%%,
// %%DAYS_AHEAD_N%% — with ISO dates relative to now, in every file under the
// given directory. Fixtures stay static in the repo; freshness is decided at
// run time.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = process.argv[2];
if (!root) {
  console.error("usage: prepare.mjs <dir>");
  process.exit(2);
}

const DAY = 86_400_000;
const now = Date.now();
const iso = (ms) => new Date(ms).toISOString().slice(0, 10);

function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === ".git") continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(path);
    else yield path;
  }
}

for (const file of walk(root)) {
  const text = readFileSync(file, "utf8");
  const out = text
    .replaceAll("%%TODAY%%", iso(now))
    .replace(/%%DAYS_AGO_(\d+)%%/g, (_, n) => iso(now - n * DAY))
    .replace(/%%DAYS_AHEAD_(\d+)%%/g, (_, n) => iso(now + n * DAY));
  if (out !== text) writeFileSync(file, out);
}
