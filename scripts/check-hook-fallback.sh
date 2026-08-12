#!/usr/bin/env bash
#
# Assert that the hook `prek install` generates still survives a nix store
# garbage-collection, by installing a real one and taking its pinned path away.
#
# The repo drives its git hooks with prek rather than pre-commit for exactly one
# property: prek's template guards the store path it pins and falls back to
# PATH, where the dev shell supplies the driver (#98). That property lives in a
# template this repo does not own and does not track — the hook is generated and
# untracked — so nothing here would notice it changing. This is what notices.
#
# It reads no fixture and asserts nothing about the text of the template. It
# installs the hook the pinned prek actually writes, rewrites the store path to
# one that does not resolve, and runs it. A template that dropped the fallback
# fails this; a template that reworded it does not.
#
#   scripts/check-hook-fallback.sh    # also `nix flake check`'s hook-fallback
set -euo pipefail

COLLECTED='/nix/store/0000000000000000000000000000000-collected-prek-0.0.0/bin/prek'

fail() {
  echo "hook-fallback: $1" >&2
  exit 1
}

work="$(mktemp -d)"
# Expanded now rather than at exit: `work` is gone from scope by then.
trap "rm -rf '$work'" EXIT

export HOME="$work/home"
mkdir -p "$HOME"
cd "$work"

git init -q .
cat >.pre-commit-config.yaml <<'CONFIG'
repos:
  - repo: local
    hooks:
      - id: noop
        name: noop
        language: system
        entry: true
CONFIG

prek install --hook-type pre-commit >/dev/null 2>&1
hook=.git/hooks/pre-commit
[ -x "$hook" ] || fail 'prek install wrote no executable pre-commit hook'

# A store path on the shebang line is the failure with no way back: the kernel
# refuses the interpreter, so the hook never runs and cannot say why.
case "$(head -1 "$hook")" in
  '#!'/nix/store/*) fail 'the shebang pins a store path, so a collection is unrecoverable' ;;
esac

pinned="$(sed -n 's|^PREK="\(/nix/store/[^"]*\)"$|\1|p' "$hook")"
[ -n "$pinned" ] || fail 'the hook pins no store path in PREK — the template moved, so this check is blind'
[ -x "$pinned" ] || fail "the pinned path is not executable as installed: $pinned"

sed "s|$pinned|$COLLECTED|" "$hook" >"$work/rewritten"
cat "$work/rewritten" >"$hook"
grep -qF "$COLLECTED" "$hook" || fail 'the collection could not be simulated — PREK was not rewritten'

# Green: with the pinned path collected and a driver on PATH, the hook runs it.
mkdir -p "$work/stub"
printf '#!/bin/sh\necho FELL-BACK\n' >"$work/stub/prek"
chmod +x "$work/stub/prek"
out="$(PATH="$work/stub:$PATH" "./$hook" 2>&1)" ||
  fail 'the hook failed with a driver on PATH, so the pinned path has no fallback'
case "$out" in
  *FELL-BACK*) ;;
  *) fail 'the fallback did not reach the driver on PATH' ;;
esac

# Red: with nothing to fall back to it refuses the commit, and names prek —
# which is the whole difference from an error about a file that plainly exists.
status=0
out="$(PATH="$work/stub-empty" "./$hook" 2>&1)" || status=$?
[ "$status" -ne 0 ] || fail 'the hook succeeded with no driver anywhere'
case "$out" in
  *prek*) ;;
  *) fail "the failure named no driver: $out" ;;
esac

echo 'hook-fallback: ok'
