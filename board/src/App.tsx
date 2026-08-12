import { BrowserRouter, Routes, Route, Navigate, NavLink } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import PlansPage from "./pages/plans/PlansPage";
import BoardView from "./pages/plans/BoardView";
import GraphView from "./pages/plans/GraphView";
import ArtifactPage from "./pages/ArtifactPage";

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

const sections = [{ to: "/plans", label: "Plans" }];

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <div className="min-h-screen bg-neutral-50 text-neutral-900">
          <header className="border-b border-neutral-200 bg-white px-6 py-3">
            <nav className="flex items-center gap-6">
              <span className="font-semibold tracking-tight">trellis</span>
              {sections.map((s) => (
                <NavLink
                  key={s.to}
                  to={s.to}
                  className={({ isActive }) =>
                    isActive
                      ? "text-sm font-medium text-neutral-900"
                      : "text-sm text-neutral-500 hover:text-neutral-900"
                  }
                >
                  {s.label}
                </NavLink>
              ))}
            </nav>
          </header>
          <main className="p-6">
            <Routes>
              <Route path="/" element={<Navigate to="/plans" replace />} />
              <Route path="/plans" element={<PlansPage />}>
                <Route index element={<BoardView />} />
                <Route path="graph" element={<GraphView />} />
              </Route>
              <Route path="/artifacts/*" element={<ArtifactPage />} />
            </Routes>
          </main>
        </div>
      </BrowserRouter>
    </QueryClientProvider>
  );
}
