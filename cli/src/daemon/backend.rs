//! The session backend seam: who carries a running session.
//!
//! An enum, not a trait — both variants ship in this binary, `match` keeps
//! the two lifecycles readable side by side, and a `dyn` seam with no third
//! implementor would be the "infrastructure ahead of evidence" 0041 names.
//!
//! `Process` is the default and the reference: headless one-shot children,
//! exit codes, budget caps (`spawn.rs`). `Herdr` is the opt-in trade
//! recorded in decision 0043: sessions as interactive panes — attachable,
//! restart-durable, budget-uncapped.

use std::path::Path;

use super::spawn::{Finished, InFlightView, Spawner};
use crate::daemon::config::RuntimeConfig;

#[cfg(unix)]
use super::herdr::pool::HerdrPool;

/// One rendered session, both halves. The process backend runs `argv` with
/// the prompt already substituted into it; the herdr backend starts the
/// agent with `argv` as its flags and submits `prompt` to the running TUI.
pub struct Launch {
    pub argv: Vec<String>,
    pub prompt: String,
}

pub enum Backend {
    Process(Spawner),
    #[cfg(unix)]
    Herdr(HerdrPool),
}

impl Backend {
    pub fn connect(root: &Path, cfg: &RuntimeConfig, once: bool) -> anyhow::Result<Backend> {
        let max = cfg.scheduler.max_concurrent;
        match cfg.harness.backend {
            super::config::BackendKind::Process => {
                let mut spawner = Spawner::new(root, max);
                if once {
                    spawner = spawner.once_per_task();
                }
                Ok(Backend::Process(spawner))
            }
            #[cfg(unix)]
            super::config::BackendKind::Herdr => {
                let mut pool = HerdrPool::connect(root, &cfg.harness.herdr, max)?;
                if once {
                    pool = pool.once_per_task();
                }
                Ok(Backend::Herdr(pool))
            }
            #[cfg(not(unix))]
            super::config::BackendKind::Herdr => {
                anyhow::bail!("the herdr backend speaks a unix socket — this platform has none")
            }
        }
    }

    /// Whether `fire` should render for the herdr shape: agent args and a
    /// separately submitted prompt, rather than one argv carrying both.
    pub fn is_herdr(&self) -> bool {
        !matches!(self, Backend::Process(_))
    }

    pub fn spawn(
        &mut self,
        key: &str,
        label: &str,
        launch: &Launch,
        token: Option<String>,
    ) -> anyhow::Result<String> {
        match self {
            Backend::Process(s) => s.spawn(key, label, &launch.argv, token),
            #[cfg(unix)]
            Backend::Herdr(p) => p.spawn(key, label, &launch.argv, &launch.prompt, token),
        }
    }

    pub fn reap(&mut self) -> Vec<Finished> {
        match self {
            Backend::Process(s) => s.reap(),
            #[cfg(unix)]
            Backend::Herdr(p) => p.reap(),
        }
    }

    /// Block until every session settles — the `--once` drain.
    pub fn drain(&mut self) -> Vec<Finished> {
        match self {
            Backend::Process(s) => s.wait_all(),
            #[cfg(unix)]
            Backend::Herdr(p) => p.drain(),
        }
    }

    /// Stop for shutdown: how many sessions were told, and what came back.
    /// The process backend signals and waits; the herdr pool applies its
    /// `on_shutdown` policy — `detach` tells nobody and keeps the sessions.
    pub fn stop_all(&mut self) -> (usize, Vec<Finished>) {
        match self {
            Backend::Process(s) => {
                let signalled = s.cancel_all();
                (signalled, s.wait_all())
            }
            #[cfg(unix)]
            Backend::Herdr(p) => p.stop_all(),
        }
    }

    pub fn is_busy(&self, key: &str) -> bool {
        match self {
            Backend::Process(s) => s.is_busy(key),
            #[cfg(unix)]
            Backend::Herdr(p) => p.is_busy(key),
        }
    }

    pub fn already_ran(&self, key: &str) -> bool {
        match self {
            Backend::Process(s) => s.already_ran(key),
            #[cfg(unix)]
            Backend::Herdr(p) => p.already_ran(key),
        }
    }

    pub fn capacity(&self) -> usize {
        match self {
            Backend::Process(s) => s.capacity(),
            #[cfg(unix)]
            Backend::Herdr(p) => p.capacity(),
        }
    }

    pub fn in_flight(&self) -> Vec<InFlightView> {
        match self {
            Backend::Process(s) => s.in_flight(),
            #[cfg(unix)]
            Backend::Herdr(p) => p.in_flight(),
        }
    }
}
