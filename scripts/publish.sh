#!/usr/bin/env bash
#
# Publish a workspace crate to crates.io and tag the release.
#
# Owner-run, always. Publishing is the one step ~/.claude/CLAUDE.md's gate 1
# keeps for the owner, and the auto-mode classifier blocks it for an agent — as
# it should: a crates.io version can be yanked but never deleted, and the name is
# claimed for good. This script exists so the owner runs one command instead of
# assembling four, not to move the gate.
#
# The token comes from Infisical (`dev` `/shared` `CARGO_REGISTRY_TOKEN`), which
# is why this wraps `infisical run` rather than expecting `cargo login` to have
# written a token to ~/.cargo/credentials.toml. Nothing here prints it.
#
# The Infisical project is named by `INFISICAL_PROJECT_ID` from the environment,
# and deliberately not committed: this repo is public, and whether an internal
# identifier belongs in it is the owner's call, not a script's. Set it in the
# untracked local `.envrc`, which is where machine-local values already live.
#
# Guards, each one a failure that is annoying to diagnose after the fact:
#   - the working tree is clean, and HEAD is the default branch at its remote tip
#   - the version is not already on crates.io (a re-publish is a hard 403)
#   - the CHANGELOG names the version being published
#   - `--dry-run` runs every guard and cargo's own verification, publishing
#     nothing, so the first real run is not also the first run
#
# Usage: scripts/publish.sh [crate] [--dry-run]     (crate defaults to okf-graph)
#
# Requires: cargo, git, infisical (logged in), curl, jq.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found on PATH" >&2; exit 1; }; }
need cargo
need git
need infisical
need curl
need jq

CRATE="okf-graph"
DRY=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY=1 ;;
    -*) echo "usage: $0 [crate] [--dry-run]" >&2; exit 2 ;;
    *) CRATE="$arg" ;;
  esac
done

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

VERSION="$(cargo metadata --no-deps --format-version 1 \
  | jq -r --arg c "$CRATE" '.packages[] | select(.name == $c) | .version')"
[ -n "$VERSION" ] || { echo "error: '$CRATE' is not a member of this workspace" >&2; exit 1; }

MANIFEST="$(cargo metadata --no-deps --format-version 1 \
  | jq -r --arg c "$CRATE" '.packages[] | select(.name == $c) | .manifest_path')"
CRATE_DIR="$(dirname "$MANIFEST")"

echo "publishing $CRATE $VERSION"

# A dirty tree publishes something no commit describes; cargo refuses anyway,
# but says so only after the verification build.
if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is not clean — commit or stash first" >&2
  exit 1
fi

# Publish from the merged history, not from a branch that may never land.
DEFAULT="$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's|^origin/||')"
DEFAULT="${DEFAULT:-main}"
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" != "$DEFAULT" ]; then
  echo "error: on '$BRANCH', not '$DEFAULT' — publish from the merged history" >&2
  exit 1
fi
git fetch --quiet origin "$DEFAULT"
if [ "$(git rev-parse HEAD)" != "$(git rev-parse "origin/$DEFAULT")" ]; then
  echo "error: local '$DEFAULT' differs from origin — pull or push first" >&2
  exit 1
fi

# crates.io answers 404 for a crate nobody has published, which is the expected
# case for a first release; anything else means the version may already be there.
PUBLISHED="$(curl -sS -H 'User-Agent: okf-tools-publish' \
  "https://crates.io/api/v1/crates/$CRATE" \
  | jq -r --arg v "$VERSION" '[.versions[]?.num // empty] | index($v) // "no"')"
if [ "$PUBLISHED" != "no" ]; then
  echo "error: $CRATE $VERSION is already on crates.io — bump the version" >&2
  exit 1
fi

# A CHANGELOG heading for the version being published; "Unreleased" ships a
# document that is wrong for that version forever, since the .crate carries it.
CHANGELOG="$CRATE_DIR/CHANGELOG.md"
if [ -f "$CHANGELOG" ] && ! grep -q "^## $VERSION" "$CHANGELOG"; then
  echo "error: $CHANGELOG has no '## $VERSION' heading — stamp the release first" >&2
  exit 1
fi

if [ "$DRY" -eq 0 ] && [ -z "${INFISICAL_PROJECT_ID:-}" ]; then
  echo "error: INFISICAL_PROJECT_ID is not set — the Infisical CLI cannot find" >&2
  echo "       the project. Add it to the untracked local .envrc:" >&2
  echo "         export INFISICAL_PROJECT_ID=<id>   # then: direnv allow" >&2
  exit 1
fi

TAG="$CRATE-v$VERSION"
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  echo "error: tag '$TAG' already exists" >&2
  exit 1
fi

if [ "$DRY" -eq 1 ]; then
  echo "dry run: every guard passed; running cargo's verification build"
  cargo publish -p "$CRATE" --dry-run
  echo "dry run: nothing was published, and no tag was created"
  exit 0
fi

# `infisical run` injects CARGO_REGISTRY_TOKEN into cargo's environment. The
# token is never echoed, and never lands in ~/.cargo/credentials.toml.
infisical run --projectId "$INFISICAL_PROJECT_ID" --env=dev --path=/shared \
  -- cargo publish -p "$CRATE"

echo "tagging $TAG"
git tag -a "$TAG" -m "$CRATE $VERSION"
git push origin "$TAG"

echo "done: https://crates.io/crates/$CRATE/$VERSION"
