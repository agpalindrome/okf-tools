//! The **derivation graph** (§5.1): a `sources[].resource` that points at
//! another concept is an edge "A derives from B". This module is the pure graph
//! logic over resolved `Derivation` edges — the transitive-ancestor walk that
//! credibility propagation follows (§5.1), and, later, cycle detection. It
//! knows nothing about the bundle or how the edges were resolved.

use std::collections::{BTreeMap, BTreeSet};

/// A derivation edge: `from` derives from `to`, both Concept IDs (§5.1). Only
/// a `sources[].resource` that resolves to a concept becomes one; an external
/// URL, a scope descriptor, or a non-concept file is a leaf source, not an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derivation {
    /// The deriving concept.
    pub from: String,
    /// The concept it derives from.
    pub to: String,
}

/// The concepts `start` transitively derives from, sorted and deduplicated and
/// excluding `start` itself. Cycle-safe: a visited set makes the walk terminate
/// even when the edges cycle, so a consumer can propagate credibility without
/// looping forever (§5.1).
pub(crate) fn ancestors<'a>(edges: &'a [Derivation], start: &str) -> Vec<&'a str> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        adjacency
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
    }

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut stack: Vec<&str> = adjacency
        .get(start)
        .into_iter()
        .flatten()
        .copied()
        .collect();
    let mut out: Vec<&str> = Vec::new();
    while let Some(node) = stack.pop() {
        if node == start || !seen.insert(node) {
            continue;
        }
        out.push(node);
        if let Some(next) = adjacency.get(node) {
            stack.extend(next.iter().copied());
        }
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &str, to: &str) -> Derivation {
        Derivation {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn ancestors_follow_a_chain_transitively() {
        // a -> b -> c ; a's ancestors are b and c.
        let edges = [edge("a", "b"), edge("b", "c")];
        assert_eq!(ancestors(&edges, "a"), ["b", "c"]);
        assert_eq!(ancestors(&edges, "b"), ["c"]);
        assert_eq!(ancestors(&edges, "c"), Vec::<&str>::new());
    }

    #[test]
    fn a_shared_ancestor_is_listed_once() {
        // a -> b, a -> c, b -> d, c -> d ; d appears once.
        let edges = [
            edge("a", "b"),
            edge("a", "c"),
            edge("b", "d"),
            edge("c", "d"),
        ];
        assert_eq!(ancestors(&edges, "a"), ["b", "c", "d"]);
    }

    #[test]
    fn ancestors_terminate_through_a_cycle() {
        // a -> b -> a ; the walk from a terminates, excluding a itself.
        let edges = [edge("a", "b"), edge("b", "a")];
        assert_eq!(ancestors(&edges, "a"), ["b"]);
        // and a self-edge does not loop.
        assert_eq!(ancestors(&[edge("x", "x")], "x"), Vec::<&str>::new());
    }
}
