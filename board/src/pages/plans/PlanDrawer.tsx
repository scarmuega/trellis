import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { FlipError, type ErrandOutcome } from "../../lib/api";
import { useDomain } from "../../lib/domain";
import { markdownComponents, withoutFrontmatter } from "../../lib/markdown";
import { FrontmatterFields } from "../../lib/fields";

/// The Errand menu (decision 0060). One errand, no types: the operator
/// writes the instruction — that *is* the errand — and picks the size it
/// runs at. It is named "Errand", not "Act", because `act` is the dispatch
/// loop's own trigger and was never requestable from here.
///
/// The model and effort choices come from the daemon's resolved complexity
/// tiers, so they are this domain's configured vocabulary rather than a list
/// baked in here that would date with the models.
function ErrandMenu({
  rel,
  complexity,
  onRequested,
}: {
  rel: string;
  complexity?: string;
  onRequested: (outcome: ErrandOutcome) => void;
}) {
  const queryClient = useQueryClient();
  const { api, key } = useDomain();
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [model, setModel] = useState("");
  const [effort, setEffort] = useState("");
  const [error, setError] = useState<string | null>(null);

  const { data: status } = useQuery({
    queryKey: key("status"),
    queryFn: api.status,
    refetchInterval: 10_000,
  });
  const dispatcherUp = status?.dispatch_running === true;
  const inFlight = (status?.dispatch?.in_flight ?? []).some(
    (s) => s.key === `plan:${rel}`,
  );

  const tiers = status?.session_tiers ?? {};
  // Absent a choice, the plan's own complexity tier sizes the session — the
  // same fallback the daemon applies, shown so the default is not a mystery.
  const fallback = tiers[complexity ?? "standard"] ?? tiers.standard;
  const uniq = (values: string[]) => [...new Set(values.filter(Boolean))];
  const models = uniq(Object.values(tiers).map((t) => t.model));
  const efforts = uniq(Object.values(tiers).map((t) => t.effort));

  const mutation = useMutation({
    mutationFn: () =>
      api.requestErrand(rel, text.trim(), {
        ...(model ? { model } : {}),
        ...(effort ? { effort } : {}),
      }),
    onSuccess: (outcome) => {
      setOpen(false);
      setText("");
      setError(null);
      onRequested(outcome);
      queryClient.invalidateQueries({ queryKey: key("status") });
    },
    onError: (e: Error) => setError(e.message),
  });

  const disabled = !dispatcherUp || inFlight || mutation.isPending;
  const disabledReason = !dispatcherUp
    ? "dispatcher not running — start `trellis dispatch run`"
    : inFlight
      ? "a session is already running on this plan"
      : undefined;

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        disabled={disabled}
        title={disabledReason}
        className="rounded border border-neutral-300 bg-white px-2.5 py-1 text-xs font-medium hover:border-neutral-500 disabled:cursor-not-allowed disabled:opacity-40"
      >
        Errand ▾
      </button>
      {open && (
        <div className="absolute right-0 z-10 mt-1 w-80 rounded-md border border-neutral-200 bg-white p-2 shadow-lg">
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            rows={4}
            autoFocus
            placeholder="what should this plan's owner do?"
            className="w-full rounded border border-neutral-200 p-1.5 text-xs focus:border-neutral-400 focus:outline-none"
          />
          <div className="mt-1.5 flex items-center gap-1.5">
            <select
              value={model}
              onChange={(e) => setModel(e.target.value)}
              className="rounded border border-neutral-200 px-1 py-0.5 text-[11px]"
            >
              <option value="">model: {fallback?.model ?? "default"}</option>
              {models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
            <select
              value={effort}
              onChange={(e) => setEffort(e.target.value)}
              className="rounded border border-neutral-200 px-1 py-0.5 text-[11px]"
            >
              <option value="">effort: {fallback?.effort ?? "default"}</option>
              {efforts.map((x) => (
                <option key={x} value={x}>
                  {x}
                </option>
              ))}
            </select>
            <button
              onClick={() => text.trim() && mutation.mutate()}
              disabled={!text.trim() || mutation.isPending}
              className="ml-auto rounded bg-neutral-900 px-2 py-1 text-xs text-white disabled:opacity-40"
            >
              Request
            </button>
          </div>
          <p className="mt-1.5 text-[10px] leading-snug text-neutral-400">
            Spawns a session as this plan's owner, which will write to the
            repo{status?.budget_enforced === false ? " with no budget cap" : ""}.
          </p>
          {error && <p className="mt-1 text-[11px] text-red-600">{error}</p>}
        </div>
      )}
    </div>
  );
}

