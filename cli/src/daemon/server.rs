//! The read-only serving surface.
//!
//! Every endpoint is a projection of what a command already prints — the same
//! `facts`, the same `dispatch::scan`, the same `escalate::list`, the same
//! rendered views — so a UI reading this API and an operator reading the CLI
//! never see two different domains.
//!
//! **Read-only over the tree, structurally.** A change to the domain enters
//! through a session bound by its mandate, the gate, and the artifact's
//! automation class — never through here. That is also why this surface is not
//! ingress: an HTTP door into `act` would be a trigger plane with no mandate
//! behind it (decision 0038).
//!
//! Two routes are not reads, and neither weakens that. `/mcp/{token}` is the
//! back-channel a spawned session calls to ask a question or report progress,
//! and `POST /api/sessions/{token}/answer` carries the reply (decision 0041).
//! Both write `.trellis/runtime/` and no artifact; both concern a session
//! already running under a mandate this surface did not grant and cannot
//! widen. Relaying a question to a human and the answer back is the "relay"
//! the daemon is chartered for, not judgment. Every other non-GET is still
//! 405, and that is worth keeping true.
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

/// Enough that a parked `trellis_await` cannot starve the board.
///
/// A session waiting on a question holds its worker for the length of one
/// bounded wait, and `max_concurrent` sessions may all be waiting at once. Three
/// workers was right when every request returned promptly; it is not now.
const WORKERS: usize = 12;

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
    let url = request.url().to_string();
    let (path, query) = match url.split_once('?') {
        Some((p, q)) => (p, q),
        None => (url.as_str(), ""),
    };
    let path = path.trim_end_matches('/').to_string();

    // The two routes that are not reads. Both concern a session already
    // running under a mandate, and both write `.trellis/runtime/` and no
    // artifact — see the module doc, which is where the argument lives.
    if request.method() == &Method::Post {
        if let Some(token) = path.strip_prefix("/mcp/").map(str::to_string) {
            return session_call(request, shared, &token);
        }
        if let Some(ticket) = path
            .strip_prefix("/api/sessions/")
            .and_then(|rest| rest.strip_suffix("/answer"))
            .map(str::to_string)
        {
            return answer(request, shared, &ticket);
        }
    }
    if request.method() != &Method::Get {
        return error(request, 405, "this surface is read-only");
    }

    match path.as_str() {
        "" => request
            .respond(Response::from_string(INDEX).with_header(header("text/html; charset=utf-8"))),
        "/api/status" => {
            let status = shared.status.lock().unwrap();
            let mut value = serde_json::to_value(&*status).unwrap_or(serde_json::Value::Null);
            drop(status);
            // Merged live rather than served from `status`, which only the
            // tick refreshes: a question raised between ticks has to be
            // answerable before the next one.
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "sessions".into(),
                    serde_json::to_value(shared.inbox.view()).unwrap(),
                );
                obj.insert(
                    "pending".into(),
                    serde_json::to_value(shared.inbox.pending()).unwrap(),
                );
            }
            json(request, &value)
        }
        p if p.starts_with("/api/") => tree_backed(request, shared, p, query),
        _ => error(request, 404, "no such path"),
    }
}

/// `POST /mcp/{token}` — one JSON-RPC message from a running session.
fn session_call(mut request: Request, shared: &Shared, token: &str) -> std::io::Result<()> {
    if !shared.inbox.known(token) {
        // Not an auth check — the token is attribution. An unknown one means
        // the session it belonged to is over.
        return error(request, 404, "no such session");
    }
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return error(request, 400, &format!("cannot read the request body: {e}"));
    }
    match super::mcp::handle(&shared.inbox, token, &body) {
        Some(response) => {
            request.respond(Response::from_string(response).with_header(json_header()))
        }
        // A notification: accepted, nothing to say back.
        None => request.respond(Response::empty(202)),
    }
}

/// `POST /api/sessions/{ticket}/answer` — the reply, from `trellis inbox` or
/// the board. Body is `{"answer": "..."}`, or the bare text.
fn answer(mut request: Request, shared: &Shared, ticket: &str) -> std::io::Result<()> {
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return error(request, 400, &format!("cannot read the request body: {e}"));
    }
    let choice = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("answer")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.trim().to_string());
    if choice.is_empty() {
        return error(request, 400, "an empty answer is not an answer");
    }
    if shared.inbox.answer(ticket, &choice) {
        json(request, &serde_json::json!({ "answered": ticket }))
    } else {
        error(
            request,
            404,
            "no such open question — it was answered already, or its session ended",
        )
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
            let derived = crate::graph::derive(&tree);
            let report = crate::dispatch::scan(&tree, &derived, &shared.sessions);
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
                    let holder = crate::org::holder(&tree, role);
                    let holder_kind = match holder.kind {
                        crate::org::HolderKind::Ref => "ref",
                        crate::org::HolderKind::Package => "package",
                        crate::org::HolderKind::None => "none",
                    };
                    serde_json::json!({
                        "role": format!("org/{role}"),
                        "mandate": format!("org/{role}/mandate.md"),
                        "purpose": fm.and_then(|f| f.get_str("purpose")),
                        "escalate_to": fm.and_then(|f| f.get_str("escalate-to")),
                        "holder_kind": holder_kind,
                        "holder_ref": holder.reference,
                        "holder_ref_kind": holder.ref_kind,
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
