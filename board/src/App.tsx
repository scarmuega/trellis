import { useEffect } from "react";
import {
  BrowserRouter,
  Routes,
  Route,
  Navigate,
  NavLink,
  Outlet,
  useParams,
} from "react-router";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import PlansPage from "./pages/plans/PlansPage";
import BoardView from "./pages/plans/BoardView";
import GraphView from "./pages/plans/GraphView";
import ArtifactPage from "./pages/ArtifactPage";
import DomainRail from "./components/DomainRail";
import { DomainProvider } from "./lib/domain";
import { fetchDomains, type DomainList } from "./lib/domains";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The daemon reads the tree per request; a gentle poll keeps the board
      // live without hammering a single-machine loopback server.
      refetchInterval: 10_000,
      retry: 1,
    },
  },
});

const sections = [{ to: "plans", label: "Plans" }];

// Which domain to open when the URL names none. The last one worked on is
// almost always the right answer; the first configured is the fallback.
const LAST = "trellis.board.domain";

function Landing({ list }: { list: DomainList }) {
  const remembered = localStorage.getItem(LAST);
  const domain =
    list.domains.find((d) => d.slug === remembered) ?? list.domains[0];
  if (!domain) return <Unconfigured list={list} />;
  return <Navigate to={`/d/${domain.slug}/plans`} replace />;
}

/// No domains means nothing to show, so say what to write and where — the
/// board cannot discover this on its own, and an empty rail explains nothing.
function Unconfigured({ list }: { list: DomainList }) {
  return (
    <div className="mx-auto max-w-2xl p-10">
      <h1 className="text-lg font-semibold">No domains configured</h1>
      <p className="mt-2 text-sm text-neutral-600">
        {list.error ?? `${list.config} lists no domains.`} The board is one
        window onto many domains; the list is yours, and lives outside all of
        them.
      </p>
      <pre className="mt-4 overflow-x-auto rounded-md bg-neutral-900 p-4 text-xs text-neutral-100">
        {`# ${list.config}

[[domains]]
slug  = "txpipe"
label = "TxPipe"
root  = "~/Brain/txpipe"          # port read from that root

[[domains]]
slug  = "tx3"
url   = "http://127.0.0.1:7366"   # or name the address outright`}
      </pre>
      <p className="mt-3 text-xs text-neutral-500">
        Each domain needs its own <code>trellis serve</code> running. Save the
        file and this page reloads itself.
      </p>
    </div>
  );
}

function DomainLayout({ list }: { list: DomainList }) {
  const { slug = "" } = useParams();
  const domain = list.domains.find((d) => d.slug === slug);

  useEffect(() => {
    if (domain) localStorage.setItem(LAST, domain.slug);
  }, [domain]);

  if (!domain) {
    return (
      <div className="p-10">
        <h1 className="text-lg font-semibold">No domain called “{slug}”</h1>
        <p className="mt-2 text-sm text-neutral-600">
          {list.domains.length > 0
            ? `${list.config} lists ${list.domains.map((d) => d.slug).join(", ")}.`
            : `${list.config} lists no domains.`}
        </p>
      </div>
    );
  }

  return (
    <DomainProvider domain={domain}>
      <header className="border-b border-neutral-200 bg-white px-6 py-3">
        <nav className="flex items-center gap-6">
          <span className="font-semibold tracking-tight">{domain.label}</span>
          {sections.map((s) => (
            <NavLink
              key={s.to}
              to={`/d/${domain.slug}/${s.to}`}
              className={({ isActive }) =>
                isActive
                  ? "text-sm font-medium text-neutral-900"
                  : "text-sm text-neutral-500 hover:text-neutral-900"
              }
            >
              {s.label}
            </NavLink>
          ))}
          <span className="ml-auto font-mono text-xs text-neutral-400">
            {domain.url.replace(/^https?:\/\//, "")}
          </span>
        </nav>
      </header>
      <main className="p-6">
        <Outlet />
      </main>
    </DomainProvider>
  );
}

function Shell() {
  const { data: list, error } = useQuery({
    queryKey: ["domains"],
    queryFn: fetchDomains,
    refetchInterval: false,
    staleTime: Infinity,
  });

  if (error)
    return (
      <p className="p-10 text-red-600">Cannot read the domain list: {String(error)}</p>
    );
  if (!list) return null;

  return (
    <div className="flex min-h-screen bg-neutral-50 text-neutral-900">
      {list.domains.length > 0 && <DomainRail domains={list.domains} />}
      <div className="min-w-0 grow">
        <Routes>
          <Route path="/" element={<Landing list={list} />} />
          <Route path="/d/:slug" element={<DomainLayout list={list} />}>
            <Route index element={<Navigate to="plans" replace />} />
            <Route path="plans" element={<PlansPage />}>
              <Route index element={<BoardView />} />
              <Route path="graph" element={<GraphView />} />
            </Route>
            <Route path="artifacts/*" element={<ArtifactPage />} />
          </Route>
        </Routes>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Shell />
      </BrowserRouter>
    </QueryClientProvider>
  );
}
