// The domain rail: one tile per domain, the way a chat client stacks servers.
//
// It is a switcher, not a portfolio view (decision 0002 stands — nothing here
// aggregates one domain's content into another's). But a tile can say whether
// the daemon behind it is up and whether anything is waiting on you there,
// because that is what decides which domain you open next.

import { useMemo } from "react";
import { NavLink } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { createApi } from "../lib/api";
import { badgeOf, toneOf, type Domain } from "../lib/domains";

function Tile({ domain }: { domain: Domain }) {
  const api = useMemo(() => createApi(domain.url), [domain.url]);
  const { data, isError } = useQuery({
    queryKey: [domain.slug, "status"],
    queryFn: api.status,
    // A domain with no daemon is the normal case here, not an incident: fail
    // fast, say so on the tile, and keep asking in case one starts.
    retry: false,
    refetchInterval: 15_000,
  });
  const waiting = Array.isArray(data?.pending) ? data.pending.length : 0;

  const title = isError
    ? `${domain.label} — no daemon at ${domain.url}`
    : `${domain.label} — ${domain.url}${
        data?.dispatch_running ? " · dispatching" : ""
      }${waiting > 0 ? ` · ${waiting} waiting on you` : ""}`;

  return (
    <NavLink to={`/d/${domain.slug}/plans`} title={title} className="group relative block">
      {({ isActive }) => (
        <>
          {/* The active marker, borrowed from every rail that works: a pill
              on the edge rather than a border that moves the tile. */}
          <span
            className={`absolute top-1/2 -left-2 h-8 w-1 -translate-y-1/2 rounded-r bg-white transition-opacity ${
              isActive ? "opacity-100" : "opacity-0 group-hover:opacity-40"
            }`}
          />
          <span
            className={`flex h-10 w-10 items-center justify-center rounded-xl text-xs font-semibold text-white transition-all ${toneOf(
              domain.slug,
            )} ${isActive ? "rounded-lg" : "opacity-70 group-hover:rounded-lg group-hover:opacity-100"} ${
              isError ? "opacity-30 grayscale" : ""
            }`}
          >
            {badgeOf(domain)}
          </span>
          {data?.dispatch_running && (
            <span
              className="absolute right-0 bottom-0 h-2.5 w-2.5 rounded-full border-2 border-neutral-900 bg-emerald-400"
              title="dispatcher running"
            />
          )}
          {waiting > 0 && (
            <span className="absolute -top-1 -right-1 flex h-4 min-w-4 items-center justify-center rounded-full border-2 border-neutral-900 bg-red-500 px-1 text-[9px] font-bold text-white">
              {waiting}
            </span>
          )}
          {/* Names live in the tooltip; the rail stays narrow. */}
          <span className="pointer-events-none absolute top-1/2 left-13 z-50 hidden -translate-y-1/2 rounded bg-neutral-900 px-2 py-1 text-xs whitespace-nowrap text-white shadow-lg group-hover:block">
            {domain.label}
          </span>
        </>
      )}
    </NavLink>
  );
}

export default function DomainRail({ domains }: { domains: Domain[] }) {
  return (
    <nav className="flex w-14 shrink-0 flex-col items-center gap-2 bg-neutral-900 py-3">
      {domains.map((d) => (
        <Tile key={d.slug} domain={d} />
      ))}
    </nav>
  );
}
