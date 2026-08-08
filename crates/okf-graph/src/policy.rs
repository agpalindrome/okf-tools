//! Consumer-settable rule levels, kept separate from [`Severity`] on purpose.
//!
//! `Severity` is §11's own verdict and stays fixed: the spec says a conformant
//! bundle MUST NOT be rejected over a dangling link, and a tool whose reading of
//! the spec moved with a flag would not be reporting the spec any more. A
//! [`Level`] answers a different question — what *this* run does about a rule —
//! and that is the consumer's to set.
//!
//! The distinction is not academic. One binary serves a consumer reading a
//! bundle someone else published, where §11's tolerance is exactly right, and a
//! producer checking a bundle it owns in its own CI, where a dangling link is a
//! link somebody forgot to write. Defaults come from `Severity`, so a run that
//! sets nothing still returns the spec's verdict.

use std::collections::HashMap;

use crate::{Checks, Rule, RuleId, Severity};

/// What a run does about a rule.
///
/// Exhaustive, like [`Severity`] and for the same reason: ignore / print / fail
/// is the whole of what a level can say, so a caller matching all three has
/// covered the domain. A fourth would be a new concept, not a new level.
///
/// Ordered `Allow < Report < Defect`, by how much each one does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Not reported at all.
    Allow,
    /// Printed; does not fail the run.
    Report,
    /// Printed, and fails the run.
    Defect,
}

impl Rule {
    /// The level this rule runs at unless a consumer says otherwise — its
    /// [`Severity`], so an unconfigured run's verdict is the spec's.
    ///
    /// Derived rather than tabulated: a second table beside `severity()` is one
    /// that can disagree with it.
    pub fn default_level(self) -> Level {
        match self.severity() {
            Severity::Defect => Level::Defect,
            Severity::Report => Level::Report,
        }
    }
}

/// A consumer's rule levels: the defaults, plus what it set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    overrides: HashMap<RuleId, Level>,
}

impl Policy {
    /// Every OKF rule at its [`Rule::default_level`], and nothing said about a
    /// caller's rules.
    pub fn new() -> Policy {
        Policy::default()
    }

    /// Every OKF rule at its default, and each of `checks` at the default its
    /// own [`Check::default_level`] declares.
    ///
    /// The convenience worth having: without it, a caller's rule needs an
    /// explicit entry to be anything other than the fallback, which is friction
    /// on the common case of "the defaults I already wrote on the checks".
    ///
    /// [`Check::default_level`]: crate::Check::default_level
    pub fn for_checks(checks: &Checks) -> Policy {
        let mut policy = Policy::new();
        for (code, check) in checks.iter() {
            policy.set(RuleId::Custom(code.clone()), check.default_level());
        }
        policy
    }

    /// Run `rule` at `level`, replacing whatever it was set to before. Last
    /// wins, which is what repeating a flag on a command line means.
    pub fn set(&mut self, rule: impl Into<RuleId>, level: Level) -> &mut Policy {
        self.overrides.insert(rule.into(), level);
        self
    }

    /// The level `rule` runs at here.
    ///
    /// An OKF rule falls back to its [`Rule::default_level`]. A caller's rule
    /// that nothing has set falls back to [`Level::Defect`]: the check fired, and
    /// absent any instruction, saying so is safer than swallowing it.
    pub fn level(&self, rule: &RuleId) -> Level {
        if let Some(level) = self.overrides.get(rule) {
            return *level;
        }
        match rule {
            RuleId::Spec(rule) => rule.default_level(),
            RuleId::Custom(_) => Level::Defect,
        }
    }

    /// The caller-rule codes this policy sets that `checks` does not register.
    ///
    /// Deliberately a question rather than an error. A `Policy` cannot validate
    /// a caller's code when it is set — the caller's list does not exist until
    /// the checks do — and rejecting one later would break a legitimate shape: a
    /// single policy shared across several check sets will name rules that are
    /// not in every run. Only the caller knows whether an unmatched code is a
    /// typo or an intentional superset, so only the caller decides.
    pub fn unmatched(&self, checks: &Checks) -> Vec<&str> {
        let mut codes: Vec<&str> = self
            .overrides
            .keys()
            .filter_map(|rule| match rule {
                RuleId::Custom(code) => (!checks.contains(code)).then_some(code.as_ref()),
                RuleId::Spec(_) => None,
            })
            .collect();
        codes.sort_unstable();
        codes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_level_is_the_severity() {
        assert_eq!(Rule::MissingType.default_level(), Level::Defect);
        assert_eq!(Rule::DanglingLink.default_level(), Level::Report);
    }

    #[test]
    fn an_unset_rule_runs_at_its_default() {
        let policy = Policy::new();
        assert_eq!(
            policy.level(&RuleId::Spec(Rule::DanglingLink)),
            Level::Report
        );
        assert_eq!(
            policy.level(&RuleId::Spec(Rule::MissingType)),
            Level::Defect
        );
    }

    /// The producer's case: fail on the one tolerated rule that matters to you
    /// without touching any of the others.
    #[test]
    fn a_set_rule_moves_and_its_neighbours_do_not() {
        let mut policy = Policy::new();
        policy.set(Rule::DanglingLink, Level::Defect);

        assert_eq!(
            policy.level(&RuleId::Spec(Rule::DanglingLink)),
            Level::Defect
        );
        assert_eq!(
            policy.level(&RuleId::Spec(Rule::LogOutOfOrder)),
            Level::Report
        );
        assert_eq!(Rule::DanglingLink.severity(), Severity::Report);
    }

    #[test]
    fn the_last_setting_wins() {
        let mut policy = Policy::new();
        policy
            .set(Rule::DanglingLink, Level::Defect)
            .set(Rule::DanglingLink, Level::Allow);

        assert_eq!(
            policy.level(&RuleId::Spec(Rule::DanglingLink)),
            Level::Allow
        );
    }
}
