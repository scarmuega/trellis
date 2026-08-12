import { NavLink, Outlet } from "react-router";
import PlanDrawer from "./PlanDrawer";

const views = [
  { to: "/plans", label: "Board", end: true },
  { to: "/plans/graph", label: "Graph", end: false },
];

export default function PlansPage() {
  return (
    <div>
      <div className="mb-4 flex items-center gap-4">
        <h1 className="text-lg font-semibold tracking-tight">Plans</h1>
        <div className="flex rounded-md border border-neutral-200 bg-white p-0.5">
          {views.map((v) => (
            <NavLink
              key={v.to}
              to={v.to}
              end={v.end}
              className={({ isActive }) =>
                isActive
                  ? "rounded bg-neutral-900 px-3 py-1 text-xs font-medium text-white"
                  : "rounded px-3 py-1 text-xs text-neutral-500 hover:text-neutral-900"
              }
            >
              {v.label}
            </NavLink>
          ))}
        </div>
      </div>
      <Outlet />
      <PlanDrawer />
    </div>
  );
}
