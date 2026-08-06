//! The herdr socket client: newline-delimited JSON over a unix socket, one
//! request per connection — the server hangs up after each exchange, so a
//! connection held open would be a bug waiting for a second request.
//!
//! Hand-rolled over `UnixStream` on `client.rs`'s precedent: a handful of
//! requests do not justify a client stack, and the workload is blocking
//! localhost I/O — the same argument that refused tokio (decision 0038).
//!
//! The protocol is pinned (`PROTOCOL`) and checked against the server's own
//! `ping` response at connect, so drift is a refusal at startup rather than a
//! misparse mid-run. Response structs deserialize leniently — unknown fields
//! ignored, optional ones defaulted — so herdr *adding* to the protocol never
//! breaks this client; only a version bump does, and says so.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

/// The herdr socket protocol this client speaks (herdr 0.8.x). The server
/// reports its own in `pong`; a mismatch is refused by `handshake`.
pub const PROTOCOL: u64 = 19;

/// Long enough to cross a local socket, short enough that a dead server is
/// reported rather than waited on.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Request ids only have to be unique per connection, and each connection
/// carries one request — but a counter costs nothing and reads better in a
/// wire capture than a constant.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct Client {
    socket: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Pong {
    pub version: String,
    pub protocol: u64,
}

/// One agent pane, as herdr reports it. Everything beyond identity and status
/// is defaulted: this struct tracks what the pool reads, not what herdr says.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentInfo {
    pub pane_id: String,
    pub workspace_id: String,
    /// `idle | working | blocked | done | unknown` — kept as a string so a
    /// state this pin has never heard of degrades to unrecognized, not
    /// unparseable.
    pub agent_status: String,
    #[serde(default)]
    pub state_change_seq: u64,
    #[serde(default)]
    pub interactive_ready: bool,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
}

