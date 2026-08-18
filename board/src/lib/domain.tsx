// Which domain the page is showing, and everything addressed relative to it.
//
// One board, many sovereign domains (decision 0062). The slug rides in the
// path — /d/{slug}/plans — so a link, a bookmark and the back button all name
// a domain rather than inheriting whichever one was last picked. Everything
// that reads or links therefore asks here rather than assuming.

import { createContext, useContext, useMemo } from "react";
import { createApi, type Api } from "./api";
import type { Domain } from "./domains";

interface DomainContext {
  domain: Domain;
  api: Api;
  /** An in-domain path, addressed absolutely: href("plans") → /d/tx3/plans */
  href: (path: string) => string;
  /** A query key scoped to this domain, so two domains never share a cache. */
  key: (...parts: unknown[]) => unknown[];
}

const Ctx = createContext<DomainContext | null>(null);

export function DomainProvider({
  domain,
  children,
}: {
  domain: Domain;
  children: React.ReactNode;
}) {
  const value = useMemo<DomainContext>(
    () => ({
      domain,
      api: createApi(domain.url),
      href: (path: string) => `/d/${domain.slug}/${path.replace(/^\/+/, "")}`,
      key: (...parts: unknown[]) => [domain.slug, ...parts],
    }),
    [domain],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useDomain(): DomainContext {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useDomain outside a domain route");
  return ctx;
}
