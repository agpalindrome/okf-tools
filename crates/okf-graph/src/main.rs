//! `okf-graph` — validate the structure and topology of an OKF Knowledge
//! Bundle. Load the bundle, print every finding, and set an exit code from the
//! defects.
//!
//! Usage: `okf-graph [--quiet] <bundle>` — one bundle directory.
//! Exit: 0 = no defects (reports may still print), 1 = one or more defects,
//! 2 = usage or IO error.
//!
//! Findings split into defects (a §11 conformance failure) and reports
//! (something the spec says to tolerate — a broken link, a missing optional
//! family, an out-of-order log). Only defects fail the run; reports are printed
//! so nothing is silently dropped. This mirrors `deon-check`; `nix run .` stays
//! `deon-check`, and this binary is `nix run .#okf-graph`.

use std::path::Path;
use std::process::ExitCode;

use okf_graph::{Bundle, Severity};

const USAGE: &str = "\
okf-graph — structural / topological validator for an OKF Knowledge Bundle

Usage:
    okf-graph [--quiet] <bundle>

Arguments:
    <bundle>     a bundle directory, searched recursively for concept files

Options:
    --quiet      print findings only (suppress the summary line)
    -h, --help   show this help

Exit codes:
    0  no defects (reports may still be printed)
    1  one or more defects
    2  usage / IO error

A *report* is a finding the spec says to tolerate (a broken link, a missing
optional family, an out-of-order log). It is printed but does not fail the run.";

fn main() -> ExitCode {
    let mut quiet = false;
    let mut bundle_path: Option<String> = None;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--quiet" => quiet = true,
            _ if bundle_path.is_none() => bundle_path = Some(arg),
            _ => {
                eprintln!("error: expected a single bundle path");
                return ExitCode::from(2);
            }
        }
    }

    let Some(bundle_path) = bundle_path else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let bundle = match Bundle::load(Path::new(&bundle_path)) {
        Ok(bundle) => bundle,
        Err(e) => {
            eprintln!("error: {bundle_path}: {e}");
            return ExitCode::from(2);
        }
    };

    for finding in bundle.findings() {
        println!("{finding}");
    }

    let defects = bundle
        .findings()
        .iter()
        .filter(|f| f.severity() == Severity::Defect)
        .count();
    let reports = bundle.findings().len() - defects;

    if !quiet {
        eprintln!(
            "{defects} defect(s), {reports} report(s) across {} concept(s) in {bundle_path}",
            bundle.len(),
        );
    }

    if defects == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