impl Client {
    /// A client for the given socket, or for herdr's default one: the
    /// `HERDR_SOCKET_PATH` the environment names, else the path the herdr CLI
    /// itself defaults to.
    pub fn new(socket: Option<&str>) -> Client {
        let path = socket
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HERDR_SOCKET_PATH").map(PathBuf::from))
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME").unwrap_or_default();
                Path::new(&home).join(".config/herdr/herdr.sock")
            });
        Client { socket: path }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    /// Ping and hold the server to this client's protocol pin. The startup
    /// gate: everything after it may assume the wire means what this file
    /// thinks it means.
    pub fn handshake(&self) -> anyhow::Result<Pong> {
        let pong: Pong = self.parse(self.request("ping", json!({}), TIMEOUT)?)?;
        if pong.protocol != PROTOCOL {
            anyhow::bail!(
                "herdr at {} speaks protocol {} but this trellis was built against {} — \
                 update whichever is older",
                self.socket.display(),
                pong.protocol,
                PROTOCOL
            );
        }
        Ok(pong)
    }

    pub fn notification_show(&self, title: &str, body: Option<&str>) -> anyhow::Result<()> {
        self.request(
            "notification.show",
            json!({ "title": title, "body": body, "sound": "request" }),
            TIMEOUT,
        )?;
        Ok(())
    }

    /// Create a workspace and return its id with its root pane's — the pane a
    /// session's agent starts in.
    pub fn workspace_create(&self, cwd: &Path, label: &str) -> anyhow::Result<(String, String)> {
        let result = self.request(
            "workspace.create",
            json!({ "cwd": cwd.display().to_string(), "label": label, "focus": false }),
            TIMEOUT,
        )?;
        let id = string_at(&result, &["workspace", "workspace_id"])?;
        let pane = string_at(&result, &["root_pane", "pane_id"])?;
        Ok((id, pane))
    }

    pub fn workspace_close(&self, workspace_id: &str) -> anyhow::Result<()> {
        self.request(
            "workspace.close",
            json!({ "workspace_id": workspace_id }),
            TIMEOUT,
        )?;
        Ok(())
    }

    /// Stamp identity tokens on a pane, so a restarted daemon can tell its
    /// own sessions from everything else herdr is showing.
    pub fn pane_report_tokens(
        &self,
        pane_id: &str,
        tokens: &[(&str, &str)],
    ) -> anyhow::Result<()> {
        let map: serde_json::Map<String, Value> = tokens
            .iter()
            .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
            .collect();
        self.request(
            "pane.report_metadata",
            json!({ "pane_id": pane_id, "source": "custom:trellis", "tokens": map }),
            TIMEOUT,
        )?;
        Ok(())
    }

    /// Start a supported agent in a pane sitting at its shell prompt. Blocks
    /// until herdr has detected the agent interactive-ready, or `timeout`.
    pub fn agent_start(
        &self,
        name: &str,
        kind: &str,
        pane_id: &str,
        args: &[String],
        timeout: Duration,
    ) -> anyhow::Result<AgentInfo> {
        let result = self.request(
            "agent.start",
            json!({
                "name": name,
                "kind": kind,
                "pane_id": pane_id,
                "args": args,
                "timeout_ms": timeout.as_millis() as u64,
            }),
            // The server holds the request while it watches the pane; the
            // socket must outwait it.
            timeout + TIMEOUT,
        )?;
        self.agent(result)
    }

    /// Submit a prompt and return immediately; completion is `agent_get`'s to
    /// observe, on the daemon's own clock.
    pub fn agent_prompt(&self, target: &str, text: &str) -> anyhow::Result<AgentInfo> {
        let result = self.request(
            "agent.prompt",
            json!({ "target": target, "text": text }),
            TIMEOUT,
        )?;
        self.agent(result)
    }

    pub fn agent_get(&self, target: &str) -> anyhow::Result<AgentInfo> {
        let result = self.request("agent.get", json!({ "target": target }), TIMEOUT)?;
        self.agent(result)
    }

    /// Block until the agent settles into one of `until`. What `wait_all`
    /// leans on, so the socket timeout stretches to cover the wait.
    pub fn agent_wait(
        &self,
        target: &str,
        until: &[&str],
        timeout: Duration,
    ) -> anyhow::Result<AgentInfo> {
        let result = self.request(
            "agent.wait",
            json!({
                "target": target,
                "until": until,
                "timeout_ms": timeout.as_millis() as u64,
            }),
            timeout + TIMEOUT,
        )?;
        self.agent(result)
    }

    pub fn agent_list(&self) -> anyhow::Result<Vec<AgentInfo>> {
        let result = self.request("agent.list", json!({}), TIMEOUT)?;
        let agents = result
            .get("agents")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("herdr's agent list names no agents field"))?;
        Ok(serde_json::from_value(agents)?)
    }

    /// The agent's screen as an operator would see it, ANSI stripped — what
    /// stands in for a log file when the session ran in a pane instead of
    /// behind one. `visible` rather than `recent`: a TUI agent's story is on
    /// its alternate screen, not in the scrollback behind it.
    pub fn agent_read(&self, target: &str, lines: u32) -> anyhow::Result<String> {
        let result = self.request(
            "agent.read",
            json!({ "target": target, "source": "visible", "lines": lines }),
            TIMEOUT,
        )?;
        string_at(&result, &["read", "text"])
    }

    /// The `agent` field every agent-shaped response carries, whatever its
    /// type tag.
    fn agent(&self, result: Value) -> anyhow::Result<AgentInfo> {
        let agent = result
            .get("agent")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("herdr's response names no agent"))?;
        Ok(serde_json::from_value(agent)?)
    }

    fn parse<T: serde::de::DeserializeOwned>(&self, result: Value) -> anyhow::Result<T> {
        Ok(serde_json::from_value(result)?)
    }

    /// One connection, one line out, one line back. Returns the `result`
    /// value; a herdr-side error becomes an `Err` carrying its code and
    /// message.
    fn request(&self, method: &str, params: Value, timeout: Duration) -> anyhow::Result<Value> {
        let stream = UnixStream::connect(&self.socket).map_err(|e| {
            anyhow::anyhow!(
                "cannot reach herdr at {}: {e} — is the herdr server running?",
                self.socket.display()
            )
        })?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(TIMEOUT))?;

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
        let line = json!({ "id": id, "method": method, "params": params }).to_string();
        let mut writer = &stream;
        writeln!(writer, "{line}")?;
        writer.flush()?;

        let mut reply = String::new();
        BufReader::new(&stream).read_line(&mut reply)?;
        if reply.trim().is_empty() {
            anyhow::bail!("herdr closed the connection without answering {method}");
        }
        let reply: Value = serde_json::from_str(&reply)
            .map_err(|e| anyhow::anyhow!("herdr sent a line that is not JSON: {e}"))?;
        if let Some(error) = reply.get("error") {
            let code = error.get("code").and_then(Value::as_str).unwrap_or("?");
            let message = error.get("message").and_then(Value::as_str).unwrap_or("?");
            anyhow::bail!("herdr refused {method}: {code}: {message}");
        }
        reply
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("herdr answered {method} with neither result nor error"))
    }
}

