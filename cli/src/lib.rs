//! Deterministic kernel for Trellis domain roots.
//!
//! Codifies the mechanical conventions of the Trellis operating model —
//! lint, dispatch scan, write gate, lifecycle porcelain, generated views —
//! while leaving every judgment call to the mandated roles. The judgment
//! boundary is explicit: whatever this kernel skips, it reports as skipped.

pub mod daemon;
pub mod dates;
pub mod dispatch;
pub mod domain;
pub mod escalate;
pub mod facts;
pub mod fmedit;
pub mod frontmatter;
pub mod gate;
pub mod gitio;
pub mod graph;
pub mod lint;
pub mod markdown;
pub mod model;
pub mod org;
pub mod plan_ops;
pub mod readiness;
pub mod refs;
pub mod registries;
pub mod root;
pub mod scaffold;
pub mod skills;
pub mod tree;
pub mod views;

/// Spec version embedded at build time from `spec/model.md`'s title.
pub fn spec_version() -> u32 {
    env!("TRELLIS_SPEC_VERSION")
        .parse()
        .expect("build.rs guarantees a number")
}
