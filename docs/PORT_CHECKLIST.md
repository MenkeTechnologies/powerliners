# PORT_CHECKLIST.md — powerliners 1:1 powerline-status port

Working list for the line-by-line port pass. Each file gets a
single checkbox; tick when the file's Rust port is verified
function-by-function AND class-by-class against its Python
counterpart in `vendor/powerline/`.

All classes must have matching field names and data types per
docs/PORT.md Rule 5. Every line of source code must be 100%
ported. Every Python function must be present in the Rust file.

**Regenerate this file:**

```sh
python3 scripts/gen_port_checklist.py > docs/PORT_CHECKLIST.md
```

---

## RULES (load-bearing — re-read before every file)

Mirrors the zshrs strategy. A file isn't ticked until ALL of them pass.

1. **Zero Rust-only structs/enums in `src/ported/`.** If a `pub struct Foo` or `pub enum Bar` doesn't exist as a Python `class Foo` in the matching `.py` file (verify via `grep -xF 'Foo' docs/powerline_py_classes.txt`), it must be removed.
2. **Every struct/enum that remains must match its Python class name exactly** (`class TmuxRenderer` → `pub struct TmuxRenderer`).
3. **Python kwargs dicts (`**kwargs`) are not converted to bespoke option structs.** Rust ports take the equivalent `&HashMap<String, serde_json::Value>` or split into explicit named params if the Python source already destructures the kwargs (preserving the Python local names).
4. **No "Rust-only abstraction" WARNING blocks for new code.** Anything that would carry that marker must be deleted or properly ported instead.
5. **Fix broken function stubs at their source, don't work around them.** If file A's port needs `humanize_bytes(...)` and file B has it stubbed, fix B's signature + body in the same commit as A. Don't write inline duplicate helpers in A.
6. **Function bodies match Python 1:1.** Cite Python `file:line` in inline comments (`// py:NNN`) on every line that mirrors a Python statement.
7. **Drift gate stays green.** Every `pub fn`/`fn` in `src/ported/` either matches a Python function name in `docs/powerline_py_functions.txt` or appears in `tests/data/fake_fn_allowlist.txt` with a citation explaining why no Python counterpart exists.
8. **Explicit `cargo build --lib` after every file** (NOT `cargo build --release`). Drift gate run after every batch.
9. **Commit per file or per ≤5-file batch.** No mass commits that bury per-file regressions.
10. **Proof of 100% port must be shown via line counts logged here.**
11. If a ported function calls a fn that doesn't exist, it must be created in the right file per the upstream layout.

---

## Tier counts

| Tier | Count |
|---|---|
| DONE | 134 |
| NEAR | 3 |
| PARTIAL | 0 |
| SPARSE | 0 |
| STUB-HEAVY | 0 |
| **Total** | **137** |

---

## ✅ DONE — verified line-by-line (zero stubs + zero Rust-only types + name-matched) (134)

