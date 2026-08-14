---
provenance: authored
status: accepted
date: 2026-08-14
---
# 0053 — One dispatcher per root, one claim per plan

## Context

Two `trellis dispatch run` daemons were live on the same domain root for
seven hours. Both judged the same plan ready and both spawned a coder session
for it. The two sessions ran twenty-eight minutes concurrently, editing the
same files in the same non-isolated checkout with everything uncommitted.
Recovery was manual: one session stood down by hand, its findings — including
a real bug it had already fixed — relayed to the other, which had no way to
know the edits under it were not its own.

The `dispatch.pid` lock decision 0046 introduced did not stop the second
daemon from starting, and would not have stopped the duplicate dispatch if it
had. Both halves of that are worth stating, because they are different
failures.

**The lock was blind.** `Pidfile::claim` read the file, asked `kill -0`
whether the recorded pid was alive, and wrote its own pid if not. Three
faults. It is not atomic, so two daemons starting together both pass the
check. `Drop` removed the file unconditionally, so any exiting dispatcher
deleted whoever's lock was recorded — including a live one's. And `--once`
skipped the claim entirely, on the reading that a single pass is harmless.
But the fault that actually let this happen is more basic: the older daemon
was running a binary installed *before* the pidfile existed, so it never
recorded anything, and a scheme that asks "is the recorded pid alive?" cannot
see a daemon that recorded nothing. Upgrade the binary, restart without
stopping the old process, and the lock is looking at an empty room.

**The lock was also the wrong invariant.** Even with two daemons refused, the
fires 60 seconds apart from the *same* daemon would have stood: its first
spawn failed, leaving no in-flight entry and no attempt stamp, so the next
tick tried again — correctly. What made that a duplicate rather than a retry
was that the plan still read `ready`. Since decision 0029 the idempotence
guard has been the taker's flip `ready → active`, and it holds only from the
moment the session gets round to claiming. Between spawn and claim the
session must start, read its mandate, read the plan, and run the command —
minutes for a coder session — and for all of that window the plan sits in the
queue with a session already on it. Every observed duplicate landed inside
that window.

Decision 0052 had just closed the mirror-image hole, where a plan stays
`active` with nobody on it. This one is the same invariant read the other
way: `ready` must mean nobody has been sent.

## Decision

**The claim on a root is a `flock`, not a pid.** `Pidfile::claim` opens the
file and takes `LOCK_EX | LOCK_NB`, holding the descriptor for the process's
lifetime. Three claims:

- **The kernel is the registrar.** A `flock` is released when its holder
  dies, however it dies, so there is no staleness to adjudicate, no
  liveness question to get wrong, and nothing for an operator to remove by
  hand. The pid still goes in the file — the board's liveness overlay and the
  refusal message read it — but it is what the lock says about itself, never
  the lock.
- **`Drop` removes only its own file**, checked against the recorded pid.
  Under `flock` nobody else can have written it, so the check is redundant
  there; it is the whole of the fix on a platform without `flock`, and it is
  the specific defect that let an exiting dispatcher unlock a live one.
- **The lock is rechecked against the path.** Acquiring it on an inode that
  is no longer the file at that path means an exiting holder was raced, and
  the claim retries against the file that replaced it. Without that, unlink-
  on-exit hands the lock to two processes.

**`--once` takes the same lock.** A pass is a pass: it scans the same plans,
spawns into the same slots, and writes the same state file. One-shot beside a
daemon is two dispatchers on one root under another name.

**The runtime claims the plan for the session it is about to start.** The
flip `ready → active` moves from the taker's first turn to the moment before
the spawn, and comes back off if the spawn fails. Three claims:

- **The plan is the guard, because the plan is the queue.** A guard living in
  a daemon's memory would have to be shared with every other daemon to mean
  anything; this one is on disk, in the one field every reader already
  consults — the next pass, another process's pass, `--once`, the board. It
  needs no new state file, no new reconcile path, and nothing to go stale.
- **Only `act`, and only from `ready`** — the same pair `relinquish` tests on
  the other end, because they are two halves of one rule. An operator errand
  aimed at a plan in some other state is an attended act and takes it as it
  finds it; a `refine` never claims at all.
- **The verdict is still the session's.** The runtime takes the plan and
  hands it over; what the session owes at the end — retire, block, or a
  declared `handoff:` — is unchanged, and 0052's relinquish still returns an
  abandoned plan to `ready`. Moving *when* the claim happens does not move
  who decides anything, so the module boundary holds.

Deliberately **not** decided here: worktree isolation for dispatched sessions.
It would have made this incident cost a wasted session instead of a corrupted
tree, and it remains worth doing — but as its own decision, on its own
argument, not as a consequence of a locking bug.

## Consequences

- The default `act` prompt no longer tells the session to claim; it says the
  plan is already claimed and not to claim it again. An instance running a
  rewritten `act` template that still calls `trellis plan claim` gets a
  refusal from the guard (`is already active`) rather than a double claim —
  loud, and in the session's own log.
- A dispatched plan is `active` on disk from the spawn, so the board shows it
  in flight a minute or two earlier than before, and a session that dies at
  startup leaves an `active` plan that relinquish returns to `ready` at
  retire. The retry cooldown bounds how fast it comes round again.
- Two dispatchers can no longer both write `dispatch-state.json`, which
  closes the last-writer-wins clobber observed alongside the duplicate
  dispatch: `seen_escalations` silently lost an entry when one daemon
  overwrote the other from a stale in-memory view. The file's own premise —
  one writer per document — is now enforced rather than assumed.
- The upgrade path that caused this stays possible in one direction: a daemon
  started from a pre-0053 binary takes no `flock`, so a new daemon will not
  see it. Stopping the old dispatcher before upgrading is the operator's step,
  and it is the last version where that is true.
- `trellis rituals` still takes no lock. It is a one-shot with its own state
  file and its own keyspace, but two overlapping cron invocations would
  double-spawn ritual sessions the same way — the identical hole, unclosed,
  and named here so it is not rediscovered as a surprise.
