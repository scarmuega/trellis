//! `trellis.toml` failure semantics (decision 0054): the file is the root
//! marker and the machine-read config, so absence means "not a root" and a
//! file that does not parse refuses the whole load — never a lint pass over
//! empty registries, which would report violations that are not there with
//! deterministic authority (decision 0037).

mod common;
use common::Fixture;

#[test]
fn a_corrupt_config_refuses_every_tree_command_by_name() {
    let f = Fixture::healthy();
    f.write("trellis.toml", "spec = [broken\n");
    for cmd in ["lint", "tree"] {
        let out = f.bin().arg(cmd).output().unwrap();
        assert!(!out.status.success(), "{cmd} should refuse");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("trellis.toml"), "{cmd} names the file: {err}");
    }
}

#[test]
fn a_missing_spec_pin_refuses_by_field_name() {
    let f = Fixture::healthy();
    f.write("trellis.toml", "carried = []\n");
    let out = f.bin().arg("lint").output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("spec"), "{err}");
}

#[test]
fn a_misspelled_key_refuses_rather_than_being_ignored() {
    let f = Fixture::healthy();
    let config = f.read("trellis.toml");
    f.write("trellis.toml", &format!("{config}\nspeling = 1\n"));
    let out = f.bin().arg("lint").output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("speling"), "{err}");
}

#[test]
fn a_stale_pin_is_item_18s_finding_on_the_config_file() {
    let f = Fixture::healthy();
    let config = f.read("trellis.toml");
    let stale = trellis::spec_version() - 1;
    f.write(
        "trellis.toml",
        &config.replace(
            &format!("spec = {}", trellis::spec_version()),
            &format!("spec = {stale}"),
        ),
    );
    let report = f.lint_json(&["--items", "18"]);
    let findings = report["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{report:#}");
    assert_eq!(findings[0]["path"], "trellis.toml", "{report:#}");
    let msg = findings[0]["message"].as_str().unwrap();
    assert!(msg.contains(&format!("v{stale}")), "{msg}");
}

#[test]
fn a_current_pin_satisfies_item_18() {
    let f = Fixture::healthy();
    let report = f.lint_json(&["--items", "18"]);
    assert_eq!(report["summary"]["violations"], 0, "{report:#}");
}

#[test]
fn an_absent_config_means_no_root_at_all() {
    let f = Fixture::healthy();
    std::fs::remove_file(f.root().join("trellis.toml")).unwrap();
    let out = f.bin().arg("root").output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("markers: trellis.toml"),
        "the discovery error names the marker set: {err}"
    );
}
