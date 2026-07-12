#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_VERSION="${1:?usage: next-codex-version.sh <upstream-version>}"
UPSTREAM_PATTERN="${UPSTREAM_VERSION//./\\.}"

if [[ ! "$UPSTREAM_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "upstream version must be X.Y.Z, got: $UPSTREAM_VERSION" >&2
  exit 1
fi

LATEST_TAG=""
ADAPTER_MAJOR=0
ADAPTER_MINOR=0
while IFS= read -r tag; do
  if [[ "$tag" =~ ^v${UPSTREAM_PATTERN}-codex\.([0-9]+)(\.([0-9]+))?$ ]]; then
    LATEST_TAG="$tag"
    ADAPTER_MAJOR="${BASH_REMATCH[1]}"
    ADAPTER_MINOR="${BASH_REMATCH[3]:-0}"
    break
  fi
done < <(git tag -l "v${UPSTREAM_VERSION}-codex.*" --sort=-version:refname)

if [ -z "$LATEST_TAG" ]; then
  printf '%s\n' "${UPSTREAM_VERSION}-codex.1.0"
  exit 0
fi

ADAPTER_MINOR=$((ADAPTER_MINOR + 1))

printf '%s\n' "${UPSTREAM_VERSION}-codex.${ADAPTER_MAJOR}.${ADAPTER_MINOR}"
