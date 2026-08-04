//! The read-only serving surface.
//!
//! Every endpoint is a projection of what a command already prints — the same
//! `facts`, the same `dispatch::scan`, the same `escalate::list`, the same
//! rendered views — so a UI reading this API and an operator reading the CLI
//! never see two different domains.
//!
//! **Read-only is structural, not a policy.** There is no write path here and
//! none is coming: a change to the domain enters through a session bound by
//! its mandate, the gate, and the artifact's automation class. That is also
//! why this surface is not ingress — an HTTP door into `act` would be a
//! trigger plane with no mandate behind it (decision 0038).
//!
//! Handlers hold no cache: each recomputes from a fresh tree. The bind
//! address defaults to loopback because the surface carries no authentication
//! — exposing it is a deliberate act, and the config comment says so.

use std::sync::Arc;

use tiny_http::{Header, Method, Request, Response, Server};

use super::config::Server as ServerConfig;
use super::{note, snapshot, Shared};
use crate::escalate;
use crate::facts;
use crate::model::PlanStatus;
use crate::views;

/// Board columns, in the order a plan travels them. `held` is not one: a hold
/// is a property of a `ready` plan, never a status (spec/model.md).
const COLUMNS: &[PlanStatus] = &[
    PlanStatus::Draft,
    PlanStatus::Ready,
    PlanStatus::Active,
    PlanStatus::Blocked,
    PlanStatus::Retired,
];

const WORKERS: usize = 3;

/// Bind, start the worker threads, and return the port actually bound —
/// which is what the caller asked for unless it asked for 0.
pub fn start(shared: Arc<Shared>, cfg: &ServerConfig) -> anyhow::Result<u16> {
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let server =
        Server::http(addr.as_str()).map_err(|e| anyhow::anyhow!("cannot listen on {addr}: {e}"))?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or_else(|| anyhow::anyhow!("listening on {addr} but it has no port"))?;

    let server = Arc::new(server);
    for _ in 0..WORKERS {
        let server = Arc::clone(&server);
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || {
            while let Ok(request) = server.recv() {
                if let Err(e) = handle(request, &shared) {
                    note(&format!("http: {e}"));
                }
            }
        });
    }
    Ok(port)
}

fn json_header() -> Header {
    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header")
}

fn header(value: &str) -> Header {
    Header::from_bytes(&b"Content-Type"[..], value.as_bytes()).expect("static header")
}

fn json(request: Request, value: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into());
    request.respond(Response::from_string(body).with_header(json_header()))
}

fn error(request: Request, code: u16, message: &str) -> std::io::Result<()> {
    let body = serde_json::json!({ "error": message }).to_string();
    request.respond(
        Response::from_string(body)
            .with_status_code(code)
            .with_header(json_header()),
    )
}

fn handle(request: Request, shared: &Shared) -> std::io::Result<()> {
    if request.method() != &Method::Get {
        return error(request, 405, "this surface is read-only");
    }
    let url = request.url().to_string();
    let (path, query) = match url.split_once('?') {
        Some((p, q)) => (p, q),
        None => (url.as_str(), ""),
    };
    let path = path.trim_end_matches('/');

    match path {
        "" => request
            .respond(Response::from_string(INDEX).with_header(header("text/html; charset=utf-8"))),
        "/api/status" => {
            let status = shared.status.lock().unwrap();
            let value = serde_json::to_value(&*status).unwrap_or(serde_json::Value::Null);
            drop(status);
            json(request, &value)
        }
        _ if path.starts_with("/api/") => tree_backed(request, shared, path, query),
        _ => error(request, 404, "no such path"),
    }
}

