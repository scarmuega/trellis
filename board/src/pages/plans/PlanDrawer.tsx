import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { api, FlipError } from "../../lib/api";

// Preset instructions for the shipped `refine` errand — UI sugar; the errand
// accepts any instruction, and other errands get the custom box.
const REFINE_PRESETS = [
  {
    label: "Simplify scope",
    instruction:
      "Simplify this plan's scope: cut what is not essential to the objective, and tighten the objective itself.",
  },
  {
    label: "Split scope",
    instruction:
      "Split this plan: carve its scope into smaller sequenced plans (filed as drafts with awaits: between them), leaving this plan as the first coherent increment.",
  },
  {
    label: "Refactor scope",
    instruction:
      "Refactor this plan's structure: reorganize objective, approach, and scope for clarity and readiness, without changing what it commits to.",
  },
];

/// The Act menu, built from the daemon's own errand table (decision 0051):
/// one section per requestable errand — refine's canned presets plus a
/// custom instruction, other errands with the custom instruction alone —
/// POSTing through serve's relay (decision 0048).
function ActMenu({ rel }: { rel: string }) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [custom, setCustom] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  const clearTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const { data: status } = useQuery({
    queryKey: ["status"],
    queryFn: api.status,
    refetchInterval: 10_000,
  });
  const { data: errands } = useQuery({
    queryKey: ["errands"],
    queryFn: api.errands,
    staleTime: 60_000,
  });
  const dispatcherUp = status?.dispatch_running === true;
  const inFlight = (status?.dispatch?.in_flight ?? []).some(
    (s) => s.key === `plan:${rel}`,
  );

  const mutation = useMutation({
    mutationFn: ({ errand, instruction }: { errand: string; instruction: string }) =>
      api.requestErrand(rel, errand, instruction),
    onSuccess: (outcome) => {
      show(
        true,
        `${outcome.errand ?? "errand"} session requested — ${outcome.owner} (${outcome.model}/${outcome.effort})`,
      );
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["artifact", rel] });
    },
    onError: (e: Error) => show(false, e.message),
  });

  const show = (ok: boolean, message: string) => {
    setResult({ ok, message });
    clearTimeout(clearTimer.current);
    clearTimer.current = setTimeout(() => setResult(null), 6000);
  };
  useEffect(() => () => clearTimeout(clearTimer.current), []);

  const run = (errand: string, instruction: string) => {
    setOpen(false);
    setCustom(null);
    setText("");
    mutation.mutate({ errand, instruction });
  };

  const disabled = !dispatcherUp || inFlight || mutation.isPending;
  const disabledReason = !dispatcherUp
    ? "dispatcher not running — start `trellis dispatch run`"
    : inFlight
      ? "a session is already running on this plan"
      : undefined;

  const available = errands ?? ["refine"];

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        disabled={disabled}
        title={disabledReason}
        className="rounded border border-neutral-300 bg-white px-2.5 py-1 text-xs font-medium hover:border-neutral-500 disabled:cursor-not-allowed disabled:opacity-40"
      >
        Act ▾
      </button>
      {open && (
        <div className="absolute right-0 z-10 mt-1 w-64 rounded-md border border-neutral-200 bg-white p-1 shadow-lg">
          {available.map((errand) => (
            <div key={errand}>
              {available.length > 1 && (
                <p className="px-2 pt-1.5 pb-0.5 text-[10px] font-medium tracking-wide text-neutral-400 uppercase">
                  {errand}
                </p>
              )}
              {errand === "refine" &&
                REFINE_PRESETS.map((a) => (
                  <button
                    key={a.label}
                    onClick={() => run("refine", a.instruction)}
                    className="block w-full rounded px-2 py-1.5 text-left text-xs hover:bg-neutral-100"
                    title={a.instruction}
                  >
                    {a.label}
                  </button>
                ))}
              {custom !== errand ? (
                <button
                  onClick={() => {
                    setCustom(errand);
                    setText("");
                  }}
                  className="block w-full rounded px-2 py-1.5 text-left text-xs text-neutral-500 hover:bg-neutral-100"
                >
                  {errand === "refine" ? "Custom…" : `${errand}…`}
                </button>
              ) : (
                <div className="p-1.5">
                  <textarea
                    value={text}
                    onChange={(e) => setText(e.target.value)}
                    rows={3}
                    autoFocus
                    placeholder="what should the owner do to this plan?"
                    className="w-full rounded border border-neutral-200 p-1.5 text-xs focus:border-neutral-400 focus:outline-none"
                  />
                  <button
                    onClick={() => text.trim() && run(errand, text.trim())}
                    disabled={!text.trim()}
                    className="mt-1 rounded bg-neutral-900 px-2 py-1 text-xs text-white disabled:opacity-40"
                  >
                    Request
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
      {result && (
        <p
          className={`absolute top-full right-0 mt-1.5 w-72 text-right text-xs ${
            result.ok ? "text-emerald-600" : "text-red-600"
          }`}
        >
          {result.message}
        </p>
      )}
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
  const [open, setOpen] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [notReady, setNotReady] = useState<string | null>(null);
  const clearTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const { data: status } = useQuery({
    queryKey: ["status"],
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
      for (const key of ["artifact", "board", "plans", "status"]) {
        queryClient.invalidateQueries({
          queryKey: key === "artifact" ? ["artifact", rel] : [key],
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

  useEffect(() => {
    if (!rel) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rel, close]);

  const { data, error, isPending } = useQuery({
    queryKey: ["artifact", rel],
    queryFn: () => api.artifact(rel!),
    enabled: !!rel,
  });

  if (!rel) return null;
  const facts = (data?.facts ?? {}) as Record<string, unknown>;
  const awaits = Array.isArray(facts.awaits) ? (facts.awaits as string[]) : [];

  return (
    <>
      <div className="fixed inset-0 z-40 bg-black/20" onClick={close} />
      <aside className="fixed inset-y-0 right-0 z-50 flex w-full max-w-xl flex-col border-l border-neutral-200 bg-white shadow-xl">
        <header className="flex items-start justify-between gap-4 border-b border-neutral-100 px-5 py-4">
          <div className="min-w-0">
            <p className="truncate font-mono text-xs text-neutral-400">{rel}</p>
            <div className="mt-2 flex flex-wrap gap-1.5">
              <StatusMenu
                rel={rel}
                current={
                  typeof facts.status === "string" ? facts.status : undefined
                }
              />
              {["owner", "type", "complexity", "effective_class"].map(
                (k) =>
                  typeof facts[k] === "string" ? (
                    <Chip key={k} label={k.replace("_", " ")} value={String(facts[k])} />
                  ) : null,
              )}
              {typeof facts.held === "string" ? (
                <Chip label="held" value={String(facts.held)} />
              ) : null}
              {/* Why an active plan can have no session on it: its taker
                  handed off and the next move is a human's. */}
              {typeof facts.handoff === "string" ? (
                <Chip label="parked" value={String(facts.handoff)} />
              ) : null}
            </div>
            {awaits.length > 0 && (
              <div className="mt-2 flex flex-wrap items-center gap-1.5 text-xs text-neutral-500">
                <span>awaits</span>
                {awaits.map((t) => (
                  <button
                    key={t}
                    onClick={() => open(t)}
                    className="rounded bg-sky-50 px-2 py-0.5 font-mono text-sky-700 hover:bg-sky-100"
                  >
                    {t.replace(/^plans\//, "").replace(/\.md$/, "")}
                  </button>
                ))}
              </div>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-3">
            {facts.status !== "retired" && <ActMenu rel={rel} />}
            <Link
              to={`/artifacts/${rel}`}
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
          {data && (
            <div className="prose prose-sm prose-neutral max-w-none">
              <Markdown remarkPlugins={[remarkGfm]}>{data.text}</Markdown>
            </div>
          )}
        </div>
      </aside>
    </>
  );
}
