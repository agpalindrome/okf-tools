//! Checks a caller brings, for requirements the OKF spec does not make.
//!
//! okf-graph extends exactly as far as the spec and no further. A producer
//! checking a bundle it owns usually wants more than that — every concept
//! carrying a `generated.at`, a house key present in every frontmatter — and
//! those are house rules rather than readings of OKF. This module is where they
//! attach, without the crate learning what any of them are.
//!
//! A check is a [`Check`] implementation, collected into [`Checks`] and run by
//! [`Bundle::check`]. Its findings carry a [`RuleId::Custom`] and flow through
//! [`Policy`] like any other, so a caller's rule can be denied, warned, or
//! allowed by the same machinery.
//!
//! [`Bundle::check`]: crate::Bundle::check
//! [`Policy`]: crate::Policy
//! [`RuleId::Custom`]: crate::RuleId::Custom

use std::fmt;
use std::sync::Arc;

use crate::{Concept, Level, Rule};

/// One of a caller's own verification requirements.
///
/// A trait rather than a closure so a rule's code, its default level, and the
/// reason it exists all live beside its logic. The reason matters more here than
/// it looks: every check is somebody's house rule, and the question a year later
/// is rarely what it checks — it is who wanted it and why.
pub trait Check {
    /// This rule's code, e.g. `TV-1`. It must be unique among the caller's own
    /// codes and must not collide with an OKF code; [`Checks::add`] enforces
    /// both.
    fn code(&self) -> &str;

    /// The level this rule runs at unless a [`Policy`] says otherwise.
    ///
    /// Defaults to [`Level::Defect`]: a caller who wrote a house rule generally
    /// means it. Override it for a rule meant to inform rather than gate.
    ///
    /// [`Policy`]: crate::Policy
    fn default_level(&self) -> Level {
        Level::Defect
    }

    /// Run against one concept. `Ok(())` passes; `Err(detail)` reports, and
    /// `detail` becomes the finding's one-line explanation.
    ///
    /// There is no third outcome. A check that *cannot* evaluate — the field it
    /// reads is malformed, something it depends on is absent — reports and says
    /// so in the detail. An inconclusive state that fell through to silence
    /// would be indistinguishable from a check that ran and was satisfied, which
    /// is the failure this codebase is built to catch.
    ///
    /// A panic is not caught. A panicking check is a bug in the caller's code,
    /// and turning it into a finding would dress a broken check up as a
    /// defective bundle.
    fn check(&self, id: &str, concept: &Concept) -> Result<(), String>;
}

/// So a caller assembling checks dynamically can hand [`Checks::add`] a boxed
/// one, rather than needing a passthrough of its own.
impl Check for Box<dyn Check> {
    fn code(&self) -> &str {
        (**self).code()
    }

    fn default_level(&self) -> Level {
        (**self).default_level()
    }

    fn check(&self, id: &str, concept: &Concept) -> Result<(), String> {
        (**self).check(id, concept)
    }
}

/// Why a check could not be registered.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckError {
    /// Two of the caller's checks share a code.
    DuplicateCode(String),
    /// A check's code collides with an OKF rule's, which would make its findings
    /// lie about where they came from.
    OkfCode(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::DuplicateCode(code) => {
                write!(f, "a check with the code `{code}` is already registered")
            }
            CheckError::OkfCode(code) => {
                write!(f, "`{code}` is an OKF rule code and cannot be reused")
            }
        }
    }
}

impl std::error::Error for CheckError {}

/// A caller's checks, with their codes verified unique.
///
/// Codes are compared ASCII-case-insensitively, matching [`Rule::from_code`], so
/// `tv-1` and `TV-1` collide. Registration fails rather than resolving a
/// collision: two rules under one code make the output ambiguous about which
/// fired and make a [`Policy`] level unresolvable, and taking the last one
/// silently is the same near-miss as a mistyped `--deny` quietly doing nothing.
///
/// [`Policy`]: crate::Policy
#[derive(Default)]
pub struct Checks {
    checks: Vec<(Arc<str>, Box<dyn Check>)>,
}

impl Checks {
    /// An empty set.
    pub fn new() -> Checks {
        Checks::default()
    }

    /// Register one check, or say why it could not be.
    pub fn add(&mut self, check: impl Check + 'static) -> Result<(), CheckError> {
        let code = check.code();
        if Rule::from_code(code).is_some() {
            return Err(CheckError::OkfCode(code.to_string()));
        }
        if self.contains(code) {
            return Err(CheckError::DuplicateCode(code.to_string()));
        }
        self.checks.push((Arc::from(code), Box::new(check)));
        Ok(())
    }

    /// Whether a check with this code is registered, ASCII-case-insensitively.
    pub fn contains(&self, code: &str) -> bool {
        self.checks
            .iter()
            .any(|(registered, _)| registered.eq_ignore_ascii_case(code))
    }

    /// Every registered code, in registration order.
    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.checks.iter().map(|(code, _)| code.as_ref())
    }

    /// How many checks are registered.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Whether no checks are registered.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// The registered checks, for running them.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Arc<str>, &dyn Check)> {
        self.checks
            .iter()
            .map(|(code, check)| (code, check.as_ref()))
    }
}

impl fmt::Debug for Checks {
    /// Codes only — a `dyn Check` has nothing else to show.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Checks")
            .field("codes", &self.codes().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Named(&'static str);

    impl Check for Named {
        fn code(&self) -> &str {
            self.0
        }
        fn check(&self, _id: &str, _concept: &Concept) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn a_check_registers_under_its_code() {
        let mut checks = Checks::new();
        checks.add(Named("TV-1")).expect("registers");

        assert!(checks.contains("TV-1"));
        assert!(checks.contains("tv-1"), "codes compare case-insensitively");
        assert_eq!(checks.codes().collect::<Vec<_>>(), ["TV-1"]);
    }

    #[test]
    fn a_duplicate_code_is_rejected_rather_than_resolved() {
        let mut checks = Checks::new();
        checks.add(Named("TV-1")).expect("registers");

        assert_eq!(
            checks.add(Named("tv-1")),
            Err(CheckError::DuplicateCode("tv-1".into()))
        );
        assert_eq!(checks.len(), 1);
    }

    /// A caller's finding claiming an OKF code would lie about its own
    /// provenance, which is the one thing the two-halved `RuleId` exists to keep
    /// honest.
    #[test]
    fn an_okf_code_cannot_be_reused() {
        let mut checks = Checks::new();

        assert_eq!(
            checks.add(Named("BUNDLE-2")),
            Err(CheckError::OkfCode("BUNDLE-2".into()))
        );
        assert!(checks.is_empty());
    }

    #[test]
    fn the_default_level_is_defect() {
        assert_eq!(Named("TV-1").default_level(), Level::Defect);
    }
}
