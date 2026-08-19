---
provenance: authored
status: accepted
date: 2026-08-19
---
# 0064 — A wait is declared, and it expires

## Context

Reported from a live root: dispatch started a second agent for a plan that was
already `active`, while the first agent sat idle waiting on a background
process it had started.

The mechanism is 0061's, working as specified. Herdr reports screen state, and
an agent waiting at its composer for a build it launched reads `idle` —
byte-identical to one that is finished, crashed, or was never handed a prompt.
`disposition` reads the plan as `active` with no `handoff:`, which is `Held`;
`observe` grants `idle_grace_secs` and returns `Recycled`; `relinquish` hands
the plan back to `ready`; and one cooldown later it is dispatched again.

Two things made it worse than a duplicate dispatch.

**The pane was left running.** `Retain::OnFailure` keeps the workspace of an
"unaccounted end" for an operator to attach to, and a recycle is one. So the
runtime handed the plan to a new session while the old one was still sitting in
its pane, still holding a claim it had been relieved of. Two agents, one
artifact — the strand 0052's "`active` means somebody is on it" exists to
prevent, arriving by another road. `spec/runtime.md` had described the recycle
as closing the workspace since 0061; the code never did.

**The declared escape hatch was a trap.** 0061 replaced 0052's footer scrape
with "parking is declared, never inferred", and the act prompt tells a session
to declare background work with `trellis plan handoff`. For work the session is
*itself* minding, that instruction is actively harmful: a handoff reads as
`Verdict`, and a verdict is not a failure, so the workspace is closed — killing
the pane the build is running in. The plan is then parked `active` and out of
the queue "until its owner moves it", and the thing that was going to move it
has just been killed.

So a session waiting on its own background work had no correct move. Declare
and it is killed; stay quiet and it is duplicated. 0061 accepted that
undeclared background work is lost; it did not intend that declaring it be
worse.

## Decision

**A wait is a third declaration, beside the verdict and the handoff.** A
session that will come back to work outliving its turn writes
`trellis plan waiting <plan> --for 30m --on "cargo build"`. While that stands,
a settled pane over the plan is `Disposition::Waiting` rather than `Held`: it
is not recycled, it keeps its plan, its pane, and its slot.

The three are distinguished by who comes back, and the act prompt now says so
in those terms: a **verdict** ends the work, a **handoff** parks the plan on
something *somebody else* moves and ends the session, a **wait** is work *this
session* returns to and keeps it alive.

**It expires, and the operator caps it.** This is the whole difference from
0052's `Waiting` phase, which 0061 removed for holding a slot forever "with no
clock on it at all". A wait is a lease: the session names a duration,
`[harness.herdr] max_wait_secs` (default two hours) bounds it, and the cap is
measured from when the lease was *written*, so tightening it also binds leases
already standing. A session that needs longer says so again — which is cheap,
and is the session proving it is still there. When the lease lapses the
disposition falls back to `Held` with the grace it was already spending
unreset, so the recycle follows on the next sighting. The lease is the extra
patience, not a second grace.

**It is a liveness claim, not a completion claim, and it lives in the
runtime's own state.** `.trellis/runtime/waiting/<slug>`, one file per plan,
written whole and renamed, failing open — absent, lapsed, and unreadable are
one answer, and the fallback is the plan's own status, which is never worse
than half a lease. Not frontmatter: an expiry is a fact about the session, and
a wall-clock timestamp churning through a git-tracked artifact is commit noise.
The precedent is `.trellis/acting-role` (0045), which is the same kind of fact
at the same scope. Completion is still only ever written to the plan, so 0061's
rule is intact.

**A recycle closes its pane, whatever `retain` says.** Handing the plan back
and leaving the session that held it running are not two decisions. The
scrollback is copied into the session log before the close, as it already was,
so this costs live attach and not evidence. `Retain` keeps its meaning for
everything else, which after this is a session herdr lost — whose workspace is
already gone.

## Consequences

- **A waiting session holds a `max_concurrent` slot.** That is 0052's cost
  returning, and it is the price of this decision. What is different is that it
  is bounded (`max_wait_secs`) and visible: the fleet label reads
  `[waiting: cargo build]`, `status.json` carries the reason, and the board
  shows it. A held slot the operator cannot account for was 0052's own
  complaint against itself. A domain that runs long builds may need
  `max_concurrent` raised.
- **A session that says nothing is still recycled**, deliberately. Nothing here
  infers anything; what changes is that a correct move now exists. A session
  that declares a wait it is not honouring buys itself at most `max_wait_secs`.
- **A one-shot pass detaches a waiting session** rather than sitting out its
  lease, as it already does for a blocked one: a drain has nobody to finish the
  build for it, and the next daemon adopts the pane.
- **The lease is cleared at the claim and at the conclusion**, mirroring the
  spent `handoff:` that `flip` already drops. Expiry would get there alone;
  these are so a next taker never inherits one.
- `slug` and `now_secs` move into the kernel (`waits`), where the CLI verb and
  the daemon can both spell them once. The daemon may depend on the kernel;
  never the reverse.

## Alternatives rejected

**Inferring the wait.** Ask the harness what background work is armed — 0052's
footer scrape. Rejected for 0061's reasons, unchanged: it is Claude-Code-
specific screen parsing, silently wrong for any other `kind`, and it protects
only sessions that decline to say what they are doing.

**A frontmatter `waiting_until:`.** Truest to "the answer lives in the plan",
and visible in git. Rejected: it puts a flapping wall-clock timestamp into an
authored artifact, and a lease is not a fact about the plan. `handoff:` already
occupies the plan-level slot for parked work, and the confusion between the two
is what this decision exists to remove.

**Leaving the recycled pane up for forensics.** It is genuinely useful and it
is what produced two writers on one plan. The scrollback tail already lands in
the session log, so the evidence survives the close.

**Raising `idle_grace_secs` and calling it done.** The knob exists and it is
the right immediate mitigation, but it is blunt: it delays every recycle by the
same amount, including the dropped-prompt case 0063 measured, and it still
cannot tell a session minding an hour-long download from a dead one.
