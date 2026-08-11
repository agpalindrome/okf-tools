#!/usr/bin/env bash
#
# Reconcile this repo's own (non-org-level) GitHub settings from version
# control. Two subjects: the repo-level ruleset in .github/rulesets/main.json
# (GitHub's native ruleset format), and the About block — description, homepage,
# topics — in .github/settings/about.json. This script diffs both against live
# state (--check) or applies both (--apply).
#
# Org-wide rules (the org `default-branch` ruleset) live in ~/github-settings
# and are managed with tofu — this script never touches them. The two layers
# compose; GitHub enforces the most restrictive.
#
# Run it yourself: it uses your `gh` auth (repo admin required). It is
# deliberately owner-run, not wired into CI, so settings never change silently.
#
# Requires: gh (authenticated), jq.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found on PATH" >&2; exit 1; }; }
need gh
need jq

ROOT="$(git rev-parse --show-toplevel)"
RULESET_FILE="$ROOT/.github/rulesets/main.json"
ABOUT_FILE="$ROOT/.github/settings/about.json"
REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
NAME="$(jq -r .name "$RULESET_FILE")"

# The fields that define the ruleset; server-added metadata (id, timestamps,
# _links, source) is dropped so the diff shows only meaningful drift.
#
# Array order is not compared, at any depth. GitHub re-canonicalizes it on every
# PUT — a newly-added rule lands last — so a positional diff reports that
# reordering as spurious drift even when the content matches exactly. `rules`
# was already sorted by type for that reason, but only `rules`: the walk() below
# extends the same treatment to bypass_actors, required status checks and
# allowed merge methods, which are the same shape and would have bitten next
# (observed 2026-08-11 on ojhermann-org/agpalindrome-aws, where a rule written
# anywhere but last cost a second PR to correct). `-S` sorts object keys; both
# sides of the diff go through this.
#
# The cost, stated plainly: if GitHub ever gives an array in a ruleset a meaning
# that depends on its order, this diff will not see a change to it. Nothing in
# the current schema is known to work that way — rules all apply and none is
# first-match — but that is reasoning about the schema, not a guarantee from it.
normalize() {
  jq -S '{name, target, enforcement, bypass_actors, conditions, rules}
         | walk(if type == "array" then sort else . end)'
}

existing_id() {
  gh api "repos/$REPO/rulesets" \
    --jq ".[] | select(.name==\"$NAME\") | .id" 2>/dev/null | head -n1
}

# An unset description or homepage reads back as null but is written as "", and
# GitHub alphabetizes topics however they were sent — so both sides are put
# through this before diffing, or an in-sync repo would report drift.
normalize_about() {
  jq -S '{description: (.description // ""), homepage: (.homepage // ""),
          topics: (.topics | sort)}'
}

live_about() {
  gh api "repos/$REPO" --jq '{description, homepage}' \
    | jq --argjson names "$(gh api "repos/$REPO/topics" --jq .names)" \
         '. + {topics: $names}'
}

# Each check_* reports its own drift and returns non-zero, so --check can run
# both and show everything that has moved rather than stopping at the first.
check_ruleset() {
  local id want live
  id="$(existing_id)"
  if [ -z "$id" ]; then
    echo "drift: no live ruleset named '$NAME' on $REPO — run --apply to create it"
    return 1
  fi
  want="$(normalize <"$RULESET_FILE")"
  live="$(gh api "repos/$REPO/rulesets/$id" | normalize)"
  if diff <(printf '%s\n' "$want") <(printf '%s\n' "$live") >/dev/null; then
    echo "in sync: ruleset '$NAME' on $REPO matches $RULESET_FILE"
  else
    echo "drift between $RULESET_FILE (<) and live ruleset '$NAME' (>):"
    diff <(printf '%s\n' "$want") <(printf '%s\n' "$live") || true
    return 1
  fi
}

check_about() {
  local want live
  want="$(normalize_about <"$ABOUT_FILE")"
  live="$(live_about | normalize_about)"
  if diff <(printf '%s\n' "$want") <(printf '%s\n' "$live") >/dev/null; then
    echo "in sync: About block on $REPO matches $ABOUT_FILE"
  else
    echo "drift between $ABOUT_FILE (<) and the live About block (>):"
    diff <(printf '%s\n' "$want") <(printf '%s\n' "$live") || true
    return 1
  fi
}

apply_ruleset() {
  local id
  id="$(existing_id)"
  if [ -n "$id" ]; then
    echo "updating ruleset '$NAME' (id $id) on $REPO"
    gh api -X PUT "repos/$REPO/rulesets/$id" --input "$RULESET_FILE" >/dev/null
  else
    echo "creating ruleset '$NAME' on $REPO"
    gh api -X POST "repos/$REPO/rulesets" --input "$RULESET_FILE" >/dev/null
  fi
  echo "applied '$NAME' to $REPO."
}

# Two calls because topics have their own endpoint; PATCHing only the two named
# fields leaves every other repo setting — visibility above all — untouched.
apply_about() {
  jq '{description, homepage}' "$ABOUT_FILE" \
    | gh api -X PATCH "repos/$REPO" --input - >/dev/null
  jq '{names: .topics}' "$ABOUT_FILE" \
    | gh api -X PUT "repos/$REPO/topics" --input - >/dev/null
  echo "applied the About block to $REPO."
}

case "${1:-}" in
  --check)
    drift=0
    check_ruleset || drift=1
    check_about || drift=1
    exit "$drift"
    ;;
  --apply)
    apply_ruleset
    apply_about
    ;;
  *)
    echo "usage: $0 [--check|--apply]" >&2
    exit 2
    ;;
esac
