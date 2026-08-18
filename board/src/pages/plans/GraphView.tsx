import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { usePlanDrawer } from "./PlanDrawer";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  MarkerType,
  type Node,
  type Edge,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import ELK from "elkjs/lib/elk.bundled.js";
import { type PlanRow } from "../../lib/api";
import { useDomain } from "../../lib/domain";

// Node identity is the plan's live address (spec rule 13), which is what an
// awaits edge names whichever side of the terminal tier its target sits on.
// The census this reads is the live one — serve sheds the tier, and the
// edges onto it, before we get here — so normalizing is belt and braces for
// a census that carries `archive/…` rows anyway.
const live = (rel: string) => rel.replace(/^archive\//, "");
const slug = (rel: string) =>
  live(rel)
    .replace(/^plans\//, "")
    .replace(/\.md$/, "");

const STATUS_STYLES: Record<string, string> = {
  draft: "border-neutral-300 bg-white text-neutral-500",
  ready: "border-sky-400 bg-sky-50",
  active: "border-emerald-500 bg-emerald-50",
  blocked: "border-red-400 bg-red-50",
  retired: "border-neutral-200 bg-neutral-100 text-neutral-400",
};

type PlanNodeData = {
  row?: PlanRow;
  label: string;
  missing?: boolean;
};

function PlanNode({ data }: NodeProps<Node<PlanNodeData>>) {
  const status = data.row?.status ?? undefined;
  const style = data.missing
    ? "border-dashed border-red-300 bg-white text-red-400"
    : (STATUS_STYLES[status ?? ""] ?? "border-neutral-300 bg-white");
  return (
    <div className={`w-52 rounded-md border px-3 py-2 shadow-sm ${style}`}>
      <Handle type="target" position={Position.Left} className="opacity-0" />
      <Handle type="source" position={Position.Right} className="opacity-0" />
      <div className="line-clamp-2 text-sm leading-snug font-medium">{data.label}</div>
      <div className="mt-0.5 flex items-center gap-2 text-[11px]">
        <span>{data.missing ? "missing" : (status ?? "no status")}</span>
        {data.row?.held ? <span title={String(data.row.held)}>⏸ held</span> : null}
        {data.row?.archived ? <span>archived</span> : null}
      </div>
    </div>
  );
}

const nodeTypes = { plan: PlanNode };

const elk = new ELK();

// Dependent → prerequisite, left to right: an edge carries the declaration, so
// “B awaits A” is drawn B → A and the arrowhead lands on what B is waiting for.
async function layout(rows: PlanRow[]): Promise<{ nodes: Node[]; edges: Edge[] }> {
  const byLive = new Map(rows.map((r) => [live(r.plan), r]));
  const ids = new Set(byLive.keys());
  const missing = new Set<string>();
  const edges: Edge[] = [];

  for (const row of rows) {
    for (const target of row.awaits) {
      const to = live(target);
      if (!ids.has(to)) missing.add(to);
      edges.push({
        id: `${live(row.plan)}->${to}`,
        source: live(row.plan),
        target: to,
        markerEnd: { type: MarkerType.ArrowClosed },
        style: row.held === target ? { stroke: "#f87171", strokeWidth: 1.5 } : undefined,
      });
    }
  }

  const nodes: Node[] = [
    ...rows.map((r) => ({
      id: live(r.plan),
      type: "plan",
      position: { x: 0, y: 0 },
      data: { row: r, label: r.title ?? slug(r.plan) } satisfies PlanNodeData,
    })),
    ...[...missing].map((id) => ({
      id,
      type: "plan",
      position: { x: 0, y: 0 },
      data: { label: slug(id), missing: true } satisfies PlanNodeData,
    })),
  ];

  const graph = await elk.layout({
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": "RIGHT",
      "elk.spacing.nodeNode": "24",
      "elk.layered.spacing.nodeNodeBetweenLayers": "80",
    },
    children: nodes.map((n) => ({ id: n.id, width: 208, height: 72 })),
    edges: edges.map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  });

  const pos = new Map(graph.children?.map((c) => [c.id, { x: c.x ?? 0, y: c.y ?? 0 }]));
  return {
    nodes: nodes.map((n) => ({ ...n, position: pos.get(n.id) ?? n.position })),
    edges,
  };
}

export default function GraphView() {
  const { open } = usePlanDrawer();
  const { api, key } = useDomain();
  const { data, error } = useQuery({ queryKey: key("plans"), queryFn: api.plans });
  const [graph, setGraph] = useState<{ nodes: Node[]; edges: Edge[] }>({
    nodes: [],
    edges: [],
  });

  useEffect(() => {
    if (!data) return;
    let stale = false;
    layout(data).then((g) => {
      if (!stale) setGraph(g);
    });
    return () => {
      stale = true;
    };
  }, [data]);

  if (error) return <p className="text-red-600">{String(error)}</p>;

  return (
    <div className="h-[calc(100vh-11rem)] rounded-md border border-neutral-200 bg-white">
      <ReactFlow
        nodes={graph.nodes}
        edges={graph.edges}
        nodeTypes={nodeTypes}
        onNodeClick={(_, node) => {
          const row = (node.data as PlanNodeData).row;
          if (row) open(row.plan);
        }}
        fitView
        nodesDraggable
        nodesConnectable={false}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
