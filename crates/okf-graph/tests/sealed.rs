//! Proof that `#[non_exhaustive]` does what the CHANGELOG's compatibility note
//! promises: a *downstream* crate cannot match `Rule` exhaustively, and cannot
//! build a frontmatter family with a struct literal. Both are legal inside this
//! crate, so no ordinary test can see the difference — the check has to compile
//! code from outside.
//!
//! `rustc` is driven directly against the already-built rlib rather than through
//! `trybuild`, which shells out to `cargo` and would need a registry inside the
//! sandboxed nix build. The positive control is the load-bearing case: without
//! it, a typo in the command line would make every negative case "pass" by
//! failing for the wrong reason.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Every `Rule` variant. A new rule has to be added here, and that is the point:
/// the day this list is complete and the match still fails, the seal is what is
/// holding.
const RULES: [&str; 24] = [
    "NotAConcept",
    "MissingType",
    "DuplicateId",
    "DanglingLink",
    "InvalidStatus",
    "MissingGeneratedBy",
    "MalformedActor",
    "MissingSourceResource",
    "MissingRuntime",
    "InvalidComputationSource",
    "DanglingPath",
    "DerivationCycle",
    "IndexFrontmatter",
    "DanglingIndexEntry",
    "UnknownOkfVersion",
    "NonIsoLogDate",
    "LogOutOfOrder",
    "DanglingLogEntry",
    "MalformedParameter",
    "IncompleteAttestation",
    "EmptyComputation",
    "MalformedTimestamp",
    "MalformedStaleAfter",
    "MalformedSourceSignal",
];

/// Compile `source` as a downstream crate against this crate's rlib.
fn compile(name: &str, source: &str) -> Output {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let file = tmp.join(format!("{name}.rs"));
    std::fs::write(&file, source).expect("writes the case");

    let deps = deps_dir();
    Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
        ])
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("okf_graph={}", rlib(&deps).display()))
        .arg("-o")
        .arg(tmp.join(format!("{name}.rmeta")))
        .arg(&file)
        .output()
        .expect("rustc runs")
}

/// The directory cargo put this test binary in, which is also where it put the
/// crate's rlib.
fn deps_dir() -> PathBuf {
    std::env::current_exe()
        .expect("the test binary has a path")
        .parent()
        .expect("it lives in a directory")
        .to_path_buf()
}

/// The newest `libokf_graph-*.rlib` in `deps`. Several can accumulate across
/// rebuilds; any of them is this crate, and the newest is this build.
fn rlib(deps: &Path) -> PathBuf {
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(deps)
        .expect("the deps directory is readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("libokf_graph-") && name.ends_with(".rlib")
        })
        .filter_map(|path| Some((path.metadata().ok()?.modified().ok()?, path)))
        .collect();
    found.sort();
    found.pop().expect("okf-graph was built before its tests").1
}

/// The control: the same harness, on code that must compile. If this fails, the
/// two cases below prove nothing.
#[test]
fn a_wildcard_match_and_field_reads_compile() {
    let output = compile(
        "control",
        r#"
        pub fn name(rule: okf_graph::Rule) -> &'static str {
            match rule {
                okf_graph::Rule::MissingType => "missing type",
                _ => rule.title(),
            }
        }
        pub fn who(generated: &okf_graph::Generated) -> Option<&str> {
            generated.by.as_deref()
        }
        "#,
    );

    assert!(
        output.status.success(),
        "the control must compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// E0004: adding a rule must not break a downstream `match`.
///
/// Every variant is listed, so the error can only come from the hidden one
/// `#[non_exhaustive]` adds. An abbreviated list would fail for the ordinary
/// reason and prove nothing — which is what the first draft of this test did,
/// caught by deleting the attribute and watching this case stay green.
#[test]
fn matching_every_rule_variant_is_rejected_downstream() {
    let arms: String = RULES
        .iter()
        .map(|rule| format!("okf_graph::Rule::{rule} => \"{rule}\",\n"))
        .collect();
    let output = compile(
        "exhaustive_rule",
        &format!(
            "pub fn code(rule: okf_graph::Rule) -> &'static str {{ match rule {{ {arms} }} }}"
        ),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "the match should not compile");
    assert!(
        stderr.contains("E0004") && stderr.contains("non-exhaustive"),
        "expected a non-exhaustive-patterns error, got:\n{stderr}"
    );
}

/// E0639: adding a frontmatter field must not break a downstream build.
#[test]
fn building_a_family_with_a_struct_literal_is_rejected_downstream() {
    let output = compile(
        "struct_literal",
        r#"
        pub fn make() -> okf_graph::Generated {
            okf_graph::Generated { by: None, at: None }
        }
        "#,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "the literal should not compile");
    assert!(
        stderr.contains("E0639"),
        "expected a non-exhaustive-struct error, got:\n{stderr}"
    );
}

/// The exception, held to its word: `Severity` is exhaustive, so both arms are
/// the whole domain and a downstream match needs no wildcard.
#[test]
fn matching_both_severities_still_compiles_downstream() {
    let output = compile(
        "exhaustive_severity",
        r#"
        pub fn fails(severity: okf_graph::Severity) -> bool {
            match severity {
                okf_graph::Severity::Defect => true,
                okf_graph::Severity::Report => false,
            }
        }
        "#,
    );

    assert!(
        output.status.success(),
        "Severity is deliberately exhaustive:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
