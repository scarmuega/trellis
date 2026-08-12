import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { api } from "../lib/api";

export default function ArtifactPage() {
  const { "*": rel = "" } = useParams();
  const { data, error, isPending } = useQuery({
    queryKey: ["artifact", rel],
    queryFn: () => api.artifact(rel),
    enabled: rel.length > 0,
  });

  if (isPending) return <p className="text-neutral-500">Loading {rel}…</p>;
  if (error) return <p className="text-red-600">{String(error)}</p>;

  return (
    <article className="mx-auto max-w-3xl">
      <p className="mb-4 font-mono text-xs text-neutral-400">{data.path}</p>
      <div className="prose prose-neutral max-w-none">
        <Markdown remarkPlugins={[remarkGfm]}>{data.text}</Markdown>
      </div>
    </article>
  );
}
