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

/// `--version` answers with no bundle to hand, and reports the crate's own
/// version rather than a string maintained beside it. The pin in a CI step is
/// an instruction; this is what lets the log assert what the instruction got.
#[test]
fn version_prints_the_crate_version_and_exits_zero() {
    for flag in ["-V", "--version"] {
        let output = okf_graph().arg(flag).output().expect("runs");
        assert_eq!(output.status.code(), Some(0), "{flag}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("okf-graph {}", env!("CARGO_PKG_VERSION")),
            "{flag}"
        );
    }
}

/// The summary names the binary that produced it. Which rules ran is a property
/// of the version — CONCEPT-15 did not exist in 0.2.0 — so a summary that does
/// not say which version it is does not say what it checked.
#[test]
fn the_summary_names_the_version_that_produced_it() {
    let output = okf_graph().arg(fixture("clean")).output().expect("runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with(&format!("okf-graph {}: ", env!("CARGO_PKG_VERSION"))),
        "stderr: {stderr}"
    );
}

/// `--quiet` suppresses the summary, so it suppresses the version with it: the
/// caller that asked for findings only did not ask for a header instead.
#[test]
fn quiet_suppresses_the_version_too() {
    let output = okf_graph()
        .arg("--quiet")
        .arg(fixture("clean"))
        .output()
        .expect("runs");
    assert!(
        String::from_utf8_lossy(&output.stderr).is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A mistyped flag names itself, in every position. Before #109 each of these
/// produced a different diagnosis — not a directory, too many paths — because
/// the token became the bundle path and failed somewhere downstream of the
/// mistake.
#[test]
fn an_unrecognised_flag_is_rejected_rather_than_read_as_a_path() {
    for args in [
        vec!["--qiuet"],
        vec!["--qiuet", "tests/fixtures/clean"],
        vec!["--alow-empty"],
    ] {
        let output = okf_graph().args(&args).output().expect("runs");
        assert_eq!(output.status.code(), Some(2), "{args:?}");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("is not an option") && stderr.contains(args[0]),
            "expected the typo to be named; {args:?} gave:\n{stderr}"
        );
        // The note is the whole mitigation for the input this change drops, so
        // it is asserted rather than left to survive on inspection.
        assert!(
            stderr.contains("./"),
            "expected the `./` escape to be offered; {args:?} gave:\n{stderr}"
        );
    }
}

/// An empty bundle path names itself. `okf-graph "$DIR"` with `DIR` unset is
/// the way this arrives, and it used to print a message with a blank where the
/// path should be — or, with a real path after it, an arity error for a caller
/// who passed one path.
#[test]
fn an_empty_bundle_path_is_rejected_and_says_why() {
    for args in [vec![""], vec!["", "tests/fixtures/clean"]] {
        let output = okf_graph().args(&args).output().expect("runs");
        assert_eq!(output.status.code(), Some(2), "{args:?}");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("bundle path is empty"),
            "{args:?} gave:\n{stderr}"
        );
        assert!(
            !stderr.contains("expected a single bundle path"),
            "an empty path is not an arity problem; {args:?} gave:\n{stderr}"
        );
    }
}

/// `--` is answered on its own terms. It is the end-of-options marker
/// everywhere else, so the person typing it has the right instinct and the
/// wrong tool here — telling them it is "not an option" would read as a
/// spelling correction.
#[test]
fn the_end_of_options_marker_is_declined_by_name() {
    let output = okf_graph()
        .args(["--", fixture("clean").to_str().expect("utf-8")])
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("`--` is not supported") && stderr.contains("./"),
        "stderr: {stderr}"
    );
}

/// The same rule in the value position. `--deny --qiuet CODE` used to report
/// that no rule has the code `--qiuet`, which names the rule table for what is
/// a forgotten argument or a mistyped flag. No rule code or date begins with a
/// dash, so nothing legitimate is caught.
#[test]
fn a_flag_argument_that_looks_like_an_option_is_rejected() {
    let cases: [(&[&str], &str); 4] = [
        (&["--deny", "--qiuet"], "rule code"),
        (&["--warn", "--quiet"], "rule code"),
        (&["--allow", "-h"], "rule code"),
        (&["--as-of", "--quiet"], "date"),
    ];

    for (args, expected) in cases {
        let output = okf_graph().args(args).output().expect("runs");
        assert_eq!(output.status.code(), Some(2), "{args:?}");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("looks like an option")
                && stderr.contains(expected)
                && stderr.contains(args[1]),
            "expected the argument to be named; {args:?} gave:\n{stderr}"
        );
    }
}

/// A rule code containing a dash is still a rule code — the guard above keys on
/// a *leading* dash, and every code in the table has an interior one.
#[test]
fn a_rule_code_with_an_interior_dash_still_works() {
    let output = okf_graph()
        .args(["--allow", "BUNDLE-2"])
        .arg(fixture("dangling"))
        .output()
        .expect("runs");

    assert_eq!(output.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("BUNDLE-2"));
}

/// The cost of the rule above, and its escape hatch. A directory whose name
/// begins with `-` is no longer a bare relative path, but `./` still reaches
/// it — which is why no `--` marker was added.
#[test]
fn a_path_starting_with_a_dash_is_reachable_through_dot_slash() {
    let root = std::env::temp_dir().join("okf-graph-cli-dash");
    let bundle = root.join("-weird");
    std::fs::create_dir_all(&bundle).expect("mkdir");
    std::fs::copy(fixture("clean/overview.md"), bundle.join("overview.md")).expect("copy");

    let bare = okf_graph()
        .current_dir(&root)
        .arg("-weird")
        .output()
        .expect("runs");
    assert_eq!(bare.status.code(), Some(2), "a bare `-weird` is now a typo");

    let escaped = okf_graph()
        .current_dir(&root)
        .arg("./-weird")
        .output()
        .expect("runs");
    assert_ne!(
        escaped.status.code(),
        Some(2),
        "`./-weird` must still reach the bundle; stderr: {}",
        String::from_utf8_lossy(&escaped.stderr)
    );
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
