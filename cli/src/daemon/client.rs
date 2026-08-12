//! `trellis inbox` — answering a session that parked a question.
//!
//! This is the contract surface for the back-channel (decision 0041): the one
//! that must work. The board shows the same state and is a glance surface, not
//! this. That is not a preference about terminals — the board is a *plan* view,
//! and only `act` sessions are one-to-one with plans, so a question from a
//! ritual or from the dispatch scan has no card to appear on.
//!
//! It talks to the daemon over the loopback API rather than reading
//! `.trellis/runtime/` directly, which keeps one writer to that state. The
//! HTTP is hand-rolled over `TcpStream` for the same reason the server is
//! `tiny_http`: three requests do not justify a client stack.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use super::config::RuntimeConfig;
use super::mcp::Pending;

/// Long enough to cross loopback, short enough that a dead daemon is reported
/// rather than waited on.
const TIMEOUT: Duration = Duration::from_secs(10);

/// How often `--watch` asks. A human is waiting on the other side of this, so
/// it is a person's idea of "promptly", not a machine's.
const POLL: Duration = Duration::from_secs(1);

/// Where the daemon said it is listening.
fn endpoint(root: &Path) -> anyhow::Result<String> {
    // The sessions live where they were spawned (decision 0046): the
    // dispatcher first, a rituals pass next, the read-only server last —
    // it answers /api/status but holds no inbox of its own.
    for file in [
        super::DISPATCH_ADDRFILE,
        super::RITUALS_ADDRFILE,
        super::SERVE_ADDRFILE,
    ] {
        let path = RuntimeConfig::runtime_dir(root).join(file);
        if let Ok(addr) = std::fs::read_to_string(&path) {
            return Ok(addr.trim().to_string());
        }
    }
    anyhow::bail!(
        "no daemon is running on this root — start `trellis dispatch run`, \
         or point at another with --addr"
    )
}

fn request(addr: &str, method: &str, path: &str, body: Option<&str>) -> anyhow::Result<String> {
    let mut stream = TcpStream::connect(addr)
        .map_err(|e| anyhow::anyhow!("cannot reach the daemon at {addr}: {e}"))?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;

    let body = body.unwrap_or("");
    // `Connection: close` is what lets the read below end at EOF instead of
    // parsing a length we would then have to trust.
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;

    let mut text = String::new();
    stream.read_to_string(&mut text)?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow::anyhow!("the daemon sent an incomplete response"))?;
    let code: u16 = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("the daemon sent no status line"))?;
    if !(200..300).contains(&code) {
        let detail = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| body.trim().to_string());
        anyhow::bail!("{detail}");
    }
    Ok(body.to_string())
}

pub fn pending(root: &Path, addr: Option<&str>) -> anyhow::Result<Vec<Pending>> {
    let addr = match addr {
        Some(a) => a.to_string(),
        None => endpoint(root)?,
    };
    let body = request(&addr, "GET", "/api/status", None)?;
    let status: serde_json::Value = serde_json::from_str(&body)?;
    let questions = status
        .get("pending")
        .cloned()
        .unwrap_or(serde_json::json!([]));
    Ok(serde_json::from_value(questions)?)
}

/// Resolve a ticket. A bare number picks that option, one-based, because
/// retyping an option the daemon just printed is a way to answer the wrong
/// question.
pub fn answer(
    root: &Path,
    addr: Option<&str>,
    ticket: &str,
    choice: &str,
) -> anyhow::Result<String> {
    let addr = match addr {
        Some(a) => a.to_string(),
        None => endpoint(root)?,
    };
    let open = pending(root, Some(&addr))?;
    let question = open
        .iter()
        .find(|p| p.ticket == ticket)
        .ok_or_else(|| anyhow::anyhow!("no open question with ticket {ticket}"))?;

    let resolved = match choice.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= question.options.len() => question.options[n - 1].clone(),
        _ => choice.to_string(),
    };
    let body = serde_json::json!({ "answer": resolved }).to_string();
    request(
        &addr,
        "POST",
        &format!("/api/sessions/{ticket}/answer"),
        Some(&body),
    )?;
    Ok(resolved)
}

/// Block until at least one question is open, then return them.
pub fn watch(root: &Path, addr: Option<&str>) -> anyhow::Result<Vec<Pending>> {
    loop {
        let open = pending(root, addr)?;
        if !open.is_empty() {
            return Ok(open);
        }
        std::thread::sleep(POLL);
    }
}

/// The listing, as a human reads it. Ends with the command that answers,
/// because a question you cannot see how to answer is only half-delivered.
pub fn render(open: &[Pending]) -> String {
    if open.is_empty() {
        return "no sessions are waiting on you".to_string();
    }
    let mut out = String::new();
    for p in open {
        out.push_str(&format!("{}  {}\n", p.ticket, p.label));
        out.push_str(&format!("  {}\n", p.question));
        for (i, option) in p.options.iter().enumerate() {
            out.push_str(&format!("  {}  {option}\n", i + 1));
        }
        if let Some(context) = &p.context {
            out.push_str(&format!("  context: {context}\n"));
        }
        out.push_str(&format!(
            "  answer: trellis inbox answer {} <choice>\n\n",
            p.ticket
        ));
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(options: &[&str]) -> Pending {
        Pending {
            ticket: "abc123".into(),
            key: "plan:plans/x.md".into(),
            label: "act plans/x.md → org/platform-eng".into(),
            question: "does the table move?".into(),
            options: options.iter().map(|s| s.to_string()).collect(),
            context: Some("plans/x.md".into()),
            asked: "2026-08-05T091100".into(),
        }
    }

    #[test]
    fn an_empty_inbox_says_so_rather_than_printing_nothing() {
        assert_eq!(render(&[]), "no sessions are waiting on you");
    }

    #[test]
    fn the_listing_numbers_the_options_and_shows_how_to_answer() {
        let text = render(&[question(&["moves with billing", "stays in core"])]);
        assert!(text.contains("1  moves with billing"), "{text}");
        assert!(text.contains("2  stays in core"), "{text}");
        assert!(text.contains("context: plans/x.md"), "{text}");
        assert!(
            text.contains("trellis inbox answer abc123 <choice>"),
            "{text}"
        );
    }

    #[test]
    fn a_question_with_no_options_still_renders() {
        let text = render(&[question(&[])]);
        assert!(text.contains("does the table move?"), "{text}");
    }
}
