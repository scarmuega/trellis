// Client for the read-only surface of `trellis serve`.
// Shapes mirror what the daemon serializes; fields the UI does not use yet
// are left open via index signatures rather than exhaustively typed.
//
// One client per domain, built over that domain's origin (decision 0062):
// the board is a singleton over many daemons, each sovereign on its own port,
// so there is no single API to hardcode a path against.

export interface PlanRow {
  plan: string;
  title?: string | null;
  status?: string | null;
  owner?: string | null;
  complexity?: string | null;
  held?: string | null;
  // What an active plan is parked on — a handoff its taker declared.
  handoff?: string | null;
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
  // The kernel's computed projection — derivations a caller cannot
  // recompute (effective class, dwell, holds).
  facts: Record<string, unknown> | null;
  // What the author declared, every key, shape preserved — so a field this
  // build has never heard of still renders.
  frontmatter: Record<string, unknown> | null;
  text: string;
}

async function getJson<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${url}: ${res.status} ${await res.text()}`);
  return res.json();
}

export interface InFlight {
  key: string;
  label?: string;
  started?: string;
  log?: string;
  // The herdr pane to attach to; absent under the process backend, whose
  // sessions are headless children with only a log.
  pane?: string;
  // What this session declared it is waiting on, while its lease is live. A
  // waiting session holds its slot, so the fleet says why.
  waiting?: string;
}

export interface SessionTier {
  model: string;
  effort: string;
  budget_usd: number;
}

export interface DaemonStatus {
  dispatch_running?: boolean;
  dispatch?: {
    in_flight?: InFlight[];
    [key: string]: unknown;
  } | null;
  // The operator's queue, merged live rather than read from the tick's
  // snapshot: an ask made between ticks has to be visible before the next
  // one, or it reads as lost and gets typed again.
  errands?: QueuedErrand[];
  // The complexity tiers this root resolved — the domain's own model/effort
  // vocabulary, so sizing an errand offers configured values rather than a
  // list baked into this client (decision 0060).
  session_tiers?: Record<string, SessionTier>;
  // False under the herdr backend, where --max-budget-usd cannot be honored.
  budget_enforced?: boolean;
  [key: string]: unknown;
}

// One ask in the daemon's durable queue (decision 0065). Nothing displaces an
// ask and nothing drops one for timing, so what is queued is what is owed.
export interface QueuedErrand {
  id: number;
  plan: string;
  instruction: string;
  // Unix seconds, so the drawer can say how long it has been waiting.
  asked_at: number;
  // "pending" — waiting its turn; "stale" — the plan changed since it was
  // written, so it is held for its author to confirm or discard; "dispatched"
  // — a session is running for it.
  state: "pending" | "stale" | "dispatched";
  model?: string | null;
  effort?: string | null;
}

export interface ErrandOutcome {
  plan: string;
  // Always "requested": an ask is never refused for timing and never
  // displaced (decision 0065).
  outcome: string;
  id?: number;
  // How many asks on this plan run before this one.
  ahead?: number;
  owner?: string;
  complexity?: string;
  model?: string;
  effort?: string;
  budget_usd?: number;
  budget_enforced?: boolean;
  delegated?: boolean;
  [key: string]: unknown;
}

export interface FlipOutcome {
  plan: string;
  outcome: string;
  from?: string;
  to?: string;
  [key: string]: unknown;
}

// Thrown on a refused flip so the UI can tell "not-ready" (offer force)
// from every other refusal.
export class FlipError extends Error {
  outcome?: string;
  constructor(message: string, outcome?: string) {
    super(message);
    this.outcome = outcome;
  }
}

/// The client for one domain, over the origin its daemon listens on. Every
/// path below is the same one the CLI's own reader uses; only the origin
/// changes, which is the whole difference between one domain and the next.
export function createApi(origin: string) {
  const at = (path: string) => `${origin}${path}`;
  return {
  status: () => getJson<DaemonStatus>(at("/api/status")),
  board: () => getJson<Board>(at("/api/board")),
  plans: () => getJson<PlanRow[]>(at("/api/plans")),
  plan: (slug: string) => getJson<Record<string, unknown>>(at(`/api/plans/${slug}`)),
  escalations: (all = false) =>
    getJson<Escalation[]>(at(`/api/escalations${all ? "?all=1" : ""}`)),
  dispatch: () => getJson<Record<string, unknown>>(at("/api/dispatch")),
  org: () => getJson<OrgRole[]>(at("/api/org")),
  artifact: (rel: string) => getJson<Artifact>(at(`/api/artifacts/${rel}`)),
  view: async (name: string): Promise<string> => {
    const res = await fetch(at(`/api/views/${name}`));
    if (!res.ok) throw new Error(`view ${name}: ${res.status}`);
    return res.text();
  },
  // Ask the plan's owner to carry out one instruction over it (decisions
  // 0048, 0060), optionally at a model/effort chosen here rather than taken
  // from the plan's complexity tier. Serve relays this to the dispatcher; a
  // refusal comes back verbatim.
  requestErrand: async (
    rel: string,
    instruction: string,
    size?: { model?: string; effort?: string },
  ): Promise<ErrandOutcome> => {
    const slug = rel.replace(/^plans\//, "").replace(/\.md$/, "");
    const res = await fetch(at(`/api/plans/${encodeURIComponent(slug)}/errand`), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ instruction, ...size }),
    });
    const body = await res.json().catch(() => null);
    if (!res.ok)
      throw new Error(body?.error ?? `the errand was refused (${res.status})`);
    return body;
  },
  // Rule on one queued ask whose plan changed under it (decision 0065).
  // "confirm" re-pins it to what the plan says now; "discard" is the only way
  // an ask leaves the queue unfired at a human's word.
  resolveErrand: async (
    rel: string,
    id: number,
    action: "confirm" | "discard",
  ): Promise<{ outcome: string }> => {
    const slug = rel.replace(/^plans\//, "").replace(/\.md$/, "");
    const res = await fetch(
      at(`/api/plans/${encodeURIComponent(slug)}/errand/${id}`),
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action }),
      },
    );
    const body = await res.json().catch(() => null);
    if (!res.ok)
      throw new Error(body?.error ?? `the errand was not resolved (${res.status})`);
    return body;
  },
  // Flip a plan's lifecycle status (decision 0049) — the same guarded move
  // as `trellis plan release | claim | unblock | retire`, relayed by serve
  // to the dispatcher. `force` overrides the readiness gate on release.
  setStatus: async (
    rel: string,
    status: string,
    force = false,
  ): Promise<FlipOutcome> => {
    const slug = rel.replace(/^plans\//, "").replace(/\.md$/, "");
    const res = await fetch(at(`/api/plans/${encodeURIComponent(slug)}/status`), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status, force }),
    });
    const body = await res.json().catch(() => null);
    if (!res.ok)
      throw new FlipError(
        body?.error ?? `status change failed (${res.status})`,
        body?.outcome,
      );
    return body;
  },
  answer: (ticket: string, answer: string) =>
    fetch(at(`/api/sessions/${encodeURIComponent(ticket)}/answer`), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ answer }),
    }),
  };
}

export type Api = ReturnType<typeof createApi>;
