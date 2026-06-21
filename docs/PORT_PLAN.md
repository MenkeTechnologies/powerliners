# PORT_PLAN.md — When 100% Faithful Python→Rust Is Possible, When It Isn't

This is the design rule for state-holding patterns in `src/ported/`.
`PORT.md` governs *what* may be ported (file/function freeze, naming).
This file governs *how* Python state translates to Rust state when
Python's GIL-protected single-interpreter model meets powerliners's
worker pool.

The short answer: **a 1:1 port works for most state. It fails in
exactly one place — Python module-level mutable state that is
expected to be globally visible across the entire process.** Those
module-level vars must become `Arc<Mutex/RwLock<…>>`, not `static`
or `thread_local!`, because worker threads (segment renderers, VCS
watchers, the daemon listener) must share them.

---

## Relationship to PORT.md (Read First)

PORT.md is the constitution. This file is a design supplement that
must operate inside PORT.md's hard constraints. Every action in the
checklist below stays within them:

| PORT.md rule | How this plan complies |
|---|---|
| `src/ported/` mirrors upstream 1:1 | All work modifies files in the 137-file mirror set. |
| No new `fn` names not in `docs/powerline_py_functions.txt` | All work modifies state holders (`static …` declarations); no new `fn` introduced. |
| Rule 2: every `fn` must carry `/// Port of … from powerline/…:NNNN` | All converted holders keep their existing `/// Port of module-level <py_name> from powerline/<file>.py:<NNNN>` doc-comment. |
| Rule 1: no abstractions without Python counterpart | "Bag-of-globals" structs that aggregate module-level vars with no matching Python class are explicit anti-pattern below. |
| Globals: "Mirror as static / Mutex<…> / thread-locals as needed for parity" | This document is the parity rubric — when each is correct vs wrong. |

Conflicts between this file and PORT.md: **PORT.md wins.** If a
proposed change here would create a new file or new fn name not in
`docs/powerline_py_functions.txt`, do not make the change — raise it.

---

## Structural Fidelity Rules

### Rule S1 — Function signatures must be identical to Python

Every ported function must have the same signature as its Python
counterpart. If Python `humanize_bytes(num, suffix='B', si_prefix=False)`
reads no module globals, the Rust port is
`pub fn humanize_bytes(num: f64, suffix: &str, si_prefix: bool)` —
NOT `pub fn humanize_bytes(num: f64, opts: &HumanizeOptions)`.

Do not "improve" signatures by threading state as parameters. The
Python module-level vars become Rust module-level holders (bucket 1
or 2 per below); the function reads them the same way Python does.
This keeps call sites identical and avoids signature drift that
compounds across the codebase.

**Exception:** Bucket 3 (Python class methods — explicit `self`) are
already explicit parameters in Python — those stay as `&self` / `&mut
self` in Rust.

### Rule S2 — Order of code elements must match Python

Within each `.rs` file, the order of:
- `use` imports (Python `import`)
- `static` / `const` declarations (Python module-level bindings)
- `struct` / `enum` definitions (Python `class`)
- `fn` definitions (Python `def`)
- inline comments

must match the order in the corresponding `.py` file. Reading
`renderer.rs` top-to-bottom should feel like reading `renderer.py`
top-to-bottom.

When reordering is required to fix drift, **comments must move with
their associated code**. A comment block explaining `unit_list` must
stay attached to the `unit_list` declaration, not be orphaned when
code moves.

---

## The Three Buckets

Every Python state slot in powerline falls into one of three buckets.
The Rust port must classify each one before writing the holder.

### Bucket 1 — Per-render-pass scratch (thread-local in Rust)

**What it is in Python:** state allocated fresh inside a function or
method that lives only for the duration of one render pass / one
segment evaluation / one config-load cycle. Python's GC reclaims it
when the function returns.

**Examples:**
- `lib/humanize_bytes.py` `div`, `exponent`, `quotient`, `unit` — all
  function-local; never module-level
