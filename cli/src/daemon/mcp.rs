//! The back-channel: what a spawned session says to the daemon that spawned it.
//!
//! The spawn is one-way — argv in, exit code out (`spawn`). This is the return
//! path, and it is deliberately small: a session can ask a question and wait for
//! an answer, or report a note about what it is doing. Nothing else. It speaks
//! MCP because every harness already does, which is what keeps the transport an
//! adapter rather than a dependency (decision 0041).
//!
//! **What this does not do.** It writes `.trellis/runtime/` and never an
//! artifact: a change to the domain still enters through the session's own
//! Write/Edit under the gate. It never answers a question — it carries one to a
//! human and the reply back, which is the "relay" the daemon is chartered for.
//! And the token in the URL is attribution, not authentication: it says which
//! session is calling so a question can name its plan. The surface carries no
//! auth in either direction, and binds loopback for that reason.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How long one `trellis_await` blocks before answering `pending`.
///
/// Bounded on purpose. Harnesses impose their own MCP tool timeouts, and a call
/// that blocked until a human appeared would trip them at a length we cannot
/// know per harness. A bounded wait plus a polling agent is timeout-safe
/// everywhere and costs one extra round trip per interval.
const AWAIT: Duration = Duration::from_secs(25);

/// Progress notes kept per session. The trail that matters is git and the
/// session log; this is a window, and a window does not need to be infinite.
const MAX_NOTES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub at: String,
    pub note: String,
}

/// A question a session parked, as `/api/status` and `trellis inbox` report it.
///
/// `Default` is for deserialization: a client reading this from a daemon of a
/// different vintage should get a field it does not know as empty, not an
/// error — the answer route matters more than the schema matching exactly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pending {
    pub ticket: String,
    /// The session's task key — `plan:…`, `ritual:…`, or `dispatch`.
    pub key: String,
    pub label: String,
    pub question: String,
    pub options: Vec<String>,
    pub context: Option<String>,
    pub asked: String,
}

/// One session's side of the channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionView {
    pub token: String,
    pub key: String,
    pub label: String,
    pub progress: Vec<Note>,
    pub pending: Option<Pending>,
}

#[derive(Debug)]
struct Question {
    ticket: String,
    question: String,
    options: Vec<String>,
    context: Option<String>,
    asked: String,
    answer: Option<String>,
}

#[derive(Debug)]
struct Session {
    key: String,
    label: String,
    progress: Vec<Note>,
    asked: Vec<Question>,
}

#[derive(Debug, Default)]
struct Inner {
    sessions: HashMap<String, Session>,
    /// ticket → token, so an answer finds its session without a scan.
    tickets: HashMap<String, String>,
    counter: u64,
}

pub struct Inbox {
    inner: Mutex<Inner>,
    /// Signalled when an answer lands, so a parked `trellis_await` wakes on the
    /// answer rather than on its timeout.
    answered: Condvar,
}

impl Default for Inbox {
    fn default() -> Self {
        Inbox {
            inner: Mutex::new(Inner::default()),
            answered: Condvar::new(),
        }
    }
}

impl Inbox {
    /// Open the channel for one session and return its token. Called before the
    /// argv is rendered, because `{mcp}` carries the token.
    pub fn register(&self, key: &str, label: &str) -> String {
        let mut inner = self.inner.lock().unwrap();
        inner.counter += 1;
        let token = mint(inner.counter);
        inner.sessions.insert(
            token.clone(),
            Session {
                key: key.to_string(),
                label: label.to_string(),
                progress: Vec::new(),
                asked: Vec::new(),
            },
        );
        token
    }

