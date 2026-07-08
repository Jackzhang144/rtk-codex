#!/usr/bin/env sh
# Tests for install.sh path traversal check (issue #1250, CWE-22).
#
# Verifies:
#   1. Safe archives (single binary, "./prefix", subdirs) are accepted.
#   2. Archives with absolute paths are rejected pre-extraction.
#   3. Archives with ".." components are rejected pre-extraction.
#   4. The check is still present in install.sh (regression guard).
#   5. The release repository can be overridden with RTK_REPO.
#   6. The installer warns when another rtk binary shadows the installed path.

set -eu

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
INSTALL_SH="$REPO_ROOT/install.sh"

if [ ! -f "$INSTALL_SH" ]; then
    echo "FAIL: install.sh not found at $INSTALL_SH"
    exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "SKIP: python3 not available — crafted tarball tests require python3"
    exit 0
fi

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# The check replicated from install.sh (keep in sync with install.sh).
# Returns 0 when archive is safe, 1 when unsafe.
check_archive() {
    if tar -tzf "$1" | grep -qE '^/|(^|/)\.\.(/|$)'; then
        return 1
    fi
    return 0
}

# --- Build safe archive using standard tar ---
mkdir -p "$TMPDIR/safe_src"
printf '#!/bin/sh\necho rtk\n' > "$TMPDIR/safe_src/rtk"
(cd "$TMPDIR/safe_src" && tar -czf "$TMPDIR/safe.tgz" rtk)

# --- Build crafted malicious archives with python ---
python3 - "$TMPDIR" <<'PY'
import sys, tarfile, io

base = sys.argv[1]


def make(name, entry):
    with tarfile.open(f"{base}/{name}", "w:gz") as t:
        info = tarfile.TarInfo(name=entry)
        data = b"pwned"
        info.size = len(data)
        t.addfile(info, io.BytesIO(data))


make("traversal.tgz", "../etc/evil")
make("absolute.tgz", "/tmp/evil_abs")
make("middle.tgz", "rtk/../../../etc/evil")
make("end_dotdot.tgz", "rtk/..")
PY

FAIL=0
pass() { printf '  PASS: %s\n' "$1"; }
fail() { printf '  FAIL: %s\n' "$1"; FAIL=1; }

echo "==> Functional checks"

if check_archive "$TMPDIR/safe.tgz"; then
    pass "safe archive accepted"
else
    fail "safe archive rejected (false positive)"
fi

for bad in traversal absolute middle end_dotdot; do
    if check_archive "$TMPDIR/$bad.tgz"; then
        fail "$bad archive accepted (should be rejected)"
    else
        pass "$bad archive rejected"
    fi
done

echo "==> Regression guard"

if grep -qF 'tar -tzf' "$INSTALL_SH" && grep -qF '\.\.' "$INSTALL_SH"; then
    pass "install.sh still contains the path-traversal check"
else
    fail "install.sh is missing the path-traversal check — was it removed?"
fi

if grep -qF 'REPO="${RTK_REPO:-rtk-ai/rtk}"' "$INSTALL_SH"; then
    pass "install.sh lets RTK_REPO override the release repository"
else
    fail "install.sh does not let RTK_REPO override the release repository"
fi

if grep -qF 'Installed binary is shadowed' "$INSTALL_SH"; then
    pass "install.sh warns when PATH resolves to another rtk binary"
else
    fail "install.sh does not warn when PATH resolves to another rtk binary"
fi

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "All install.sh path traversal tests passed"
    exit 0
else
    echo "Some tests failed"
    exit 1
fi