- `renderer.py` per-call format buffers
- `segment.py` per-segment-invocation arg dict

**Rust port:** plain function-local `let` bindings.

**Why locals are correct, not wrong:** in Python the binding is
function-scoped; in Rust likewise. No `thread_local!` needed — Rust
already isolates per-call-frame state by definition.

**Code stays 1:1 with Python.** `div = 1000 if si_prefix else 1024`
in Python becomes `let div: f64 = if si_prefix { 1000.0 } else {
1024.0 };` in Rust. Same name, same lifetime.

### Bucket 2 — Module-level mutable state shared across threads

**What it is in Python:** a module-level binding (`name = ...` at
file scope, no class) that holds runtime state mutated during
program execution. In Python, all threads see the same binding
(GIL-protected); modules are imported once per interpreter.

**Examples:**
- `lib/memoize.py` cache dicts — module-level mutable dict shared
  by every memoized function
- `lib/threaded.py` worker registry — module-level list of started
  threads
- `lib/watcher/inotify.py` global FD watcher state
- `version.py` `__version__` — string constant (read-only, but
  module-level)
- `lib/humanize_bytes.py` `unit_list` — tuple constant (read-only,
  module-level)

**Read-only constants** (`__version__`, `unit_list`, etc.) →
`pub const NAME: …` (compile-time) or `pub static NAME: …` (runtime
init).

**Mutable shared state** (memoize cache, thread registry, watcher
FDs) → `OnceLock<RwLock<…>>` for read-mostly tables,
`OnceLock<Mutex<…>>` for read/write parity.

**Never `thread_local!` for shared mutable state.** If thread A
populates the memoize cache and thread B reads from it, both threads
must hit the same cache. TLS would silently shard the cache per
worker — ghost cache misses, duplicated VCS shellouts, double-fired
watchers. The bug only appears at scale; it's invisible in
single-thread tests.

**Cost of correctness:** every read takes a lock. For read-mostly
tables use `RwLock` so parallel readers don't serialize. For
high-mutation tables (active-thread set, watcher FD pool) `Mutex` is
fine.

### Bucket 3 — Class instance state

**What it is in Python:** state attached to a `class` instance via
`self.attr = ...` in `__init__` or methods. Each instance has its
own copy.

**Examples:**
- `class Powerline` — `self.renderer`, `self.config_loader`,
  `self.use_daemon_threads`, etc.
- `class Renderer` — `self.segments`, `self.theme`, `self.colorscheme`
- `class ConfigLoader` — `self.config_files`, `self.watchers`

**Rust port:** Rust `struct` with field names matching Python
attribute names exactly. Methods on the struct take `&self` or `&mut
self`.

**Code stays 1:1 with Python.** Same struct fields, same method
signatures, same `.attr` access patterns.

---

## Decision Rule (Mechanical)

For every Python state slot you encounter while porting:

1. Is it inside `def`/`__init__`/`method` body, allocated fresh per
   call? → **bucket 1, Rust local `let`.**
2. Is it at module scope (`name = ...` outside any class/def) and
   does it represent a constant (no mutations after init)? →
   **bucket 2, `pub const` (compile-time) or `pub static` (runtime
   init).**
3. Is it at module scope and mutated during program execution? →
   **bucket 2, `OnceLock<RwLock<…>>` / `OnceLock<Mutex<…>>`.**
4. Is it `self.attr` inside a class? → **bucket 3, Rust struct
   field.**

If you can't decide between mutable-module-level (3) and
class-instance (4), check the upstream: if Python wraps the state in
a `class` (even a singleton), Rust uses a struct. If Python leaves
it as a bare module-level binding, Rust uses `OnceLock<Lock<...>>`.

---

## Where 100% Faithful Port Works

For bucket 1 (locals) and bucket 3 (instance state), the port is
mechanical and the Rust code reads as a transliteration of the
Python code. This covers the majority of `src/ported/`:

| Area | Python source | Rust port |
|---|---|---|
| Number formatting | `lib/humanize_bytes.py` locals | function-local `let` bindings |
| Class Powerline | `__init__.py:class Powerline` | `pub struct Powerline { ... }` |
| Class Renderer | `renderer.py:class Renderer` | `pub struct Renderer { ... }` |
| Class Theme | `theme.py:class Theme` | `pub struct Theme { ... }` |
| Class Segment | `segment.py:class Segment` | `pub struct Segment { ... }` |
| Class ConfigLoader | `lib/config.py:class ConfigLoader` | `pub struct ConfigLoader { ... }` |
| Tmux renderer | `renderers/tmux.py:class TmuxRenderer` | `pub struct TmuxRenderer { ... }` |
| Shell renderers | `renderers/shell/*.py:class XshRenderer` | `pub struct XshRenderer { ... }` (per shell) |

These files preserve Python structure, function names, and control
flow. PORT.md's "every line traces back to upstream Python" rule is
satisfied trivially.

---

## Where 100% Faithful Port Cannot Work

For bucket 2 mutable state (memoize cache, thread registry, watcher
FDs), a literal Python-to-Rust translation gives a data-race-free
*but semantically broken* program: each worker thread sees its own
copy of the cache. The port must wrap the table in `OnceLock<Lock>`
even though Python uses a bare module-level dict.

This is the *only* sanctioned deviation from 1:1 fidelity, and it's
forced by the threading model, not by stylistic preference.

The Rust holder is still in the same `src/ported/<x>.rs` that maps
to `powerline/<x>.py`. The function bodies still mirror Python.
Only the *storage primitive* changes — and its name should still
match the Python identifier (`MEMOIZE_CACHE`, `THREAD_REGISTRY`,
etc.) so the trace from Rust call site to Python source remains
obvious.

### Status of bucket 2 ports

| Python state | Rust file | Holder primitive | Status |
|---|---|---|---|
| `version.__version__` | `version.rs` | `pub const __version__: &str` | DONE |
| `lib/humanize_bytes.unit_list` | `lib/humanize_bytes.rs` | `pub const unit_list: [(&str, usize); 6]` | DONE |
| `lib/memoize.py` module-level cache | `lib/memoize.rs` | `OnceLock<Mutex<HashMap<…>>>` | DONE |
| `lib/threaded.py` thread registry | `lib/threaded.rs` | `Mutex<Option<JoinHandle>>` + `Arc<Mutex<bool>>` | DONE |
| `lib/watcher/inotify.py` FD pool | `lib/watcher/inotify.rs` | `OnceLock<Mutex<…>>` | DONE |
| `lib/watcher/stat.py` mtime cache | `lib/watcher/stat.rs` | `Mutex<HashMap<PathBuf, SystemTime>>` | DONE |
| `lib/vcs/__init__.py` driver registry | `lib/vcs/mod.rs` | `OnceLock<…>` per holder | DONE |
| `segments/common/*.py` per-segment caches | `segments/common/*.rs` | per-segment `OnceLock<Mutex/RwLock<…>>` | DONE |

---

## Anti-patterns

### 1. Bag-of-globals struct

A struct that aggregates every module-level binding from a Python
source file "to thread one parameter through instead of N." This
violates PORT.md Rule 1 (no abstractions without Python counterpart)
and duplicates the corresponding `OnceLock<Lock>` set.

**Rule:** if Python declares N module-level bindings, Rust declares
N `static` / `OnceLock<Lock<…>>` entries. No aggregation struct
unless Python has a `class` to point at.

❌ Bad — invented `RendererState` struct aggregating
`renderer.py` module-level vars:

```rust
pub struct RendererState {
    pub theme_cache: HashMap<String, Theme>,
    pub colorscheme_cache: HashMap<String, Colorscheme>,
}
```