/// What became of the ask, for as long as it is worth knowing (decision
/// 0060). The six-second toast this replaces said a session had been
/// requested and then took the only trace of it away.
function ErrandStatus({
  rel,
  outcome,
  onDismiss,
}: {
  rel: string;
  outcome: ErrandOutcome;
  onDismiss: () => void;
}) {
  const { api, key } = useDomain();
  const { data: status } = useQuery({
    queryKey: key("status"),
    queryFn: api.status,
    refetchInterval: 5_000,
  });
  const session = (status?.dispatch?.in_flight ?? []).find(
    (s) => s.key === `plan:${rel}`,
  );
  // Queued until the loop's next wake picks it up; running once it appears
  // in flight; finished when it has been and is gone.
  const [wasRunning, setWasRunning] = useState(false);
  useEffect(() => {
    if (session) setWasRunning(true);
  }, [session]);
  const phase = session ? "running" : wasRunning ? "finished" : "queued";

  const tone =
    phase === "running"
      ? "border-emerald-300 bg-emerald-50 text-emerald-900"
      : phase === "finished"
        ? "border-neutral-200 bg-neutral-50 text-neutral-600"
        : "border-sky-300 bg-sky-50 text-sky-900";

  return (
    <div className={`mb-4 rounded-md border px-3 py-2 text-xs ${tone}`}>
      <div className="flex items-start gap-2">
        <div className="min-w-0 grow">
          <p className="font-medium">
            errand {phase}
            {outcome.outcome === "replaced" && phase === "queued"
              ? " — replaced the ask already waiting on this plan"
              : ""}
          </p>
          <p className="mt-0.5 opacity-80">
            as {outcome.owner} · {outcome.model}/{outcome.effort}
            {outcome.budget_enforced === false
              ? " · budget unenforced (herdr)"
              : outcome.budget_usd
                ? ` · $${outcome.budget_usd}`
                : ""}
          </p>
          {outcome.delegated && (
            <p className="mt-0.5 opacity-80">
              delegated execution — running under your own mandate at your
              direction; approvals stay yours
            </p>
          )}
          {session?.waiting && (
            <p className="mt-0.5 opacity-80">
              waiting on {session.waiting} — the runtime is holding off on
              judging this session a stall
            </p>
          )}
          {session?.pane && (
            <p className="mt-0.5 font-mono text-[11px] opacity-70">
              herdr pane {session.pane}
            </p>
          )}
        </div>
        <button
          onClick={onDismiss}
          className="shrink-0 opacity-50 hover:opacity-100"
          aria-label="Dismiss"
        >
          ✕
        </button>
      </div>
    </div>
  );
}

// The lifecycle moves the status route accepts (decision 0049), keyed by the
// plan's current status. Retired is terminal and offers nothing.
const TRANSITIONS: Record<string, { label: string; to: string }[]> = {
  draft: [
    { label: "Release → ready", to: "ready" },
    { label: "Retire", to: "retired" },
  ],
  ready: [
    { label: "Claim → active", to: "active" },
    { label: "Retire", to: "retired" },
  ],
  active: [{ label: "Retire", to: "retired" }],
  blocked: [
    { label: "Unblock → ready", to: "ready" },
    { label: "Retire", to: "retired" },
  ],
};

const STATUS_CHIP: Record<string, string> = {
  draft: "bg-neutral-100 text-neutral-600",
  ready: "bg-sky-50 text-sky-700",
  active: "bg-emerald-50 text-emerald-700",
  blocked: "bg-red-50 text-red-700",
  retired: "bg-neutral-100 text-neutral-400",
};

