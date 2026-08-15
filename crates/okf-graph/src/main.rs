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

use okf_graph::{Bundle, Date, Finding, Level, Policy, Rule};

const USAGE: &str = "\
okf-graph — structural / topological validator for an OKF Knowledge Bundle

Usage:
    okf-graph [--quiet] [--allow-empty] [--as-of <DATE>]
              [--deny|--warn|--allow <CODE>]... <bundle>

Arguments:
    <bundle>       a bundle directory, searched recursively for concept files

Options:
    --deny <CODE>  fail the run on this rule
    --warn <CODE>  print this rule but do not fail on it
    --allow <CODE> do not report this rule at all
    --allow-empty  accept a bundle holding no concepts
    --as-of <DATE> the `YYYY-MM-DD` day to read `stale_after` against
                   (default: today, UTC)
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
`--allow` is counted in the summary rather than vanishing.

A bundle holding no concepts is a usage error (exit 2), not a clean run. A
mistyped path or a bundle that failed to generate would otherwise produce a
green run indistinguishable from a valid one. Pass `--allow-empty` where an
empty bundle is expected.

CONCEPT-15 is the one rule whose answer depends on the day: §5.5 says a concept
is stale when `today >= stale_after`. It is a report, since a stale concept is
still conformant — `--deny CONCEPT-15` gates on it, and `--as-of` asks the
question about another day, which is also what keeps a CI run reproducible.";

fn main() -> ExitCode {
    let mut quiet = false;
    let mut allow_empty = false;
    let mut policy = Policy::new();
    let mut as_of: Option<Date> = None;
    let mut bundle_path: Option<String> = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--quiet" => quiet = true,
            "--allow-empty" => allow_empty = true,
            "--as-of" => {
                let Some(date) = args.next() else {
                    eprintln!("error: --as-of needs a date, e.g. `--as-of 2026-08-15`");
                    return ExitCode::from(2);
                };
                let Some(date) = Date::parse(&date) else {
                    eprintln!("error: `{date}` is not a `YYYY-MM-DD` date");
                    return ExitCode::from(2);
                };
                as_of = Some(date);
            }
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

    // A bundle with no concepts examined nothing, and a green run there is
    // indistinguishable from a green run over a valid bundle — which is how a
    // mistyped path or a bundle that never generated passes CI. Not a rule: §11
    // has no opinion that a bundle must hold concepts, so a `Severity` would
    // misstate the spec. It is the same class as naming a path that is not a
    // directory, and it exits the same way.
    if bundle.is_empty() && !allow_empty {
        eprintln!(
            "error: {bundle_path}: no concepts found (pass --allow-empty if that is expected)"
        );
        return ExitCode::from(2);
    }

    // Staleness is asked separately because it is the one question the bundle
    // alone does not answer (§5.5), and the answer joins the rest here rather
    // than in a second summary a reader would have to add up themselves.
    let staleness = bundle.stale_as_of(as_of.unwrap_or_else(Date::today));
    let found: Vec<&Finding> = bundle.findings().iter().chain(staleness.iter()).collect();
    let findings: Vec<&Finding> = found
        .iter()
        .copied()
        .filter(|f| policy.level(&f.rule) > Level::Allow)
        .collect();
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
    let silenced = found.len() - findings.len();

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

    // Counted rather than asked of `Bundle::fails`, which reads the load's own
    // findings and so cannot see a `--deny CONCEPT-15`.
    if defects > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
