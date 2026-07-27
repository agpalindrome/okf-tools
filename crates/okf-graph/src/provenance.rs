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

/// The derivation cycles in `edges`, each a list of Concept IDs in cycle order
/// — `a -> b -> a` is returned as `["a", "b"]`, and a self-edge `a -> a` as
/// `["a"]`. Each distinct cycle (by member set) is returned once.
pub(crate) fn cycles(edges: &[Derivation]) -> Vec<Vec<String>> {
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut nodes: BTreeSet<&str> = BTreeSet::new();
    for edge in edges {
        adjacency
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.as_str());
        nodes.insert(edge.from.as_str());
        nodes.insert(edge.to.as_str());
    }

    let mut color: BTreeMap<&str, Color> = BTreeMap::new();
    let mut path: Vec<&str> = Vec::new();
    let mut found: Vec<Vec<String>> = Vec::new();
    let mut seen: BTreeSet<BTreeSet<String>> = BTreeSet::new();
    for &node in &nodes {
        if !color.contains_key(node) {
            visit(
                node, &adjacency, &mut color, &mut path, &mut found, &mut seen,
            );
        }
    }
    found
}

#[derive(Clone, Copy, PartialEq)]
enum Color {
    Gray,
    Black,
}

/// Depth-first visit for [`cycles`]: a back-edge to a node still on the path
/// (`Gray`) closes a cycle, recorded as the path slice from that node.
fn visit<'a>(
    node: &'a str,
    adjacency: &BTreeMap<&'a str, Vec<&'a str>>,
    color: &mut BTreeMap<&'a str, Color>,
    path: &mut Vec<&'a str>,
    found: &mut Vec<Vec<String>>,
    seen: &mut BTreeSet<BTreeSet<String>>,
) {
    color.insert(node, Color::Gray);
    path.push(node);
    for &next in adjacency.get(node).into_iter().flatten() {
        match color.get(next) {
            Some(Color::Gray) => {
                let start = path.iter().position(|&n| n == next).unwrap_or(0);
                let cycle: Vec<String> = path[start..].iter().map(|s| s.to_string()).collect();
                if seen.insert(cycle.iter().cloned().collect()) {
                    found.push(cycle);
                }
            }
            Some(Color::Black) => {}
            None => visit(next, adjacency, color, path, found, seen),
        }
    }
    color.insert(node, Color::Black);
    path.pop();
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

    #[test]
    fn an_acyclic_graph_has_no_cycles() {
        let edges = [
            edge("a", "b"),
            edge("a", "c"),
            edge("b", "d"),
            edge("c", "d"),
        ];
        assert!(cycles(&edges).is_empty());
    }

    #[test]
    fn a_two_cycle_and_a_self_loop_are_each_found_once() {
        assert_eq!(cycles(&[edge("a", "b"), edge("b", "a")]), [vec!["a", "b"]]);
        assert_eq!(cycles(&[edge("x", "x")]), [vec!["x"]]);
    }

    #[test]
    fn a_cycle_off_an_acyclic_prefix_is_found() {
        // a -> b -> c -> b : the b<->c cycle is reported, a is not in it.
        let edges = [edge("a", "b"), edge("b", "c"), edge("c", "b")];
        assert_eq!(cycles(&edges), [vec!["b", "c"]]);
    }
}
