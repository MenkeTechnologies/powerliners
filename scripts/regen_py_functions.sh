#!/usr/bin/env bash
# Regenerate docs/powerline_py_functions.txt from upstream powerline-status.
#
# This is the source-of-truth allowlist consumed by
# tests/ported_fn_names_match_py.rs — every fn name ported into
# src/ported/ must exist in this file (or in tests/data/fake_fn_allowlist.txt
# for maintainer-approved Rust-only exemptions).
#
# Run from the repo root:
#   ./scripts/regen_py_functions.sh
#
# Requires: git, python3.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "[regen] cloning upstream powerline-status into $TMPDIR …"
git clone --depth 1 --quiet https://github.com/powerline/powerline "$TMPDIR/powerline"

echo "[regen] extracting `def NAME(` from powerline/**/*.py …"
python3 - <<PY
import re, pathlib
upstream = pathlib.Path("$TMPDIR/powerline/powerline")
names = set()
for p in upstream.rglob("*.py"):
    src = p.read_text(errors="ignore")
    for m in re.finditer(r'^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(', src, re.MULTILINE):
        names.add(m.group(1))
print(f"  extracted {len(names)} unique function names")
out = pathlib.Path("$ROOT/docs/powerline_py_functions.txt")
header = (
    "# Function names extracted from upstream powerline-status Python source\n"
    "# (github.com/powerline/powerline) at vendoring time.\n"
    "#\n"
    "# Used by tests/ported_fn_names_match_py.rs to enforce that every Rust\n"
    "# fn name under src/ported/ corresponds to a real upstream Python fn.\n"
    "#\n"
    "# Regenerate with scripts/regen_py_functions.sh (vendors upstream).\n"
)
out.write_text(header + "\n".join(sorted(names)) + "\n")
print(f"  wrote {out}")
PY

echo "[regen] done"
