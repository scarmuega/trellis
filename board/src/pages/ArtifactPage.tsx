import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useDomain } from "../lib/domain";
import { markdownComponents, withoutFrontmatter } from "../lib/markdown";
import { FrontmatterFields } from "../lib/fields";

export default function ArtifactPage() {
  const { "*": rel = "" } = useParams();
  const { api, key } = useDomain();
  const { data, error, isPending } = useQuery({
    queryKey: key("artifact", rel),
    queryFn: () => api.artifact(rel),
    enabled: rel.length > 0,
  });

  if (isPending) return <p className="text-neutral-500">Loading {rel}…</p>;
  if (error) return <p className="text-red-600">{String(error)}</p>;

  return (
    <article className="mx-auto max-w-3xl">
      <p className="mb-3 font-mono text-xs text-neutral-400">{data.path}</p>
      <div className="mb-6 rounded-md border border-neutral-200 bg-neutral-50/60 px-3 py-2.5">
        <FrontmatterFields frontmatter={(data.frontmatter ?? {}) as Record<string, unknown>} />
      </div>
      <div className="prose prose-neutral max-w-none">
        <Markdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
          {withoutFrontmatter(data.text)}
        </Markdown>
      </div>
    </article>
  );
}
