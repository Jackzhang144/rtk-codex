#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPT="$ROOT/scripts/next-codex-version.sh"
REPO="$(mktemp -d)"
trap 'rm -rf "$REPO"' EXIT

git -C "$REPO" init -q
git -C "$REPO" config user.email test@example.com
git -C "$REPO" config user.name test
git -C "$REPO" commit --allow-empty -qm "chore: initial"
MAIN_BRANCH="$(git -C "$REPO" branch --show-current)"

assert_version() {
  local expected="$1"
  local actual
  actual="$(cd "$REPO" && bash "$SCRIPT" 0.42.4)"
  [ "$actual" = "$expected" ] || {
    echo "expected $expected, got $actual" >&2
    exit 1
  }
}

git -C "$REPO" tag v0.42.4-codex.2.1
git -C "$REPO" branch upstream
git -C "$REPO" commit --allow-empty -qm "fix(hooks): preserve Codex approval"
assert_version "0.42.4-codex.2.2"

git -C "$REPO" checkout -q upstream
git -C "$REPO" commit --allow-empty -qm "feat(grep): add upstream behavior"
git -C "$REPO" checkout -q "$MAIN_BRANCH"
git -C "$REPO" merge --no-ff -qm "merge: sync upstream develop" upstream
assert_version "0.42.4-codex.2.2"

git -C "$REPO" commit --allow-empty -qm "feat(hooks): add Codex policy"
assert_version "0.42.4-codex.2.2"

NEW_REPO="$(mktemp -d)"
trap 'rm -rf "$REPO" "$NEW_REPO"' EXIT
git -C "$NEW_REPO" init -q
git -C "$NEW_REPO" config user.email test@example.com
git -C "$NEW_REPO" config user.name test
git -C "$NEW_REPO" commit --allow-empty -qm "chore: initial"
actual="$(cd "$NEW_REPO" && bash "$SCRIPT" 0.43.0)"
[ "$actual" = "0.43.0-codex.1.0" ] || {
  echo "expected 0.43.0-codex.1.0, got $actual" >&2
  exit 1
}

git -C "$NEW_REPO" tag v0.43.0-codex.2
actual="$(cd "$NEW_REPO" && bash "$SCRIPT" 0.43.0)"
[ "$actual" = "0.43.0-codex.2.1" ] || {
  echo "expected 0.43.0-codex.2.1, got $actual" >&2
  exit 1
}
