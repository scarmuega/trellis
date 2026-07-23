#!/usr/bin/env node
// Deterministic enforcement gate for Trellis domain roots (PreToolUse on
// Write|Edit). Guards only what needs no judgment and no role attribution:
//   1. decisions/ is append-only (spec rule 6) — a *committed* accepted decision
//      is never edited; an uncommitted draft (not yet in HEAD) stays editable
//   2. `provenance: generated` artifacts are never hand-edited — writes are
//      allowed only under an acting-role marker (`.trellis/acting-role`,
//      written by /trellis:act)
//   3. new .md artifacts without frontmatter get a warning (non-blocking)
// Outside a Trellis root the gate is inert. Mandate-scope enforcement is
// stage-2 territory (spec/runtime.md). Fails open: a broken gate must never
// brick a session.

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

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

// Append-only freezes a decision once its accepted state is *committed* — shared
// memory across agent generations (spec rule 6). We read the HEAD version, not
// the working tree or the index: a decision drafted/accepted this session (or
// merely `git add`ed) is not yet in HEAD and stays editable. Outside a git repo
// the "committed" notion is meaningless, so we fall back to always-frozen rather
// than silently dropping the guarantee.
function committedDecisionIsAccepted(root, rel) {
  const inRepo = spawnSync("git", ["-C", root, "rev-parse", "--git-dir"], {
    encoding: "utf8",
  });
  if (inRepo.status !== 0) return true; // not a git repo (or git missing): frozen

  const posixRel = rel.split(path.sep).join("/");
  const show = spawnSync("git", ["-C", root, "show", `HEAD:${posixRel}`], {
    encoding: "utf8",
  });
  if (show.status !== 0) return false; // not in HEAD → uncommitted → editable

  const match = (show.stdout || "").match(/^---\r?\n([\s\S]*?)\r?\n---/);
  return /^status:\s*accepted\b/m.test(match ? match[1] : "");
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

  // 1. decisions/ is append-only: a committed accepted decision is never edited.
  //    An uncommitted draft (not yet in HEAD) is still part of the session.
  if (/^decisions[\\/]\d{4}-[^\\/]+\.md$/.test(rel) && exists) {
    const fm = frontmatter(filePath);
    if (
      /^status:\s*accepted\b/m.test(fm) &&
      committedDecisionIsAccepted(root, rel)
    ) {
      deny(
        `${rel} is a committed accepted decision — decisions/ is append-only (spec rule 6). ` +
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
