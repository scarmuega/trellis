// Client for the read-only surface of `trellis serve`.
// Shapes mirror what the daemon serializes; fields the UI does not use yet
// are left open via index signatures rather than exhaustively typed.

export interface PlanRow {
  plan: string;
  title?: string | null;
  status?: string | null;
  owner?: string | null;
  complexity?: string | null;
  held?: string | null;
  awaits: string[];
  archived: boolean;
  [key: string]: unknown;
}

export interface Board {
  columns: Record<string, PlanRow[]>;
  unplaced: PlanRow[];
  order: string[];
}

export interface Escalation {
  [key: string]: unknown;
}

export interface OrgRole {
  role: string;
  mandate: string;
  purpose?: string | null;
  escalate_to?: string | null;
  holder_kind: "ref" | "package" | "none";
  holder_ref?: string | null;
  holder_ref_kind?: string | null;
}

export interface Artifact {
  path: string;
  kind: string;
  facts: Record<string, unknown> | null;
  text: string;
}

async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`${path}: ${res.status} ${await res.text()}`);
  return res.json();
}

export interface DaemonStatus {
  dispatch_running?: boolean;
  dispatch?: {
    in_flight?: { key: string; label?: string }[];
    [key: string]: unknown;
  } | null;
  [key: string]: unknown;
}

export interface RefineOutcome {
  plan: string;
  outcome: string;
  owner?: string;
  model?: string;
  effort?: string;
  [key: string]: unknown;
}

export const api = {
  status: () => getJson<DaemonStatus>("/api/status"),
  board: () => getJson<Board>("/api/board"),
  plans: () => getJson<PlanRow[]>("/api/plans"),
  plan: (slug: string) => getJson<Record<string, unknown>>(`/api/plans/${slug}`),
  escalations: (all = false) =>
    getJson<Escalation[]>(`/api/escalations${all ? "?all=1" : ""}`),
  dispatch: () => getJson<Record<string, unknown>>("/api/dispatch"),
  org: () => getJson<OrgRole[]>("/api/org"),
  artifact: (rel: string) => getJson<Artifact>(`/api/artifacts/${rel}`),
  view: async (name: string): Promise<string> => {
    const res = await fetch(`/api/views/${name}`);
    if (!res.ok) throw new Error(`view ${name}: ${res.status}`);
    return res.text();
  },
  // Ask the plan's owner to reshape its content (decision 0048). Serve
  // relays this to the dispatcher; a refusal's reason comes back verbatim.
  refine: async (rel: string, instruction: string): Promise<RefineOutcome> => {
    const slug = rel.replace(/^plans\//, "").replace(/\.md$/, "");
    const res = await fetch(`/api/plans/${encodeURIComponent(slug)}/refine`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ instruction }),
    });
    const body = await res.json().catch(() => null);
    if (!res.ok) throw new Error(body?.error ?? `refine failed (${res.status})`);
    return body;
  },
  answer: (ticket: string, answer: string) =>
    fetch(`/api/sessions/${encodeURIComponent(ticket)}/answer`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ answer }),
    }),
};
