# okf-tools

A Cargo workspace of tools for validating an [OKF][okf] Knowledge Bundle,
split by layer — topology beneath semantics, both beneath any application:

- **`okf-graph`** — topology and identity: what Concepts exist, what points at
  what, what resolves and what dangles. The structural / topological validator,
  built out; a concept's body is carried as an opaque payload and never
  interpreted. See [`crates/okf-graph/README.md`](crates/okf-graph/README.md).
- **`okf-normative`** — semantics: the normative reading over a validated graph
  (the mechanical / judgment / election cut, grounding, coverage, conflict,
  defeat, termination at the seam). A skeleton — nothing is implemented yet.
- **`deon`** — the archived reference implementation the two supersede, kept as
  a green behavioural baseline (`publish = false`); see
  [`crates/deon/README.md`](crates/deon/README.md).

`okf-normative` depends on `okf-graph`, never the reverse. The order of
precedence is **okf > `okf-graph` > `okf-normative` > applications**: the spec
is upstream of everything here, and topology is upstream of semantics.

## Run it

```sh
# Validate a bundle's structure and topology (exit 0 clean, 1 on a defect).
nix run .#okf-graph -- crates/okf-graph/tests/fixtures/clean

# The archived deon checker stays `nix run .`.
nix run . -- crates/deon/examples/
```

`nix flake check` — the single required check on `main` — builds the workspace,
runs `cargo fmt --check` and `clippy -D warnings`, and tests every crate.

## More

The architecture across the stack — how these crates relate to
[Pacioli](https://github.com/ojhermann-org/pacioli), to the Knowledge Bundles,
and to the applied accounting agent — lives on the [`okf-stack` project
board][board]; per-repo docs replace it as each repo is built out. The design
records live under [`docs/`](docs/), and the [friction log](docs/okf-friction.md)
tracks where the OKF format fights the tools.

[okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[board]: https://github.com/orgs/ojhermann-org/projects/8