| | File | Py LOC | Py fns | Py classes | Py methods | Rs LOC | `/// Port of` | `// py:` |
|---|---|---|---|---|---|---|---|---|
| [x] | `powerline/renderers/__init__.py` ↔ `src/ported/renderers/mod.rs` | 0 | 0 | 0 | 0 | 8 | 0 | 0 |
| [x] | `powerline/listers/__init__.py` ↔ `src/ported/listers/mod.rs` | 0 | 0 | 0 | 0 | 3 | 0 | 0 |
| [x] | `powerline/commands/__init__.py` ↔ `src/ported/commands/mod.rs` | 0 | 0 | 0 | 0 | 5 | 0 | 0 |
| [x] | `powerline/bindings/__init__.py` ↔ `src/ported/bindings/mod.rs` | 0 | 0 | 0 | 0 | 8 | 0 | 0 |
| [x] | `powerline/bindings/ipython/__init__.py` ↔ `src/ported/bindings/ipython/mod.rs` | 0 | 0 | 0 | 0 | 4 | 0 | 0 |
| [x] | `powerline/bindings/qtile/__init__.py` ↔ `src/ported/bindings/qtile/mod.rs` | 0 | 0 | 0 | 0 | 1 | 0 | 0 |
| [x] | `powerline/segments/common/__init__.py` ↔ `src/ported/segments/common/mod.rs` | 0 | 0 | 0 | 0 | 9 | 0 | 0 |
| [x] | `powerline/selectors/__init__.py` ↔ `src/ported/selectors/mod.rs` | 0 | 0 | 0 | 0 | 1 | 0 | 0 |
| [x] | `powerline/renderers/shell/rcsh.py` ↔ `src/ported/renderers/shell/rcsh.rs` | 3 | 0 | 0 | 0 | 12 | 1 | 2 |
| [x] | `powerline/matchers/__init__.py` ↔ `src/ported/matchers/mod.rs` | 3 | 0 | 0 | 0 | 1 | 0 | 0 |
| [x] | `powerline/matchers/vim/plugin/__init__.py` ↔ `src/ported/matchers/vim/plugin/mod.rs` | 3 | 0 | 0 | 0 | 3 | 0 | 0 |
| [x] | `powerline/segments/vim/plugin/__init__.py` ↔ `src/ported/segments/vim/plugin/mod.rs` | 3 | 0 | 0 | 0 | 7 | 0 | 0 |
| [x] | `powerline/bindings/pdb/__main__.py` ↔ `src/ported/bindings/pdb/__main__.rs` | 4 | 0 | 0 | 0 | 0 | 0 | 0 |
| [x] | `powerline/config.py` ↔ `src/ported/config.rs` | 6 | 0 | 0 | 0 | 49 | 4 | 6 |
| [x] | `powerline/bindings/lemonbar/powerline-lemonbar.py` ↔ `src/ported/bindings/lemonbar/powerline-lemonbar.rs` | 45 | 0 | 0 | 0 | 116 | 3 | 21 |
| [x] | `powerline/bindings/bar/powerline-bar.py` ↔ `src/ported/bindings/bar/powerline-bar.rs` | 47 | 0 | 0 | 0 | 62 | 3 | 25 |
| [x] | `powerline/lib/monotonic.py` ↔ `src/ported/lib/monotonic.rs` | 72 | 0 | 0 | 0 | 50 | 2 | 4 |
| [x] | `powerline/segments/ipython.py` ↔ `src/ported/segments/ipython.rs` | 5 | 1 | 0 | 0 | 27 | 1 | 3 |
| [x] | `powerline/selectors/vim.py` ↔ `src/ported/selectors/vim.rs` | 6 | 1 | 0 | 0 | 13 | 1 | 3 |
| [x] | `powerline/matchers/vim/plugin/nerdtree.py` ↔ `src/ported/matchers/vim/plugin/nerdtree.rs` | 8 | 1 | 0 | 0 | 49 | 2 | 7 |
| [x] | `powerline/matchers/vim/plugin/commandt.py` ↔ `src/ported/matchers/vim/plugin/commandt.rs` | 9 | 1 | 0 | 0 | 42 | 1 | 5 |
| [x] | `powerline/version.py` ↔ `src/ported/version.rs` | 10 | 1 | 0 | 0 | 34 | 2 | 10 |
| [x] | `powerline/renderers/shell/ksh.py` ↔ `src/ported/renderers/shell/ksh.rs` | 10 | 0 | 1 | 1 | 48 | 7 | 8 |
| [x] | `powerline/lint/selfcheck.py` ↔ `src/ported/lint/selfcheck.rs` | 12 | 1 | 0 | 0 | 34 | 1 | 9 |
| [x] | `powerline/bindings/awesome/powerline-awesome.py` ↔ `src/ported/bindings/awesome/powerline-awesome.rs` | 12 | 1 | 0 | 0 | 17 | 1 | 7 |
| [x] | `powerline/lib/url.py` ↔ `src/ported/lib/url.rs` | 13 | 1 | 0 | 0 | 64 | 2 | 9 |
| [x] | `powerline/lint/markedjson/__init__.py` ↔ `src/ported/lint/markedjson/mod.rs` | 14 | 1 | 0 | 0 | 60 | 1 | 9 |
| [x] | `powerline/segments/tmux.py` ↔ `src/ported/segments/tmux.rs` | 15 | 1 | 0 | 0 | 34 | 1 | 9 |
| [x] | `powerline/commands/lint.py` ↔ `src/ported/commands/lint.rs` | 17 | 1 | 0 | 0 | 65 | 1 | 17 |
| [x] | `powerline/lib/humanize_bytes.py` ↔ `src/ported/lib/humanize_bytes.rs` | 18 | 1 | 0 | 0 | 57 | 2 | 12 |
| [x] | `powerline/commands/daemon.py` ↔ `src/ported/commands/daemon.rs` | 18 | 1 | 0 | 0 | 72 | 1 | 9 |
| [x] | `powerline/segments/vim/plugin/nerdtree.py` ↔ `src/ported/segments/vim/plugin/nerdtree.rs` | 19 | 1 | 0 | 0 | 20 | 1 | 14 |
| [x] | `powerline/renderers/shell/tcsh.py` ↔ `src/ported/renderers/shell/tcsh.rs` | 20 | 0 | 1 | 1 | 47 | 4 | 21 |
| [x] | `powerline/segments/vim/plugin/capslock.py` ↔ `src/ported/segments/vim/plugin/capslock.rs` | 21 | 1 | 0 | 0 | 21 | 1 | 13 |
| [x] | `powerline/listers/pdb.py` ↔ `src/ported/listers/pdb.rs` | 30 | 1 | 0 | 0 | 96 | 1 | 24 |
| [x] | `powerline/commands/lemonbar.py` ↔ `src/ported/commands/lemonbar.rs` | 30 | 1 | 0 | 0 | 70 | 1 | 30 |
| [x] | `powerline/segments/vim/plugin/syntastic.py` ↔ `src/ported/segments/vim/plugin/syntastic.rs` | 35 | 1 | 0 | 0 | 16 | 1 | 26 |
| [x] | `powerline/segments/vim/plugin/tagbar.py` ↔ `src/ported/segments/vim/plugin/tagbar.rs` | 40 | 1 | 0 | 0 | 16 | 1 | 28 |
| [x] | `powerline/segments/vim/plugin/ale.py` ↔ `src/ported/segments/vim/plugin/ale.rs` | 44 | 1 | 0 | 0 | 22 | 1 | 35 |
| [x] | `powerline/lib/debug.py` ↔ `src/ported/lib/debug.rs` | 75 | 1 | 0 | 0 | 46 | 3 | 79 |
| [x] | `powerline/renderers/shell/bash.py` ↔ `src/ported/renderers/shell/bash.rs` | 76 | 0 | 1 | 1 | 39 | 6 | 80 |
| [x] | `powerline/matchers/vim/plugin/gundo.py` ↔ `src/ported/matchers/vim/plugin/gundo.rs` | 9 | 2 | 0 | 0 | 48 | 2 | 7 |
| [x] | `powerline/lib/path.py` ↔ `src/ported/lib/path.rs` | 12 | 2 | 0 | 0 | 65 | 2 | 6 |
| [x] | `powerline/lemonbar.py` ↔ `src/ported/lemonbar.rs` | 14 | 0 | 1 | 2 | 70 | 5 | 14 |
| [x] | `powerline/lint/markedjson/loader.py` ↔ `src/ported/lint/markedjson/loader.rs` | 20 | 0 | 1 | 2 | 62 | 4 | 18 |
| [x] | `powerline/lib/__init__.py` ↔ `src/ported/lib/mod.rs` | 22 | 2 | 0 | 0 | 165 | 2 | 20 |
| [x] | `powerline/renderers/i3bar.py` ↔ `src/ported/renderers/i3bar.rs` | 24 | 0 | 1 | 2 | 63 | 4 | 21 |
| [x] | `powerline/renderers/pango_markup.py` ↔ `src/ported/renderers/pango_markup.rs` | 28 | 0 | 1 | 2 | 103 | 5 | 26 |
| [x] | `powerline/segments/vim/plugin/coc.py` ↔ `src/ported/segments/vim/plugin/coc.rs` | 43 | 2 | 0 | 0 | 76 | 2 | 39 |
| [x] | `powerline/listers/i3wm.py` ↔ `src/ported/listers/i3wm.rs` | 51 | 2 | 0 | 0 | 170 | 2 | 39 |
| [x] | `powerline/lib/watcher/__init__.py` ↔ `src/ported/lib/watcher/mod.rs` | 60 | 2 | 0 | 0 | 66 | 2 | 35 |
| [x] | `powerline/lint/inspect.py` ↔ `src/ported/lint/inspect.rs` | 76 | 2 | 0 | 0 | 67 | 3 | 59 |
| [x] | `powerline/segments/common/time.py` ↔ `src/ported/segments/common/time.rs` | 96 | 2 | 0 | 0 | 311 | 4 | 65 |
| [x] | `powerline/bindings/pdb/__init__.py` ↔ `src/ported/bindings/pdb/mod.rs` | 139 | 2 | 0 | 0 | 40 | 3 | 88 |
| [x] | `powerline/segments/common/sys.py` ↔ `src/ported/segments/common/sys.rs` | 148 | 2 | 0 | 0 | 311 | 5 | 89 |
| [x] | `powerline/matchers/vim/__init__.py` ↔ `src/ported/matchers/vim/mod.rs` | 10 | 3 | 0 | 0 | 69 | 3 | 7 |
| [x] | `powerline/renderers/ipython/__init__.py` ↔ `src/ported/renderers/ipython/mod.rs` | 28 | 0 | 1 | 3 | 152 | 4 | 16 |
| [x] | `powerline/lib/memoize.py` ↔ `src/ported/lib/memoize.rs` | 30 | 1 | 1 | 2 | 129 | 5 | 28 |
| [x] | `powerline/renderers/pdb.py` ↔ `src/ported/renderers/pdb.rs` | 36 | 0 | 1 | 3 | 152 | 6 | 28 |
| [x] | `powerline/segments/__init__.py` ↔ `src/ported/segments/mod.rs` | 46 | 1 | 1 | 2 | 36 | 4 | 25 |
| [x] | `powerline/bindings/wm/__init__.py` ↔ `src/ported/bindings/wm/mod.rs` | 47 | 3 | 0 | 0 | 110 | 6 | 31 |
| [x] | `powerline/bindings/ipython/since_7.py` ↔ `src/ported/bindings/ipython/since_7.rs` | 60 | 0 | 2 | 3 | 304 | 7 | 48 |
| [x] | `powerline/bindings/ipython/since_5.py` ↔ `src/ported/bindings/ipython/since_5.rs` | 62 | 0 | 2 | 3 | 341 | 6 | 53 |
| [x] | `powerline/segments/vim/plugin/commandt.py` ↔ `src/ported/segments/vim/plugin/commandt.rs` | 77 | 3 | 0 | 0 | 71 | 4 | 63 |
| [x] | `powerline/segments/common/mail.py` ↔ `src/ported/segments/common/mail.rs` | 80 | 0 | 1 | 3 | 209 | 6 | 50 |
| [x] | `powerline/lib/shell.py` ↔ `src/ported/lib/shell.rs` | 92 | 3 | 0 | 0 | 135 | 5 | 51 |
| [x] | `powerline/segments/common/bat.py` ↔ `src/ported/segments/common/bat.rs` | 265 | 3 | 0 | 0 | 403 | 7 | 247 |
| [x] | `powerline/lint/markedjson/nodes.py` ↔ `src/ported/lint/markedjson/nodes.rs` | 30 | 0 | 5 | 4 | 157 | 9 | 30 |
| [x] | `powerline/renderers/lemonbar.py` ↔ `src/ported/renderers/lemonbar.rs` | 43 | 0 | 1 | 4 | 107 | 7 | 39 |
| [x] | `powerline/bindings/wm/awesome.py` ↔ `src/ported/bindings/wm/awesome.rs` | 44 | 2 | 1 | 2 | 80 | 5 | 39 |
| [x] | `powerline/lint/markedjson/tokens.py` ↔ `src/ported/lint/markedjson/tokens.rs` | 46 | 0 | 11 | 4 | 198 | 15 | 46 |
| [x] | `powerline/lib/overrides.py` ↔ `src/ported/lib/overrides.rs` | 62 | 4 | 0 | 0 | 161 | 5 | 46 |
| [x] | `powerline/renderers/tmux.py` ↔ `src/ported/renderers/tmux.rs` | 68 | 1 | 1 | 3 | 328 | 6 | 71 |
| [x] | `powerline/segments/common/vcs.py` ↔ `src/ported/segments/common/vcs.rs` | 73 | 0 | 2 | 4 | 233 | 6 | 50 |
| [x] | `powerline/listers/vim.py` ↔ `src/ported/listers/vim.rs` | 91 | 4 | 0 | 0 | 265 | 5 | 77 |
| [x] | `powerline/commands/main.py` ↔ `src/ported/commands/main.rs` | 170 | 4 | 0 | 0 | 421 | 5 | 160 |
| [x] | `powerline/segments/common/wthr.py` ↔ `src/ported/segments/common/wthr.rs` | 198 | 0 | 1 | 4 | 428 | 10 | 117 |
| [x] | `powerline/pdb.py` ↔ `src/ported/pdb.rs` | 38 | 0 | 1 | 5 | 207 | 6 | 36 |
| [x] | `powerline/lint/imp.py` ↔ `src/ported/lint/imp.rs` | 41 | 2 | 1 | 3 | 266 | 6 | 42 |
| [x] | `powerline/segments/pdb.py` ↔ `src/ported/segments/pdb.rs` | 43 | 5 | 0 | 0 | 101 | 5 | 30 |
| [x] | `powerline/commands/config.py` ↔ `src/ported/commands/config.rs` | 90 | 1 | 2 | 4 | 258 | 7 | 85 |
| [x] | `powerline/lib/vcs/git.py` ↔ `src/ported/lib/vcs/git.rs` | 167 | 2 | 1 | 3 | 376 | 14 | 108 |
| [x] | `powerline/shell.py` ↔ `src/ported/shell.rs` | 28 | 0 | 1 | 6 | 253 | 7 | 18 |
| [x] | `powerline/lib/watcher/stat.py` ↔ `src/ported/lib/watcher/stat.rs` | 33 | 0 | 1 | 6 | 137 | 7 | 28 |
| [x] | `powerline/renderers/ipython/pre_5.py` ↔ `src/ported/renderers/ipython/pre_5.rs` | 38 | 0 | 4 | 6 | 156 | 10 | 27 |
| [x] | `powerline/lib/vcs/mercurial.py` ↔ `src/ported/lib/vcs/mercurial.rs` | 68 | 1 | 1 | 5 | 227 | 9 | 59 |
| [x] | `powerline/bindings/ipython/post_0_11.py` ↔ `src/ported/bindings/ipython/post_0_11.rs` | 102 | 2 | 2 | 4 | 398 | 14 | 76 |
| [x] | `powerline/lint/markedjson/resolver.py` ↔ `src/ported/lint/markedjson/resolver.rs` | 110 | 0 | 3 | 6 | 252 | 13 | 97 |
| [x] | `powerline/colorscheme.py` ↔ `src/ported/colorscheme.rs` | 120 | 2 | 1 | 4 | 369 | 12 | 76 |
| [x] | `powerline/segments/shell.py` ↔ `src/ported/segments/shell.rs` | 157 | 5 | 1 | 1 | 389 | 7 | 100 |
| [x] | `powerline/bindings/qtile/widget.py` ↔ `src/ported/bindings/qtile/widget.rs` | 43 | 0 | 2 | 7 | 227 | 10 | 32 |
| [x] | `powerline/lint/context.py` ↔ `src/ported/lint/context.rs` | 51 | 1 | 2 | 6 | 168 | 9 | 41 |
| [x] | `powerline/bindings/tmux/__init__.py` ↔ `src/ported/bindings/tmux/mod.rs` | 57 | 7 | 0 | 0 | 111 | 11 | 32 |
| [x] | `powerline/lib/dict.py` ↔ `src/ported/lib/dict.rs` | 68 | 7 | 0 | 0 | 180 | 8 | 39 |
| [x] | `powerline/lib/watcher/tree.py` ↔ `src/ported/lib/watcher/tree.rs` | 76 | 0 | 2 | 7 | 114 | 9 | 50 |
| [x] | `powerline/lib/vcs/bzr.py` ↔ `src/ported/lib/vcs/bzr.rs` | 86 | 1 | 2 | 6 | 290 | 10 | 75 |
| [x] | `powerline/lib/encoding.py` ↔ `src/ported/lib/encoding.rs` | 97 | 7 | 0 | 0 | 98 | 7 | 73 |
| [x] | `powerline/lib/inotify.py` ↔ `src/ported/lib/inotify.rs` | 132 | 1 | 2 | 6 | 44 | 7 | 107 |
| [x] | `powerline/segments/common/env.py` ↔ `src/ported/segments/common/env.rs` | 166 | 3 | 1 | 4 | 441 | 10 | 102 |
| [x] | `powerline/lint/markedjson/events.py` ↔ `src/ported/lint/markedjson/events.rs` | 65 | 0 | 14 | 8 | 267 | 17 | 67 |
| [x] | `powerline/renderers/ipython/since_7.py` ↔ `src/ported/renderers/ipython/since_7.rs` | 71 | 0 | 2 | 8 | 397 | 13 | 46 |
| [x] | `powerline/renderers/shell/__init__.py` ↔ `src/ported/renderers/shell/mod.rs` | 157 | 1 | 2 | 7 | 509 | 10 | 95 |
| [x] | `powerline/lib/unicode.py` ↔ `src/ported/lib/unicode.rs` | 221 | 8 | 1 | 0 | 327 | 15 | 138 |
| [x] | `powerline/segments/common/net.py` ↔ `src/ported/segments/common/net.rs` | 254 | 2 | 2 | 6 | 770 | 21 | 147 |
| [x] | `powerline/lint/__init__.py` ↔ `src/ported/lint/mod.rs` | 573 | 8 | 0 | 0 | 1430 | 54 | 430 |
| [x] | `powerline/lint/markedjson/composer.py` ↔ `src/ported/lint/markedjson/composer.rs` | 80 | 0 | 2 | 9 | 450 | 15 | 52 |
| [x] | `powerline/lint/markedjson/reader.py` ↔ `src/ported/lint/markedjson/reader.rs` | 107 | 0 | 2 | 9 | 172 | 11 | 111 |
| [x] | `powerline/renderers/vim.py` ↔ `src/ported/renderers/vim.rs` | 152 | 0 | 1 | 9 | 596 | 13 | 104 |
| [x] | `powerline/segment.py` ↔ `src/ported/segment.rs` | 399 | 9 | 0 | 0 | 1140 | 17 | 258 |
| [x] | `powerline/segments/i3wm.py` ↔ `src/ported/segments/i3wm.rs` | 259 | 10 | 0 | 0 | 679 | 17 | 161 |
| [x] | `powerline/ipython.py` ↔ `src/ported/ipython.rs` | 51 | 0 | 3 | 11 | 252 | 13 | 38 |
| [x] | `powerline/renderers/ipython/since_5.py` ↔ `src/ported/renderers/ipython/since_5.rs` | 100 | 0 | 3 | 11 | 389 | 17 | 71 |
| [x] | `powerline/theme.py` ↔ `src/ported/theme.rs` | 151 | 6 | 1 | 6 | 444 | 13 | 148 |
| [x] | `powerline/lint/markedjson/markedvalue.py` ↔ `src/ported/lint/markedjson/markedvalue.rs` | 115 | 4 | 6 | 9 | 274 | 17 | 132 |
| [x] | `powerline/bindings/config.py` ↔ `src/ported/bindings/config.rs` | 232 | 11 | 1 | 2 | 600 | 20 | 151 |
| [x] | `powerline/bindings/vim/__init__.py` ↔ `src/ported/bindings/vim/mod.rs` | 371 | 11 | 1 | 3 | 252 | 17 | 211 |
| [x] | `powerline/bindings/ipython/pre_0_11.py` ↔ `src/ported/bindings/ipython/pre_0_11.rs` | 111 | 1 | 7 | 14 | 429 | 23 | 66 |
| [x] | `powerline/lib/watcher/inotify.py` ↔ `src/ported/lib/watcher/inotify.rs` | 212 | 0 | 5 | 15 | 545 | 19 | 153 |
| [x] | `powerline/renderer.py` ↔ `src/ported/renderer.rs` | 503 | 1 | 1 | 14 | 1280 | 25 | 373 |
| [x] | `powerline/segments/common/players.py` ↔ `src/ported/segments/common/players.rs` | 522 | 2 | 11 | 13 | 682 | 25 | 274 |
| [x] | `powerline/lib/vcs/__init__.py` ↔ `src/ported/lib/vcs/mod.rs` | 209 | 9 | 2 | 7 | 634 | 16 | 130 |
| [x] | `powerline/lint/markedjson/parser.py` ↔ `src/ported/lint/markedjson/parser.rs` | 207 | 0 | 2 | 17 | 575 | 22 | 107 |
| [x] | `powerline/lint/markedjson/error.py` ↔ `src/ported/lint/markedjson/error.rs` | 197 | 3 | 4 | 16 | 610 | 23 | 147 |
| [x] | `powerline/bindings/zsh/__init__.py` ↔ `src/ported/bindings/zsh/mod.rs` | 181 | 7 | 4 | 13 | 590 | 24 | 137 |
| [x] | `powerline/lib/config.py` ↔ `src/ported/lib/config.rs` | 179 | 2 | 3 | 19 | 628 | 24 | 156 |
| [x] | `powerline/lint/markedjson/constructor.py` ↔ `src/ported/lint/markedjson/constructor.rs` | 240 | 1 | 3 | 21 | 703 | 23 | 139 |
| [x] | `powerline/vim.py` ↔ `src/ported/vim.rs` | 271 | 4 | 2 | 18 | 512 | 23 | 137 |
| [x] | `powerline/lib/watcher/uv.py` ↔ `src/ported/lib/watcher/uv.rs` | 160 | 3 | 5 | 21 | 360 | 25 | 119 |
| [x] | `powerline/lint/checks.py` ↔ `src/ported/lint/checks.rs` | 752 | 26 | 0 | 0 | 1071 | 33 | 414 |
| [x] | `powerline/lib/threaded.py` ↔ `src/ported/lib/threaded.rs` | 199 | 0 | 3 | 29 | 831 | 35 | 163 |
| [x] | `powerline/lint/markedjson/scanner.py` ↔ `src/ported/lint/markedjson/scanner.rs` | 283 | 0 | 3 | 29 | 570 | 32 | 174 |
| [x] | `powerline/lint/spec.py` ↔ `src/ported/lint/spec.rs` | 637 | 0 | 1 | 32 | 1018 | 35 | 320 |
| [x] | `powerline/segments/vim/__init__.py` ↔ `src/ported/segments/vim/mod.rs` | 650 | 32 | 2 | 2 | 869 | 34 | 330 |
| [x] | `powerline/__init__.py` ↔ `src/ported/mod.rs` | 813 | 13 | 3 | 32 | 1784 | 49 | 456 |

