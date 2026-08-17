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
//! Four routes are not reads, and none weakens that. `/mcp/{token}` is the
//! back-channel a spawned session calls to ask a question or report progress,
//! and `POST /api/sessions/{token}/answer` carries the reply (decision 0041).
//! Both write `.trellis/runtime/` and no artifact; both concern a session
//! already running under a mandate this surface did not grant and cannot
//! widen. `POST /api/plans/{slug}/errands/{name}` (decisions 0048, 0051 —
//! `/refine` is the alias the route shipped under) enqueues an operator's ask
//! that the plan's own `owner:` run a configured errand over it: honored
//! locally only on the dispatcher's socket — where the loop, still the only
//! spawner, re-validates and fires it — and answered by serve purely as a
//! relay to that socket. The errand is a `[prompts]` template, the plan is
//! already in the domain, and the mandate is the one it already declares
//! (decision 0029), so nothing here grants or widens anything. Relaying is
//! the verb the daemon is chartered for, not judgment. `POST /api/plans/{slug}/status` (decision 0049) is the operator's
//! own lifecycle verbs behind the window: the same guarded flip `trellis plan
//! release | claim | unblock | retire` performs, behind the same gates
//! (readiness on release, holds on claim), honored on the dispatcher's socket
//! and relayed by serve — the surface grants nothing the shell did not
//! already, and serve itself still holds no write path. Every other non-GET
//! is still 405, and that is worth keeping true.
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

    // The routes that are not reads — see the module doc, which is where
    // the argument for each lives.
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
        if let Some(rest) = path.strip_prefix("/api/plans/") {
            // `/{slug}/errands/{name}` requests any configured errand
            // (decision 0051); `/{slug}/refine` is the alias the route
            // shipped under (decision 0048).
            if let Some((slug, name)) = rest.split_once("/errands/") {
                let (slug, name) = (slug.to_string(), name.to_string());
                return errand(request, shared, &slug, &name);
            }
            if let Some(slug) = rest.strip_suffix("/refine").map(str::to_string) {
                return errand(request, shared, &slug, "refine");
            }
            if let Some(slug) = rest.strip_suffix("/status").map(str::to_string) {
                return status_flip(request, shared, &slug);
            }
        }
    }
    if request.method() != &Method::Get {
        return error(request, 405, "this surface is read-only");
    }

    match path.as_str() {
        "" => request
            .respond(Response::from_string(INDEX).with_header(header("text/html; charset=utf-8"))),
        // The requestable errand names (decision 0051): the [prompts] keys
        // minus the triggers' own. Config-derived, so every process answers
        // from the same runtime.toml it loaded — no relay needed.
        "/api/errands" => json(
            request,
            &serde_json::to_value(&shared.errands_available).unwrap_or(serde_json::Value::Null),
        ),
        "/api/status" => {
            let status = shared.status.lock().unwrap();
            let mut value = serde_json::to_value(&*status).unwrap_or(serde_json::Value::Null);
            drop(status);
            // The read-only server has no pass of its own: it overlays the
            // dispatcher's snapshot from disk, labelled with whether the
            // process behind it is still alive (decision 0046).
            if shared.overlay_status {
                if let Some(obj) = value.as_object_mut() {
                    let snapshot = std::fs::read_to_string(
                        crate::daemon::config::RuntimeConfig::runtime_dir(&shared.root)
                            .join(crate::daemon::STATUS_FILE),
                    )
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
                    let dispatch_alive = crate::daemon::dispatch_running(&shared.root);
                    obj.insert(
                        "dispatch".into(),
                        snapshot.unwrap_or(serde_json::Value::Null),
                    );
                    obj.insert("dispatch_running".into(), dispatch_alive.into());
                }
            }
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

/// `POST /api/plans/{slug}/errands/{name}` — an operator asks the plan's
/// `owner:` to run one of the configured errands over it (decisions 0048,
/// 0051; `refine` is the shipped one and `…/refine` its alias). Honored
/// locally only where the dispatch loop runs — the handler validates
/// mechanically and enqueues; the loop is the only spawner. Serve answers the
/// same path purely as a relay to the dispatcher's socket — including for
/// errand names it does not know, because the errand table is the
/// dispatcher's config to judge; a rituals pass refuses it.
fn errand(mut request: Request, shared: &Shared, slug: &str, name: &str) -> std::io::Result<()> {
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return error(request, 400, &format!("cannot read the request body: {e}"));
    }
    let instruction = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("instruction")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default()
        .trim()
        .to_string();
    if instruction.is_empty() {
        return error(
            request,
            400,
            "an empty instruction is not an instruction — send {\"instruction\": \"…\"}",
        );
    }
    if instruction.len() > 4096 {
        return error(
            request,
            400,
            "the instruction is longer than 4096 bytes — an errand brief, not a document",
        );
    }
    let rel = format!("plans/{}.md", slug.trim_end_matches(".md"));

    if shared.dispatches {
        if super::TRIGGER_ERRANDS.contains(&name) {
            return error(
                request,
                404,
                &format!(
                    "{name} belongs to the runtime's own trigger — the dispatch loop and \
                     the cadence pass fire it; it is not an operator request"
                ),
            );
        }
        if !shared.errands_available.iter().any(|e| e == name) {
            return error(
                request,
                404,
                &format!(
                    "no errand named {name} — this root's [prompts] carries: {}",
                    shared.errands_available.join(", ")
                ),
            );
        }
        let (tree, _, _, _) = match snapshot(&shared.root) {
            Ok(s) => s,
            Err(e) => return error(request, 500, &format!("cannot read the root: {e}")),
        };
        let owner = match super::validate_errand(&tree, &rel) {
            Ok(owner) => owner,
            Err(refusal) => {
                let (code, outcome) = match &refusal {
                    super::ErrandRefusal::NotAPlan => (404, "not-a-plan"),
                    super::ErrandRefusal::Terminal => (409, "terminal"),
                    super::ErrandRefusal::Unowned => (409, "unowned"),
                };
                let body = serde_json::json!({
                    "plan": rel,
                    "outcome": outcome,
                    "error": refusal.describe(&rel),
                })
                .to_string();
                return request.respond(
                    Response::from_string(body)
                        .with_status_code(code)
                        .with_header(json_header()),
                );
            }
        };
        let key = super::state::key_plan(&rel);
        let in_flight = shared
            .status
            .lock()
            .unwrap()
            .in_flight
            .iter()
            .any(|s| s.key == key);
        if in_flight {
            // The snapshot is only as fresh as the last pass. Nudge one now:
            // if that session actually ended, the pass reaps it and a retry
            // succeeds in a second instead of a tick.
            shared
                .nudge
                .store(true, std::sync::atomic::Ordering::SeqCst);
            let body = serde_json::json!({
                "plan": rel,
                "outcome": "in-flight",
                "error": format!(
                    "{rel} already has a session running — one plan, one session; \
                     if it just finished, retry in a moment"
                ),
            })
            .to_string();
            return request.respond(
                Response::from_string(body)
                    .with_status_code(409)
                    .with_header(json_header()),
            );
        }
        let (complexity, session) =
            crate::dispatch::plan_session(&tree, &shared.sessions, &rel);
        shared.request_errand(name, &rel, &instruction);
        return json(
            request,
            &serde_json::json!({
                "plan": rel,
                "errand": name,
                "outcome": "requested",
                "owner": owner,
                "complexity": complexity.as_str(),
                "model": session.model,
                "effort": session.effort,
                "budget_usd": session.budget_usd,
            }),
        );
    }

    if shared.overlay_status {
        // Serve relays to the process chartered to spawn, and decides
        // nothing — not even the refusals, which come back verbatim.
        let addr_path = super::config::RuntimeConfig::runtime_dir(&shared.root)
            .join(super::DISPATCH_ADDRFILE);
        let Ok(addr) = std::fs::read_to_string(&addr_path) else {
            return error(
                request,
                503,
                "no dispatcher is running on this root — start `trellis dispatch run`",
            );
        };
        let forward = serde_json::json!({ "instruction": instruction }).to_string();
        return match super::client::raw(
            addr.trim(),
            "POST",
            &format!("/api/plans/{slug}/errands/{name}"),
            Some(&forward),
        ) {
            Ok((code, body)) => request.respond(
                Response::from_string(body)
                    .with_status_code(code)
                    .with_header(json_header()),
            ),
            Err(e) => error(
                request,
                503,
                &format!("the dispatcher did not answer: {e} — is `trellis dispatch run` up?"),
            ),
        };
    }

    error(request, 405, "this process runs no dispatch loop")
}

/// `POST /api/plans/{slug}/status` — the operator's lifecycle verbs behind
/// the window (decision 0049). Body is `{"status": "ready | active |
/// retired", "force": false}`. The flip is exactly the guarded edit `trellis
/// plan release | claim | unblock | retire` performs — same source-state
/// guard, same readiness gate on release, same hold check on claim — so who
/// may flip stays the invoker's mandate, not this tool's business
/// (plan_ops). Honored where the dispatch loop runs; serve relays.
fn status_flip(mut request: Request, shared: &Shared, slug: &str) -> std::io::Result<()> {
    let mut body = String::new();
    if let Err(e) = request.as_reader().read_to_string(&mut body) {
        return error(request, 400, &format!("cannot read the request body: {e}"));
    }
    let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();
    let target = parsed
        .as_ref()
        .and_then(|v| v.get("status"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let force = parsed
        .as_ref()
        .and_then(|v| v.get("force"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let to = match PlanStatus::parse(&target) {
        Some(PlanStatus::Draft) => {
            return error(
                request,
                400,
                "nothing flips back to draft — draft is where a plan starts",
            );
        }
        Some(PlanStatus::Blocked) => {
            return error(
                request,
                400,
                "blocking is an escalation, not a bare flip — `trellis plan block` records who is blocked and on what",
            );
        }
        Some(to) => to,
        None => {
            return error(
                request,
                400,
                "send {\"status\": \"ready | active | retired\"} — the transitions the lifecycle verbs perform",
            );
        }
    };
    let rel = format!("plans/{}.md", slug.trim_end_matches(".md"));

    if shared.dispatches {
        let (tree, _, derived, _) = match snapshot(&shared.root) {
            Ok(s) => s,
            Err(e) => return error(request, 500, &format!("cannot read the root: {e}")),
        };
        let refuse = |request: Request, code: u16, outcome: &str, message: String| {
            let body = serde_json::json!({
                "plan": rel,
                "outcome": outcome,
                "error": message,
            })
            .to_string();
            request.respond(
                Response::from_string(body)
                    .with_status_code(code)
                    .with_header(json_header()),
            )
        };
        if !matches!(tree.get(&rel).map(|a| &a.kind), Some(crate::tree::Kind::Plan)) {
            return refuse(
                request,
                404,
                "not-a-plan",
                format!("{rel} is not a plan in this root"),
            );
        }
        let key = super::state::key_plan(&rel);
        let in_flight = shared
            .status
            .lock()
            .unwrap()
            .in_flight
            .iter()
            .any(|s| s.key == key);
        if in_flight {
            // Same staleness bargain as refine: nudge a pass so a session
            // that actually ended is reaped and a retry lands in a second.
            shared
                .nudge
                .store(true, std::sync::atomic::Ordering::SeqCst);
            return refuse(
                request,
                409,
                "in-flight",
                format!(
                    "{rel} has a session running — flipping status under it would fight the session; \
                     if it just finished, retry in a moment"
                ),
            );
        }
        let abs = tree.root.path.join(&rel);
        // plan_ops speaks in the path it was handed; the board speaks in tree
        // addresses, so refusals shed the absolute root prefix.
        let in_tree_vocabulary =
            |e: anyhow::Error| e.to_string().replace(&format!("{}/", tree.root.path.display()), "");
        let from: &[PlanStatus] = match to {
            PlanStatus::Ready => &[PlanStatus::Draft, PlanStatus::Blocked],
            PlanStatus::Active => &[PlanStatus::Ready],
            _ => &[
                PlanStatus::Draft,
                PlanStatus::Ready,
                PlanStatus::Active,
                PlanStatus::Blocked,
            ],
        };
        let current = match crate::plan_ops::current_status(&abs) {
            Ok(c) => c,
            Err(e) => return refuse(request, 409, "illegal", in_tree_vocabulary(e)),
        };
        if to == PlanStatus::Ready && current == PlanStatus::Draft && !force {
            match crate::readiness::check(&tree, &derived, &rel) {
                Ok(report) if !report.mechanical_pass => {
                    let fails: Vec<String> = report
                        .items
                        .iter()
                        .filter(|i| i.status == crate::readiness::ItemStatus::Fail)
                        .map(|i| format!("{} — {}", i.title, i.detail))
                        .collect();
                    return refuse(
                        request,
                        409,
                        "not-ready",
                        format!(
                            "{rel} fails the mechanical readiness share: {} (walk checks/plan-readiness.md, or send force: true)",
                            fails.join("; ")
                        ),
                    );
                }
                Ok(_) => {}
                Err(e) => return refuse(request, 500, "error", e.to_string()),
            }
        }
        if to == PlanStatus::Active {
            if let Some((target, status)) = derived.hold(&rel) {
                return refuse(
                    request,
                    409,
                    "held",
                    format!(
                        "{rel} is held — awaits {target} (status: {}); the hold clears when every target retires",
                        status
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_else(|| "missing".into())
                    ),
                );
            }
        }
        return match crate::plan_ops::flip(&abs, from, to) {
            Ok(was) => {
                // The pass the nudge wakes refreshes status.json — and a plan
                // released into an agent-operated domain is released to its
                // agents, so a due dispatch may fire on it now, not next tick.
                shared
                    .nudge
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                json(
                    request,
                    &serde_json::json!({
                        "plan": rel,
                        "outcome": "flipped",
                        "from": was.as_str(),
                        "to": to.as_str(),
                    }),
                )
            }
            Err(e) => refuse(request, 409, "illegal", in_tree_vocabulary(e)),
        };
    }

    if shared.overlay_status {
        // Serve relays and decides nothing — refusals come back verbatim.
        let addr_path = super::config::RuntimeConfig::runtime_dir(&shared.root)
            .join(super::DISPATCH_ADDRFILE);
        let Ok(addr) = std::fs::read_to_string(&addr_path) else {
            return error(
                request,
                503,
                "no dispatcher is running on this root — start `trellis dispatch run`",
            );
        };
        let forward = serde_json::json!({ "status": target, "force": force }).to_string();
        return match super::client::raw(
            addr.trim(),
            "POST",
            &format!("/api/plans/{slug}/status"),
            Some(&forward),
        ) {
            Ok((code, body)) => request.respond(
                Response::from_string(body)
                    .with_status_code(code)
                    .with_header(json_header()),
            ),
            Err(e) => error(
                request,
                503,
                &format!("the dispatcher did not answer: {e} — is `trellis dispatch run` up?"),
            ),
        };
    }

    error(request, 405, "this process runs no dispatch loop")
}

/// The plan census the read routes project, cut the way `trellis plan list`
/// cuts it: the live census by default, the whole thing under `?archived=1`,
/// which is `--archived` spelled for a URL. Both routes that serve this are
/// attention surfaces, and the terminal tier leaves attention (spec rule 13).
///
/// The cut itself lives in `facts::live_only` so the command and the endpoint
/// cannot drift — an operator reading the CLI and a UI reading the API see
/// one domain, which is the whole charter of this module.
fn plan_census(
    tree: &crate::tree::Tree,
    git: &crate::gitio::Git,
    derived: &crate::graph::Derived,
    today: crate::dates::Date,
    query: &str,
) -> Vec<facts::PlanRow> {
    let mut rows = facts::plan_rows(tree, git, derived, today);
    if !query
        .split('&')
        .any(|p| p == "archived=1" || p == "archived=true")
    {
        facts::live_only(&mut rows);
    }
    rows
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
            let rows = plan_census(&tree, &git, &derived, today, query);
            json(request, &serde_json::to_value(rows).unwrap())
        }

        "/api/board" => {
            let rows = plan_census(&tree, &git, &derived, today, query);
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
                // `facts` is the computed projection — the derivations a
                // caller cannot recompute (effective class, dwell, holds) —
                // and answers only questions the kernel knew to ask.
                // `frontmatter` is what the author declared, every key, shape
                // preserved, so a surface can render a field this binary has
                // never heard of. Both, because neither contains the other.
                let frontmatter = a.fm.as_ref().and_then(|fm| fm.fields());
                return json(
                    request,
                    &serde_json::json!({
                        "path": a.rel,
                        "kind": format!("{:?}", a.kind).to_lowercase(),
                        "facts": facts,
                        "frontmatter": frontmatter,
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
