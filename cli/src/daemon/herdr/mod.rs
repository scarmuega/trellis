//! The herdr binding — a terminal workspace manager as notification surface
//! and, opted into, as session surface (decision 0043).
//!
//! Two adapters over one socket. `channel` fills the escalation-push seam
//! 0036 chartered: a newly opened record becomes a toast in the operator's
//! terminal. `pool` is the opt-in session backend: sessions run as
//! interactive agents in herdr panes — attachable, visible in the sidebar,
//! and they survive a daemon restart. The process spawner stays the default;
//! nothing here is required to serve a domain.
//!
//! The boundary the daemon's crate doc draws holds unchanged: herdr carries
//! sessions and notifications, and decides nothing. Herdr's `blocked` is a
//! *session-level* stall — an agent visibly waiting at its own UI — and never
//! becomes a plan-level escalation record, which only the session itself may
//! write, under its mandate. The two layers meet nowhere.

pub mod channel;
pub mod client;
pub mod pool;
