# PORT.md — Rules for Bots Contributing to `powerliners`

`powerliners` is a **1:1 Rust port of [powerline/powerline](https://github.com/powerline/powerline)**. The goal is 100% behavioral parity with upstream powerline-status. This is **not** a reimplementation, not a rewrite, not "inspired by" powerline. Every line of Rust code must trace back to a specific line of upstream Python code in `vendor/powerline/powerline/`.

If you are a bot (Copilot, Claude, GPT, Cursor, Aider, any LLM agent), **read this file before writing a single line of code**. Violations are deleted on sight by the maintainer. No exceptions.

---

## READ THIS FIRST — The Four Rules in One Screen

If you read nothing else in this file, read this. Every violation is deleted on sight; the maintainer does not negotiate.

Do not use fully qualified names that are not in Python. Python imports the names. So Rust does too. No imports inside functions, only imports at top of file organized by file.

### Rule 0 — ASK BEFORE INVENTING ANY NEW FN/STRUCT/STATIC NAME

**This rule overrides every other rule below.** If you (the bot) catch yourself about to write a `fn`, `struct`, `enum`, `type`, or `static` under `src/ported/` whose name does NOT exist in upstream powerline Python source, you must **STOP and ASK THE MAINTAINER FIRST**. You do not get to:

- "just add a tiny helper because it's only 3 lines"
- "factor out a Rust-only wrapper for borrow-checker reasons"
- "add a `_take`/`_set`/`_get`/`_clear`/`_is_some`/`_fill_*`/`_check_*` accessor for a thread_local"
- "split one Python function into `foo` + `foo_impl` for argument routing"
- "add a Rust-only sentinel like `LEX_TABS_INITED` or `PARSER_*_DEPTH`"
- "introduce a `*State`/`*Table`/`*Builder`/`*Config`/`*Context` aggregate"
- "add an `error()`/`set_error()`/`check_limit()`/`check_recursion()` paranoia helper"

even if the helper looks "obviously useful," "trivially small," "locally scoped," "obviously safe," or "what any reasonable Rust programmer would do." **None of those are reasons. Permission is the only reason.**

**The required flow when you think a Rust-only helper is needed:**

1. **STOP**. Do not write the helper.
2. State to the maintainer: *"I'm about to add `fn <name>` (or `struct <Name>` / `static <NAME>`) under `src/ported/<file>.rs` because <one-sentence reason>. This name does not exist in upstream powerline Python. May I proceed?"*
3. **Wait for explicit permission.** Phrases that count as permission: "yes", "y", "ok", "go", "approved", "fine". Anything else — silence, "let me think", "why?", "what about X instead?" — is NOT permission.
4. If permission is granted, add the name AND immediately also add it to `tests/data/fake_fn_allowlist.txt` with the maintainer's approval recorded in the commit message ("approved 2026-MM-DD").
5. If permission is denied, the work goes back to either (a) using a real Python-named port, (b) inlining the logic at call sites, or (c) abandoning the change.

**Test enforcement:** `tests/ported_fn_names_match_py.rs` rejects any fn under `src/ported/` whose name is neither in `docs/powerline_py_functions.txt` nor in `tests/data/fake_fn_allowlist.txt`. Adding a new name to the allowlist without prior maintainer approval is itself a violation — the allowlist is not a free pass, it's the audit trail of granted exceptions.

---

**Rule A — Names must exist in upstream powerline Python.** This applies to **every declaration** in `src/ported/`, not just functions:

| Rust decl                                  | Must exist in Python as                                          | Verify with                                                                                |
|--------------------------------------------|-------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| `fn <name>`                                | `def <name>(` function definition (top-level or method)     | `grep -xF '<name>' docs/powerline_py_functions.txt`                                        |
| `struct <Name>` / `enum <Name>`            | `class <Name>` definition                                   | `grep -xF '<Name>' docs/powerline_py_classes.txt`                                          |
| `static <NAME>` / `thread_local! { static <NAME> }` | module-level binding in Python                       | `grep -nE '^<name>[[:space:]]*=' vendor/powerline/powerline/**/*.py`                       |
| `type <Name>`                              | `class <Name>` or `TypeVar('<Name>')`                       | `grep -xF '<Name>' docs/powerline_py_classes.txt`                                          |

If `grep` returns nothing, the name is invented. **Delete or rename.**

**Rule B — Signatures must be identical to Python.**

Python `def humanize_bytes(num, suffix='B', si_prefix=False)` → Rust `fn humanize_bytes(num: f64, suffix: &str, si_prefix: bool)`. NOT `humanize_bytes(bytes: u64, options: &HumanizeOptions)`. No threading state through as extra params. No splitting one Python fn into many Rust fns. No merging many Python fns into one. No reordering params. No renaming params for "Rust idiom" (`num`, not `bytes`; `pl`, not `powerline`; `seg`, not `segment`).

**Rule C — Every decl lives in the file that mirrors its Python definition file.**

| Python definition is in...     | Rust port goes in...                  |
|---------------------------|---------------------------------------|
| `powerline/foo.py`               | `src/ported/foo.rs`                   |
| `powerline/foo/__init__.py`      | `src/ported/foo/mod.rs`               |
| `powerline/foo/bar.py`           | `src/ported/foo/bar.rs`               |
| `powerline/lib/vcs/git.py`       | `src/ported/lib/vcs/git.rs`           |
| `powerline/segments/common/net.py` | `src/ported/segments/common/net.rs` |
| `powerline/bindings/tmux/__init__.py` | `src/ported/bindings/tmux/mod.rs` |
| `powerline/renderers/tmux.py`    | `src/ported/renderers/tmux.rs`        |

A class declared in `powerline/colorscheme.py` does not belong in `src/ported/renderer.rs` just because `renderer.py` uses it. Same for fns: `powerline/lib/threaded.py` fns → `src/ported/lib/threaded.rs`, never re-homed to "wherever they're called from."

**Rule D — Bag-of-globals aggregator types are banned.**

❌ Python declares N module-level variables → Rust aggregates them into one struct.

If Python has `unit_list = ...; div = 1024; some_cache = {}` (three separate module-level bindings), the Rust port has three separate `thread_local!` entries (bucket 1) or three separate `Arc<RwLock<…>>` holders (bucket 2). **No struct `BagState { unit_list, div, some_cache }`** unless Python declares a `class BagState` at the same scope.

**Rule E — Local variables must use the Python source's exact names.**

This applies to **every variable inside a function body**, not just function names and file-level declarations:

```python
def humanize_bytes(num, suffix='B', si_prefix=False):
    if num == 0:
        return '0 ' + suffix
    div = 1000 if si_prefix else 1024
    exponent = min(int(log(num, div)) if num else 0, len(unit_list) - 1)
    quotient = float(num) / div ** exponent
    unit, decimals = unit_list[exponent]
    ...
```

Rust port — same names, same order, same scope:

```rust
pub fn humanize_bytes(num: f64, suffix: &str, si_prefix: bool) -> String {
    if num == 0.0 {                                        // py:14
        return format!("0 {}", suffix);                    // py:15
    }
    let div: f64 = if si_prefix { 1000.0 } else { 1024.0 }; // py:16
    let exponent: usize = ...;                              // py:17
    let quotient: f64 = num / div.powi(exponent as i32);    // py:18
    let (unit, decimals) = unit_list()[exponent];           // py:19
    ...
}
```

❌ **Forbidden renames inside function bodies**:
- `num` → `bytes`, `suffix` → `unit_suffix`, `si_prefix` → `use_si` (params)
- `div` → `divisor`, `exponent` → `exp`, `quotient` → `q` (locals)
- `unit` → `unit_str`, `decimals` → `n_decimals` (tuple destructuring)
- `i` → `idx`, `n` → `count`, `ret` → `result`
- Combining multiple Python locals into a tuple or struct
- Reordering local declarations
- Dropping a local just because Rust doesn't force you to declare it

**The Python name is the canonical name.** If the Python author chose `div` for the divisor, the Rust port keeps `div`. "Rust idiom" is not an excuse to rename anything — same convention that applies to function params per Rule B.

---

## ABSOLUTE FREEZE on `src/ported/`

**`src/ported/` is FROZEN to the upstream tree. No new files outside the mirror. No new functions whose name is not already in powerline's Python source.**

### File freeze (`src/ported/**.rs`)

- ❌ **You may NOT create any new `.rs` file under `src/ported/` that does not mirror a real `.py` file in upstream.** The legal file set is the **133 files in upstream `powerline/`** (run `find vendor/powerline/powerline -name '*.py' | sort` to see the exhaustive list). That set is the universe.
- Mapping rule: `powerline/<x>.py` → `src/ported/<x>.rs`, and `powerline/<dir>/__init__.py` → `src/ported/<dir>/mod.rs`.
- ❌ No new directories under `src/ported/` that don't mirror an upstream directory.

### Function freeze (`fn` names in `src/ported/**.rs`)

- ❌ **You may NOT introduce any new `fn` under `src/ported/` whose name does not already exist as a function in upstream powerline Python source** (`vendor/powerline/powerline/**/*.py`). Verify every name with:

  ```sh
  grep -xF '<name>' docs/powerline_py_functions.txt
  ```

- If `grep` returns nothing, the name is invented. Invented names are the drift signature.
- The Rule 3 exemptions for trait-impl methods (`fn new`, `fn drop`, `fn fmt`, `fn clone`, `fn from`, `fn next`, `fn poll`, etc.) and for `#[test]` functions still apply. Nothing else is exempt.
- ❌ Do not "rename to fit." Reusing a real powerline name like `humanize_bytes` for a function that does something different is worse than inventing a new name. Cite the Python source line; if the behavior doesn't match, the port is wrong.

### Struct / enum / typedef / static freeze

- ❌ **You may NOT introduce any `struct`, `enum`, `type`, `union`, or top-level `static` under `src/ported/` whose name does not already exist as a `class` / module-level binding in upstream powerline Python source.** Verify with:

  ```sh
  grep -xF '<Name>' docs/powerline_py_classes.txt
  # or for module-level constants:
  grep -nE '^<name>[[:space:]]*=' vendor/powerline/powerline/**/*.py
  ```

- ❌ Invented Rust-only aggregator structs are the **bag-of-globals anti-pattern**.
- ❌ Invented Rust-only "convenience" types — `RenderConfig`, `SegmentCache`, `*State`, `*Builder`, `*Config`, `*Context` — are deleted on sight when their Python counterpart does not exist in the matching `powerline/<file>.py`.
- ❌ Trivial wrapper `enum`s that re-encode Python string constants as Rust enum variants. The Python source uses string keys directly — the Rust port reads the same `&str` keys, NOT `match SegmentKind::Tmux => …`.

---

## Scope: `src/ported/` Is Strict-Port Territory

**Every Rust file under `src/ported/` is bound by every rule in this document — no grandfathering, no "legacy" exemptions, no "we'll fix it later".**

If a file lives under `src/ported/`:

- It **must** mirror a real Python file under `vendor/powerline/powerline/` (same stem, same relative subpath).
- Every `fn` in it **must** carry the `/// Port of <py_name>() from powerline/...:NNNN` doc-comment.
- Every `fn` name **must** appear in `docs/powerline_py_functions.txt` or be one of the narrow exemptions (trait-impls, tests).
- Every line **must** trace back to a specific upstream Python line. No invented helpers, no "cleaner" abstractions, no idiomatic-Rust refactors, no convenience wrappers.

The only two locations in the entire tree where non-ported code may exist are `src/extensions/` and `src/bin/`. The crate root file `src/lib.rs` is the only sanctioned non-port file outside those — it is explicitly carved out below and is **not** a precedent for adding more.

---

## NO SHORTCUTS — 100% LINE-BY-LINE COVERAGE

When the maintainer asks to port a Python source file, the result is a **complete 1:1 port**, not a partial one. "Faithful port" means every function, class, module-level constant, and decorator the Python source defines has a real Rust counterpart with matching name, signature, and control flow.

### When stubs ARE acceptable

The `// WARNING: NOT IN <FILE>.PY` marker is **only** appropriate when the Python definition genuinely lives in a *different* `powerline/*.py` file. The marker must always carry a file:line citation to the canonical home.

The marker is **not** appropriate for any function defined in the *same* Python source file the port covers.

### Audit requirement before declaring a port done

Before writing the commit message, run a sanity check:

```sh
# List every function defined in the Python source file:
grep -nE '^[[:space:]]*def [a-zA-Z_]' vendor/powerline/powerline/<path>.py

# List every fn in the Rust port:
grep -nE '^(pub )?(pub\(crate\) )?fn ' src/ported/<path>.rs

# Walk both lists side-by-side. Every Python name must appear in the
# Rust list. Stubs that don't match a different-file definition
# are blockers — not "follow-up commits".
```

---

## EXACT TRANSLATION — Same Names, Same Types, Same Calls, Every Line Cited

A true port is a **LINE-BY-LINE EXACT TRANSLATION** of the Python source. "Faithful port" is not a vibe; it is a checklist of literal correspondences any reviewer can audit in seconds.

### 1. Argument names match Python exactly (case-sensitive)

Python `def humanize_bytes(num, suffix='B', si_prefix=False)` ports as Rust `fn humanize_bytes(num: f64, suffix: &str, si_prefix: bool)`.

- `num`, NOT `bytes`. `suffix`, NOT `unit_suffix`. `si_prefix`, NOT `use_si`. The Python name is the canonical name; renaming for "Rust idiom" is a violation.
- Default arguments in Python become explicit values at call sites OR (acceptable, when a Rust default is needed) `#[derive(Default)]` patterns that mirror Python's positional/kwarg dispatch. Document the choice with `// py-default:<value>` so the round-trip is obvious.

### 2. Argument datatypes match Python through the canonical type map

| Python type / convention   | Rust type                          |
|---------------|------------------------------------|
| `str`         | `&str` (read) / `String` (own)    |
| `bytes`       | `&[u8]` / `Vec<u8>`               |
| `int`         | `i64` (Python int is unbounded; pick the narrowest that round-trips) |
| `float`       | `f64`                             |
| `bool`        | `bool`                            |
| `None`        | `Option<T>::None` or `()`         |
| `list[T]`     | `Vec<T>`                          |
| `tuple[A, B]` | `(A, B)`                          |
| `dict[K, V]`  | `HashMap<K, V>` / `BTreeMap<K, V>` (match Python's ordering semantics; `dict` is insertion-ordered ≥3.7) |
| `set[T]`      | `HashSet<T>` / `BTreeSet<T>`      |
| `Optional[T]` | `Option<T>`                       |
| `Union[A, B]` | `enum` with two variants (named after the union)|
| `Callable`    | `Fn` / `FnMut` / `FnOnce` trait bound |
| `Any`         | `serde_json::Value` for config payloads; `Box<dyn Any>` only with maintainer approval |

If the right-hand-side type doesn't exist as a Rust port yet, port the underlying `class` first (matching name + fields), then use it.

### 3. Called function names match Python exactly

If Python calls `log(num, div)`, Rust calls `log(num, div)` — i.e. the port matches the Python call shape, mapped via the canonical lib (`f64::log` is `num.log(div)` — citation still on the Python call). NEVER `math_log_safe`, NEVER `compute_log`, NEVER `safe_log_or_default`.

### 4. Every line carries a `// py:NNN` citation

```rust
if num == 0.0 {                                            // py:14
    return format!("0 {}", suffix);                        // py:15
}
let div: f64 = if si_prefix { 1000.0 } else { 1024.0 };    // py:16
let exponent: usize = ...;                                 // py:17
let quotient: f64 = num / div.powi(exponent as i32);       // py:18
let (unit, decimals) = unit_list()[exponent];              // py:19
```

Every Rust statement that ports a Python statement carries a comment with the Python source line. Block-level `// py:NNN-MMM` is acceptable for contiguous chunks. The doc-comment above the `fn` cites the function origin; the inline `// py:NNN` comments cite each statement. **Both are required.**

### 5. Local variables: same names, same order, same scope

Python declares locals at first use:

```python
div = 1000 if si_prefix else 1024
exponent = min(int(log(num, div)) if num else 0, len(unit_list) - 1)
quotient = float(num) / div ** exponent
unit, decimals = unit_list[exponent]
```

Rust mirrors that order:

```rust
let div: f64 = if si_prefix { 1000.0 } else { 1024.0 };       // py:16
let exponent: usize = ...;                                    // py:17
let quotient: f64 = num / div.powi(exponent as i32);          // py:18
let (unit, decimals) = unit_list()[exponent];                 // py:19
```

Same names (`div`, `exponent`, `quotient`, `unit`, `decimals`), same order, same scope. Don't combine into tuples, don't reorder, don't `let mut` only the ones Rust forces you to — be conservative.

### 6. Control flow keeps Python idioms

| Python construct                              | Rust mirror                                      |
|------------------------------------------|--------------------------------------------------|
| `for x in iter:`                         | `for x in iter { ... }`                          |
| `while cond:`                            | `while cond { ... }`                             |
| `if a and b:`                            | `if a && b { ... }`                              |
| `try: ... except E: ...`                 | `match ... { Ok(_) => ..., Err(_) => ... }` (citation cites the Python try/except line range) |
| `with ctx() as x: ...`                   | RAII pattern — bind `let x = ctx();` then drop at scope end; citation cites the `with` line |
| list comprehension `[f(x) for x in xs]`  | `xs.iter().map(f).collect::<Vec<_>>()` — preserve order; cite the comprehension line |
| dict comprehension `{k: v for ...}`      | `iter.map(|(k, v)| (k, v)).collect::<HashMap<_,_>>()` |
| generator `yield x`                      | iterator `impl Iterator` (port the body; citation on `yield`) |

Don't "improve" Python control flow into iterator chains beyond what the Python code already does. The structure of the Python code IS the structure of the Rust code.

### 7. Python source comments and docstrings port over

**This is a hard rule, not a "nice to have."** The Python source's inline comments and docstrings encode load-bearing context that the code alone doesn't:
- WHY a config key is rejected
- WHICH bug a workaround fixes
- WHEN a branch is reachable
- WHAT the Python author considered (and rejected) elsewhere
- The **WHY behind the WHAT** — every comment is a load-bearing intent record. Stripping them and reconstructing intent later is archaeology, not porting.

`# without arguments, display limits` becomes `// without arguments, display limits` (Rust `//`) in the same position, on the same line or block, as the Python source. Don't drop them, don't paraphrase them, don't translate idioms.

**Required:**
- Every Python inline comment in the function body MUST appear in the Rust port. Translate `#` to `//` line-form; preserve multi-line block comments as `//`-prefixed blocks at the same indentation.
- Python function docstrings (`'''...'''` immediately after the `def`) become Rust doc-comments (`///`) on the Rust port, alongside the `/// Port of <name>() from powerline/<file>.py:<line>.` citation. **Both** go on the port, not just the citation.
- Class docstrings carry over verbatim onto the Rust struct as `///`.
- Module docstrings (the `'''...'''` at the very top of the `.py` file) become Rust `//!` module doc-comments at the top of the corresponding `.rs` file.
- The `# vim:fileencoding=utf-8:noet` modeline at the top of every powerline `.py` file is the upstream convention; carry it forward as a Rust file-top `// vim:fileencoding=utf-8:noet` comment to make the upstream lineage visually obvious.

**Rust-only architectural notes** (e.g. "uses thread_local because Python module-level state doesn't survive Rayon workers", or "!!! WARNING: RUST-ONLY HELPER !!!" blocks for borrow-checker adapters) go in their **OWN** comment block, separately from the verbatim Python comment carry-overs. Don't conflate the two — a reader must be able to tell at a glance whether a comment originated in the Python source or is a Rust-port-specific note.

**Checking for completeness:** before declaring a port faithful, diff the Python function's comment density against the Rust port. A faithful port has comparable comment density. A Python body with 10 docstring lines + 8 inline `#` comments porting to a Rust body with 2 comments is a warning sign — comments were dropped.

### 8. Top-level declaration order matches Python exactly

The order of imports / module-level constants / `class` / `def` definitions in the Rust port mirrors the order in the Python source file, top to bottom.

- This makes side-by-side review trivial: a reviewer with `humanize_bytes.py` open in one pane and `humanize_bytes.rs` in the other can scroll both at the same rate and check correspondence by eye.
- It also lets the `// py:NNN` citations climb monotonically down the Rust file. If two adjacent Rust fns cite `py:670` then `py:519`, the ordering is wrong — fix it before committing.
- Reorder ONLY when forced by Rust's compiler (e.g., a `pub use` re-export that must precede its consumers); document the deviation with a `// reordered from py:NNN — Rust requires X` comment.

### 9. Function bodies port too — never bare-`return` a fn whose body is unported

When porting a fn whose body depends on subsystems not yet ported (config loader, segment registry, threaded watcher), DO NOT take the shortcut of "this depends on X which isn't ported, so the whole body returns 1 / 0 / no-op." The body lives in the same Python file as the function declaration — by Rule 1, it must be ported in full.

The correct approach for an in-file fn body:

1. Port the FULL Python body line-by-line, every Python statement → matching Rust statement with `// py:NNN` citation.
2. Stub the EXTERN dependencies (fns / globals from OTHER Python files) locally with file:line citations to their home file.
3. The body STILL EXECUTES — branches still take, increments still happen, mutations to module-local statics still apply. The extern-dep return values produce a degenerate runtime trace until the real ports land.
4. Add a test that exercises the full body to prove the body actually runs without panicking.

---

## The Three Hard Rules

### 1. PORT-ONLY. NO ADHOC IMPLEMENTATIONS.

You are translating Python → Rust. You are not designing software.

- You **may** write a Rust function if and only if it is a port of a specific Python function that exists in `vendor/powerline/powerline/**/*.py`.
- You **may not** invent helper functions, utility wrappers, "cleaner" abstractions, traits, builders, or any other code that does not have a direct Python counterpart in upstream powerline.
- "Refactoring for idiomatic Rust" is **forbidden**. The structure of the Python code is the structure of the Rust code. Same function names (modulo the renaming rules above), same control flow, same module-level state, same field layout where feasible.
- If a Python function uses a generator (`yield`), your Rust port returns an `impl Iterator` mirroring the same control flow. Do not "improve" it.
- If you cannot find a matching Python function for code you want to write, **stop and do not write it**. Ask the maintainer or pick a different task.

#### The TWO and ONLY TWO exceptions

There are exactly two locations in the tree where new, non-ported code is permitted to exist. **Nowhere else.**

1. **`src/extensions/`** — the **only** place for features that powerline-status does not have. This is where genuinely new functionality lives: anything that goes beyond upstream's behavior (Cranelift JIT for segment dispatch, persistent worker pool, rkyv zero-copy IPC, integration with zshrs runtime, etc.). Code here is **not** a port and is not expected to map to any Python function.
2. **`src/bin/`** — binary entry points (`powerline`, `powerline-daemon`, `powerline-render`, `powerline-config`, `powerline-lint`). These mirror upstream's `scripts/` directory and dispatch into `src/ported/commands/*` for the actual logic.

Everything outside these two locations is a **port**. No exceptions.

### 2. EVERY FUNCTION MUST CITE ITS PYTHON SOURCE.

Every `fn` in the Rust tree must carry a doc-comment of this exact form immediately above the signature:

```rust
/// Port of `<py_function_name>()` from `powerline/<subdir>/<file>.py:<line>`.
///
/// <one-line summary mirroring the Python function's purpose or docstring opening>
pub fn <rust_name>(...) -> ... {
    ...
}
```

Required:
- The Python function name in backticks with `()`.
- The path **relative to `vendor/powerline/`** (so `powerline/lib/humanize_bytes.py:11`, not `vendor/powerline/powerline/lib/humanize_bytes.py:11`).
- The line number of the Python function's definition (the line with the `def`).

### 3. NAMES MUST EXIST IN UPSTREAM POWERLINE.

The allowlist of legal names is in:

- **`docs/powerline_py_functions.txt`** — unique function names.
- **`docs/powerline_py_classes.txt`** — unique class names.
- **`docs/powerline_py_functions_with_locations.txt`** — same names with `powerline/path.py:line` for cross-reference.

A Rust function name is **legal** if and only if it is one of:

1. **Identical** to a name in `powerline_py_functions.txt` (e.g. Python `humanize_bytes` → Rust `humanize_bytes`).
2. A standard Rust trait-impl method (`fn new`, `fn drop`, `fn fmt`, `fn clone`, `fn default`, `fn from`, `fn into`, `fn as_ref`, `fn deref`, `fn eq`, `fn hash`, `fn partial_cmp`, `fn cmp`, `fn next`, `fn poll`, `fn serialize`, `fn deserialize`) — and only when it directly wraps a Python class method or attribute access.
3. A Rust `#[test]` or `#[cfg(test)]` function — tests are exempt from the Python-name rule but must still describe what Python behavior they verify.

Anything else — `make_pretty_helper`, `parse_args_v2`, `init_state_new`, `fancy_iter`, `RustyOptions::build`, etc. — **will be deleted**.

---

## File Layout: 1:1 with powerline — NO NEW FILES EVER

The Rust source tree is split into exactly **two** top-level directories under `src/` (plus `src/lib.rs` and `src/bin/`):

| dir                | purpose                                                            |
|--------------------|--------------------------------------------------------------------|
| `src/ported/`      | The 1:1 port. Every file here mirrors a `powerline/<...>.py`.      |
| `src/extensions/`  | Features powerline-status does **not** have. The only sanctioned non-port dir. |
| `src/bin/`         | Binary entry points (`powerline`, `powerline-daemon`, ...).        |

Mapping:

| upstream Python dir       | Rust dir                    |
|------------------------|-----------------------------|
| `powerline/`           | `src/ported/`               |
| `powerline/lib/`       | `src/ported/lib/`           |
| `powerline/lib/vcs/`   | `src/ported/lib/vcs/`       |
| `powerline/lib/watcher/`| `src/ported/lib/watcher/`  |
| `powerline/lint/`      | `src/ported/lint/`          |
| `powerline/lint/markedjson/` | `src/ported/lint/markedjson/` |
| `powerline/bindings/`  | `src/ported/bindings/`      |
| `powerline/bindings/tmux/` | `src/ported/bindings/tmux/` |
| `powerline/bindings/zsh/` | `src/ported/bindings/zsh/` |
| `powerline/renderers/` | `src/ported/renderers/`     |
| `powerline/renderers/shell/` | `src/ported/renderers/shell/` |
| `powerline/segments/`  | `src/ported/segments/`      |
| `powerline/segments/common/` | `src/ported/segments/common/` |
| `powerline/segments/vim/`    | `src/ported/segments/vim/` |
| `powerline/commands/`  | `src/ported/commands/`      |
| `powerline/matchers/`  | `src/ported/matchers/`      |
| `powerline/listers/`   | `src/ported/listers/`       |
| `powerline/selectors/` | `src/ported/selectors/`     |

File naming:

- `powerline/<x>.py` → `src/ported/<x>.rs`
- `powerline/<dir>/__init__.py` → `src/ported/<dir>/mod.rs`
- `powerline/<dir>/<x>.py` → `src/ported/<dir>/<x>.rs`

No renames of any kind. No `_port`, no `_rs`, no `_impl`, no `_v2`, no stripping of any prefix or suffix. The Rust file stem is **byte-for-byte identical** to the Python file stem.

If your port of `humanize_bytes()` (defined in `powerline/lib/humanize_bytes.py`) ends up in `src/ported/anything_other_than_lib/humanize_bytes.rs`, you have done it wrong. Move it.

If it ends up anywhere outside `src/ported/` (e.g. `src/foo.rs` at the crate root, or under `src/extensions/`), it will be deleted on sight.

---

## Adhoc Code: 100% Banned, Deleted on Sight

Adhoc implementation is **forbidden absolutely**. Not "discouraged." Not "should be ported eventually." **Banned.**

The maintainer runs purges that delete any function or file which:

- Has **no** `/// Port of ... from powerline/...` doc-comment, **or**
- Carries the `/// WARNING: THIS IS ADHOC IMPLEMENTATION AND NOT A FAITHFUL PORT` marker, **or**
- Has a name that is not in `docs/powerline_py_functions.txt` and is not one of the allowed exemptions in Rule 3, **or**
- Lives in a Rust file under `src/ported/` that has no corresponding Python file under `vendor/powerline/powerline/`, **or**
- Lives in the wrong file per the 1:1 mapping (e.g. a port of `humanize_bytes` outside `src/ported/lib/humanize_bytes.rs`), **or**
- Lives outside `src/ported/`, `src/extensions/`, or `src/bin/`.

If your PR adds adhoc code, **all of it will be deleted** — the function, the file, the module declaration. Without discussion.

---

## Workflow for Bots

Before writing any code:

1. Identify the Python function you intend to port. Get its exact name, file, and line. Confirm it appears in `docs/powerline_py_functions.txt`.
2. Identify the destination Rust file using the 1:1 mapping table.
3. Read the Python function in full. Read every helper it calls. Read the relevant `class` definitions.
4. Translate line-by-line. Preserve identifier names. Where Python names collide with Rust keywords, use `r#name` (e.g. `r#type`, `r#match`).
5. Add the `/// Port of ... from powerline/...:NNNN` doc-comment.
6. Add inline `// py:<line>` tags on every non-trivial translated statement.
7. Carry over EVERY Python comment and docstring (Rule 7).
8. Run `cargo build --lib` and `cargo test --lib`. Do not regress the baseline.

---

## What You Must Never Do

- ❌ **Create any new Rust file outside `src/ported/`** other than the two sanctioned exceptions (`src/extensions/`, `src/bin/`).
- ❌ **Create any directory under `src/ported/` that doesn't mirror an upstream Python directory.**
- ❌ **Add any `fn` under `src/ported/` whose name does not already exist as a function in `vendor/powerline/powerline/**/*.py`.** Verify with `grep -xF '<name>' docs/powerline_py_functions.txt`.
- ❌ **Add any `struct`, `enum`, `type`, `union`, or top-level `static` under `src/ported/` whose name does not already exist as a `class` / module-level binding in upstream powerline Python source.**
- ❌ **Place a class in the wrong file.** A class declared in `powerline/colorscheme.py` belongs in `src/ported/colorscheme.rs`, NOT in `src/ported/renderer.rs`.
- ❌ **Change a function signature.** Rust signature is identical to Python.
- ❌ **Rename a local variable inside a function body.** Python `div, exponent, quotient` → Rust `div, exponent, quotient`, NOT `divisor`/`exp`/`q`.
- ❌ Invent a function with a name not in `docs/powerline_py_functions.txt`.
- ❌ Write "helper" / "utility" / "convenience" functions or files.
- ❌ Add new modules like `helpers`, `common`, `prelude`, `error`, `state`, `runtime`, `ffi`, `macros`, `types`, `safe_*`, `rusty_*` — none correspond to any `powerline/*.py`.
- ❌ Refactor Python control flow into Rust iterators / combinators / traits unless the Python code already does the equivalent.
- ❌ Add abstraction layers (traits, generics, builders) that aren't in the Python source.
- ❌ Split one Python function across multiple Rust files.
- ❌ Combine multiple Python functions into one Rust function.
- ❌ Add `_port`, `_rs`, `_impl`, `_v2`, `_new`, `_safe`, `_ext` suffixes.
- ❌ Skip the `/// Port of ...` doc-comment.
- ❌ Skip inline `// py:NNN` citations.
- ❌ Drop Python comments / docstrings when porting.
- ❌ Cite a Python function that doesn't exist or doesn't actually correspond.
- ❌ "Stub" a function with `unimplemented!()` and call it ported.
- ❌ Translate from your memory of powerline's behavior. Read the Python source.

---

## What You Should Do

- ✅ Pick one Python function, port it faithfully, cite it precisely.
- ✅ Mirror Python identifier names, class field names, file layout.
- ✅ Mirror Python control flow (generators → iterators, `with` → RAII).
- ✅ Mirror module-level state as `static` / `Mutex<...>` / `thread_local!` as needed for parity, not Rust elegance.
- ✅ Carry over EVERY `#` comment and `"""..."""` docstring with line:line correspondence.
- ✅ Cross-reference `docs/powerline_py_functions_with_locations.txt` to verify every name and location.
- ✅ Keep the build green. Keep the test baseline.

---

## Sources of Truth

- **Python source**: `vendor/powerline/powerline/**/*.py` (vendored upstream).
- **Function allowlist**: `docs/powerline_py_functions.txt` (regenerated by `scripts/extract_py_names.py`).
- **Class allowlist**: `docs/powerline_py_classes.txt`.
- **Location index**: `docs/powerline_py_functions_with_locations.txt`.

---

## TL;DR

> **Rule 0: ASK FIRST.** Adding any `fn`/`struct`/`enum`/`static` under `src/ported/` whose name does NOT exist in upstream powerline Python requires EXPLICIT MAINTAINER PERMISSION before you write the code.
>
> **`src/ported/` mirrors `vendor/powerline/powerline/` 1:1.**
>
> Every name must exist in upstream powerline Python:
> - **Functions**: name appears in `vendor/powerline/powerline/**/*.py` as a `def`.
> - **Classes / module bindings**: name appears as a `class` or module-level `name = ...`.
> - **Local variables inside fn bodies**: same names as Python, in the same order, at the same scope.
> - **Signatures**: identical to Python.
> - **File placement**: every fn / class lives in the Rust file that mirrors its Python definition file.
> - **Code order**: top-to-bottom order of decls/classes/fns/comments matches Python.
> - **Citations**: every fn carries `/// Port of <py_fn>() from powerline/<file>.py:<NNNN>`; every non-trivial statement carries an inline `// py:NNN` comment.
> - **Comments**: every Python `#` comment and `"""..."""` docstring ports over to the Rust port at the corresponding position.
>
> Every file is a strict 1:1 port of its `powerline/*.py`. Adhoc code anywhere else is deleted on sight.
