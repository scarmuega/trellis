#!/usr/bin/env node
// Grade a focus run against a fixture's grading contract.
//
//   node grade.mjs <run-output-file> <expected.yaml> [fixture-work-dir]
//
// Findings are the fenced ```yaml blocks in the run output whose `kind:` is
// one of the five finding kinds (see checks/plan-effectiveness.md). Grading is
// deterministic on kind, checklist item, and refs (substring match; `a|b` in
// the contract means either substring satisfies the entry). When a work dir is
// given, a clean `git status` there asserts the never-writes boundary.
// Exit 0 = all checks pass; 1 = failures; 2 = usage error.
import { readFileSync } from "node:fs";
import { execSync } from "node:child_process";

const [outputPath, expectedPath, workDir] = process.argv.slice(2);
if (!outputPath || !expectedPath) {
  console.error("usage: grade.mjs <run-output-file> <expected.yaml> [fixture-work-dir]");
  process.exit(2);
}

const KINDS = new Set(["candidate", "gap", "blocker", "risk", "challenge"]);

// --- extract finding records from the run output ---
const output = readFileSync(outputPath, "utf8");
const findings = [];
const fence = /```ya?ml\n([\s\S]*?)```/g;
for (let match; (match = fence.exec(output)); ) {
  const record = {};
  let lastKey = null;
  for (const raw of match[1].split("\n")) {
    const line = raw.replace(/\s+$/, "");
    const kv = line.match(/^([A-Za-z][\w-]*):\s*(.*)$/);
    if (kv) {
      record[kv[1]] = kv[2].trim();
      lastKey = kv[1];
      continue;
    }
    const li = line.match(/^\s*-\s+(.*)$/); // block-list continuation
    if (li && lastKey) {
      record[lastKey] = (record[lastKey] ? record[lastKey] + ", " : "") + li[1].trim();
    }
  }
  if (!record.kind || !KINDS.has(record.kind.split(/\s/)[0])) continue;
  findings.push({
    kind: record.kind.split(/\s/)[0],
    item: record.item ? parseInt(record.item, 10) : null,
    refs: (record.refs ?? "").replace(/^\[|\]$/g, ""),
  });
}

// --- parse the grading contract (purpose-built subset of YAML) ---
function parseExpected(text) {
  const expected = { must_find: [], must_not_flag: [], max_findings: null };
  let section = null;
  let current = null;
  for (const raw of text.split("\n")) {
    if (/^\s*#/.test(raw) || !raw.trim()) continue;
    let match;
    if (/^must_find:/.test(raw)) {
      section = "must_find";
      continue;
    }
    if ((match = raw.match(/^must_not_flag:\s*(.*)$/))) {
      section = null;
      expected.must_not_flag = match[1]
        .replace(/^\[|\]$/g, "")
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      continue;
    }
    if ((match = raw.match(/^max_findings:\s*(\d+)\s*$/))) {
      section = null;
      expected.max_findings = Number(match[1]);
      continue;
    }
    if (section === "must_find") {
      if ((match = raw.match(/^\s*-\s+([\w-]+):\s*(.*)$/))) {
        current = { [match[1]]: match[2].trim() };
        expected.must_find.push(current);
        continue;
      }
      if (current && (match = raw.match(/^\s+([\w-]+):\s*(.*)$/))) {
        current[match[1]] = match[2].trim();
      }
    }
  }
  for (const entry of expected.must_find) {
    entry.item = entry.item ? parseInt(entry.item, 10) : null;
    entry.refs_include = (entry.refs_include ?? "")
      .replace(/^\[|\]$/g, "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
  }
  return expected;
}

const expected = parseExpected(readFileSync(expectedPath, "utf8"));

// --- checks ---
const results = [];

for (const want of expected.must_find) {
  const hit = findings.find(
    (f) =>
      f.kind === want.kind &&
      (want.item == null || f.item === want.item) &&
      want.refs_include.every((entry) =>
        entry.split("|").some((alt) => f.refs.includes(alt)),
      ),
  );
  const label = `must_find ${want.kind}${want.item != null ? ` item ${want.item}` : ""} ~ ${want.refs_include.join(" & ")}`;
  results.push({ label, pass: Boolean(hit) });
}

for (const pattern of expected.must_not_flag) {
  const offender = findings.find((f) => f.refs.includes(pattern));
  results.push({
    label: `must_not_flag ${pattern}`,
    pass: !offender,
    detail: offender && `flagged as ${offender.kind} item ${offender.item}`,
  });
}

if (expected.max_findings != null) {
  results.push({
    label: `max_findings ${expected.max_findings} (got ${findings.length})`,
    pass: findings.length <= expected.max_findings,
  });
}

if (workDir) {
  let detail = "";
  let clean = false;
  try {
    detail = execSync("git status --porcelain", { cwd: workDir }).toString().trim();
    clean = detail === "";
  } catch (error) {
    detail = String(error);
  }
  results.push({ label: "never-writes boundary (git status clean)", pass: clean, detail });
}

let failed = 0;
for (const r of results) {
  console.log(`${r.pass ? "PASS" : "FAIL"}  ${r.label}`);
  if (!r.pass && r.detail) console.log(`      ${r.detail.split("\n").join("\n      ")}`);
  if (!r.pass) failed++;
}
console.log(
  `${results.length - failed}/${results.length} checks passed; ${findings.length} finding record(s) extracted`,
);
process.exit(failed ? 1 : 0);
