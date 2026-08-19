//! Guarded plan lifecycle transitions. The flip itself is a one-line
//! frontmatter edit; the guard refuses illegal source states so an agent
//! cannot claim a draft or re-release a retired plan. Who may flip is the
//! invoker's mandate, not the tool's business.

use std::path::Path;

use anyhow::bail;

use crate::fmedit;
use crate::model::PlanStatus;

/// What an `active` plan is parked on, if anything — a PR awaiting its
/// owner's verdict, or any other ref a human must act on before the work can
/// move (schema in `spec/model.md`).
pub const HANDOFF: &str = "handoff";

pub fn current_status(path: &Path) -> anyhow::Result<PlanStatus> {
    let raw = fmedit::get(path, "status")?
        .ok_or_else(|| anyhow::anyhow!("{} declares no status:", path.display()))?;
    PlanStatus::parse(&raw)
        .ok_or_else(|| anyhow::anyhow!("{} has illegal status: {raw}", path.display()))
}

/// Flip `status:` after checking the source state is legal for the move.
pub fn flip(path: &Path, from: &[PlanStatus], to: PlanStatus) -> anyhow::Result<PlanStatus> {
    let current = current_status(path)?;
    if current == to {
        bail!("{} is already {}", path.display(), to.as_str());
    }
    if !from.contains(&current) {
        bail!(
            "{} is {} — this transition starts from {}",
            path.display(),
            current.as_str(),
            from.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    fmedit::set_scalar(path, "status", to.as_str(), false)?;
    // A claim is a fresh attempt: whatever the last taker parked the plan on
    // is spent, and a stale handoff would park it again the moment this
    // session ends. Absent is the common case and costs one read.
    if to == PlanStatus::Active {
        fmedit::remove(path, HANDOFF)?;
    }
    Ok(current)
}

/// The mandated relay hand-off (decision 0059): a new owner and `ready` in
/// one guarded move. Legal from `active` — the session-verdict case — or
/// `ready` — the operator reassigning a queued plan. Clears any stale
/// `handoff:` ref: a passed plan must not stay parked on the previous
/// taker's proposal. Whether the target role exists is the caller's check;
/// this op owns only the frontmatter mechanics.
pub fn pass(path: &Path, to_owner: &str) -> anyhow::Result<PlanStatus> {
    let current = current_status(path)?;
    if !matches!(current, PlanStatus::Active | PlanStatus::Ready) {
        bail!(
            "{} is {} — a pass starts from active | ready",
            path.display(),
            current.as_str()
        );
    }
    if fmedit::get(path, "owner")?.is_none() {
        bail!(
            "{} declares no owner: — a pass moves ownership, it does not invent it",
            path.display()
        );
    }
    fmedit::set_scalar(path, "owner", to_owner, false)?;
    if current == PlanStatus::Active {
        fmedit::set_scalar(path, "status", PlanStatus::Ready.as_str(), false)?;
    }
    fmedit::remove(path, HANDOFF)?;
    Ok(current)
}

/// Record (or clear) what an active plan is parked on. Guarded to `active`
/// for the same reason the flips are guarded: a handoff on a plan nobody
/// holds is a claim about work that is not happening.
pub fn set_handoff(path: &Path, reference: Option<&str>) -> anyhow::Result<()> {
    let current = current_status(path)?;
    if current != PlanStatus::Active {
        bail!(
            "{} is {} — a handoff records what an active plan awaits",
            path.display(),
            current.as_str()
        );
    }
    match reference {
        Some(reference) if !reference.trim().is_empty() => {
            fmedit::set_scalar(path, HANDOFF, reference.trim(), true)
        }
        _ => fmedit::remove(path, HANDOFF).map(|_| ()),
    }
}

/// What an active plan is declared to be parked on, if anything. Empty is
/// absent: a `handoff:` with nothing after it says no more than no key at
/// all, and both mean the plan is still somebody's to advance.
pub fn handoff(path: &Path) -> anyhow::Result<Option<String>> {
    Ok(fmedit::get(path, HANDOFF)?.filter(|h| !h.trim().is_empty()))
}

/// The frontmatter an errand's instruction depends on, in a fixed order.
///
/// Deliberately *not* every key. The runtime writes to a plan constantly —
/// `flip` moves `status:` and clears `handoff:`, `relinquish` moves it back —
/// so a fingerprint over the whole file would go stale the first time anything
/// dispatched, which is the opposite of what it is for. What is left is what an
/// operator was looking at when they wrote the ask: the substance, plus the
/// fields that decide *who* would act on it and *how big* that session is.
const FINGERPRINTED: &[&str] = &[
    "owner",
    "type",
    "complexity",
    "awaits",
    "contexts",
    "subdomains",
    "metrics",
    "tags",
];

/// FNV-1a 64. Hand-rolled because this value is *persisted*: an errand queued
/// under one binary is judged under the next one, and
/// `std::collections::hash_map::DefaultHasher` is explicitly not stable across
/// Rust releases — an upgrade would silently invalidate every standing ask.
/// Eight lines and a fixed constant is the whole cost of never having to think
/// about that again. Not a security hash and never used as one.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// What a plan *said* when an ask was written about it.
///
/// An errand is free text an operator wrote while reading one plan, and it
/// stays coherent only for as long as that plan does. This is the cheapest
/// honest answer to "is this still the same plan": its body, plus the declared
/// fields the ask depends on (`FINGERPRINTED`), hashed to one stable number.
///
/// It is a proxy and not a guarantee — an ask pointing at something outside the
/// plan ("check the CI failure on PR #1228") goes stale while this stays
/// green — so it bounds the queue rather than authorising it.
pub fn fingerprint(path: &Path) -> anyhow::Result<u64> {
    let text = std::fs::read_to_string(path)?;
    let fm = crate::frontmatter::extract(&text);
    // The body is everything after the closing fence; a file with no
    // frontmatter at all is all body.
    let body: String = match &fm {
        Some(fm) => text
            .lines()
            .skip(fm.close_line as usize)
            .collect::<Vec<_>>()
            .join("\n"),
        None => text.clone(),
    };
    let mut canonical = String::new();
    if let Some(fm) = &fm {
        for key in FINGERPRINTED {
            // Rendered the same whichever way it was read, because the rest of
            // the kernel already treats `tags: dolos` and `tags: [dolos]` as
            // one thing (`get_list` normalises a scalar into a one-element
            // list) — so an author reflowing that spelling has not changed the
            // plan an ask was written about. `get_str` is the fallback for
            // frontmatter too broken for `get_list`'s block scan.
            let value = fm
                .get_list(key)
                .filter(|items| !items.is_empty())
                .or_else(|| fm.get_str(key).map(|v| vec![v]))
                .map(|items| {
                    items
                        .iter()
                        .map(|i| i.trim())
                        .collect::<Vec<_>>()
                        .join(",")
                });
            if let Some(value) = value {
                canonical.push_str(&format!("{key}={value}\n"));
            }
        }
    }
    canonical.push_str("---\n");
    canonical.push_str(body.trim_end());
    Ok(fnv1a(canonical.as_bytes()))
}

/// What a plan was left in when the session dispatched to advance it ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Relinquished {
    /// `active` with nobody working it and nothing declared to await: flipped
    /// back to `ready`, so the scan picks it up again next tick.
    Released,
    /// `active` and parked on a declared handoff — the PR is the taker's
    /// event and the verdict is the owner's (`awaits:`, same argument), so
    /// the plan stays out of the queue until a human moves it.
    Parked(String),
    /// The session left a verdict of its own, or never claimed the plan.
    /// Nothing owed either way.
    Untouched(PlanStatus),
}

/// Return an abandoned plan to the queue. A session that ends without
/// flipping the status leaves `active` — claimed, and nobody holding it —
/// which no scan reads and no tick retries: the plan strands until a human
/// notices. `ready` is where it belongs, because `ready` is exactly the claim
/// this makes: released, and nobody has taken it.
///
/// The rule lives in the kernel, not the daemon: it is a statement about the
/// plan lifecycle, and the daemon decides nothing.
pub fn relinquish(path: &Path) -> anyhow::Result<Relinquished> {
    let current = current_status(path)?;
    if current != PlanStatus::Active {
        return Ok(Relinquished::Untouched(current));
    }
    if let Some(handoff) = handoff(path)? {
        return Ok(Relinquished::Parked(handoff));
    }
    fmedit::set_scalar(path, "status", PlanStatus::Ready.as_str(), false)?;
    Ok(Relinquished::Released)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(dir: &Path, frontmatter: &str) -> std::path::PathBuf {
        let path = dir.join("plan.md");
        std::fs::write(
            &path,
            format!("---\nprovenance: authored\n{frontmatter}---\n# Plan\n"),
        )
        .unwrap();
        path
    }

    /// The test the naive design fails. The runtime moves `status:` and
    /// `handoff:` on its own — claiming, parking, relinquishing — so a
    /// fingerprint that noticed would invalidate every queued ask the first
    /// time anything dispatched, which is the opposite of the point.
    #[test]
    fn the_runtimes_own_writes_do_not_change_a_plans_fingerprint() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = plan(dir.path(), "status: ready\nowner: org/coder\n");
        let before = fingerprint(&p).unwrap();

        flip(&p, &[PlanStatus::Ready], PlanStatus::Active).unwrap();
        assert_eq!(fingerprint(&p).unwrap(), before, "a claim is not a rewrite");

        set_handoff(&p, Some("https://forge/pr/7")).unwrap();
        assert_eq!(fingerprint(&p).unwrap(), before, "nor is a park");

        relinquish(&p).unwrap();
        assert_eq!(fingerprint(&p).unwrap(), before, "nor is being handed back");
    }

    /// What it *must* notice: the substance the ask was written against, and
    /// the fields that decide who would act on it.
    #[test]
    fn a_rewrite_changes_it_and_so_does_a_new_owner() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = plan(dir.path(), "status: ready\nowner: org/coder\n");
        let before = fingerprint(&p).unwrap();

        let text = std::fs::read_to_string(&p).unwrap();
        std::fs::write(&p, format!("{text}\nA paragraph the ask was not about.\n")).unwrap();
        let after_body = fingerprint(&p).unwrap();
        assert_ne!(after_body, before, "the body is the substance");

        crate::fmedit::set_scalar(&p, "owner", "org/founder", false).unwrap();
        assert_ne!(
            fingerprint(&p).unwrap(),
            after_body,
            "a different owner is a different act"
        );
    }

    /// It is written to disk and read back under a later binary, so the value
    /// is pinned rather than merely deterministic-for-now.
    #[test]
    fn the_hash_is_pinned_not_merely_deterministic() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(b"foobar"), 0x85944171f73967e8);
    }

    /// Reflowing a field's spelling is not a rewrite. The kernel already reads
    /// `tags: dolos` and `tags: [dolos]` as one thing, so an ask written
    /// against either is still an ask about the same plan.
    #[test]
    fn a_scalar_and_its_one_element_list_are_the_same_plan() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        std::fs::write(&a, "---\ntags: [dolos]\n---\n# Plan\n").unwrap();
        std::fs::write(&b, "---\ntags: dolos\n---\n# Plan\n").unwrap();
        assert_eq!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
    }

    /// But a field the ask does depend on, changing value, is.
    #[test]
    fn a_different_value_in_a_fingerprinted_field_is_a_different_plan() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.md");
        let b = dir.path().join("b.md");
        std::fs::write(&a, "---\ntags: [dolos]\n---\n# Plan\n").unwrap();
        std::fs::write(&b, "---\ntags: [dolos, governance]\n---\n# Plan\n").unwrap();
        assert_ne!(fingerprint(&a).unwrap(), fingerprint(&b).unwrap());
    }

    #[test]
    fn an_abandoned_active_plan_goes_back_to_the_queue() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(dir.path(), "status: active\n");
        assert_eq!(relinquish(&path).unwrap(), Relinquished::Released);
        assert_eq!(current_status(&path).unwrap(), PlanStatus::Ready);
        // Idempotent: a second reap of the same plan finds nothing to do.
        assert_eq!(
            relinquish(&path).unwrap(),
            Relinquished::Untouched(PlanStatus::Ready)
        );
    }

    #[test]
    fn a_declared_handoff_parks_the_plan_instead() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(dir.path(), "status: active\nhandoff: https://forge/pr/7\n");
        assert_eq!(
            relinquish(&path).unwrap(),
            Relinquished::Parked("https://forge/pr/7".into())
        );
        assert_eq!(current_status(&path).unwrap(), PlanStatus::Active);
    }

    #[test]
    fn a_verdict_of_the_sessions_own_is_left_alone() {
        let dir = tempfile::TempDir::new().unwrap();
        for status in ["blocked", "retired", "draft", "ready"] {
            let path = plan(dir.path(), &format!("status: {status}\n"));
            assert_eq!(
                relinquish(&path).unwrap(),
                Relinquished::Untouched(PlanStatus::parse(status).unwrap())
            );
        }
    }

    #[test]
    fn claiming_clears_the_last_takers_handoff() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(
            dir.path(),
            "status: ready\nhandoff: https://forge/pr/7\ntype: initiative\n",
        );
        flip(&path, &[PlanStatus::Ready], PlanStatus::Active).unwrap();
        assert_eq!(fmedit::get(&path, HANDOFF).unwrap(), None);
        assert_eq!(
            fmedit::get(&path, "type").unwrap().as_deref(),
            Some("initiative")
        );
        assert_eq!(relinquish(&path).unwrap(), Relinquished::Released);
    }

    #[test]
    fn a_pass_moves_the_owner_and_readies_an_active_plan() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(
            dir.path(),
            "status: active\nowner: org/coder\nhandoff: https://forge/pr/7\ntype: initiative\n",
        );
        assert_eq!(pass(&path, "org/qa").unwrap(), PlanStatus::Active);
        assert_eq!(current_status(&path).unwrap(), PlanStatus::Ready);
        assert_eq!(
            fmedit::get(&path, "owner").unwrap().as_deref(),
            Some("org/qa")
        );
        // The previous taker's parked proposal is spent…
        assert_eq!(fmedit::get(&path, HANDOFF).unwrap(), None);
        // …and unrelated frontmatter is untouched.
        assert_eq!(
            fmedit::get(&path, "type").unwrap().as_deref(),
            Some("initiative")
        );
    }

    #[test]
    fn a_pass_on_a_ready_plan_changes_only_the_owner() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(dir.path(), "status: ready\nowner: org/coder\n");
        assert_eq!(pass(&path, "org/qa").unwrap(), PlanStatus::Ready);
        assert_eq!(current_status(&path).unwrap(), PlanStatus::Ready);
        assert_eq!(
            fmedit::get(&path, "owner").unwrap().as_deref(),
            Some("org/qa")
        );
    }

    #[test]
    fn a_pass_refuses_terminal_and_unowned_plans() {
        let dir = tempfile::TempDir::new().unwrap();
        for status in ["draft", "blocked", "retired"] {
            let path = plan(dir.path(), &format!("status: {status}\nowner: org/coder\n"));
            let err = pass(&path, "org/qa").unwrap_err();
            assert!(err.to_string().contains("active | ready"), "{err}");
        }
        let path = plan(dir.path(), "status: active\n");
        let err = pass(&path, "org/qa").unwrap_err();
        assert!(err.to_string().contains("no owner"), "{err}");
    }

    #[test]
    fn a_handoff_is_recorded_only_while_the_plan_is_held() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = plan(dir.path(), "status: ready\n");
        let err = set_handoff(&path, Some("https://forge/pr/7")).unwrap_err();
        assert!(err.to_string().contains("is ready"), "{err}");

        flip(&path, &[PlanStatus::Ready], PlanStatus::Active).unwrap();
        set_handoff(&path, Some("https://forge/pr/7")).unwrap();
        assert_eq!(
            fmedit::get(&path, HANDOFF).unwrap().as_deref(),
            Some("https://forge/pr/7")
        );
        set_handoff(&path, None).unwrap();
        assert_eq!(fmedit::get(&path, HANDOFF).unwrap(), None);
    }
}
