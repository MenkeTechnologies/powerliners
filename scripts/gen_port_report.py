#!/usr/bin/env python3
"""Regenerate ``docs/port_report.html`` for powerliners.

Walks upstream powerline-status Python (cloned fresh into a tmpdir on
every run) and the Rust port under ``src/ported/``, then writes a styled
HTML report listing per-py-file porting coverage.

Patterned after ``zshrs/scripts/gen_port_report.py`` — same bot/LLM
friendly markers (leading ``<!--PORT-REPORT-SCHEMA-->`` comment, embedded
JSON dataset in a ``<script id="port-report-data" type="application/json">``
block, ``<!-- BEGIN-GROUP py_file=... -->`` markers per group, and
``<!-- SYM ... -->`` trailing comments per row).

Run from the repo root:

    ./scripts/gen_port_report.py

Requires: git, python3 (stdlib only, no third-party deps).
"""
from __future__ import annotations

import datetime
import html
import json
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RS_PORT_DIR = ROOT / "src" / "ported"
OUT_HTML = ROOT / "docs" / "port_report.html"

UPSTREAM_REMOTE = "https://github.com/powerline/powerline"

PY_FN_RE = re.compile(
    r"^\s*(?:async\s+)?def\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(", re.MULTILINE
)
# Match top-level + impl-block Rust fn definitions.
RS_FN_RE = re.compile(
    r"^\s*(?:pub(?:\s*\([^)]+\))?\s+)?(?:unsafe\s+|async\s+|const\s+|extern\s+\"C\"\s+)*"
    r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*[(<]",
    re.MULTILINE,
)


def clone_upstream() -> Path:
    """Clone upstream powerline into a fresh tmpdir and return the path."""
    tmp = Path(tempfile.mkdtemp(prefix="powerliners-portreport-"))
    print(f"[port-report] cloning {UPSTREAM_REMOTE} → {tmp}/powerline …",
          file=sys.stderr)
    subprocess.run(
        ["git", "clone", "--depth", "1", "--quiet", UPSTREAM_REMOTE,
         str(tmp / "powerline")],
        check=True,
    )
    return tmp / "powerline"


def walk_py(root: Path) -> dict[str, list[str]]:
    """Per-py-file index: ``rel_path -> [fn_name, ...]`` in source order."""
    out: dict[str, list[str]] = {}
    pkg = root / "powerline"
    if not pkg.is_dir():
        raise SystemExit(f"upstream layout unexpected: missing {pkg}")
    for py in sorted(pkg.rglob("*.py")):
        rel = py.relative_to(root).as_posix()
        src = py.read_text(errors="replace")
        fns = [m.group(1) for m in PY_FN_RE.finditer(src)]
        out[rel] = fns
    return out


def walk_rs(root: Path) -> dict[str, list[str]]:
    """Per-rs-file index: ``rel_path -> [fn_name, ...]`` in source order."""
    out: dict[str, list[str]] = {}
    if not root.is_dir():
        return out
    for rs in sorted(root.rglob("*.rs")):
        rel = rs.relative_to(ROOT).as_posix()
        src = rs.read_text(errors="replace")
        fns = [m.group(1) for m in RS_FN_RE.finditer(src)]
        out[rel] = fns
    return out


def py_to_rs_path(py_rel: str) -> str:
    """Map an upstream py path to its expected Rust counterpart.

    ``powerline/lib/path.py`` → ``src/ported/lib/path.rs``
    ``powerline/bindings/ipython/since_7.py`` → ``src/ported/bindings/ipython/since_7.rs``
    """
    # Strip the leading `powerline/` package prefix.
    if py_rel.startswith("powerline/"):
        rel = py_rel[len("powerline/"):]
    else:
        rel = py_rel
    return f"src/ported/{rel[:-3]}.rs"


def build_dataset(py_idx: dict[str, list[str]],
                  rs_idx: dict[str, list[str]]) -> list[dict]:
    """Build per-py-file rows with port coverage."""
    rows: list[dict] = []
    for py_rel, py_fns in py_idx.items():
        rs_rel = py_to_rs_path(py_rel)
        rs_fns = rs_idx.get(rs_rel, [])
        ported = [f for f in py_fns if f in rs_fns]
        missing = [f for f in py_fns if f not in rs_fns]
        rs_extra = [f for f in rs_fns if f not in py_fns]
        rows.append({
            "py_file": py_rel,
            "rs_file": rs_rel,
            "rs_exists": rs_rel in rs_idx,
            "py_fn_total": len(py_fns),
            "ported_fns": ported,
            "missing_fns": missing,
            "rs_extra_fns": rs_extra,
            "coverage": (len(ported) / len(py_fns)) if py_fns else 0.0,
        })
    rows.sort(key=lambda r: (r["coverage"] == 0, -r["py_fn_total"], r["py_file"]))
    return rows


SCHEMA = """\
<!--PORT-REPORT-SCHEMA
columns:
  py_file       upstream .py source path (relative to powerline/ clone root)
  rs_file       expected Rust counterpart (relative to powerliners/ repo root)
  rs_exists     true iff a file at rs_file is present in src/ported/
  py_fn_total   count of top-level `def NAME(` in py_file
  ported_count  count of py fns whose name appears in rs_file's `fn NAME` set
  missing_count py_fn_total - ported_count
  rs_extra      Rust fns in rs_file whose name does NOT appear in upstream py
                (these need to be in tests/data/fake_fn_allowlist.txt to pass
                the audit test — see tests/ported_fn_names_match_py.rs)
  coverage      ported_count / py_fn_total (0.0 to 1.0)
-->
"""


def render_html(rows: list[dict],
                py_total_fns: int,
                rs_total_fns: int,
                upstream_sha: str) -> str:
    """Render the full HTML report."""
    today = datetime.date.today().isoformat()
    ported_total = sum(len(r["ported_fns"]) for r in rows)
    overall_cov = (ported_total / py_total_fns) if py_total_fns else 0.0

    # Bot/LLM-friendly embedded JSON. Stays parseable even when the visual
    # UI is collapsed or styled-off.
    data_json = json.dumps(
        {
            "generated": today,
            "upstream": {"remote": UPSTREAM_REMOTE, "sha": upstream_sha},
            "totals": {
                "py_files": len(rows),
                "py_fns": py_total_fns,
                "rs_files_in_ported": len({r["rs_file"] for r in rows if r["rs_exists"]}),
                "rs_fns_in_ported": rs_total_fns,
                "ported_fns": ported_total,
                "coverage": overall_cov,
            },
            "rows": rows,
        },
        indent=2,
    )

    parts: list[str] = []
    parts.append("<!DOCTYPE html>")
    parts.append('<html lang="en">')
    parts.append("<head>")
    parts.append('  <meta charset="utf-8">')
    parts.append('  <meta name="viewport" content="width=device-width, initial-scale=1">')
    parts.append('  <meta name="color-scheme" content="dark light">')
    parts.append(
        '  <meta name="description" content="powerliners port report — per-py-file coverage of the Rust port against upstream powerline-status.">'
    )
    parts.append("  <title>powerliners — Port Report</title>")
    parts.append(
        '  <link rel="preconnect" href="https://fonts.googleapis.com">'
    )
    parts.append(
        '  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>'
    )
    parts.append(
        '  <link href="https://fonts.googleapis.com/css2?family=Orbitron:wght@400;700;900&amp;family=Share+Tech+Mono&amp;display=swap" rel="stylesheet">'
    )
    parts.append('  <link rel="stylesheet" href="hud-static.css">')
    parts.append("  <style>")
    parts.append(
        """    .report-main { max-width: 86rem; padding: 1.5rem 1.5rem 4rem; margin: 0 auto; }
    .stat-row { display: flex; gap: 14px; flex-wrap: wrap; margin: 1rem 0 1.5rem; }
    .stat-card {
      background: var(--bg-card); border: 1px solid var(--cyan); border-radius: 2px;
      padding: 10px 16px; box-shadow: 0 0 12px var(--cyan-dim); min-width: 8rem;
    }
    .stat-card .n {
      font-family: 'Orbitron', sans-serif; font-size: 22px; font-weight: 900;
      color: var(--cyan); letter-spacing: 1.5px;
    }
    .stat-card .l {
      font-size: 9.5px; letter-spacing: 1.5px; color: var(--text-muted);
      text-transform: uppercase; margin-top: 2px;
    }
    table.port {
      width: 100%; border-collapse: collapse; font-size: 11.5px;
      font-family: 'Share Tech Mono', ui-monospace, monospace;
    }
    table.port th {
      background: var(--bg-secondary); color: var(--cyan);
      font-family: 'Orbitron', sans-serif; font-size: 10px; font-weight: 700;
      letter-spacing: 1px; text-transform: uppercase; text-align: left;
      padding: 6px 8px; border: 1px solid var(--border); position: sticky; top: 0;
    }
    table.port td { padding: 5px 8px; border: 1px solid var(--border); vertical-align: top; }
    table.port tr.zero td { background: color-mix(in srgb, #f55 8%, transparent); }
    table.port tr.partial td { background: color-mix(in srgb, var(--accent) 6%, transparent); }
    table.port tr.full td { background: color-mix(in srgb, var(--cyan) 6%, transparent); }
    table.port td.coverage { text-align: right; font-weight: 700; color: var(--cyan); }
    table.port tr.zero td.coverage { color: #f55; }
    table.port tr.full td.coverage { color: #39ff14; }
    table.port td code { background: var(--bg); color: var(--accent-light); padding: 0 4px; }
    .group-title {
      font-family: 'Orbitron', sans-serif; font-size: 12px;
      letter-spacing: 1.5px; text-transform: uppercase;
      color: var(--accent); margin: 1.6rem 0 0.4rem;
    }
    .report-header {
      border-bottom: 1px solid var(--border); padding: 1.2rem 1.5rem;
      background: linear-gradient(180deg, #070714 0%, #0d0d22 60%, var(--bg-secondary) 100%);
    }
    .report-header-inner { max-width: 86rem; margin: 0 auto; }
    .report-brand {
      font-family: 'Orbitron', sans-serif; font-size: 1.5rem; font-weight: 900;
      letter-spacing: 2.5px; text-transform: uppercase;
      background: linear-gradient(90deg, var(--cyan), #fff, var(--accent), var(--cyan));
      background-size: 300% 100%;
      -webkit-background-clip: text; background-clip: text;
      -webkit-text-fill-color: transparent;
      filter: drop-shadow(0 0 12px var(--cyan-glow));
      margin: 0 0 0.4rem;
    }
    .report-crumbs {
      font-family: 'Share Tech Mono', ui-monospace, monospace;
      font-size: 11px; color: var(--text-dim); letter-spacing: 0.04em;
    }
    .report-crumbs a { color: var(--cyan); text-decoration: none; }
    .report-crumbs .sep { color: var(--text-muted); margin: 0 6px; }
    .report-crumbs .current { color: var(--accent); }
"""
    )
    parts.append("  </style>")
    parts.append("</head>")
    parts.append('<body class="crt">')
    parts.append(SCHEMA)
    parts.append('<header class="report-header"><div class="report-header-inner">')
    parts.append('  <h1 class="report-brand">// POWERLINERS — PORT REPORT</h1>')
    parts.append('  <nav class="report-crumbs" aria-label="Breadcrumb">')
    parts.append('    <a href="index.html">Docs</a>')
    parts.append('    <span class="sep">/</span>')
    parts.append('    <a href="report.html">Engineering report</a>')
    parts.append('    <span class="sep">/</span>')
    parts.append('    <span class="current">Port report</span>')
    parts.append('    <span class="sep">/</span>')
    parts.append(
        '    <a href="https://github.com/MenkeTechnologies/powerliners" target="_blank" rel="noopener noreferrer">GitHub</a>'
    )
    parts.append("  </nav>")
    parts.append(
        f'  <p style="margin-top:.5rem;color:var(--text-dim);font-family:\'Share Tech Mono\',ui-monospace,monospace;font-size:11px;letter-spacing:.04em">Generated {today} from upstream <code>{html.escape(upstream_sha)}</code></p>'
    )
    parts.append("</div></header>")
    parts.append('<main class="report-main">')

    parts.append('<div class="stat-row">')
    parts.append(
        f'  <div class="stat-card"><div class="n">{len(rows)}</div><div class="l">upstream py files</div></div>'
    )
    parts.append(
        f'  <div class="stat-card"><div class="n">{py_total_fns}</div><div class="l">upstream py fns</div></div>'
    )
    parts.append(
        f'  <div class="stat-card"><div class="n">{ported_total}</div><div class="l">ported fns</div></div>'
    )
    parts.append(
        f'  <div class="stat-card"><div class="n">{rs_total_fns}</div><div class="l">rust fns total</div></div>'
    )
    parts.append(
        f'  <div class="stat-card"><div class="n">{overall_cov*100:.1f}%</div><div class="l">overall coverage</div></div>'
    )
    parts.append("</div>")

    parts.append('<h2 class="group-title">// PER-FILE COVERAGE</h2>')
    parts.append("<table class=\"port\">")
    parts.append("  <thead><tr>")
    parts.append(
        "    <th>upstream py</th><th>rust port</th><th>py fns</th><th>ported</th><th>missing</th><th>rs extra</th><th class=\"coverage\">cov</th>"
    )
    parts.append("  </tr></thead><tbody>")

    for r in rows:
        cov = r["coverage"]
        klass = "zero" if cov == 0 else ("full" if cov >= 1.0 else "partial")
        parts.append(f'  <!-- BEGIN-GROUP py_file={html.escape(r["py_file"])} -->')
        py_short = r["py_file"].replace("powerline/", "")
        rs_short = r["rs_file"].replace("src/ported/", "")
        rs_exists_marker = "✔" if r["rs_exists"] else "✘"
        ported_str = ", ".join(f"<code>{html.escape(n)}</code>" for n in r["ported_fns"]) or "—"
        missing_str = ", ".join(f"<code>{html.escape(n)}</code>" for n in r["missing_fns"]) or "—"
        extra_str = ", ".join(f"<code>{html.escape(n)}</code>" for n in r["rs_extra_fns"]) or "—"
        parts.append(
            f'  <tr class="{klass}">'
            f'<td><code>{html.escape(py_short)}</code></td>'
            f'<td>{rs_exists_marker} <code>{html.escape(rs_short)}</code></td>'
            f'<td>{r["py_fn_total"]}</td>'
            f'<td>{ported_str}</td>'
            f'<td>{missing_str}</td>'
            f'<td>{extra_str}</td>'
            f'<td class="coverage">{cov*100:.0f}%</td>'
            f'</tr>'
        )
        # SYM comment carries the whole row as key=value pairs for grep-friendly parsing.
        parts.append(
            "  <!-- SYM "
            f'py_file={html.escape(r["py_file"])} '
            f'rs_file={html.escape(r["rs_file"])} '
            f'rs_exists={r["rs_exists"]} '
            f'py_fn_total={r["py_fn_total"]} '
            f'ported_count={len(r["ported_fns"])} '
            f'missing_count={len(r["missing_fns"])} '
            f'rs_extra_count={len(r["rs_extra_fns"])} '
            f"coverage={cov:.4f} -->"
        )
        parts.append(f'  <!-- END-GROUP py_file={html.escape(r["py_file"])} -->')

    parts.append("  </tbody>")
    parts.append("</table>")

    # Embedded JSON for non-HTML consumers.
    parts.append('<script id="port-report-data" type="application/json">')
    parts.append(data_json)
    parts.append("</script>")

    parts.append("</main>")
    parts.append("</body></html>")
    return "\n".join(parts) + "\n"


def get_upstream_sha(upstream: Path) -> str:
    return subprocess.run(
        ["git", "-C", str(upstream), "rev-parse", "--short=12", "HEAD"],
        check=True, capture_output=True, text=True,
    ).stdout.strip()


def main() -> int:
    upstream_dir = clone_upstream()
    try:
        sha = get_upstream_sha(upstream_dir)
        py_idx = walk_py(upstream_dir)
        rs_idx = walk_rs(RS_PORT_DIR)
        py_total = sum(len(v) for v in py_idx.values())
        rs_total = sum(len(v) for v in rs_idx.values())
        rows = build_dataset(py_idx, rs_idx)
        OUT_HTML.parent.mkdir(parents=True, exist_ok=True)
        OUT_HTML.write_text(render_html(rows, py_total, rs_total, sha))
        ported = sum(len(r["ported_fns"]) for r in rows)
        cov = (ported / py_total) * 100 if py_total else 0
        print(
            f"[port-report] upstream={sha} py_files={len(rows)} py_fns={py_total} "
            f"rs_fns={rs_total} ported={ported} coverage={cov:.1f}% → {OUT_HTML.relative_to(ROOT)}",
            file=sys.stderr,
        )
    finally:
        shutil.rmtree(upstream_dir.parent, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