/// A string at a path into a JSON value, with the path in the error — the
/// difference between "herdr changed" and an hour of printf.
fn string_at(value: &Value, path: &[&str]) -> anyhow::Result<String> {
    let mut at = value;
    for key in path {
        at = at
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("herdr's response names no {}", path.join(".")))?;
    }
    at.as_str()
        .map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("herdr's {} is not a string", path.join(".")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    /// A one-shot fake: accepts connections, answers each first line from the
    /// script, connection by connection — the shape the real server has.
    fn serve(dir: &Path, replies: Vec<String>) -> PathBuf {
        let path = dir.join("herdr.sock");
        let listener = UnixListener::bind(&path).unwrap();
        std::thread::spawn(move || {
            for reply in replies {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                let _ = reader.read_line(&mut line);
                let request: Value = serde_json::from_str(&line).unwrap();
                let id = request["id"].as_str().unwrap();
                let mut writer = &stream;
                let _ = writeln!(writer, "{}", reply.replace("{id}", id));
                // Dropping the stream hangs up, as herdr does.
            }
        });
        path
    }

    fn client(dir: &Path, replies: Vec<&str>) -> Client {
        let socket = serve(dir, replies.into_iter().map(str::to_string).collect());
        Client::new(Some(socket.to_str().unwrap()))
    }

    #[test]
    fn a_matching_protocol_shakes_hands() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = client(
            dir.path(),
            vec![r#"{"id":"{id}","result":{"type":"pong","version":"0.8.0","protocol":19}}"#],
        );
        let pong = c.handshake().unwrap();
        assert_eq!(pong.version, "0.8.0");
    }

    #[test]
    fn a_protocol_the_pin_does_not_know_is_refused_by_name() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = client(
            dir.path(),
            vec![r#"{"id":"{id}","result":{"type":"pong","version":"9.9.9","protocol":99}}"#],
        );
        let err = c.handshake().unwrap_err().to_string();
        assert!(err.contains("99"), "{err}");
        assert!(err.contains("19"), "{err}");
    }

    #[test]
    fn a_herdr_error_carries_its_code_and_message() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = client(
            dir.path(),
            vec![r#"{"id":"{id}","error":{"code":"not_found","message":"pane not found"}}"#],
        );
        let err = c.agent_get("p9").unwrap_err().to_string();
        assert!(err.contains("not_found"), "{err}");
        assert!(err.contains("pane not found"), "{err}");
    }

    #[test]
    fn a_workspace_creation_yields_its_id_and_root_pane() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = client(
            dir.path(),
            vec![
                r#"{"id":"{id}","result":{"type":"workspace_created","workspace":{"workspace_id":"w7"},"tab":{},"root_pane":{"pane_id":"p12"}}}"#,
            ],
        );
        let (ws, pane) = c.workspace_create(Path::new("/tmp/x"), "act plans/x.md").unwrap();
        assert_eq!(ws, "w7");
        assert_eq!(pane, "p12");
    }

    #[test]
    fn an_agent_answer_parses_leniently() {
        let dir = tempfile::TempDir::new().unwrap();
        // Fields this pin has never heard of, and several it knows omitted.
        let c = client(
            dir.path(),
            vec![
                r#"{"id":"{id}","result":{"type":"agent_info","agent":{"pane_id":"p1","workspace_id":"w1","agent_status":"working","novel_field":{"deep":true}}}}"#,
            ],
        );
        let agent = c.agent_get("p1").unwrap();
        assert_eq!(agent.agent_status, "working");
        assert_eq!(agent.state_change_seq, 0);
        assert!(agent.tokens.is_empty());
    }

    #[test]
    fn a_missing_server_is_reported_as_unreachable() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = Client::new(Some(dir.path().join("nope.sock").to_str().unwrap()));
        let err = c.handshake().unwrap_err().to_string();
        assert!(err.contains("cannot reach herdr"), "{err}");
    }
}