✅ Good — each Python module-level var becomes its own holder:

```rust
pub static THEME_CACHE: OnceLock<RwLock<HashMap<String, Theme>>> = OnceLock::new();
pub static COLORSCHEME_CACHE: OnceLock<RwLock<HashMap<String, Colorscheme>>> = OnceLock::new();
```

### 2. `Mutex` for read-only constants

Using `static FOO: Mutex<T>` for state that is never mutated. The
lock contention serializes worker threads for no semantic reason.

**Rule:** if the Python source declares the binding once at module
load and never mutates it, the Rust port is `pub const NAME: …` or
`pub static NAME: …` (no lock). `Mutex` is reserved for mutable
shared state.

### 3. `thread_local!` for shared module-level state

Putting the memoize cache or thread registry in TLS because "Python
puts it at module scope." This silently per-shards the cache. The
data race goes away (each thread has its own copy) but the program
is wrong — every memoized call misses on first hit per thread,
duplicated VCS shellouts cost a fortune.

**Rule:** read 3+ Python call sites for the binding. If two
different worker contexts must see the same data, it's bucket 2 —
`OnceLock<Lock>`, not TLS.

### 4. Converting class methods to free fns

Python `class Foo: def bar(self, x): ...` ports to Rust
`impl Foo { fn bar(&self, x: …) -> … { ... } }`. Do **NOT** port to
a free fn `fn bar(foo: &Foo, x: …) -> …` — that breaks method-call
sites and obscures the class membership.

**Rule:** if Python declares it inside `class Foo:`, Rust declares it
inside `impl Foo`. Same name, same signature (with implicit `self`
becoming `&self` / `&mut self`).

---

## Conversion Plan — Phases

Order is set by blast radius. Tick each phase only when all items
are committed and `cargo build` is green.

**Per-item discipline (PORT.md workflow):**

1. Read the Python source line cited in the bullet (`powerline/<file>.py:<NNNN>`).
2. Verify the holder name matches the Python identifier byte-for-byte
   (uppercased to Rust SCREAMING_SNAKE convention for mutable shared
   statics, kept lowercase for read-only constants).
3. Make the change. No new `fn`, no new file.
4. Update or keep the `/// Port of module-level <py_name> from
   powerline/<file>.py:<NNNN>` doc-comment on the holder.
5. `cargo build --lib && cargo test --lib -- <module>` (targeted,
   not full suite — per global preferences).
6. Commit citing the Python line in the message body.

**End-of-phase discipline (PORT.md workflow step 8):**

- `python3 scripts/extract_py_names.py` — refresh allowlists.
- `python3 scripts/gen_port_report.py` — refresh
  `docs/port_report.html`.
- `python3 scripts/gen_port_checklist.py > docs/PORT_CHECKLIST.md` —
  refresh per-file tier table.

### Phase 0 — Scaffolding (DONE)

- [x] Vendor upstream at `vendor/powerline/` (137 .py files)
- [x] Generate `docs/powerline_py_functions.txt` +
      `docs/powerline_py_classes.txt` +
      `docs/powerline_py_functions_with_locations.txt`
- [x] Mirror `src/ported/` to upstream tree
- [x] Write `docs/PORT.md` doctrine
- [x] Write `docs/PORT_PLAN.md` (this file)
- [x] Add `tests/ported_fn_names_match_py.rs` drift gate

### Phase 1 — Leaf ports (DONE)

134/137 upstream Python files at DONE tier. The 3 remaining NEAR files
are class-only Python sources at the citation-density-classifier ceiling
(see README `STATUS` section); their Rust ports are functionally complete.

See `docs/PORT_CHECKLIST.md` for the per-file tier table.

### Phase 2 — Core orchestration (DONE at the unit level)

- [x] `lib/encoding.rs`, `lib/unicode.rs`, `lib/path.rs`,
      `lib/shell.rs`, `lib/dict.rs` (utility substrate)
