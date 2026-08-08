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

use okf_graph::{Bundle, Level, Policy, Rule};

const USAGE: &str = "\
okf-graph — structural / topological validator for an OKF Knowledge Bundle

Usage:
    okf-graph [--quiet] [--deny|--warn|--allow <CODE>]... <bundle>

Arguments:
    <bundle>       a bundle directory, searched recursively for concept files

Options:
    --deny <CODE>  fail the run on this rule
    --warn <CODE>  print this rule but do not fail on it
    --allow <CODE> do not report this rule at all
    --quiet        print findings only (suppress the summary line)
    -h, --help     show this help

Exit codes:
    0  no defects (reports may still be printed)
    1  one or more defects
    2  usage / IO error

A *report* is a finding the spec says to tolerate (a broken link, a missing
optional family, an out-of-order log). It is printed but does not fail the run.

The defaults are the spec's: §6 and §11 say a consumer MUST NOT reject a bundle
over a dangling link. A producer checking a bundle it owns is not that consumer,
so a rule can be moved by code — `--deny BUNDLE-2` fails on a dangling link and
leaves every other tolerated rule alone. Repeating a code is last-wins, and
`--allow` is counted in the summary rather than vanishing.";

fn main() -> ExitCode {
    let mut quiet = false;
    let mut policy = Policy::new();
    let mut bundle_path: Option<String> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--quiet" => quiet = true,
            "--deny" | "--warn" | "--allow" => {
                let level = match arg.as_str() {
                    "--deny" => Level::Defect,
                    "--warn" => Level::Report,
                    _ => Level::Allow,
                };
                let Some(code) = args.next() else {
                    eprintln!("error: {arg} needs a rule code, e.g. `{arg} BUNDLE-2`");
                    return ExitCode::from(2);
                };
                let Some(rule) = Rule::from_code(&code) else {
                    eprintln!("error: no rule has the code `{code}`");
                    return ExitCode::from(2);
                };
                policy.set(rule, level);
            }
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

    let findings = bundle.findings_at(&policy);
    for finding in &findings {
        println!("{finding}");
    }

    let defects = findings
        .iter()
        .filter(|f| policy.level(&f.rule) == Level::Defect)
        .count();
    let reports = findings.len() - defects;
    // What `--allow` dropped. Printed whenever it is non-zero: a run that
    // examined less than it looks like it did should say so, or a silenced rule
    // reads as a rule that found nothing.
    let silenced = bundle.findings().len() - findings.len();

    if !quiet {
        let silenced = if silenced > 0 {
            format!(", {silenced} silenced")
        } else {
            String::new()
        };
        eprintln!(
            "{defects} defect(s), {reports} report(s){silenced} across {} concept(s) in {bundle_path}",
            bundle.len(),
        );
    }

    if bundle.fails(&policy) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
