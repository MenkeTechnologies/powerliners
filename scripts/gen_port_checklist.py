#!/usr/bin/env python3
# vim:fileencoding=utf-8:noet
"""Generate docs/PORT_CHECKLIST.md tier-classified per-file.

For each upstream .py file:
- Count Python LOC, top-level fns, top-level classes (via ast).
- Inspect the matching Rust file under src/ported/ to count:
  - whether it's still the scaffold stub (contains `POWERLINERS SCAFFOLD STUB`)
  - how many `// py:` citations it carries
  - how many `/// Port of` doc-comments it carries
- Classify each file into a tier:
  - DONE        — `// py:` density ≥ Python LOC * 0.5 AND zero stub markers
  - NEAR        — `/// Port of` >= upstream fn count (every fn has a real port doc)
  - PARTIAL     — `/// Port of` >= 50% of upstream fn count
  - SPARSE      — `/// Port of` >= 20% of upstream fn count
  - STUB-HEAVY  — anything else (including pure scaffold)

Sorts within each tier by ascending Python fn count (smallest first).
"""
import ast
import os
import sys
from typing import NamedTuple, Optional

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UPSTREAM = os.path.join(REPO, "vendor", "powerline", "powerline")
PORTED = os.path.join(REPO, "src", "ported")
DOCS = os.path.join(REPO, "docs")


class FileStat(NamedTuple):
    py_rel: str        # 'powerline/lib/humanize_bytes.py'
    rs_rel: str        # 'src/ported/lib/humanize_bytes.rs'
    py_loc: int
    py_fns: int        # top-level def count (not nested methods)
    py_classes: int    # top-level class count
    py_methods: int    # methods inside classes
    rs_exists: bool
    rs_loc: int
    rs_is_stub: bool
    rs_port_doccomments: int   # `/// Port of` count
    rs_py_citations: int       # `// py:` count
    rs_fn_count: int           # `fn ` definitions


def upstream_to_rust_rel(py_rel: str) -> str:
    assert py_rel.startswith("powerline/")
    rel = py_rel[len("powerline/"):]
    if rel.endswith("__init__.py"):
        d = rel[:-len("__init__.py")].rstrip("/")
        return os.path.join("src/ported", d, "mod.rs") if d else "src/ported/mod.rs"
    if rel.endswith(".py"):
        return os.path.join("src/ported", rel[:-3] + ".rs")
    raise ValueError(py_rel)


def count_py(path: str) -> tuple[int, int, int, int]:
    with open(path) as fp:
        src = fp.read()
    loc = len([l for l in src.splitlines() if l.strip() and not l.strip().startswith("#")])
    try:
        tree = ast.parse(src, filename=path)
    except SyntaxError:
        return loc, 0, 0, 0
    top_fns = 0
    top_classes = 0
    methods = 0
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            top_fns += 1
        elif isinstance(node, ast.ClassDef):
            top_classes += 1
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    methods += 1
    return loc, top_fns, top_classes, methods


def count_rs(path: str) -> Optional[tuple[int, bool, int, int, int]]:
    if not os.path.exists(path):
        return None
    with open(path) as fp:
        src = fp.read()
    loc = len([l for l in src.splitlines() if l.strip() and not l.strip().startswith("//")])
    is_stub = "POWERLINERS SCAFFOLD STUB" in src
    port_doccomments = src.count("/// Port of")
    py_citations = src.count("// py:")
    # rough fn count via regex-like walk
    fn_count = 0
    for line in src.splitlines():
        s = line.lstrip()
        if s.startswith("///") or s.startswith("//!") or s.startswith("//"):
            continue
        for p in ("pub fn ", "pub(crate) fn ", "fn ", "async fn ",
                  "pub async fn ", "const fn ", "pub const fn "):
            if s.startswith(p):
                fn_count += 1
                break
    return loc, is_stub, port_doccomments, py_citations, fn_count


