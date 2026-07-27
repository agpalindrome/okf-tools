//! End-to-end tests for the `okf-graph` binary (issue #48): run the real
//! executable against fixture bundles and assert its exit codes and output.
//! `CARGO_BIN_EXE_okf-graph` is the path Cargo hands an integration test.

use std::path::PathBuf;
use std::process::Command;

fn okf_graph() -> Command {
    Command::new(env!("CARGO_BIN_EXE_okf-graph"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A clean bundle: no defects, exit 0.
#[test]
fn a_clean_bundle_exits_zero() {
    let output = okf_graph().arg(fixture("clean")).output().expect("runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A bundle with defects: exit 1, and the aggregated findings — a defect and a
/// report, from different rule families — are printed.
#[test]
fn a_bundle_with_defects_exits_one_and_prints_findings() {
    let output = okf_graph().arg(fixture("broken")).output().expect("runs");
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CONCEPT-2"),
        "expected a defect; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("BUNDLE-2"),
        "expected a report; stdout:\n{stdout}"
    );
}

/// A path that cannot be read is a usage/IO error: exit 2.
#[test]
fn a_missing_path_exits_two() {
    let output = okf_graph()
        .arg(fixture("does-not-exist"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(2));
}

/// No bundle argument prints usage and exits 2.
#[test]
fn no_arguments_exits_two() {
    let output = okf_graph().output().expect("runs");
    assert_eq!(output.status.code(), Some(2));
}
