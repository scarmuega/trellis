#!/usr/bin/env node
// Deterministic enforcement gate for Trellis domain roots (PreToolUse on
// Write|Edit). Guards only what needs no judgment and no role attribution:
//   1. decisions/ is append-only (spec rule 6) — accepted decisions never edited
//   2. `provenance: generated` artifacts are never hand-edited — writes are
//      allowed only under an acting-role marker (`.trellis/acting-role`,
//      written by /trellis:act)
//   3. new .md artifacts without frontmatter get a warning (non-blocking)
// Outside a Trellis root the gate is inert. Mandate-scope enforcement is
// stage-2 territory (spec/runtime.md). Fails open: a broken gate must never
// brick a session.

import fs from "node:fs";
import path from "node:path";

function out(obj) {
  process.stdout.write(JSON.stringify(obj));
  process.exit(0);
}

function deny(reason) {
  out({
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: reason,
    },
  });
}

function warn(message) {
  out({ systemMessage: message });
}

function isTrellisRoot(dir) {
  try {
    return (
      fs.statSync(path.join(dir, "conventions.md")).isFile() &&
      fs.statSync(path.join(dir, "problem")).isDirectory() &&
      fs.statSync(path.join(dir, "solution")).isDirectory() &&
      fs.statSync(path.join(dir, "org")).isDirectory()
    );
  } catch {
    return false;
  }
}

function findRoot(startDir) {
  let dir = startDir;
  for (;;) {
    if (isTrellisRoot(dir)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function frontmatter(file) {
  try {
    const text = fs.readFileSync(file, "utf8").slice(0, 4096);
    const match = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
    return match ? match[1] : "";
  } catch {
    return "";
  }
}

try {
  const input = JSON.parse(fs.readFileSync(0, "utf8"));
  const toolInput = input.tool_input || {};
  if (!toolInput.file_path) process.exit(0);

  const filePath = path.resolve(input.cwd || process.cwd(), toolInput.file_path);
  const root = findRoot(path.dirname(filePath));
  if (!root) process.exit(0);

  // The plugin's own template/ is shaped like a root; don't gate plugin dev.
  if (
    path.basename(root) === "template" &&
    fs.existsSync(path.join(path.dirname(root), ".claude-plugin"))
  ) {
    process.exit(0);
  }

  const rel = path.relative(root, filePath);
  const exists = fs.existsSync(filePath);

  // 1. decisions/ is append-only: accepted decisions are never edited.
  if (/^decisions[\\/]\d{4}-[^\\/]+\.md$/.test(rel) && exists) {
    const fm = frontmatter(filePath);
    if (/^status:\s*accepted\b/m.test(fm)) {
      deny(
        `${rel} is an accepted decision — decisions/ is append-only (spec rule 6). ` +
          `Supersede it with a new numbered decision that names this one instead of editing it.`
      );
    }
  }

  // 2. Generated artifacts: only an attributed acting role may write them.
  if (exists && /^provenance:\s*generated\b/m.test(frontmatter(filePath))) {
    const marker = path.join(root, ".trellis", "acting-role");
    if (!fs.existsSync(marker)) {
      deny(
        `${rel} carries provenance: generated — hand-edits are blocked. ` +
          `Fix the generator or the source artifact instead; a mandated agent ` +
          `regenerating it must act through /trellis:act <role> so the write is attributed.`
      );
    }
  }

  // 3. New markdown artifact without frontmatter: warn, don't block.
  if (
    !exists &&
    input.tool_name === "Write" &&
    rel.endsWith(".md") &&
    !rel.startsWith(".") &&
    typeof toolInput.content === "string" &&
    !/^---\r?\n[\s\S]*?\bprovenance:/.test(toolInput.content)
  ) {
    warn(
      `trellis gate: new artifact ${rel} declares no provenance/owner frontmatter — ` +
        `every artifact in a Trellis root carries provenance: and owner: (see the instance's conventions.md).`
    );
  }

  process.exit(0);
} catch {
  process.exit(0); // fail open
}
