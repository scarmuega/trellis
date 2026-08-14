//! The spawn prompt's skill index (decision 0055): the runtime resolves the
//! skills a session is entitled to — the acting role's holder package plus,
//! for a plan session, each bounded context the plan names — and renders an
//! index into `{skills}`. Names and paths, never bodies; no skills, no block.

mod common;

use common::{Fixture, ANCHOR};

const FM: &str = "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

fn fixture() -> Fixture {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{prompt}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\", \"{{prompt}}\"]\n"
        ),
    );
    f
}

#[test]
fn an_act_prompt_indexes_skills_from_both_homes() {
    let f = fixture();
    // Identity-bound technique: no description frontmatter, so the H1 serves.
    f.write(
        "org/founder/holder/skills/door-outreach/README.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Door outreach\n\nHow this holder pitches a door.\n",
    );
    // Business procedure, plugin-style spelling: SKILL.md with a description.
    f.write(
        "solution/kit-kitchen/skills/kit-costing/SKILL.md",
        "---\ndescription: Cost a kit from its bill of materials\n---\n# Kit costing\n",
    );
    f.write(
        "plans/stock-doors.md",
        &format!("{FM}status: ready\ntype: initiative\ncontexts: [solution/kit-kitchen]\n---\n# Stock doors\n"),
    );

    f.dispatch_once(ANCHOR, &[]);

    let argv = &f.invocations()[0];
    // The first line still discriminates (the herdr needle, decision 0050).
    assert_eq!(
        argv[0],
        "plans/stock-doors.md — dispatched act as founder (trellis runtime)."
    );
    let prompt = argv.join("\n");
    assert!(prompt.contains("## Skills"), "{prompt}");
    assert!(
        prompt.contains(
            "- door-outreach — Door outreach → org/founder/holder/skills/door-outreach/README.md"
        ),
        "{prompt}"
    );
    assert!(
        prompt.contains(
            "- kit-costing — Cost a kit from its bill of materials → solution/kit-kitchen/skills/kit-costing/SKILL.md"
        ),
        "{prompt}"
    );
    // Holder technique lists before context procedure.
    assert!(
        prompt.find("door-outreach").unwrap() < prompt.find("kit-costing").unwrap(),
        "{prompt}"
    );
}

#[test]
fn a_ritual_prompt_carries_only_the_executors_holder_skills() {
    let f = fixture();
    f.write(
        "rituals.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Rituals\n\n\
         | ritual     | cadence | executor    | procedure              |\n\
         |------------|---------|-------------|------------------------|\n\
         | lint sweep | weekly  | org/steward | run the lint checklist |\n",
    );
    f.write(
        "org/steward/holder/skills/lint-sweep/README.md",
        "# Lint sweep\n\nThe checklist, in order.\n",
    );
    // A context skill is a decoy here: rituals carry no plan, so no
    // context home applies.
    f.write(
        "solution/kit-kitchen/skills/kit-costing/SKILL.md",
        "---\ndescription: Cost a kit from its bill of materials\n---\n# Kit costing\n",
    );

    f.rituals_once(ANCHOR, &[]);

    let argv = &f.invocations()[0];
    assert_eq!(argv[0], "lint sweep");
    let prompt = argv[1..].join("\n");
    assert!(
        prompt
            .contains("- lint-sweep — Lint sweep → org/steward/holder/skills/lint-sweep/README.md"),
        "{prompt}"
    );
    assert!(!prompt.contains("kit-costing"), "{prompt}");
}

#[test]
fn no_skills_renders_nothing_and_the_spawn_still_succeeds() {
    let f = fixture();
    f.write(
        "plans/stock-doors.md",
        &format!("{FM}status: ready\ntype: initiative\ncontexts: [solution/kit-kitchen]\n---\n# Stock doors\n"),
    );

    f.dispatch_once(ANCHOR, &[]);

    let argv = &f.invocations()[0];
    let prompt = argv.join("\n");
    assert!(!prompt.contains("## Skills"), "{prompt}");
    // The rest of the prompt rendered in full around the empty value.
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");
}
