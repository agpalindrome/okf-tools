# okf-tools

A Cargo workspace of tools for validating an [OKF][okf] Knowledge Bundle:

- **`okf-graph`** — topology and identity (what Concepts exist, what points at
  what, what resolves and what dangles).
- **`okf-normative`** — the normative reading over a validated graph.
- **`deon`** — the archived reference implementation the two supersede; see
  [`crates/deon/README.md`](crates/deon/README.md).

🚧 **Work in progress.** This README is a placeholder and will be filled in as
the crates are built out. Until then, the architecture lives on the
[`okf-stack` project board][board], and each crate's own docs and module
headers carry the current detail.

[okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md
[board]: https://github.com/orgs/ojhermann-org/projects/8
