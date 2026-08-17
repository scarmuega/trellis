//! The first shipped escalation transport (decision 0043): a newly opened
//! record becomes a herdr toast.
//!
//! 0036 chartered exactly this — "a binding that needs a push adds one *over*
//! the records" — and 0038 built the seam and refused to fill it ahead of
//! evidence. The evidence arrived: an operator who lives in a herdr session
//! and reads escalations from the board a tick late, or not at all. The
//! record in the tree stays the system of record; this only ever announces
//! one.

use super::client::Client;
use crate::daemon::channels::Channel;
use crate::daemon::mcp::Pending;
use crate::escalate::ListedRecord;

pub struct HerdrChannel {
    client: Client,
}

impl HerdrChannel {
    pub fn new(client: Client) -> HerdrChannel {
        HerdrChannel { client }
    }
}

impl Channel for HerdrChannel {
    fn name(&self) -> &str {
        "herdr"
    }

    /// One toast per record: who must act, in the title; what is asked, in
    /// the body. Failure is the caller's to log and never fatal — the trait
    /// says so, and a domain's clock outranks a notification.
    fn escalation_opened(&self, record: &ListedRecord) -> anyhow::Result<()> {
        let title = format!(
            "escalation: {} → {}",
            record.artifact,
            record.to.as_deref().unwrap_or("?")
        );
        self.client
            .notification_show(&title, record.asks.as_deref())?;
        Ok(())
    }

    /// The ask channel's low-latency half: a parked question becomes a toast
    /// that names the command that answers it.
    fn question_parked(&self, pending: &Pending) -> anyhow::Result<()> {
        self.client.notification_show(
            &format!("session asks: {}", pending.label),
            Some(&format!(
                "{} — answer: trellis inbox answer {} <choice>",
                pending.question, pending.ticket
            )),
        )?;
        Ok(())
    }
}
