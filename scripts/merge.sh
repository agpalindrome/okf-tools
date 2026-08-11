#!/usr/bin/env bash
#
# Merge an owner-authored PR from the CLI as a ruleset bypass actor.
#
# `gh pr merge` can't do this: --admin overrides *classic* branch protection,
# not rulesets, and doesn't circumvent the merge queue; worse, `gh pr merge`
# refuses at pre-flight when you *have* ruleset bypass authority (cli/cli #8746,
# #13388). The raw REST merge endpoint DOES engage your bypass correctly — even
# with the merge queue + required review — so this script wraps that, then
# deletes the head branch and fast-forwards local main.
#
# It bypasses the required owner review, so it is for merging owner-authored PRs
# (which can't be self-approved). External contributions go through the normal
# gate (owner review -> queue). The auto-mode classifier blocks an *unsolicited*
# agent self-merge, exactly as it blocks --admin — but the agent MAY run this
# when the owner explicitly asks it to merge a specific PR (standing grant in
# ~/.claude/CLAUDE.md, Repo rights -> Where a repo differs, where okf-tools
# shares an entry with pacioli).
#
# Unless --force is passed, it refuses when `gh pr checks` exits non-zero. The
# required check here is `nix flake check`. Observed on PR #96 (2026-08-11):
# the guard passed on a green run, --force was not needed, and a non-required
# [code]smith check sitting in `skipping` did not hold it back. What `gh pr
# checks` does on a PR with no checks at all is untested — the claim this
# header used to make about that is what went stale, so it is not replaced with
# another. --force stays owner-run/confirm-first.
#
# Usage: scripts/merge.sh <pr-number> [--force]
#
# Requires: gh (authenticated, with bypass authority), jq, git.
set -euo pipefail

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' not found on PATH" >&2; exit 1; }; }
need gh
need jq
need git

FORCE=0
PR=""
for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    [0-9]*) PR="$arg" ;;
    *) echo "usage: $0 <pr-number> [--force]" >&2; exit 2 ;;
  esac
done
[ -n "$PR" ] || { echo "usage: $0 <pr-number> [--force]" >&2; exit 2; }

REPO="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"

read -r STATE BRANCH CROSS < <(
  gh pr view "$PR" --json state,headRefName,isCrossRepository \
    --jq '[.state, .headRefName, .isCrossRepository] | @tsv'
)
[ "$STATE" = "OPEN" ] || { echo "error: PR #$PR is $STATE, not OPEN" >&2; exit 1; }

# Refuse to bypass CI unless --force: require all checks green first.
if [ "$FORCE" -ne 1 ]; then
  if ! gh pr checks "$PR" >/dev/null 2>&1; then
    echo "error: PR #$PR has failing or pending required checks — refusing." >&2
    echo "       re-run with --force to merge anyway (bypasses CI)." >&2
    exit 1
  fi
fi

echo "squash-merging PR #$PR on $REPO (ruleset bypass)"
gh api -X PUT "repos/$REPO/pulls/$PR/merge" -f merge_method=squash \
  --jq '"merged: \(.merged)  sha: \(.sha)"'

if [ "$CROSS" = "true" ]; then
  echo "head branch is on a fork — leaving it for the contributor to delete."
else
  echo "deleting branch '$BRANCH'"
  gh api -X DELETE "repos/$REPO/git/refs/heads/$BRANCH" >/dev/null 2>&1 \
    || echo "  (branch already gone)"
fi

# Fast-forward the default branch and prune stale tracking refs (the `git up`
# rhythm). Switch to the default branch *first*: the caller is usually still on
# the just-merged head branch, whose upstream is now gone and which a squash
# merge leaves non-fast-forwardable — so ff'ing the current branch is wrong.
echo "syncing local repo"
DEFAULT="$(gh repo view --json defaultBranchRef --jq .defaultBranchRef.name)"
git remote update -p >/dev/null 2>&1 || true
if [ -n "$DEFAULT" ]; then
  git checkout "$DEFAULT" >/dev/null 2>&1 || echo "  (couldn't switch to '$DEFAULT')"
  git merge --ff-only "origin/$DEFAULT" >/dev/null 2>&1 \
    || echo "  (local '$DEFAULT' not fast-forwardable — resolve by hand)"
fi
# Drop the merged local head branch if it lingers (squash leaves it non-ff, so
# -D; the PR is confirmed merged and the remote branch is already deleted).
if [ "$CROSS" != "true" ] && [ -n "$BRANCH" ] && [ "$BRANCH" != "$DEFAULT" ] \
   && git show-ref --verify --quiet "refs/heads/$BRANCH"; then
  git branch -D "$BRANCH" >/dev/null 2>&1 && echo "deleted local branch '$BRANCH'"
fi
echo "done."