/// The status chip, made actionable: the guarded flips the CLI's lifecycle
/// verbs perform, POSTed to the status route through serve's relay. A
/// release the readiness gate refuses offers the same --force the CLI does.
function StatusMenu({ rel, current }: { rel: string; current?: string }) {
  const queryClient = useQueryClient();
  const { api, key } = useDomain();
  const [open, setOpen] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [notReady, setNotReady] = useState<string | null>(null);
  const clearTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const { data: status } = useQuery({
    queryKey: key("status"),
    queryFn: api.status,
    refetchInterval: 10_000,
  });
  const dispatcherUp = status?.dispatch_running === true;
  const inFlight = (status?.dispatch?.in_flight ?? []).some(
    (s) => s.key === `plan:${rel}`,
  );

  const show = (ok: boolean, message: string) => {
    setResult({ ok, message });
    clearTimeout(clearTimer.current);
    clearTimer.current = setTimeout(() => setResult(null), 6000);
  };
  useEffect(() => () => clearTimeout(clearTimer.current), []);

  const mutation = useMutation({
    mutationFn: ({ to, force }: { to: string; force?: boolean }) =>
      api.setStatus(rel, to, force),
    onSuccess: (outcome) => {
      setNotReady(null);
      show(true, `${outcome.from} → ${outcome.to}`);
      for (const what of ["artifact", "board", "plans", "status"]) {
        queryClient.invalidateQueries({
          queryKey: what === "artifact" ? key("artifact", rel) : key(what),
        });
      }
    },
    onError: (e: Error) => {
      if (e instanceof FlipError && e.outcome === "not-ready") {
        setNotReady(e.message);
      } else {
        show(false, e.message);
      }
    },
  });

  const run = (to: string, force = false) => {
    setOpen(false);
    setNotReady(null);
    mutation.mutate({ to, force });
  };

  const moves = TRANSITIONS[current ?? ""] ?? [];
  const chip = STATUS_CHIP[current ?? ""] ?? "bg-neutral-100 text-neutral-600";
  if (moves.length === 0) {
    return <Chip label="status" value={current ?? "?"} />;
  }

  const disabled = !dispatcherUp || inFlight || mutation.isPending;
  const disabledReason = !dispatcherUp
    ? "dispatcher not running — status changes ride its socket (`trellis dispatch run`)"
    : inFlight
      ? "a session is running on this plan"
      : undefined;

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        disabled={disabled}
        title={disabledReason}
        className={`rounded px-2 py-0.5 text-xs hover:ring-1 hover:ring-neutral-400 disabled:cursor-not-allowed disabled:opacity-50 ${chip}`}
      >
        <span className="opacity-60">status</span> {current} ▾
      </button>
      {open && (
        <div className="absolute left-0 z-10 mt-1 w-44 rounded-md border border-neutral-200 bg-white p-1 shadow-lg">
          {moves.map((m) => (
            <button
              key={m.label}
              onClick={() => run(m.to)}
              className="block w-full rounded px-2 py-1.5 text-left text-xs hover:bg-neutral-100"
            >
              {m.label}
            </button>
          ))}
        </div>
      )}
      {notReady && (
        <div className="absolute top-full left-0 z-10 mt-1.5 w-80 rounded-md border border-amber-200 bg-amber-50 p-2 text-xs text-amber-800 shadow">
          <p>{notReady}</p>
          <button
            onClick={() => run("ready", true)}
            className="mt-1.5 rounded bg-amber-600 px-2 py-1 text-white hover:bg-amber-700"
          >
            Release anyway
          </button>
        </div>
      )}
      {result && (
        <p
          className={`absolute top-full left-0 mt-1.5 w-72 text-xs ${
            result.ok ? "text-emerald-600" : "text-red-600"
          }`}
        >
          {result.message}
        </p>
      )}
    </div>
  );
}

// The drawer is addressed by the `plan` search param, so it overlays either
// view (/plans or /plans/graph) without changing the route, and a URL with a
// drawer open restores as one.
export function usePlanDrawer() {
  const [params, setParams] = useSearchParams();
  const open = useCallback(
    (rel: string) =>
      setParams((p) => {
        p.set("plan", rel);
        return p;
      }),
    [setParams],
  );
  const close = useCallback(
    () =>
      setParams((p) => {
        p.delete("plan");
        return p;
      }),
    [setParams],
  );
  return { rel: params.get("plan"), open, close };
}

