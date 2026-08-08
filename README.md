# okf-tools

A Cargo workspace of tools for validating an [OKF][okf] Knowledge Bundle,
split by layer — topology beneath semantics, both beneath any application:

- **`okf-graph`** — topology and identity: what Concepts exist, what points at
  what, what resolves and what dangles. See
  [`crates/okf-graph/README.md`](crates/okf-graph/README.md).
- **`okf-normative`** — semantics: the normative reading over a validated
  graph. A skeleton — nothing is implemented yet.
- **`deon`** — the archived reference implementation the two supersede, kept as
  a green behavioural baseline (`publish = false`); see
  [`crates/deon/README.md`](crates/deon/README.md).

The order of precedence is **okf > `okf-graph` > `okf-normative` >
applications**: the spec is upstream of everything here, and topology is
upstream of semantics.

## Run it

```sh
# Validate a bundle's structure and topology (exit 0 clean, 1 on a defect).
nix run .#okf-graph -- crates/okf-graph/tests/fixtures/clean

# The archived deon checker stays `nix run .`.
nix run . -- crates/deon/examples/
```

From another flake, `packages.<system>.okf-graph` is the validator on its own —
what a devShell or a CI step wants, where an app cannot go. `packages.default`
is the whole workspace, `deon-check` included.

`nix flake check` — the single required check on `main` — builds the workspace,
runs `cargo fmt --check` and `clippy -D warnings`, and tests every crate.

[okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
