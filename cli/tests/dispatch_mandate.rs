//! The spawn prompt's rendered role context (decision 0058): the runtime
//! inlines the acting role's mandate and, when the holder is a local
//! package, its holder/system.md — the reads the act procedure asks for,
//! already done at spawn. The procedure itself arrives with its
//! interactive-only spans stripped: one source, two renderings.

mod common;

use common::{FakeHerdr, Fixture, ANCHOR};

fn fixture() -> (Fixture, FakeHerdr) {
    let f = Fixture::healthy();
    let herdr = common::wire_herdr(&f, "", "", &["working", "idle"], None);
    (f, herdr)
}

#[test]
fn an_act_prompt_renders_the_owners_mandate_and_package() {
    let (f, herdr) = fixture();
    f.write(
        "org/coder/mandate.md",
        "---\nprovenance: authored\nowner: org/founder\npurpose: land the code-bearing plans\nescalate-to: org/founder\nholder: holder/\n---\n# Coder\n",
    );
    f.write(
        "org/coder/holder/system.md",
        "---\nprovenance: authored\nowner: org/coder\n---\n# Coder holder\n\nDeliver as a PR, never self-merge.\n",
    );
    f.write(
        "plans/land-it.md",
        "---\nprovenance: authored\nowner: org/coder\nstatus: ready\ntype: initiative\nsubdomains: [problem/outdoor-retail-channel.md]\n---\n# Land it\n",
    );

    f.dispatch_once(ANCHOR, &[]);

    let prompt = &herdr.prompts()[0];
    // The first line still discriminates (the herdr needle, decision 0050).
    assert_eq!(
        prompt.lines().next().unwrap(),
        "plans/land-it.md — dispatched act as coder (trellis runtime)."
    );

    assert!(
        prompt.contains("## Mandate — org/coder/mandate.md"),
        "{prompt}"
    );
    assert!(
        prompt.contains("purpose: land the code-bearing plans"),
        "the mandate arrives verbatim, frontmatter included: {prompt}"
    );
    assert!(
        prompt.contains("## Holder — org/coder/holder/system.md"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Deliver as a PR, never self-merge."),
        "{prompt}"
    );
    // Mandate before holder before the procedure.
    assert!(
        prompt.find("## Mandate").unwrap() < prompt.find("## Holder").unwrap()
            && prompt.find("## Holder").unwrap() < prompt.find("Bind to the domain root").unwrap(),
        "{prompt}"
    );
}

#[test]
fn a_ref_holder_renders_no_package_and_the_procedure_arrives_stripped() {
    let (f, herdr) = fixture();
    // The skeleton founder's holder is a ref (a human, undeclared kind) —
    // nothing to inline; the procedure's holder branch stays authoritative.
    f.write(
        "plans/stock-doors.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nsubdomains: [problem/outdoor-retail-channel.md]\n---\n# Stock doors\n",
    );

    f.dispatch_once(ANCHOR, &[]);

    let prompt = &herdr.prompts()[0];
    assert!(
        prompt.contains("## Mandate — org/founder/mandate.md"),
        "{prompt}"
    );
    assert!(!prompt.contains("## Holder"), "{prompt}");
    // One source, two renderings: the interactive-only spans never reach a
    // dispatched session.
    assert!(!prompt.contains("$ARGUMENTS"), "{prompt}");
    assert!(!prompt.contains("No input means"), "{prompt}");
    assert!(!prompt.contains("interactive-only"), "{prompt}");
    // The branches every dispatch flavor still needs survive the strip.
    assert!(prompt.contains("never impersonate"), "{prompt}");
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");
    // The verdict clause spells its exits as commands (decision 0059).
    assert!(
        prompt.contains("`trellis plan pass plans/stock-doors.md --to <role>`"),
        "{prompt}"
    );
    assert!(
        prompt.contains("`trellis plan retire plans/stock-doors.md`"),
        "{prompt}"
    );
}

#[test]
fn a_ritual_prompt_renders_the_executors_mandate() {
    let (f, herdr) = fixture();
    f.write(
        "rituals.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Rituals\n\n\
         | ritual     | cadence | executor    | procedure              |\n\
         |------------|---------|-------------|------------------------|\n\
         | lint sweep | weekly  | org/steward | run the lint checklist |\n",
    );
    f.write(
        "org/steward/mandate.md",
        "---\nprovenance: authored\nowner: org/founder\npurpose: keep the conventions honest\nescalate-to: org/founder\n---\n# Steward\n",
    );

    f.rituals_once(ANCHOR, &[]);

    let prompt = &herdr.prompts()[0];
    assert_eq!(
        prompt.lines().next().unwrap(),
        "ritual lint sweep — executed by org/steward (trellis runtime)."
    );
    assert!(
        prompt.contains("## Mandate — org/steward/mandate.md"),
        "{prompt}"
    );
    assert!(prompt.contains("keep the conventions honest"), "{prompt}");
}
