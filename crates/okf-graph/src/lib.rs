//! **Topology and identity** of an OKF Knowledge Bundle read as a knowledge
//! graph: what Concepts exist, what they are called, what points at what, what
//! resolves and what dangles.
//!
//! Node bodies are carried as **opaque payloads** and are not interpreted here.
//! This crate knows nothing about colors, norms, obligations, or the
//! mechanical/judgment seam — that is [`okf-normative`]'s work, and it depends
//! on this crate rather than the other way around.
//!
//! The boundary is easy to violate inside a workspace, so here is a falsifiable
//! test for it: **if this crate's test suite cannot be read by someone who has
//! never heard of deontic logic, the boundary is not real.**
//!
//! Graph properties are checked **per edge kind, not over the whole graph**.
//! OKF links carry no type (SPEC §6): an edge takes its kind from where the
//! reference sits — a body-link, a `resource`, a `sources` entry, the
//! parent/child of the directory tree, an Attested-Computation path — and the
//! algebra differs by kind (parent/child is a tree, derivation must not cycle,
//! body-links may cycle harmlessly). A single global acyclicity rule would
//! reject a correct Bundle. See `docs/okf-graph-DESIGN.md` for the model.
//!
//! So far this crate models the unit the graph is made of: an OKF
//! [`Concept`] — one markdown document, read as a [`Frontmatter`] block and a
//! [`Body`] (SPEC §4). Bundles, ids, links, and resolution come next.
//!
//! [`okf-normative`]: https://github.com/ojhermann-org/okf-tools

mod bundle;
mod concept;
mod finding;
mod index;
mod links;
mod log;
mod paths;
mod provenance;
mod timestamp;

pub use bundle::Bundle;
pub use concept::{
    Attester, Body, Concept, ConceptError, Executor, Frontmatter, Generated, Parameter, Source,
    Status, UsageWindow, Verification,
};
pub use finding::{Finding, Rule, Severity};
pub use links::{links_in, Link, LinkKind};
pub use paths::{classify_path, resolve_path, PathKind};
pub use provenance::Derivation;
pub use timestamp::Timestamp;