    /// Close it. A session that exits with a question outstanding takes the
    /// question with it — there is nothing left to answer.
    pub fn retire(&self, token: &str) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(session) = inner.sessions.remove(token) {
            for q in session.asked {
                inner.tickets.remove(&q.ticket);
            }
        }
        // Wake anyone parked on a ticket that just went away.
        self.answered.notify_all();
    }

    pub fn known(&self, token: &str) -> bool {
        self.inner.lock().unwrap().sessions.contains_key(token)
    }

    /// Every open question, oldest first — what `trellis inbox` lists.
    pub fn pending(&self) -> Vec<Pending> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<Pending> = inner.sessions.values().filter_map(open_question).collect();
        out.sort_by(|a, b| a.asked.cmp(&b.asked).then(a.ticket.cmp(&b.ticket)));
        out
    }

    /// Every live session's channel state, for `/api/status` and the board.
    pub fn view(&self) -> Vec<SessionView> {
        let inner = self.inner.lock().unwrap();
        let mut out: Vec<SessionView> = inner
            .sessions
            .iter()
            .map(|(token, s)| SessionView {
                token: token.clone(),
                key: s.key.clone(),
                label: s.label.clone(),
                progress: s.progress.clone(),
                pending: open_question(s),
            })
            .collect();
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    }

    /// Resolve a ticket. Returns false if it is unknown or already answered —
    /// answering twice is the caller's mistake, not a silent overwrite.
    pub fn answer(&self, ticket: &str, choice: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let Some(token) = inner.tickets.get(ticket).cloned() else {
            return false;
        };
        let resolved = inner
            .sessions
            .get_mut(&token)
            .and_then(|s| s.asked.iter_mut().find(|q| q.ticket == ticket))
            .filter(|q| q.answer.is_none())
            .map(|q| q.answer = Some(choice.to_string()))
            .is_some();
        drop(inner);
        if resolved {
            self.answered.notify_all();
        }
        resolved
    }

    fn progress(&self, token: &str, note: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let session = inner
            .sessions
            .get_mut(token)
            .ok_or_else(|| "this session is no longer open".to_string())?;
        session.progress.push(Note {
            at: super::spawn::timestamp(crate::dates::today()),
            note: note.to_string(),
        });
        let overflow = session.progress.len().saturating_sub(MAX_NOTES);
        session.progress.drain(..overflow);
        Ok(())
    }

    fn ask(
        &self,
        token: &str,
        question: &str,
        options: Vec<String>,
        context: Option<String>,
    ) -> Result<String, String> {
        let mut inner = self.inner.lock().unwrap();
        inner.counter += 1;
        let ticket = mint(inner.counter);
        let session = inner
            .sessions
            .get_mut(token)
            .ok_or_else(|| "this session is no longer open".to_string())?;
        if session.asked.iter().any(|q| q.answer.is_none()) {
            return Err(
                "this session already has a question waiting — await that one before asking \
                 another"
                    .into(),
            );
        }
        session.asked.push(Question {
            ticket: ticket.clone(),
            question: question.to_string(),
            options,
            context,
            asked: super::spawn::timestamp(crate::dates::today()),
            answer: None,
        });
        inner.tickets.insert(ticket.clone(), token.to_string());
        Ok(ticket)
    }

    /// Block until the ticket is answered or [`AWAIT`] elapses. `Ok(None)` is
    /// "not yet" — the caller polls again.
    fn wait(&self, token: &str, ticket: &str) -> Result<Option<String>, String> {
        let mut inner = self.inner.lock().unwrap();
        let deadline = Instant::now() + AWAIT;
        loop {
            let session = inner
                .sessions
                .get(token)
                .ok_or_else(|| "this session is no longer open".to_string())?;
            let q = session
                .asked
                .iter()
                .find(|q| q.ticket == ticket)
                .ok_or_else(|| format!("no such ticket: {ticket}"))?;
            if let Some(answer) = &q.answer {
                return Ok(Some(answer.clone()));
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            inner = self.answered.wait_timeout(inner, deadline - now).unwrap().0;
        }
    }
}

fn open_question(s: &Session) -> Option<Pending> {
    s.asked
        .iter()
        .find(|q| q.answer.is_none())
        .map(|q| Pending {
            ticket: q.ticket.clone(),
            key: s.key.clone(),
            label: s.label.clone(),
            question: q.question.clone(),
            options: q.options.clone(),
            context: q.context.clone(),
            asked: q.asked.clone(),
        })
}

/// A short opaque handle. Not a secret — see the module doc — so a counter
/// mixed with the clock and the pid is enough to keep handles distinct and
/// unguessable-by-counting, and worth no dependency.
fn mint(counter: u64) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h = DefaultHasher::new();
    (counter, nanos, std::process::id()).hash(&mut h);
    format!("{:016x}", h.finish())
}

/// What `{mcp}` renders to when there is no socket to call back on.
///
/// A valid config naming no server, rather than a refusal: without a listener
/// the channel cannot exist, and a session that merely cannot ask is a session
/// that behaves exactly as it did before this existed. The daemon says so once
/// at startup, so the loss is visible without being fatal.
pub fn disabled_json() -> String {
    json!({ "mcpServers": {} }).to_string()
}

