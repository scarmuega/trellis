// Shared markdown rendering for artifact bodies.
//
// An escalation record is body content, not a sidebar: it lives in the
// artifact it concerns, under `## Escalations`, as a fenced `yaml` block
// (spec/runtime.md). Rendered as a code block it reads as a wall of
// unwrapped YAML with the one line that matters — `asks:`, the question a
// human must answer — buried between a date and a paragraph of evidence.
// So the fence is intercepted and rendered as a card in place: the ask
// first, the ceremony as chrome, and the long `attempted:`/`blocked:`
// narration folded away until asked for.

import { isValidElement, type ReactNode } from "react";
import type { Components } from "react-markdown";

/// Drop the leading `---` frontmatter block from an artifact's text.
///
/// The body is served verbatim, frontmatter included, and react-markdown has
/// no notion of it — so it renders as a run-together paragraph of
/// `key: value` pairs above the title. Wherever the fields are shown
/// properly (the drawer's grid), that paragraph is the same data twice, the
/// second time unreadable.
export function withoutFrontmatter(text: string): string {
  if (!text.startsWith("---")) return text;
  const end = text.indexOf("\n---", 3);
  if (end === -1) return text;
  return text.slice(text.indexOf("\n", end + 1) + 1).replace(/^\s+/, "");
}

/// The record's fields, in the schema's own order (spec/runtime.md).
const FIELDS = ["raised", "by", "to", "status", "asks", "attempted", "blocked"] as const;
type Field = (typeof FIELDS)[number];
export type EscalationRecord = Partial<Record<Field, string>>;

/// Parse a yaml fence as an escalation record, or `null` if it is not one.
///
/// Deliberately the kernel's parser rather than a real YAML load
/// (`model.rs`'s `EscalationRecord::from_fence`): line-prefixed `key: value`,
/// the value cut at a trailing ` #` comment. Matching it exactly is what
/// keeps this card and `trellis escalate list` showing one string — a
/// second, cleverer parse here would render text the CLI does not agree the
/// record says.
export function parseEscalation(text: string): EscalationRecord | null {
  const record: EscalationRecord = {};
  for (const line of text.split("\n")) {
    const match = /^([a-z]+):(.*)$/.exec(line);
    if (!match) continue;
    const key = match[1] as Field;
    if (!FIELDS.includes(key)) continue;
    const value = match[2].split(" #")[0].trim();
    if (value) record[key] = value;
  }
  // The spine of the shape, so an ordinary yaml fence that happens to carry
  // a `to:` is not mistaken for a record.
  return record.raised && record.status ? record : null;
}

const role = (ref: string) => ref.replace(/^org\//, "");

function Field({ label, value }: { label: string; value: string }) {
  return (
    <details className="group mt-2 [&_summary::-webkit-details-marker]:hidden">
      <summary className="cursor-pointer list-none text-xs text-neutral-500 hover:text-neutral-900">
        <span className="inline-block w-3 transition-transform group-open:rotate-90">›</span>
        {label}
      </summary>
      <p className="mt-1 pl-3 text-xs leading-relaxed whitespace-pre-wrap text-neutral-600">
        {value}
      </p>
    </details>
  );
}

export function EscalationCard({ record }: { record: EscalationRecord }) {
  const open = record.status === "open";
  return (
    <div
      className={`not-prose my-4 rounded-lg border p-4 ${
        open ? "border-amber-300 bg-amber-50/70" : "border-neutral-200 bg-neutral-50"
      }`}
    >
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
        <span
          className={`rounded px-1.5 py-0.5 font-medium ${
            open ? "bg-amber-200 text-amber-900" : "bg-neutral-200 text-neutral-600"
          }`}
        >
          {open ? "escalation — open" : `escalation — ${record.status}`}
        </span>
        {record.by && record.to && (
          <span className="text-neutral-500">
            {role(record.by)} <span className="text-neutral-400">→</span> {role(record.to)}
          </span>
        )}
        {record.raised && <span className="text-neutral-400">{record.raised}</span>}
      </div>

      {/* The ask is the record: the one line a human is being asked to rule
          on. Everything else is why it is being asked. */}
      {record.asks && (
        <p
          className={`mt-2 text-sm leading-relaxed whitespace-pre-wrap ${
            open ? "text-amber-950" : "text-neutral-700"
          }`}
        >
          {record.asks}
        </p>
      )}

      {/* The schema's own order: what was tried, then what stopped. */}
      {record.attempted && <Field label="what was tried" value={record.attempted} />}
      {record.blocked && <Field label="what stopped" value={record.blocked} />}
    </div>
  );
}

/// `pre`, not `code`: react-markdown wraps a fence's `code` element in a
/// `pre`, so replacing the inner element alone would leave the card inside a
/// monospace block. Every other fence falls through untouched.
export const markdownComponents: Components = {
  pre({ children, ...props }) {
    const child = Array.isArray(children) ? children[0] : children;
    if (isValidElement<{ className?: string; children?: ReactNode }>(child)) {
      const language = child.props.className ?? "";
      if (/\blanguage-yaml\b/.test(language)) {
        const record = parseEscalation(String(child.props.children ?? ""));
        if (record) return <EscalationCard record={record} />;
      }
    }
    return <pre {...props}>{children}</pre>;
  },
};
