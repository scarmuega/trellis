import { useQuery } from "@tanstack/react-query";
import { type PlanRow } from "../../lib/api";
import { useDomain } from "../../lib/domain";
import { usePlanDrawer } from "./PlanDrawer";

function Card({ plan, onOpen }: { plan: PlanRow; onOpen: (rel: string) => void }) {
  const slug = plan.plan.replace(/^plans\//, "").replace(/\.md$/, "");
  return (
    <button
      onClick={() => onOpen(plan.plan)}
      className="block w-full cursor-pointer rounded-md border border-neutral-200 bg-white p-3 text-left shadow-sm hover:border-neutral-400"
    >
      <div className="text-sm font-medium">{plan.title ?? slug}</div>
      <div className="mt-1 flex gap-2 text-xs text-neutral-500">
        {plan.owner ? <span>{String(plan.owner)}</span> : null}
        {plan.complexity ? <span>{String(plan.complexity)}</span> : null}
      </div>
    </button>
  );
}

export default function BoardView() {
  const { open } = usePlanDrawer();
  const { api, key } = useDomain();
  const { data, error, isPending } = useQuery({
    queryKey: key("board"),
    queryFn: api.board,
  });

  if (isPending) return <p className="text-neutral-500">Loading board…</p>;
  if (error)
    return (
      <p className="text-red-600">
        Cannot reach the daemon — is <code>trellis serve</code> running?{" "}
        <span className="text-neutral-500">({String(error)})</span>
      </p>
    );

  return (
    <div className="flex gap-4 overflow-x-auto">
      {data.order.map((status) => (
        <section key={status} className="w-64 shrink-0">
          <h2 className="mb-2 text-xs font-semibold tracking-wide text-neutral-500 uppercase">
            {status} · {data.columns[status]?.length ?? 0}
          </h2>
          <div className="space-y-2">
            {(data.columns[status] ?? []).map((p) => (
              <Card key={p.plan} plan={p} onOpen={open} />
            ))}
          </div>
        </section>
      ))}
      {data.unplaced.length > 0 && (
        <section className="w-64 shrink-0">
          <h2 className="mb-2 text-xs font-semibold tracking-wide text-red-500 uppercase">
            unplaced · {data.unplaced.length}
          </h2>
          <div className="space-y-2">
            {data.unplaced.map((p) => (
              <Card key={p.plan} plan={p} onOpen={open} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