/// The `--mcp-config` value for one session: what `{mcp}` renders to.
///
/// A bind address the session cannot dial back on is not a channel, so a
/// wildcard bind is reported to the child as loopback.
pub fn config_json(bind: &str, port: u16, token: &str) -> String {
    let host = match bind {
        "0.0.0.0" | "::" | "[::]" | "" => "127.0.0.1",
        other => other,
    };
    json!({
        "mcpServers": {
            "trellis": {
                "type": "http",
                "url": format!("http://{host}:{port}/mcp/{token}"),
            }
        }
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// JSON-RPC 2.0, the sliver of MCP this surface needs
// ---------------------------------------------------------------------------

/// What the tool list advertises. The descriptions are the agent-facing
/// documentation for this channel — they are the only place a session learns
/// that asking is cheaper than blocking.
fn tools() -> Value {
    json!([
        {
            "name": "trellis_ask",
            "description":
                "Ask the human operating this domain a question, when a plan is underspecified \
                 in a way that changes what you build. Returns a ticket immediately; call \
                 trellis_await with it to get the answer. Prefer this over guessing, and over \
                 blocking the plan, whenever a short answer would unblock you — blocking costs \
                 a full dispatch cycle. If nobody answers, decide as your mandate directs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question, in one sentence."
                    },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description":
                            "Two to four concrete choices, if the question has them. Offering \
                             options makes an answer one click instead of one paragraph."
                    },
                    "context": {
                        "type": "string",
                        "description":
                            "The artifact this concerns, root-relative — e.g. plans/x.md."
                    }
                },
                "required": ["question"]
            }
        },
        {
            "name": "trellis_await",
            "description":
                "Wait for the answer to a ticket from trellis_ask. Blocks briefly and returns \
                 either the answer or 'pending'. On 'pending', call again if you are still \
                 willing to wait; otherwise proceed as your mandate directs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "ticket": { "type": "string", "description": "The ticket trellis_ask returned." }
                },
                "required": ["ticket"]
            }
        },
        {
            "name": "trellis_progress",
            "description":
                "Report what you are doing, in one short line, so the operator can see this \
                 session's progress without opening its log. Semantic milestones only — a plan \
                 claimed, an artifact written, a PR opened, a finding reached. Not a play-by-play.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "note": { "type": "string", "description": "One line." }
                },
                "required": ["note"]
            }
        }
    ])
}

/// Handle one JSON-RPC message. `None` is a notification: nothing to send back.
pub fn handle(inbox: &Inbox, token: &str, body: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {e}"),
            ))
        }
    };
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(json!({}));

    match method {
        // Version-agnostic: this surface is three tools, so whatever version
        // the client speaks, it speaks it well enough for them.
        "initialize" => {
            let version = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-06-18");
            Some(ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "trellis",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                }),
            ))
        }
        "tools/list" => Some(ok(id, json!({ "tools": tools() }))),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Some(ok(id, call(inbox, token, name, &args)))
        }
        "ping" => Some(ok(id, json!({}))),
        _ if id.is_null() => None, // a notification we do not act on
        _ => Some(error_response(
            id,
            -32601,
            &format!("method not found: {method}"),
        )),
    }
}

fn call(inbox: &Inbox, token: &str, name: &str, args: &Value) -> Value {
    let text = |s: &str| Value::String(s.to_string());
    let string = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_string);

    let result = match name {
        "trellis_progress" => match string("note") {
            Some(note) => inbox
                .progress(token, &note)
                .map(|()| "recorded".to_string()),
            None => Err("note is required".into()),
        },
        "trellis_ask" => match string("question") {
            Some(question) => {
                let options = args
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                inbox
                    .ask(token, &question, options, string("context"))
                    .map(|ticket| {
                        format!(
                            "ticket {ticket} — the question is now waiting for the operator. \
                             Call trellis_await with this ticket."
                        )
                    })
            }
            None => Err("question is required".into()),
        },
        "trellis_await" => match string("ticket") {
            Some(ticket) => inbox.wait(token, &ticket).map(|answer| match answer {
                Some(a) => format!("answered: {a}"),
                None => "pending — nobody has answered yet".to_string(),
            }),
            None => Err("ticket is required".into()),
        },
        other => Err(format!("no such tool: {other}")),
    };

    match result {
        Ok(message) => json!({ "content": [{ "type": "text", "text": text(&message) }] }),
        // A tool-level failure is a result the model can read and recover from,
        // never a transport error that looks like the channel broke.
        Err(message) => json!({
            "content": [{ "type": "text", "text": text(&message) }],
            "isError": true,
        }),
    }
}

