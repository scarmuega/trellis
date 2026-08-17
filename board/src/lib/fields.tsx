// Frontmatter rendering: every declared field, not a hand-picked few.
//
// The drawer used to render four keys it knew by name and drop the rest,
// which put the author's own declarations — `contexts:`, `decisions:`,
// `metrics:`, `pr:` — behind "full page ↗" and a raw YAML paragraph. A
// domain adds fields (rule 4: the root is the boundary), so a surface that
// renders only what this build was compiled to expect goes stale the moment
// an instance declares something new.
//
// So: iterate the fields, pick a component per field *type*, and fall back
// to read-only text for anything unrecognized. A new key renders the day it
// is authored — plainly, but never invisibly.

import type { ReactNode } from "react";
import { Link } from "react-router";

/// The component vocabulary. A type is a rendering decision, not a schema:
/// several fields share one, and an unknown field is assigned one by shape.
export type FieldType = "status" | "refs" | "labels" | "url" | "role" | "text" | "raw";

/// Field name → type, for the fields the model names (spec/model.md's plan
/// schema). Everything absent here is inferred below — the dictionary is a
/// shortcut for what we know, never the set of what renders.
const KNOWN: Record<string, FieldType> = {
  status: "status",
  owner: "role",
  awaits: "refs",
  subdomains: "refs",
  contexts: "refs",
  metrics: "refs",
  decisions: "refs",
  tags: "labels",
  pr: "url",
  handoff: "url",
  type: "text",
  complexity: "text",
  provenance: "text",
};

/// A tree address carries a `/`; a tag does not. That single distinction is
/// what separates a list worth linking from a list worth chipping, and it
/// reads off the value rather than off a list of field names we would have
/// to keep extending.
///
/// A glob is excluded deliberately: a mandate's `scope:` is written as
/// `problem/*`, which addresses a set, not an artifact — linking it would
/// promise a page that cannot exist.
const looksLikeAddress = (s: string) =>
  s.includes("/") && !/^https?:\/\//.test(s) && !/[*?]/.test(s);

