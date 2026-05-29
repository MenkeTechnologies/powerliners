#!/usr/bin/env python3
# vim:fileencoding=utf-8:noet
"""Scaffold src/ported/ as 1:1 mirror of vendor/powerline/powerline/.

For each upstream .py file, emit a Rust stub at the matching path with:
  - File-top vim modeline (carries upstream convention)
  - //! Module doc-comment citing the upstream Python file
  - For dir/__init__.py: emit src/ported/<dir>/mod.rs with `pub mod` decls
    for each .py sibling in that directory.
  - For file.py: emit src/ported/file.rs as an empty module stub.

The generated stubs contain NO function bodies. Real ports replace each
stub with a faithful 1:1 translation per docs/PORT.md.

This script is idempotent for the stub headers BUT will not overwrite
files that already contain ported function bodies — it detects bodies
by checking for the `Port of` doc-comment marker and skips those files.
"""

import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UPSTREAM = os.path.join(REPO, "vendor", "powerline", "powerline")
DEST = os.path.join(REPO, "src", "ported")

STUB_MARKER = "// !!! POWERLINERS SCAFFOLD STUB !!! Replace with real port."


def upstream_to_rust_path(upstream_rel: str) -> str:
    """powerline/lib/vcs/git.py -> src/ported/lib/vcs/git.rs
       powerline/lib/__init__.py -> src/ported/lib/mod.rs
       powerline/__init__.py -> src/ported/mod.rs
    """
    # Strip leading 'powerline/'
    assert upstream_rel.startswith("powerline/") or upstream_rel == "powerline"
    rel = upstream_rel[len("powerline/"):] if upstream_rel != "powerline" else ""
    if rel.endswith("__init__.py"):
        dir_part = rel[: -len("__init__.py")].rstrip("/")
        if dir_part:
            return os.path.join(DEST, dir_part, "mod.rs")
        return os.path.join(DEST, "mod.rs")
    if rel.endswith(".py"):
        return os.path.join(DEST, rel[:-3] + ".rs")
    raise ValueError(f"unexpected upstream path: {upstream_rel}")


def is_real_port(path: str) -> bool:
    if not os.path.exists(path):
        return False
    with open(path) as fp:
        head = fp.read(4096)
    # Real ports have `/// Port of` doc-comments; stubs don't.
    return "/// Port of" in head and STUB_MARKER not in head


def stub_for_file(upstream_rel: str, submodules: list) -> str:
    """Emit stub Rust source for a given upstream Python file."""
    lines = [
        "// vim:fileencoding=utf-8:noet",
        f"//! Port of `{upstream_rel}`.",
        "//!",
        "//! " + STUB_MARKER,
        "//! See `docs/PORT.md` for the port doctrine.",
        "",
    ]
    for sub in sorted(submodules):
        lines.append(f"pub mod {sub};")
    if submodules:
        lines.append("")
    return "\n".join(lines)


def main():
    # Build the file tree from upstream
    files_by_dir: dict[str, list[str]] = {}
    for dirpath, dirs, files in os.walk(UPSTREAM):
        rel_dir = os.path.relpath(dirpath, os.path.dirname(UPSTREAM))  # 'powerline' or 'powerline/lib' etc.
        for f in files:
            if f.endswith(".py"):
                files_by_dir.setdefault(rel_dir, []).append(f)

    # Compute, for each directory (i.e. each __init__.py mirror), the list of
    # submodules: sibling .py files (minus __init__.py) + child subdirs that
    # contain an __init__.py.
    dirs_with_init = {d for d, fs in files_by_dir.items() if "__init__.py" in fs}

    submodules_for_dir: dict[str, list[str]] = {}
    for d, fs in files_by_dir.items():
        subs = []
        for f in fs:
            if f == "__init__.py":
                continue
            subs.append(f[:-3])
        # child subdirs (one level deep) that have __init__.py
        for child in dirs_with_init:
            parent = os.path.dirname(child)
            if parent == d:
                subs.append(os.path.basename(child))
        submodules_for_dir[d] = subs

    written = 0
    skipped = 0
    for d, fs in files_by_dir.items():
        for f in fs:
            upstream_rel = os.path.join(d, f)  # 'powerline/lib/foo.py'
            dest = upstream_to_rust_path(upstream_rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)

            if is_real_port(dest):
                skipped += 1
                continue

            if f == "__init__.py":
                content = stub_for_file(upstream_rel, submodules_for_dir.get(d, []))
            else:
                content = stub_for_file(upstream_rel, [])

            with open(dest, "w") as fp:
                fp.write(content + "\n")
            written += 1

    print(f"scaffold: wrote {written} stub files, skipped {skipped} real ports", file=sys.stderr)


if __name__ == "__main__":
    main()