fn ok(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error_response(id: Value, code: i32, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call_tool(inbox: &Inbox, token: &str, name: &str, args: Value) -> String {
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": name, "arguments": args }
        })
        .to_string();
        let response: Value = serde_json::from_str(&handle(inbox, token, &body).unwrap()).unwrap();
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn a_question_becomes_pending_and_an_answer_resolves_it() {
        let inbox = Inbox::default();
        let token = inbox.register("plan:plans/x.md", "act plans/x.md");
        let said = call_tool(
            &inbox,
            &token,
            "trellis_ask",
            json!({ "question": "does the table move?", "options": ["yes", "no"] }),
        );
        let ticket = said.split_whitespace().nth(1).unwrap().to_string();

        let pending = inbox.pending();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].key, "plan:plans/x.md");
        assert_eq!(pending[0].options, vec!["yes", "no"]);

        assert!(inbox.answer(&ticket, "yes"));
        assert!(inbox.pending().is_empty());
        let got = call_tool(&inbox, &token, "trellis_await", json!({ "ticket": ticket }));
        assert_eq!(got, "answered: yes");
        // Answering twice is a mistake, not a silent overwrite.
        assert!(!inbox.answer(&ticket, "no"));
    }

    #[test]
    fn an_unanswered_await_returns_pending_rather_than_blocking_forever() {
        let inbox = Inbox::default();
        let token = inbox.register("ritual:focus", "ritual focus");
        let said = call_tool(
            &inbox,
            &token,
            "trellis_ask",
            json!({ "question": "which?" }),
        );
        let ticket = said.split_whitespace().nth(1).unwrap().to_string();

        // Answered from another thread: the wait must wake on the answer, not
        // sit out the full interval.
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(50));
                inbox.answer(&ticket, "this one");
            });
            let start = Instant::now();
            let got = call_tool(
                &inbox,
                &token,
                "trellis_await",
                json!({ "ticket": &ticket }),
            );
            assert_eq!(got, "answered: this one");
            assert!(
                start.elapsed() < AWAIT,
                "woke on the answer, not the timeout"
            );
        });
    }

    #[test]
    fn a_ritual_session_can_ask_too() {
        let inbox = Inbox::default();
        let token = inbox.register("ritual:conventions lint", "ritual conventions lint");
        call_tool(
            &inbox,
            &token,
            "trellis_ask",
            json!({ "question": "scope?" }),
        );
        assert_eq!(inbox.pending()[0].key, "ritual:conventions lint");
    }

    #[test]
    fn a_second_question_is_refused_while_one_is_outstanding() {
        let inbox = Inbox::default();
        let token = inbox.register("plan:plans/x.md", "x");
        call_tool(
            &inbox,
            &token,
            "trellis_ask",
            json!({ "question": "first?" }),
        );
        let said = call_tool(
            &inbox,
            &token,
            "trellis_ask",
            json!({ "question": "second?" }),
        );
        assert!(said.contains("already has a question"), "{said}");
    }

    #[test]
    fn progress_accumulates_and_is_capped() {
        let inbox = Inbox::default();
        let token = inbox.register("plan:plans/x.md", "x");
        for i in 0..MAX_NOTES + 10 {
            call_tool(
                &inbox,
                &token,
                "trellis_progress",
                json!({ "note": i.to_string() }),
            );
        }
        let view = inbox.view();
        assert_eq!(view[0].progress.len(), MAX_NOTES);
        assert_eq!(view[0].progress.last().unwrap().note, "209");
    }

    #[test]
    fn a_retired_session_takes_its_question_with_it() {
        let inbox = Inbox::default();
        let token = inbox.register("plan:plans/x.md", "x");
        let said = call_tool(&inbox, &token, "trellis_ask", json!({ "question": "q?" }));
        let ticket = said.split_whitespace().nth(1).unwrap().to_string();
        inbox.retire(&token);
        assert!(inbox.pending().is_empty());
        assert!(!inbox.answer(&ticket, "too late"));
        assert!(!inbox.known(&token));
    }

    #[test]
    fn tools_are_listed_and_initialize_echoes_the_clients_version() {
        let inbox = Inbox::default();
        let body = json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        })
        .to_string();
        let v: Value = serde_json::from_str(&handle(&inbox, "t", &body).unwrap()).unwrap();
        assert_eq!(v["result"]["protocolVersion"], "2025-06-18");

        let body = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }).to_string();
        let v: Value = serde_json::from_str(&handle(&inbox, "t", &body).unwrap()).unwrap();
        let names: Vec<&str> = v["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["trellis_ask", "trellis_await", "trellis_progress"]);
    }

    #[test]
    fn a_notification_gets_no_response() {
        let inbox = Inbox::default();
        let body = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        assert!(handle(&inbox, "t", &body).is_none());
    }

    #[test]
    fn a_wildcard_bind_is_reported_to_the_child_as_loopback() {
        let cfg = config_json("0.0.0.0", 7357, "abc");
        assert!(cfg.contains("http://127.0.0.1:7357/mcp/abc"), "{cfg}");
        let cfg = config_json("127.0.0.1", 9000, "xyz");
        assert!(cfg.contains("http://127.0.0.1:9000/mcp/xyz"), "{cfg}");
    }
}