- [x] `lib/memoize.rs` (bucket 2 cache holder)
- [x] `lib/threaded.rs` (bucket 2 thread registry)
- [x] `lib/config.rs` (config loader)
- [x] `colorscheme.rs` (Colorscheme class)
- [x] `theme.rs` (Theme class)
- [x] `segment.rs` (Segment class + dispatch)
- [x] `renderer.rs` (Renderer base class)
- [x] `mod.rs` (Powerline class + `__init__.py` orchestrator)

Function-level ports complete; orchestrator integration (the wire-up
that makes `powerline-render` produce a real statusline) is the
remaining substrate work tracked in the README `What's not yet wired`
section.

### Phase 3 — tmux pipeline (DONE at the unit level)

- [x] `renderers/tmux.rs` (TmuxRenderer)
- [x] `bindings/tmux/mod.rs` (tmux client)
- [x] `segments/common/{net,sys,time,bat,wthr}.rs`

### Phase 4 — Shell + zsh bindings (DONE)

- [x] `renderers/shell/{bash,readline,tcsh,ksh,rcsh,zsh}.rs`
- [x] `bindings/shell/mod.rs`
- [x] `bindings/zsh/mod.rs`

### Phase 5 — Linter (DONE)

- [x] `lint/markedjson/*.rs` (~7,400 LOC subtree)
- [x] `lint/checks.rs`, `lint/spec.rs`, `lint/imp.rs`, `lint/inspect.rs`

### Phase 6 — Daemon (DONE at the lifecycle level)

- [x] `commands/daemon.rs` argparser
- [x] `scripts/powerline_daemon.rs` — UNIX socket bind, daemonize,
      pidfile lock, accept loop, EOF shutdown
- [x] `src/bin/powerline-daemon.rs` binary entry

Render path is real end-to-end: the `Powerline` orchestrator
(`do_render` in `src/ported/renderer.rs`) produces actual statusline
markup, pinned by the 45 byte-for-byte scenarios in
`tests/daemon_parity.rs`.

### Phase 7 — Parity verification (ongoing)

- [x] `tests/parity_against_upstream.rs` — 462 tests piping identical
      inputs through upstream Python and the Rust port, asserting
      byte/value-identical results
- [x] 11 real port bugs surfaced by the harness and fixed in-tree

### Phase 8 — Performance wins beyond 1:1

- [ ] Cranelift JIT for segment dispatch
- [ ] rkyv zero-copy IPC for daemon protocol
- [ ] Persistent worker pool integration with zshrs

---

## Quick-Reference Decision Card

```
Python declaration                    →  Rust holder
──────────────────────────────────────────────────────────────────
def foo(self, x): ...                 →  impl Foo { fn foo(&self, x: …) … }
def foo(x): ...                       →  pub fn foo(x: …) -> …
local: x = ...  (inside def)          →  let x = ...;                     (bucket 1)
NAME = "literal"  (module-level)      →  pub const NAME: &str = "literal" (bucket 2 read-only)
CACHE = {}  (module-level, mutated)   →  pub static CACHE: OnceLock<RwLock<HashMap<…>>>
                                                                          (bucket 2 mutable)
self.attr = ... (inside __init__)     →  struct field                     (bucket 3)
```

If unsure between bucket 2 mutable and bucket 3: does the value
exist independently of any class instance? Yes → bucket 2. Owned by
a specific instance → bucket 3.

---

## Test Invariants (Bucket 2 Specifically)

The `ported_fn_names_match_py.rs` test guards against invented fn
names. Bucket 2 needs its own behavioral pin: a multi-threaded test
that spawns N workers, has each one mutate the shared cache (set a
unique memoize key, register a unique segment), then verifies all N
mutations are visible from a single observer thread. If TLS ever
sneaks back in for a bucket-2 holder, this test fails. Add
`tests/shared_state_visible.rs` when the first bucket-2 holder
(`lib/memoize.rs`) goes live.