function Chip({ label, value }: { label: string; value: string }) {
  return (
    <span className="rounded bg-neutral-100 px-2 py-0.5 text-xs text-neutral-600">
      <span className="text-neutral-400">{label}</span> {value}
    </span>
  );
}

export default function PlanDrawer() {
  const { rel, open, close } = usePlanDrawer();
  const { api, key, href } = useDomain();
  const [errand, setErrand] = useState<ErrandOutcome | null>(null);

  // The status row belongs to the plan it was requested on, not to the
  // drawer — switching plans must not show one plan's session under another.
  useEffect(() => setErrand(null), [rel]);

  useEffect(() => {
    if (!rel) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rel, close]);

  const { data, error, isPending } = useQuery({
    queryKey: key("artifact", rel),
    queryFn: () => api.artifact(rel!),
    enabled: !!rel,
  });

  if (!rel) return null;
  const facts = (data?.facts ?? {}) as Record<string, unknown>;
  const frontmatter = (data?.frontmatter ?? {}) as Record<string, unknown>;
  // Derived readings, which are not frontmatter and do not belong in its
  // grid: the kernel computed them, nobody declared them.
  const derived = ["effective_class", "held", "dwell_days", "open_escalations"] as const;

  return (
    <>
      <div className="fixed inset-0 z-40 bg-black/20" onClick={close} />
      <aside className="fixed inset-y-0 right-0 z-50 flex w-full max-w-xl flex-col border-l border-neutral-200 bg-white shadow-xl">
        <header className="flex items-start justify-between gap-4 border-b border-neutral-100 px-5 py-4">
          <div className="min-w-0">
            <p className="truncate font-mono text-xs text-neutral-400">{rel}</p>
            {/* The header carries what the kernel computed; what the author
                declared is the grid below, field by field. */}
            <div className="mt-2 flex flex-wrap gap-1.5">
              {derived.map((k) =>
                facts[k] !== null && facts[k] !== undefined && facts[k] !== 0 ? (
                  <Chip key={k} label={k.replace(/_/g, " ")} value={String(facts[k])} />
                ) : null,
              )}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-3">
            {facts.status !== "retired" && (
              <ErrandMenu
                rel={rel}
                complexity={
                  typeof frontmatter.complexity === "string"
                    ? frontmatter.complexity
                    : undefined
                }
                onRequested={setErrand}
              />
            )}
            <Link
              to={href(`artifacts/${rel}`)}
              className="text-xs text-neutral-400 hover:text-neutral-900"
              title="Open as full page"
            >
              full page ↗
            </Link>
            <button
              onClick={close}
              className="rounded p-1 text-neutral-400 hover:bg-neutral-100 hover:text-neutral-900"
              aria-label="Close"
            >
              ✕
            </button>
          </div>
        </header>
        <div className="grow overflow-y-auto px-5 py-4">
          {isPending && <p className="text-sm text-neutral-500">Loading {rel}…</p>}
          {error && <p className="text-sm text-red-600">{String(error)}</p>}
          {errand && (
            <ErrandStatus
              rel={rel}
              outcome={errand}
              onDismiss={() => setErrand(null)}
            />
          )}
          {data && (
            <>
              <div className="mb-5 rounded-md border border-neutral-200 bg-neutral-50/60 px-3 py-2.5">
                <FrontmatterFields
                  frontmatter={frontmatter}
                  onOpenPlan={open}
                  overrides={{
                    // Status is a verb here, not a value: the same guarded
                    // flips the CLI's lifecycle commands perform.
                    status: (
                      <StatusMenu
                        rel={rel}
                        current={
                          typeof frontmatter.status === "string"
                            ? frontmatter.status
                            : undefined
                        }
                      />
                    ),
                  }}
                />
              </div>
              <div className="prose prose-sm prose-neutral max-w-none">
                <Markdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                  {withoutFrontmatter(data.text)}
                </Markdown>
              </div>
            </>
          )}
        </div>
      </aside>
    </>
  );
}