## 🟢 NEAR — every Python fn has a `/// Port of` doc-comment; body density may still be partial (3)

| | File | Py LOC | Py fns | Py classes | Py methods | Rs LOC | `/// Port of` | `// py:` |
|---|---|---|---|---|---|---|---|---|
| [ ] | `powerline/renderers/shell/readline.py` ↔ `src/ported/renderers/shell/readline.rs` | 8 | 0 | 1 | 0 | 22 | 4 | 4 |
| [ ] | `powerline/renderers/shell/zsh.py` ↔ `src/ported/renderers/shell/zsh.rs` | 9 | 0 | 1 | 0 | 29 | 5 | 4 |
| [ ] | `powerline/bindings/i3/powerline-i3.py` ↔ `src/ported/bindings/i3/powerline-i3.rs` | 34 | 0 | 1 | 0 | 40 | 4 | 24 |

## 🟡 PARTIAL — 50–100% of Python fns ported; rest stubbed with citations (0)

*(empty)*

## 🟠 SPARSE — 20–50% of Python fns ported (0)

*(empty)*

## 🔴 STUB-HEAVY — <20% ported (mostly scaffold stubs) (0)

*(empty)*

---

## Plan-of-attack ordering

Work the **STUB-HEAVY** tier from smallest fn-count to largest first (quick wins validate the cadence), then **SPARSE**, then **PARTIAL**, then **NEAR**, then a final pass on the **DONE** tier to spot-check.

Within each tier, the table above is already ordered ascending by `(py_fns + py_methods, py_loc)`.

---

*Last generated by `scripts/gen_port_checklist.py` from a vendor snapshot in `vendor/powerline/`.*