/// Everything that reads the domain. One tree load per request, shared by
/// whichever projection the path asked for.
fn tree_backed(request: Request, shared: &Shared, path: &str, query: &str) -> std::io::Result<()> {
    let (tree, git, derived, today) = match snapshot(&shared.root) {
        Ok(s) => s,
        Err(e) => return error(request, 500, &format!("cannot read the root: {e}")),
    };

    match path {
        "/api/plans" => {
            let rows = facts::plan_rows(&tree, &git, &derived, today);
            json(request, &serde_json::to_value(rows).unwrap())
        }

        "/api/board" => {
            let rows = facts::plan_rows(&tree, &git, &derived, today);
            let mut columns = serde_json::Map::new();
            for status in COLUMNS {
                let in_column: Vec<&facts::PlanRow> = rows
                    .iter()
                    .filter(|r| r.status.as_deref() == Some(status.as_str()))
                    .collect();
                columns.insert(
                    status.as_str().to_string(),
                    serde_json::to_value(in_column).unwrap(),
                );
            }
            // A plan whose status is missing or not a legal one belongs
            // nowhere on the board — which is exactly why it is shown.
            let unplaced: Vec<&facts::PlanRow> = rows
                .iter()
                .filter(|r| r.status.as_deref().and_then(PlanStatus::parse).is_none())
                .collect();
            json(
                request,
                &serde_json::json!({
                    "columns": columns,
                    "unplaced": unplaced,
                    "order": COLUMNS.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                }),
            )
        }

        "/api/escalations" => {
            let all = query.split('&').any(|p| p == "all=1" || p == "all=true");
            let records = escalate::list(&tree, all);
            json(request, &serde_json::to_value(records).unwrap())
        }

        "/api/dispatch" => {
            // The same scan the tick runs, under the same session map,
            // reported without spawning anything: what the daemon would do,
            // not what it has done.
            let report = crate::dispatch::scan(&tree, &shared.sessions);
            json(request, &serde_json::to_value(report).unwrap())
        }

        "/api/org" => {
            let mut roles = tree.roles();
            roles.sort();
            let rows: Vec<serde_json::Value> = roles
                .iter()
                .map(|role| {
                    let mandate = tree.get(&format!("org/{role}/mandate.md"));
                    let fm = mandate.and_then(|m| m.fm.as_ref());
                    let holder_ref = tree
                        .get(&format!("org/{role}/holder/ref.md"))
                        .and_then(|a| a.fm.as_ref())
                        .and_then(|f| f.get_str("ref"));
                    let holder_kind = if holder_ref.is_some() {
                        "ref"
                    } else if tree.get(&format!("org/{role}/holder/system.md")).is_some() {
                        "package"
                    } else {
                        "none"
                    };
                    serde_json::json!({
                        "role": format!("org/{role}"),
                        "mandate": format!("org/{role}/mandate.md"),
                        "purpose": fm.and_then(|f| f.get_str("purpose")),
                        "escalate_to": fm.and_then(|f| f.get_str("escalate-to")),
                        "holder_kind": holder_kind,
                        "holder_ref": holder_ref,
                    })
                })
                .collect();
            json(request, &serde_json::Value::Array(rows))
        }

        _ => {
            if let Some(slug) = path.strip_prefix("/api/plans/") {
                let rel = format!("plans/{}", slug.trim_end_matches(".md"));
                let rel = format!("{rel}.md");
                return match facts::artifact(&tree, &git, &derived, &rel, today) {
                    Some(value) => json(request, &value),
                    None => error(request, 404, &format!("{rel} is not a plan in this root")),
                };
            }
            if let Some(name) = path.strip_prefix("/api/views/") {
                return match views::render_named(name, &tree, &git, today) {
                    Some(rendered) => request.respond(
                        Response::from_string(rendered)
                            .with_header(header("text/markdown; charset=utf-8")),
                    ),
                    None => error(
                        request,
                        404,
                        &format!("no such view ({})", views::NAMES.join(" | ")),
                    ),
                };
            }
            if let Some(rel) = path.strip_prefix("/api/artifacts/") {
                // Only paths already in the tree index are servable, so
                // traversal is impossible by construction rather than by
                // sanitizing: `../` is not a key the index holds.
                let Some(a) = tree.get(rel) else {
                    return error(
                        request,
                        404,
                        &format!("{rel} is not an artifact in this root"),
                    );
                };
                let facts = facts::artifact(&tree, &git, &derived, rel, today);
                return json(
                    request,
                    &serde_json::json!({
                        "path": a.rel,
                        "kind": format!("{:?}", a.kind).to_lowercase(),
                        "facts": facts,
                        "text": a.text,
                    }),
                );
            }
            error(request, 404, "no such path")
        }
    }
}

/// The board page: one file, no build step, no package manager. It reads the
/// same API a richer UI would.
pub const INDEX: &str = include_str!("ui/index.html");
