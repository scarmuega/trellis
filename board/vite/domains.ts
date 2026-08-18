// The domains this board is a window onto.
//
// One domain is one root is one daemon (decision 0002), each on its own port,
// so the board reads a list of them rather than a single API (decision 0062).
// The list is the operator's, not any domain's: it lives outside every root,
// at ~/.trellis/board.toml.
//
// Resolving an entry means reading the filesystem — a root's live addrfile,
// its runtime.toml — which the browser cannot do. So it happens here, and the
// SPA is handed the finished list at GET /board.json.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { parse } from "smol-toml";
import type { PreviewServer, Plugin, ViteDevServer } from "vite";

export interface Domain {
  slug: string;
  label: string;
  /** Origin of that domain's `trellis serve`, no trailing slash. */
  url: string;
  /** One or two characters for the rail's tile; defaults to the slug's. */
  badge?: string;
}

export interface DomainList {
  /** Where the list was read from, so a UI can say what to edit. */
  config: string;
  domains: Domain[];
  /** Why the list is empty, when it is. */
  error?: string;
}

const DEFAULT_PORT = 7357;

export function configPath(): string {
  return (
    process.env.TRELLIS_BOARD_CONFIG ??
    path.join(os.homedir(), ".trellis", "board.toml")
  );
}

function expand(p: string): string {
  if (p === "~") return os.homedir();
  if (p.startsWith("~/")) return path.join(os.homedir(), p.slice(2));
  return path.resolve(p);
}

/** A wildcard bind is not an address to call; loopback is what it means here. */
function reachable(host: string): string {
  return host === "0.0.0.0" || host === "::" || host === "[::]"
    ? "127.0.0.1"
    : host;
}

function origin(value: string): string {
  const trimmed = value.trim().replace(/\/+$/, "");
  return /^https?:\/\//.test(trimmed) ? trimmed : `http://${trimmed}`;
}

/** Where a root says it is being served, preferring what is true now. */
function endpointOf(root: string): string {
  // The addrfile is written by a live daemon and removed when it exits, so it
  // is right even when the port was assigned rather than configured.
  const addr = path.join(root, ".trellis", "runtime", "serve.addr");
  try {
    const text = fs.readFileSync(addr, "utf8").trim();
    if (text) {
      const [host, port] = text.split(":");
      return origin(`${reachable(host)}:${port ?? DEFAULT_PORT}`);
    }
  } catch {
    // No daemon has run here, or it exited cleanly. Fall through to what the
    // root is configured to bind, which is what it will bind next.
  }
  try {
    const cfg = parse(fs.readFileSync(path.join(root, "runtime.toml"), "utf8"));
    const server = (cfg as Record<string, any>).server ?? {};
    const host = reachable(String(server.bind ?? "127.0.0.1"));
    return origin(`${host}:${server.port ?? DEFAULT_PORT}`);
  } catch {
    // No runtime.toml, or an unreadable one: the daemon's own default.
    return `http://127.0.0.1:${DEFAULT_PORT}`;
  }
}

export function readDomains(): DomainList {
  const config = configPath();
  let text: string;
  try {
    text = fs.readFileSync(config, "utf8");
  } catch {
    return {
      config,
      domains: [],
      error: `no such file — list your domains in ${config}`,
    };
  }

  let parsed: Record<string, any>;
  try {
    parsed = parse(text) as Record<string, any>;
  } catch (e) {
    return { config, domains: [], error: `${config} is not valid TOML: ${e}` };
  }

  const entries = Array.isArray(parsed.domains) ? parsed.domains : [];
  const domains: Domain[] = [];
  for (const entry of entries) {
    const slug = String(entry?.slug ?? "").trim();
    // A domain with no slug has no address in the URL and no identity in the
    // rail. Skipping it beats guessing one.
    if (!slug) continue;
    const url = entry.url
      ? origin(String(entry.url))
      : entry.root
        ? endpointOf(expand(String(entry.root)))
        : `http://127.0.0.1:${DEFAULT_PORT}`;
    domains.push({
      slug,
      label: String(entry.label ?? slug),
      url,
      ...(entry.badge ? { badge: String(entry.badge) } : {}),
    });
  }

  return {
    config,
    domains,
    ...(domains.length === 0 && entries.length === 0
      ? { error: `${config} lists no [[domains]]` }
      : {}),
  };
}

/**
 * Serve the list, re-read per request: editing board.toml and refreshing is
 * the whole ceremony for adding a domain. The dev server also watches the
 * file, so an edit reloads the page on its own.
 */
export function domains(): Plugin {
  const serve = (server: ViteDevServer | PreviewServer) => {
    server.middlewares.use("/board.json", (_req, res) => {
      res.setHeader("Content-Type", "application/json");
      res.setHeader("Cache-Control", "no-store");
      res.end(JSON.stringify(readDomains()));
    });
  };
  return {
    name: "trellis-board-domains",
    configureServer(server) {
      serve(server);
      const config = configPath();
      server.watcher.add(config);
      server.watcher.on("all", (_event, changed) => {
        if (changed === config) server.ws.send({ type: "full-reload" });
      });
    },
    configurePreviewServer: serve,
  };
}
