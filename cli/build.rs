// Extracts the spec version from ../spec/model.md's title line
// ("# Trellis — specification (vN)") and embeds it as TRELLIS_SPEC_VERSION.
// The build FAILS if the title regex misses: a binary that cannot name its
// spec version would silently break lint item 18.

use std::fs;
use std::path::Path;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let spec = Path::new(&manifest).join("../spec/model.md");
    println!("cargo:rerun-if-changed={}", spec.display());
    println!(
        "cargo:rerun-if-changed={}",
        Path::new(&manifest).join("../template").display()
    );

    let text =
        fs::read_to_string(&spec).unwrap_or_else(|e| panic!("cannot read {}: {e}", spec.display()));
    let title = text.lines().next().unwrap_or("");
    let version = title
        .split("(v")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .and_then(|digits| digits.parse::<u32>().ok())
        .unwrap_or_else(|| panic!("spec/model.md title does not carry a (vN) version: {title:?}"));
    println!("cargo:rustc-env=TRELLIS_SPEC_VERSION={version}");
}
