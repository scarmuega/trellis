// The domains this board is a window onto — the list the host resolved from
// ~/.trellis/board.toml (see vite/domains.ts). Resolving it needs the
// filesystem, so the browser is handed the finished answer.

export interface Domain {
  slug: string;
  label: string;
  /** Origin of that domain's `trellis serve`, no trailing slash. */
  url: string;
  badge?: string;
}

export interface DomainList {
  config: string;
  domains: Domain[];
  error?: string;
}

export async function fetchDomains(): Promise<DomainList> {
  const res = await fetch("/board.json");
  if (!res.ok) throw new Error(`/board.json: ${res.status}`);
  return res.json();
}

/// Two characters that tell eight domains apart at a glance: initials when the
/// name has parts, its opening otherwise. A `badge` in board.toml wins.
export function badgeOf(domain: Domain): string {
  if (domain.badge) return domain.badge.slice(0, 2);
  const words = domain.label.split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  if (words.length > 1) return (words[0][0] + words[1][0]).toUpperCase();
  const one = words[0] ?? domain.slug;
  return (one[0] ?? "?").toUpperCase() + (one[1] ?? "");
}

// Stable per-slug tile colors — written out because Tailwind reads class
// names, not expressions that produce them.
const TONES = [
  "bg-sky-600",
  "bg-emerald-600",
  "bg-violet-600",
  "bg-amber-600",
  "bg-rose-600",
  "bg-teal-600",
  "bg-indigo-600",
  "bg-orange-600",
];

export function toneOf(slug: string): string {
  let hash = 0;
  for (const c of slug) hash = (hash * 31 + c.charCodeAt(0)) >>> 0;
  return TONES[hash % TONES.length];
}
