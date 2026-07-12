#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
grep -A5 'uses: softprops/action-gh-release@v2' "$ROOT/.github/workflows/release.yml" \
  | grep -Fq 'target_commitish: ${{ github.sha }}' || {
  echo "expected release tag to target github.sha" >&2
  exit 1
}
