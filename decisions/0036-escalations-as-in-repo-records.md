---
provenance: authored
status: accepted
date: 2026-08-03
---
# 0036 — Escalations as in-repo records, owner-written

## Context
The reference binding sent every escalation to a forge issue (0019), and four
plugin members hardcoded `gh issue create` as the default: `agents/coder.md` (the
readiness gate and the blocker), `commands/act.md` (beyond-authority),
`commands/focus.md` (a finding for an absent owner), `commands/ritual.md`
(deviations). The layering was already correct — `spec/runtime.md`'s `escalation`
contract service says only "deliver to the `escalate-to:` role's holder with an
audit trail," and the instance declares its channel in `conventions.md` — but the
*default* was a forge issue, and members named the tool inline rather than
deferring to the declaration.

The dogfooding evidence is the coder: dispatched on an underspecified plan, it
flips the plan `blocked` and files the question as an issue. The plan then asserts
a defect while naming none, and the question that would clear it lives in a system
no later session reads. That is precisely the failure the Loop observability
pattern names — a loop whose termination condition is filed outside the tree. It
also splits one event across two systems: the status flip is an artifact edit, the
reason for it is a forge thread, and nothing links them structurally.

Three further costs. The channel is unlintable: no check can ask "does this
blocked plan state its blocker," because the answer is in an API. It is
unportable, contradicting 0019's own premise that a binding is swappable — an
escalation should survive changing forges. And it is invisible to a checkout,
which is the substrate premise (0001).

## Decision
**An escalation is body content in the artifact it concerns** — a fenced `yaml`
block under an `## Escalations` heading, carrying `raised`, `by`, `to`, `status:
open|resolved`, and the question asked of a human. Schema in
`template/conventions.md`; the reference binding's `escalation` row now names it.
The artifact that carries the problem carries the escalation, so a blocked plan
states its own blocker.

**Only the artifact's `owner:` writes a record.** An escalation is authored
content, so it obeys the ownership rule like any other content: an agent acting
*as* the owner writes its own (the coder under dispatch owns the plan it was
dispatched on), and a role whose mandate does not reach the artifact reports the
finding for the owner to transcribe. This is what keeps `org/focus`
("writes nothing into the root") and `org/steward` ("never edit `authored`
artifacts") exactly as strict as they were — both escalate by reporting, which is
what they already did without forge access. It also resolves a standing
contradiction: the metric sweep told the steward to "annotate plans," which its
own boundary forbade; it now reports and the owner annotates.

**No new artifact kind.** Escalations are not a directory (rule 7: a new
directory is a new kind with its own lifecycle and owner — an escalation has
neither; it inherits both from its host). They are not frontmatter either, since
an artifact carries zero or many.

**Lint item 24** makes the channel checkable, which the forge never was: records
are well-formed, every `status: blocked` plan carries at least one `open` record,
and no record sits in a `generated` artifact. A held plan (`awaits:`) is
explicitly not covered — it carries no defect and owes no record.

The forge keeps its other two jobs: **ingress** (`role:{name}` labels) and
**approvals** (PRs). Only the escalation channel moves.

Non-normative throughout: spec stays **v16**, no frontmatter schema change, no new
status, no new kind, no new plane or contract service. The contract service's
obligation is unchanged; this is a different way to satisfy it.

## Consequences
The escalation trail is now git history — attributable, diffable, reviewable in
the PR that raises it, and portable across bindings. A blocked plan is
self-describing, so the Execution health reading "blocked-heavy means the
escalation channel is not clearing" becomes computable from the tree instead of
from an issue tracker, and plan-effectiveness item 9 (blocked past a ritual
cadence) gains the blocker's actual text at no cost. Dedup improves: focus reads
the artifact's own records rather than searching issues for a matching finding.

**The notification half of the contract service is not satisfied.** A committed
record pings nobody, where an assigned issue did. Recipients read the root, a
generated view over open records, or the raising session's report. This is
recorded as a known limit of the binding; a push channel layered over the records
is the pull-triggered fix, and the record stays the system of record either way.

**An advisory role's finding is durable only once its owner transcribes it.**
Under the interactive plane the owner is usually present and this is a keystroke.
Under the scheduled plane it is not: a weekly focus or lint run whose findings
nobody transcribes leaves them in a workflow log, which is strictly worse than an
auto-filed issue. Accepted deliberately as the price of the ownership boundary —
the alternative was letting write-nothing roles write. If the loss shows up in
practice, the fix is a per-role escalation inbox the role owns (and may therefore
write), not a carve-out in the ownership rule.

Existing instances holding escalations as forge issues are not migrated: the old
issues stay valid history, and new escalations land as records. Eval fixtures are
untouched — no fixture carries an escalation, and lint item 24 fires only on
`blocked` plans, of which the graded fixtures have none with a missing record.

## Alternatives rejected
**A root-level `escalations/` directory** — the flat-kind shape `plans/` uses.
Rejected on rule 7: an escalation has no lifecycle or owner of its own, and
splitting it from its artifact recreates the two-systems problem in one repo,
where the plan again asserts `blocked` while the reason lives elsewhere.
**`org/{role}/escalations/`** — a per-role inbox mirroring "mandates are always
local." Better notification story, and it would let advisory roles write their own
findings, but it scatters the record away from the artifact it concerns and needs
a generated roll-up to answer "what is blocked." Kept as the named fix if the
transcription gap bites.
**A carve-out letting any mandated role append to `## Escalations` regardless of
`scope:`** — solves the advisory-role gap directly, and it is tempting because an
escalation is addressed *to* the owner rather than authored *by* them. Rejected:
it costs focus the write-nothing property 0025 deliberately gave it, and it
introduces a second class of content with different write rules, which every
future member would then have to reason about.
**Keeping the forge with records as a mirror** — two systems of record, the
original problem with an extra sync step.
**Leaving it to each instance with no default** — the members already needed *a*
default to name, and naming the forge is what produced this.