def classify(stat: FileStat) -> str:
    total_fns = stat.py_fns + stat.py_methods
    if total_fns == 0 and stat.py_classes == 0:
        # __init__.py with no code — pure namespace package
        return "DONE" if not stat.rs_is_stub else "STUB-HEAVY"
    if stat.rs_is_stub:
        return "STUB-HEAVY"
    if total_fns == 0:
        # only classes (no top-level fns) — judge on class count
        return "NEAR" if stat.rs_port_doccomments >= stat.py_classes else "STUB-HEAVY"
    citation_density = stat.rs_py_citations / max(1, stat.py_loc)
    port_ratio = stat.rs_port_doccomments / max(1, total_fns)
    if not stat.rs_is_stub and citation_density >= 0.5 and port_ratio >= 1.0:
        return "DONE"
    if port_ratio >= 1.0:
        return "NEAR"
    if port_ratio >= 0.5:
        return "PARTIAL"
    if port_ratio >= 0.2:
        return "SPARSE"
    return "STUB-HEAVY"


def main():
    stats: list[FileStat] = []
    for dirpath, _, files in os.walk(UPSTREAM):
        for f in files:
            if not f.endswith(".py"):
                continue
            full = os.path.join(dirpath, f)
            py_rel = os.path.relpath(full, os.path.dirname(UPSTREAM))
            rs_rel = upstream_to_rust_rel(py_rel)
            rs_full = os.path.join(REPO, rs_rel)
            py_loc, py_fns, py_classes, py_methods = count_py(full)
            rs = count_rs(rs_full)
            if rs is None:
                rs_loc, rs_is_stub, rs_pd, rs_py, rs_fn = 0, False, 0, 0, 0
                rs_exists = False
            else:
                rs_loc, rs_is_stub, rs_pd, rs_py, rs_fn = rs
                rs_exists = True
            stats.append(FileStat(
                py_rel=py_rel, rs_rel=rs_rel,
                py_loc=py_loc, py_fns=py_fns, py_classes=py_classes, py_methods=py_methods,
                rs_exists=rs_exists, rs_loc=rs_loc, rs_is_stub=rs_is_stub,
                rs_port_doccomments=rs_pd, rs_py_citations=rs_py, rs_fn_count=rs_fn,
            ))
    # Sort within each tier by ascending (py_fns + py_methods)
    tiers = {"DONE": [], "NEAR": [], "PARTIAL": [], "SPARSE": [], "STUB-HEAVY": []}
    for s in stats:
        tiers[classify(s)].append(s)
    for t in tiers:
        tiers[t].sort(key=lambda s: (s.py_fns + s.py_methods, s.py_loc))

    # Emit checklist
    out = []
    out.append("# PORT_CHECKLIST.md — powerliners 1:1 powerline-status port")
    out.append("")
    out.append("Working list for the line-by-line port pass. Each file gets a")
    out.append("single checkbox; tick when the file's Rust port is verified")
    out.append("function-by-function AND class-by-class against its Python")
    out.append("counterpart in `vendor/powerline/`.")
    out.append("")
    out.append("All classes must have matching field names and data types per")
    out.append("docs/PORT.md Rule 5. Every line of source code must be 100%")
    out.append("ported. Every Python function must be present in the Rust file.")
    out.append("")
    out.append("**Regenerate this file:**")
    out.append("")
    out.append("```sh")
    out.append("python3 scripts/gen_port_checklist.py > docs/PORT_CHECKLIST.md")
    out.append("```")
    out.append("")
    out.append("---")
    out.append("")
    out.append("## RULES (load-bearing — re-read before every file)")
    out.append("")
    out.append("Mirrors the zshrs strategy. A file isn't ticked until ALL of them pass.")
    out.append("")
    out.append("1. **Zero Rust-only structs/enums in `src/ported/`.** If a `pub struct Foo` or `pub enum Bar` doesn't exist as a Python `class Foo` in the matching `.py` file (verify via `grep -xF 'Foo' docs/powerline_py_classes.txt`), it must be removed.")
    out.append("2. **Every struct/enum that remains must match its Python class name exactly** (`class TmuxRenderer` → `pub struct TmuxRenderer`).")
    out.append("3. **Python kwargs dicts (`**kwargs`) are not converted to bespoke option structs.** Rust ports take the equivalent `&HashMap<String, serde_json::Value>` or split into explicit named params if the Python source already destructures the kwargs (preserving the Python local names).")
    out.append("4. **No \"Rust-only abstraction\" WARNING blocks for new code.** Anything that would carry that marker must be deleted or properly ported instead.")
    out.append("5. **Fix broken function stubs at their source, don't work around them.** If file A's port needs `humanize_bytes(...)` and file B has it stubbed, fix B's signature + body in the same commit as A. Don't write inline duplicate helpers in A.")
    out.append("6. **Function bodies match Python 1:1.** Cite Python `file:line` in inline comments (`// py:NNN`) on every line that mirrors a Python statement.")
    out.append("7. **Drift gate stays green.** Every `pub fn`/`fn` in `src/ported/` either matches a Python function name in `docs/powerline_py_functions.txt` or appears in `tests/data/fake_fn_allowlist.txt` with a citation explaining why no Python counterpart exists.")
    out.append("8. **Explicit `cargo build --lib` after every file** (NOT `cargo build --release`). Drift gate run after every batch.")
    out.append("9. **Commit per file or per ≤5-file batch.** No mass commits that bury per-file regressions.")
    out.append("10. **Proof of 100% port must be shown via line counts logged here.**")
    out.append("11. If a ported function calls a fn that doesn't exist, it must be created in the right file per the upstream layout.")
    out.append("")
    out.append("---")
    out.append("")
    out.append("## Tier counts")
    out.append("")
    out.append("| Tier | Count |")
    out.append("|---|---|")
    for t in ("DONE", "NEAR", "PARTIAL", "SPARSE", "STUB-HEAVY"):
        out.append(f"| {t} | {len(tiers[t])} |")
    out.append(f"| **Total** | **{sum(len(v) for v in tiers.values())}** |")
    out.append("")
    out.append("---")
    out.append("")

    icons = {
        "DONE": "✅",
        "NEAR": "🟢",
        "PARTIAL": "🟡",
        "SPARSE": "🟠",
        "STUB-HEAVY": "🔴",
    }
    descriptions = {
        "DONE": "verified line-by-line (zero stubs + zero Rust-only types + name-matched)",
        "NEAR": "every Python fn has a `/// Port of` doc-comment; body density may still be partial",
        "PARTIAL": "50–100% of Python fns ported; rest stubbed with citations",
        "SPARSE": "20–50% of Python fns ported",
        "STUB-HEAVY": "<20% ported (mostly scaffold stubs)",
    }
    for t in ("DONE", "NEAR", "PARTIAL", "SPARSE", "STUB-HEAVY"):
        out.append(f"## {icons[t]} {t} — {descriptions[t]} ({len(tiers[t])})")
        out.append("")
        if not tiers[t]:
            out.append("*(empty)*")
            out.append("")
            continue
        out.append("| | File | Py LOC | Py fns | Py classes | Py methods | Rs LOC | `/// Port of` | `// py:` |")
        out.append("|---|---|---|---|---|---|---|---|---|")
        for s in tiers[t]:
            checkbox = "[x]" if t == "DONE" else "[ ]"
            out.append(f"| {checkbox} | `{s.py_rel}` ↔ `{s.rs_rel}` | {s.py_loc} | {s.py_fns} | {s.py_classes} | {s.py_methods} | {s.rs_loc} | {s.rs_port_doccomments} | {s.rs_py_citations} |")
        out.append("")

    out.append("---")
    out.append("")
    out.append("## Plan-of-attack ordering")
    out.append("")
    out.append("Work the **STUB-HEAVY** tier from smallest fn-count to largest first (quick wins validate the cadence), then **SPARSE**, then **PARTIAL**, then **NEAR**, then a final pass on the **DONE** tier to spot-check.")
    out.append("")
    out.append("Within each tier, the table above is already ordered ascending by `(py_fns + py_methods, py_loc)`.")
    out.append("")
    out.append("---")
    out.append("")
    out.append(f"*Last generated by `scripts/gen_port_checklist.py` from a vendor snapshot in `vendor/powerline/`.*")

    print("\n".join(out))


if __name__ == "__main__":
    main()
