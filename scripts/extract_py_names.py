#!/usr/bin/env python3
# vim:fileencoding=utf-8:noet
"""Regenerate docs/powerline_py_functions.txt and friends from vendor/powerline/."""
import ast, os, sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.join(REPO, "vendor", "powerline", "powerline")
DOCS = os.path.join(REPO, "docs")

rows = []
for dirpath, _, files in os.walk(ROOT):
    for f in files:
        if not f.endswith(".py"): continue
        full = os.path.join(dirpath, f)
        rel = os.path.relpath(full, os.path.join(REPO, "vendor", "powerline"))
        try:
            src = open(full).read()
            tree = ast.parse(src, filename=full)
        except Exception as e:
            print(f"# parse-error {rel}: {e}", file=sys.stderr); continue
        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                rows.append((node.name, rel, node.lineno, "fn"))
            elif isinstance(node, ast.ClassDef):
                rows.append((node.name, rel, node.lineno, "class"))
rows.sort()

with open(os.path.join(DOCS, "powerline_py_functions_with_locations.txt"), "w") as fp:
    for n, r, l, k in rows:
        fp.write(f"{n}\t{r}:{l}\t{k}\n")

with open(os.path.join(DOCS, "powerline_py_functions.txt"), "w") as fp:
    seen = set()
    for n, _, _, k in rows:
        if k == "fn" and n not in seen:
            seen.add(n); fp.write(n + "\n")

with open(os.path.join(DOCS, "powerline_py_classes.txt"), "w") as fp:
    seen = set()
    for n, _, _, k in rows:
        if k == "class" and n not in seen:
            seen.add(n); fp.write(n + "\n")

print(f"rows: {len(rows)}", file=sys.stderr)
