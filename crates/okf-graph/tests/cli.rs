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

/// `--deny` on a tolerated rule is the producer's case: the `dangling` fixture
/// passes by default and fails once `BUNDLE-2` is denied.
#[test]
fn denying_a_report_turns_a_passing_run_into_a_failing_one() {
    let clean = okf_graph().arg(fixture("dangling")).output().expect("runs");
    assert_eq!(clean.status.code(), Some(0));

    let denied = okf_graph()
        .args(["--deny", "BUNDLE-2"])
        .arg(fixture("dangling"))
        .output()
        .expect("runs");
    assert_eq!(denied.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&denied.stdout).contains("BUNDLE-2"));
}

/// `--allow` silences a rule, and the summary counts what it dropped rather
/// than letting a silenced finding read as a rule that found nothing.
#[test]
fn allowing_a_rule_silences_it_and_the_summary_says_so() {
    let output = okf_graph()
        .args(["--allow", "BUNDLE-2"])
        .arg(fixture("dangling"))
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("BUNDLE-2"));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("1 silenced"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `--warn` takes a defect down to a report: still printed, no longer fatal.
#[test]
fn warning_a_defect_down_prints_it_and_exits_zero() {
    let output = okf_graph()
        .args(["--warn", "CONCEPT-2"])
        .arg(fixture("missing-type"))
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("CONCEPT-2"));
}

/// A rule code is case-insensitive, and repeating one is last-wins.
#[test]
fn a_code_reads_in_any_case_and_the_last_setting_wins() {
    let output = okf_graph()
        .args(["--deny", "bundle-2", "--allow", "BUNDLE-2"])
        .arg(fixture("dangling"))
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("BUNDLE-2"));
}

/// A code no rule has is a usage error, not a silent no-op — a mistyped code
/// that quietly does nothing looks exactly like one that worked.
#[test]
fn an_unknown_rule_code_exits_two() {
    let output = okf_graph()
        .args(["--deny", "BUNDEL-2"])
        .arg(fixture("dangling"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(2));

    let missing = okf_graph().arg("--deny").output().expect("runs");
    assert_eq!(missing.status.code(), Some(2));
}

/// `--as-of` pins the day §5.5 is read against, so a run is reproducible: the
/// same bundle is stale on one day and clean on the day before, and neither
/// answer moves with the machine's clock.
#[test]
fn as_of_pins_the_day_staleness_is_read_against() {
    let stale = okf_graph()
        .args(["--as-of", "2026-08-15"])
        .arg(fixture("stale"))
        .output()
        .expect("runs");
    assert_eq!(
        stale.status.code(),
        Some(0),
        "a stale concept still conforms"
    );
    let stdout = String::from_utf8_lossy(&stale.stdout);
    assert!(stdout.contains("CONCEPT-15"), "stdout:\n{stdout}");
    assert!(stdout.contains("expired.md"), "stdout:\n{stdout}");
    assert!(
        !stdout.contains("current.md") && !stdout.contains("undated.md"),
        "only the expired concept is stale; stdout:\n{stdout}"
    );

    let earlier = okf_graph()
        .args(["--as-of", "2025-12-31"])
        .arg(fixture("stale"))
        .output()
        .expect("runs");
    assert_eq!(earlier.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&earlier.stdout).is_empty());
}

/// The producer's case, and the reason CONCEPT-15 is a rule rather than a note:
/// `--deny` gates CI on a concept that has gone stale.
#[test]
fn denying_staleness_fails_the_run() {
    let output = okf_graph()
        .args(["--deny", "CONCEPT-15", "--as-of", "2026-08-15"])
        .arg(fixture("stale"))
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("1 defect(s)"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// An `--as-of` that is not a date is a usage error, like a mistyped rule code:
/// silently falling back to today would answer a question nobody asked.
#[test]
fn a_malformed_as_of_exits_two() {
    let bad = okf_graph()
        .args(["--as-of", "15-08-2026"])
        .arg(fixture("stale"))
        .output()
        .expect("runs");
    assert_eq!(bad.status.code(), Some(2));

    let missing = okf_graph().arg("--as-of").output().expect("runs");
    assert_eq!(missing.status.code(), Some(2));
}

/// A bundle with no concepts is a usage error, not a clean run: a mistyped path
/// or a bundle that never generated would otherwise pass CI green.
#[test]
fn a_bundle_with_no_concepts_exits_two() {
    let dir = std::env::temp_dir().join("okf-graph-cli-empty");
    std::fs::create_dir_all(&dir).expect("mkdir");

    let output = okf_graph().arg(&dir).output().expect("runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("no concepts found"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Reserved files are not concepts, so a directory holding only an `index.md`
/// is empty for this purpose — and its own findings do not rescue it.
#[test]
fn reserved_files_alone_are_still_an_empty_bundle() {
    let output = okf_graph()
        .arg(fixture("reserved-only"))
        .output()
        .expect("runs");
    assert_eq!(output.status.code(), Some(2));
}

/// `--allow-empty` is the opt-out for a caller that expects one.
#[test]
fn allow_empty_accepts_a_bundle_with_no_concepts() {
    let dir = std::env::temp_dir().join("okf-graph-cli-empty-ok");
    std::fs::create_dir_all(&dir).expect("mkdir");

    let output = okf_graph()
        .arg("--allow-empty")
        .arg(&dir)
        .output()
        .expect("runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
