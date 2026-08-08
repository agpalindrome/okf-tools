{
  description = "deon — colored deontic norm language + static checker (judgment-side sibling to Pacioli)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
    git-hooks.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, git-hooks, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      # The Cargo workspace built hermetically: `cargo fmt --check` + `cargo
      # clippy -D warnings` gate the build, and `cargo test` (buildRustPackage's
      # default checkPhase) runs the acceptance suite across every member — the
      # seeds clean, and a red fixture per check. Deps are vendored from
      # Cargo.lock, so it needs no network. Exposed as both `packages.default`
      # and a flake check, so `nix flake check` — the one required CI status —
      # covers Rust too.
      #
      # Members: `okf-graph` and `okf-normative` (skeletons), and `deon-check`,
      # the archived reference implementation that still carries the binary and
      # the acceptance suite.
      deonCheckFor =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        pkgs.rustPlatform.buildRustPackage {
          pname = "deon-check";
          version = "0.1.0";
          # Only the workspace inputs — keeps the build pure and off target/
          # etc. `crates/` carries each member's src, plus deon's tests/ and
          # examples/, which its acceptance tests read via CARGO_MANIFEST_DIR.
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./crates
            ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [
            pkgs.clippy
            pkgs.rustfmt
          ];
          preBuild = ''
            cargo fmt --check
            cargo clippy --all-targets -- -D warnings
          '';
        };

      # Just the bundle validator, for a flake taking okf-tools as an *input*:
      # an app cannot go in a devShell or buildInputs, and the workspace package
      # puts `deon-check` on PATH too, under a pname naming neither the repo nor
      # the tool asked for (#73).
      #
      # A symlink over the workspace build, not a second `buildRustPackage` with
      # `-p okf-graph`: a separate build would recompile the shared crates and
      # re-run the suite for no new coverage, and consumers would share no store
      # path with `packages.default`. The workspace derivation stays a runtime
      # dependency, so `deon-check` is still in the closure — off PATH, which is
      # what the consumer was asking for.
      okfGraphFor =
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          workspace = deonCheckFor system;
        in
        pkgs.runCommandLocal "okf-graph-0.1.0"
          {
            meta.mainProgram = "okf-graph";
          }
          ''
            mkdir -p "$out/bin"
            ln -s ${workspace}/bin/okf-graph "$out/bin/okf-graph"
          '';

      # Fast, hermetic hygiene checks: Nix formatting/lint, markdown, and
      # whitespace. Mirrors Pacioli's hygiene set *minus* the Lean/nix-proof
      # gates (deon has no Lean yet); the Lean seam joins later.
      hooksFor =
        system:
        git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            # `nixfmt` (not `nixfmt-rfc-style`): as of nixpkgs 25.11 the RFC 166
            # formatter *is* `pkgs.nixfmt`, and the old alias warns on eval.
            nixfmt.enable = true;
            deadnix.enable = true;
            statix.enable = true;
            check-merge-conflicts.enable = true;
            check-added-large-files.enable = true;
            trim-trailing-whitespace.enable = true;
            end-of-file-fixer.enable = true;
            check-yaml.enable = true;
            markdownlint = {
              enable = true;
              settings.configuration = {
                MD013 = {
                  # line length — prose wraps at 80 for terminal review; tables
                  # and code blocks (the abstract-syntax grammar, ASCII) can't
                  # reflow, so exempt them.
                  line_length = 80;
                  tables = false;
                  code_blocks = false;
                };
                MD033 = false; # inline HTML
                # duplicate headings, restricted to siblings: a CHANGELOG repeats
                # `### Added` under every release, and that repetition is the
                # format rather than a mistake.
                MD024.siblings_only = true;
                MD036 = false; # emphasis-as-heading — prose uses emphasis stylistically
                MD040 = false; # fenced code language not required (grammar blocks)
                MD025.front_matter_title = ""; # OKF norm files carry a YAML front-matter title
              };
            };
          };
        };
    in
    {
      packages = forAllSystems (system: {
        default = deonCheckFor system;
        okf-graph = okfGraphFor system;
      });

      # `nix run .#okf-graph -- <bundle>` runs the structural validator; `nix
      # run .` stays `deon-check` via packages.default, leaving CLAUDE.md's
      # documented invocation untouched.
      apps = forAllSystems (system: {
        okf-graph = {
          type = "app";
          program = "${okfGraphFor system}/bin/okf-graph";
        };
      });

      checks = forAllSystems (system: {
        pre-commit = hooksFor system;
        deon-check = deonCheckFor system;
      });

      devShells = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          hooks = hooksFor system;
        in
        {
          default = pkgs.mkShell {
            inherit (hooks) shellHook;
            # hygiene tools + the Rust toolchain for local `cargo` work (mkShell's
            # stdenv provides the C compiler the build scripts link against).
            buildInputs = hooks.enabledPackages ++ [
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
            ];
          };
        }
      );

      formatter = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        pkgs.writeShellApplication {
          name = "fmt";
          runtimeInputs = [
            pkgs.nixfmt
            pkgs.findutils
          ];
          text = ''
            find . -name '*.nix' -not -path './.git/*' -print0 | xargs -0 nixfmt
          '';
        }
      );
    };
}
