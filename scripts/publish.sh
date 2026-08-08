#!/usr/bin/env bash
#
# Publish a workspace crate to crates.io and tag the release.
#
# Restored after okf-tools#82 removed it. That removal was right at the time —
# the release path was being reconsidered, and a wrapper nobody runs rots. The
# path is now settled: ~/.claude/CLAUDE.md's *Publishing a release the owner has
# named* states five conditions under which a release the owner has already
# asked for may be carried out. Four of them are mechanical, and this script is
# where they are checked rather than remembered. The fifth — that the owner named
# this release — is not something a script can verify, and stays a judgment made
# before it is run.
#
# So this is runnable by the owner or by an agent, and the guards are the same
# either way. It does not widen anything: gate 1 still owns the decision.
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
#   - the version is not already on crates.io
#   - the CHANGELOG names the version being published
#   - `--dry-run` runs every guard and cargo's own verification, publishing
#     nothing, so the first real run is not also the first run
#
# Usage: scripts/publish.sh [crate] [--dry-run]     (crate defaults to okf-graph)
#
# Requires: cargo, git, curl, jq, and the infisical CLI (logged in). All but
# infisical are in this repo's devShell; see `find_infisical` for that one.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found on PATH" >&2; exit 1; }; }
need cargo
need git
need curl
need jq

# The infisical CLI is not in this repo's devShell and is not installed
# globally: it comes from ~/infisical's, which is the seat that owns secrets.
# Resolved at runtime rather than pinned — a hardcoded /nix/store path goes stale
# on the next infisical release, silently, and this script runs rarely enough
# that nobody would notice until a release day.
find_infisical() {
  local found
  found="$(command -v infisical 2>/dev/null || true)"
  if [ -z "$found" ] && [ -d "$HOME/infisical" ]; then
    found="$(nix develop "$HOME/infisical" -c bash -c 'command -v infisical' 2>/dev/null || true)"
  fi
  [ -n "$found" ] || {
    echo "error: cannot find the infisical CLI — not on PATH, and not resolvable" >&2
    echo "       from ~/infisical's devShell. That seat owns the token." >&2
    return 1
  }
  printf '%s' "$found"
}

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

# crates.io answers 404 for a version nobody has published and 200 for one that
# exists — but only when the request carries a User-Agent. Without one it answers
# 403, which is neither, so the header is load-bearing rather than politeness
# (observed 2026-08-08). Anything other than 200 or 404 means the check did not
# run, and that is a stop: a re-publish cannot be undone by trying again.
STATUS="$(curl -sS -o /dev/null -w '%{http_code}' \
  -H 'User-Agent: okf-tools-publish' \
  "https://crates.io/api/v1/crates/$CRATE/$VERSION")"
case "$STATUS" in
  404) ;;
  200) echo "error: $CRATE $VERSION is already on crates.io — bump the version" >&2; exit 1 ;;
  *)   echo "error: crates.io answered $STATUS for $CRATE $VERSION — cannot tell" >&2
       echo "       whether the version is taken, so refusing to publish" >&2
       exit 1 ;;
esac

# A CHANGELOG heading for the version being published; "Unreleased" ships a
# document that is wrong for that version forever, since the .crate carries it.
CHANGELOG="$CRATE_DIR/CHANGELOG.md"
if [ -f "$CHANGELOG" ] && ! grep -q "^## $VERSION" "$CHANGELOG"; then
  echo "error: $CHANGELOG has no '## $VERSION' heading — stamp the release first" >&2
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
  echo "note: the infisical step is not exercised by a dry run"
  exit 0
fi

if [ -z "${INFISICAL_PROJECT_ID:-}" ]; then
  echo "error: INFISICAL_PROJECT_ID is not set — the Infisical CLI cannot find" >&2
  echo "       the project. Add it to the untracked local .envrc:" >&2
  echo "         export INFISICAL_PROJECT_ID=<id>   # then: direnv allow" >&2
  exit 1
fi

INFISICAL="$(find_infisical)"

# `infisical run` injects CARGO_REGISTRY_TOKEN into cargo's environment. The
# token is never echoed, and never lands in ~/.cargo/credentials.toml.
"$INFISICAL" run --projectId "$INFISICAL_PROJECT_ID" --env=dev --path=/shared \
  -- cargo publish -p "$CRATE"

echo "tagging $TAG"
git tag -a "$TAG" -m "$CRATE $VERSION"
git push origin "$TAG"

echo "done: https://crates.io/crates/$CRATE/$VERSION"