export function inferType(value: unknown): FieldType {
  if (typeof value === "string") {
    if (/^https?:\/\//.test(value)) return "url";
    if (/^org\/[a-z0-9-]+$/.test(value)) return "role";
    return "text";
  }
  if (typeof value === "number" || typeof value === "boolean" || value === null) return "text";
  if (Array.isArray(value)) {
    const strings = value.filter((v): v is string => typeof v === "string");
    if (strings.length !== value.length) return "raw";
    return strings.every(looksLikeAddress) && strings.length > 0 ? "refs" : "labels";
  }
  return "raw";
}

export const typeOf = (key: string, value: unknown): FieldType =>
  KNOWN[key] ?? inferType(value);

/// Reading order for the fields the model names: what this is, where it
/// stands, then what it points at. Anything else sorts after, alphabetically.
const ORDER = [
  "provenance",
  "owner",
  "status",
  "type",
  "complexity",
  "awaits",
  "handoff",
  "subdomains",
  "contexts",
  "decisions",
  "metrics",
  "pr",
  "tags",
];

const rank = (key: string) => {
  const i = ORDER.indexOf(key);
  return i === -1 ? ORDER.length : i;
};

const label = (rel: string) =>
  rel
    .replace(/^plans\//, "")
    .replace(/^decisions\//, "")
    .replace(/\.md$/, "");

function Empty() {
  return <span className="text-xs text-neutral-300">—</span>;
}

/// Tree addresses, linked. A plan opens in the drawer (staying on the board);
/// anything else opens its artifact page. An address carrying a `#fragment`
/// links by its file part — the fragment names a section, not an artifact.
function Refs({ value, onOpenPlan }: { value: string[]; onOpenPlan?: (rel: string) => void }) {
  if (value.length === 0) return <Empty />;
  return (
    <span className="flex flex-wrap gap-1">
      {value.map((ref) => {
        const [path] = ref.split("#");
        const isPlan = /^plans\/.+\.md$/.test(path);
        const className =
          "rounded bg-sky-50 px-1.5 py-0.5 font-mono text-[11px] text-sky-700 hover:bg-sky-100";
        return isPlan && onOpenPlan ? (
          <button key={ref} onClick={() => onOpenPlan(path)} className={className}>
            {label(ref)}
          </button>
        ) : (
          <Link key={ref} to={`/artifacts/${path}`} className={className}>
            {label(ref)}
          </Link>
        );
      })}
    </span>
  );
}

function Labels({ value }: { value: string[] }) {
  if (value.length === 0) return <Empty />;
  return (
    <span className="flex flex-wrap gap-1">
      {value.map((v) => (
        <span key={v} className="rounded bg-neutral-100 px-1.5 py-0.5 text-[11px] text-neutral-600">
          {v}
        </span>
      ))}
    </span>
  );
}

function Url({ value }: { value: string }) {
  return (
    <a
      href={value}
      target="_blank"
      rel="noreferrer"
      className="text-xs break-all text-sky-700 hover:underline"
    >
      {value.replace(/^https?:\/\//, "")} ↗
    </a>
  );
}

function Role({ value }: { value: string }) {
  return (
    <Link
      to={`/artifacts/org/${value.replace(/^org\//, "")}/mandate.md`}
      className="rounded bg-neutral-100 px-1.5 py-0.5 text-[11px] text-neutral-700 hover:bg-neutral-200"
    >
      {value}
    </Link>
  );
}

/// The fallback, and the reason the whole thing is safe: a field nobody
/// anticipated still shows its value verbatim, read-only.
function Text({ value }: { value: unknown }) {
  if (value === null || value === undefined || value === "") return <Empty />;
  return <span className="text-xs text-neutral-700">{String(value)}</span>;
}

/// A shape no scalar renderer fits — a nested map like a mandate's
/// `authority:`, a mixed list. Still read-only text, but indented: a
/// one-line JSON dump of a three-level map is technically the value and
/// practically unreadable, which is the failure this whole file exists to
/// undo.
function Raw({ value }: { value: unknown }) {
  return (
    <pre className="font-mono text-[11px] leading-snug whitespace-pre-wrap text-neutral-600">
      {JSON.stringify(value, null, 2)
        // The braces and quotes are noise once it is indented; the shape is
        // already carried by the indentation.
        .replace(/^[{[]\n|\n[}\]]$/g, "")
        .replace(/^ {2}/gm, "")
        .replace(/"([^"]+)":/g, "$1:")
        .replace(/^(\s*)"(.*)",?$/gm, "$1$2")
        .trim()}
    </pre>
  );
}

function Field({
  name,
  value,
  onOpenPlan,
}: {
  name: string;
  value: unknown;
  onOpenPlan?: (rel: string) => void;
}) {
  switch (typeOf(name, value)) {
    case "refs":
      return <Refs value={(Array.isArray(value) ? value : [value]) as string[]} onOpenPlan={onOpenPlan} />;
    case "labels":
      return <Labels value={(Array.isArray(value) ? value : [value]) as string[]} />;
    case "url":
      return typeof value === "string" ? <Url value={value} /> : <Text value={value} />;
    case "role":
      return typeof value === "string" ? <Role value={value} /> : <Text value={value} />;
    case "raw":
      return <Raw value={value} />;
    default:
      return <Text value={value} />;
  }
}

/// The declared frontmatter as a label/value grid.
///
/// Ordered by `ORDER` — identity, then lifecycle, then the relationships
/// that point outward — with anything unrecognized after it, alphabetically.
/// The wire order is the serializer's, so an order worth reading is this
/// view's to impose; an unknown field sorting last is also what makes a new
/// one easy to spot.
///
/// `overrides` is how a field that needs more than a renderer gets one:
/// `status` is the lifecycle menu, which owns mutations this module has no
/// business knowing about.
export function FrontmatterFields({
  frontmatter,
  overrides,
  onOpenPlan,
}: {
  frontmatter: Record<string, unknown>;
  overrides?: Record<string, ReactNode>;
  onOpenPlan?: (rel: string) => void;
}) {
  const entries = Object.entries(frontmatter).sort(
    ([a], [b]) => rank(a) - rank(b) || a.localeCompare(b),
  );
  if (entries.length === 0) return null;
  return (
    <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-3 gap-y-1.5">
      {entries.map(([name, value]) => (
        <div key={name} className="contents">
          <dt className="text-[11px] whitespace-nowrap text-neutral-400">{name}</dt>
          <dd className="min-w-0">
            {overrides?.[name] ?? <Field name={name} value={value} onOpenPlan={onOpenPlan} />}
          </dd>
        </div>
      ))}
    </dl>
  );
}
