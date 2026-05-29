```
 ____   _____        _______ ____  _     ___ _   _ _____ ____  ____
|  _ \ / _ \ \      / / ____|  _ \| |   |_ _| \ | | ____|  _ \/ ___|
| |_) | | | \ \ /\ / /|  _| | |_) | |    | ||  \| |  _| | |_) \___ \
|  __/| |_| |\ V  V / | |___|  _ <| |___ | || |\  | |___|  _ < ___) |
|_|    \___/  \_/\_/  |_____|_| \_\_____|___|_| \_|_____|_| \_\____/
```

<p align="center">
<code>// RUST PORT OF python's powerline-status. STATUSBAR-AS-A-NATIVE-BINARY. ZERO PYTHON RUNTIME.</code>
</p>

---

[![Status](https://img.shields.io/badge/status-134%2F137%20DONE-39ff14.svg)](#status)
[![Tests](https://img.shields.io/badge/lib%20tests-1879%20passing-39ff14.svg)](#status)
[![Parity](https://img.shields.io/badge/parity%20tests-219%20vs%20upstream-05d9e8.svg)](#status)
[![Bugs Fixed](https://img.shields.io/badge/port%20bugs%20fixed-11-d300c5.svg)](#status)
[![Source](https://img.shields.io/badge/port_of-powerline--status-05d9e8.svg)](https://github.com/powerline/powerline)
[![Language](https://img.shields.io/badge/lang-rust-d300c5.svg)](https://www.rust-lang.org/)
[![Target](https://img.shields.io/badge/target-tmux%20%7C%20zsh%20%7C%20vim-39ff14.svg)](#targets)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

### `[SIGNAL // POWERLINE WITHOUT THE PYTHON IMPORT COST]`

> *// jacking your prompt off the python interpreter — same segments, same theme grammar, native exec speed //*

---

## `> SYSTEM OVERVIEW`

**powerliners** is a Rust port of [`powerline-status`](https://github.com/powerline/powerline) — the canonical Python-driven statusline/prompt renderer used in tmux, zsh, bash, vim, ipython, and shell continuation lines. The Python implementation pays a ~50–150 ms interpreter-startup tax on every render (every prompt redraw, every tmux refresh). powerliners is a single static binary: zero-import, zero-GC, sub-millisecond render.

Drop-in compatible with the existing `powerline/config` JSON theme + segment files so users can keep their themes unchanged.

---

## `> WHY A PORT?`

```
[x] python startup is the killer — ~100 ms per render on the default tmux+powerline setup
[x] tmux refreshes the statusline every interval, and per-window — startup cost compounds
[x] zsh's prompt redraws after every keystroke when `precmd` hooks fire
[x] a 100 ms latency tax on every keystroke-induced redraw turns interactive shells into slideshows
[x] rust gives us: a static binary, microsecond startup, zero runtime deps, cross-arch builds
[x] preserve the exact powerline theme grammar — users keep their .json themes verbatim
```

---

## `> TARGETS`

```
[x] tmux statusline / continuation lines
[x] zsh prompt (PS1 / RPROMPT)
[x] bash prompt (PS1 / PROMPT_COMMAND)
[x] vim statusline
[x] ipython / python REPL prompt (via shell hook, not embedded)
```

---

## `> STATUS`

```
[port progress]   134 / 137 upstream .py files at DONE tier (97.8%)
[remaining]       3 NEAR — class-only Python sources at classifier ceiling
[partial/sparse]  0 / 0 — no degraded files
[lib tests]       1879 passing, 0 failing, 0 ignored
[parity tests]    219 against live upstream Python — every assertion runs the
                  Python interpreter on the vendored powerline and compares
                  byte/value identical with the Rust port
[port bugs fixed] 11 surfaced by the parity harness and corrected in the
                  Rust port (see git log for the full list)
[drift gate]      green — every ported fn name matches docs/powerline_py_functions.txt
[citation rule]   every Rust body line annotated // py:NNN against the upstream source line
```

The port is structurally complete at the function level. Citation-density
tier classifier (`scripts/gen_port_checklist.py`) requires `// py:NNN`
citation density >= 0.5 plus a `/// Port of <py_fn>()` doccomment per
Python function for DONE classification. All upstream Python files with
function bodies are at DONE.

The 3 remaining NEAR files (`renderers/shell/readline.py`,
`renderers/shell/zsh.py`, `bindings/i3/powerline-i3.py`) are class-only
Python sources with `py_methods == 0` — the classifier routes class-only
files through a NEAR-or-STUB-HEAVY branch (NEAR when
`rs_port_doccomments >= py_classes`), bypassing the citation-density check.
These files' Rust ports are complete and the classifier acknowledges them
as NEAR; promoting them to DONE would require a classifier amendment.

### What's wired end-to-end

| Binary | Mirrors | What it does |
|---|---|---|
| `powerliners` | new — combined demo CLI | `version` / `attached-clients` / `tmux-version` / `humanize-bytes <N>` |
| `powerline-config` | `scripts/powerline-config` | tmux / shell known-function dispatch |
| `powerline-lint` | `scripts/powerline-lint` | argparse + check pipeline (markedjson loader + Spec checks live; orchestrator integration partial) |
| `powerline-render` | `scripts/powerline-render` | argparse + ext lookup (full render path depends on the `Powerline` orchestrator) |
| `powerline-daemon` | `scripts/powerline-daemon` | UNIX-socket bind + daemonize + pidfile lock + accept loop + EOF shutdown + end-to-end statusline rendering against a real `~/.config/powerline/themes/...` JSON tree |

### End-to-end render

`powerline-daemon` produces real `#[fg=…,bg=…]…` tmux markup from a
user's powerline config root via the wire format compatible with the
upstream Python `powerline` C client. The render path covers:

- Config cascade load (`_find_config_files` + `load_json_config` + `mergedicts`)
- Colorscheme alias chasing + cterm color resolution (`Colorscheme::get_highlighting`)
- Segment preparation via `gen_segment_getter` returning a `Theme.segments` table
- Segment dispatch through `process_segment` / `process_segment_lister`
- Renderer loop (`do_render` / `_render_segments` / `_render_length`) with
  hard/soft divider insertion and per-side outer padding
- TmuxRenderer `#[…]` markup emission with `term_truecolor` cterm path

Adapters wired in `src/bin/powerline-daemon.rs` (sanctioned bin location)
cover the built-in segments: `hostname`, `date`, `cpu_load_percent`,
`system_load`, `uptime`, `external_ip`, `internal_ip`, `vcs.branch`,
`vcs.stash`, `bat.battery`, `net.network_load`, `players.spotify`, plus
`powerlinemem.mem_usage` as a darwin/linux platform probe.

Point it at a config root via `POWERLINE_CONFIG_PATHS`:

```sh
POWERLINE_CONFIG_PATHS=~/.config/powerline ./target/debug/powerline-daemon --foreground --socket /tmp/powerliners
```

The Python `powerline` C client already installed via pip talks to it
unchanged — same `argc\0arg\0arg\0cwd\0KEY=VAL\0...\0\0` wire format
and same `EOF\0\0` shutdown sentinel.

Regenerate the per-file tier table from the live source via:

```sh
python3 scripts/gen_port_checklist.py > docs/PORT_CHECKLIST.md
```

Regenerate the function-coverage report via:

```sh
python3 scripts/gen_port_report.py
```

---

## `> MIGRATION TUTORIAL`

Drop-in replacement for the Python `powerline-daemon`. The C client
shipped with `powerline-status` (installed via `pip install
powerline-status`) talks to our daemon unchanged — same wire format,
same `EOF\0\0` shutdown, same `/tmp/powerline-ipc-$UID` socket path.

### Step 1: Build the Rust daemon

```sh
git clone https://github.com/MenkeTechnologies/powerliners
cd powerliners
cargo build --release --bin powerline-daemon
```

The release binary lands at `target/release/powerline-daemon`.

### Step 2: Verify parity against your config

Before swapping anything, run the daemon against a copy of your real
config root and confirm the rendered tmux markup matches what you
currently see:

```sh
# Spawn our daemon on a throwaway socket
POWERLINE_CONFIG_PATHS=~/.config/powerline \
  ./target/release/powerline-daemon \
  --foreground \
  --socket /tmp/powerliners-probe &

# Render the right side via the wire protocol
python3 <<'PY'
import socket
s = socket.socket(socket.AF_UNIX)
s.connect("/tmp/powerliners-probe")
s.send(b"2\x00tmux\x00right\x00/tmp\x00HOME=/tmp\x00\x00")
print(s.recv(8192).decode("utf-8", "replace"))
PY

# Compare against the Python upstream (powerline-status must be installed)
powerline-render tmux right -p ~/.config/powerline
```

If the two outputs match byte-for-byte for your common segments,
proceed. If they don't, file an issue with the divergence — the suite
covers 36 byte-for-byte scenarios but real configs hit
combinations we haven't asserted on.

### Step 3: Stop the Python daemon

```sh
powerline-daemon -k
```

### Step 4: Replace the binary in `$PATH`

The simplest approach is symlinking the Rust binary into a directory
that comes before `powerline-status`'s `~/.local/bin` (or wherever
`pip` put it) in your `$PATH`:

```sh
ln -sf "$(pwd)/target/release/powerline-daemon" /usr/local/bin/powerline-daemon
# verify the new resolution
which powerline-daemon   # must report the symlink, not the Python script
```

The Python C client at `~/.local/bin/powerline` (or `powerline-render`
as fallback) does NOT need to be replaced — it speaks the same wire
protocol to whichever daemon is bound to the socket.

### Step 5: Restart tmux

Your existing `~/.tmux.conf` invocations work unchanged. The
canonical line:

```tmux
run-shell -b "powerline-daemon -q &>/dev/null || exit 0"
```

now spawns the Rust binary. Kill and reattach tmux to confirm:

```sh
tmux kill-server
tmux new-session
```

The status bar should look identical. The `powerline-daemon` process
in `ps aux` should now be a `target/release/powerline-daemon` invocation
rather than the Python shebang.

### Step 6: Confirm

```sh
ps -p $(cat /tmp/powerline-ipc-$UID.pid) -o args=
# expected: /usr/local/bin/powerline-daemon -q
ls -la $(which powerline-daemon)
# expected: lrwxr-xr-x  ... -> .../target/release/powerline-daemon
```

### Rollback

Re-resolve `powerline-daemon` to the Python script:

```sh
powerline-daemon -k
rm /usr/local/bin/powerline-daemon
which powerline-daemon   # should now resolve to ~/.local/bin/powerline-daemon (Python)
powerline-daemon -q
```

Everything is byte-compatible — no config edits, no `.tmux.conf` edits,
no shell-rc edits.

### Known divergences from Python upstream

These are the *only* areas where output may differ. Each is documented
under the test suite's "inherent divergence" notes:

1. **Live-data segments** (`cpu_load_percent`, `network_load`, ...) are
   sampled per-render via subprocess probes in our daemon; the Python
   upstream uses `psutil` with a different sampling cadence. Numeric
   values may differ by a sampling-window's worth of data; the
   markup framing is identical.
2. **Threaded segment caching**: Python's `ThreadedSegment` polls in a
   background thread and renders the last-known value; our daemon
   samples on-demand. Latency profile differs (we may block briefly
   when network/disk segments hit); output content matches.
3. **psutil-only features**: Python upstream errors loudly when
   `psutil` is missing and skips affected segments. Our daemon resolves
   the same data via OS subprocess probes (`top`, `vm_stat`,
   `netstat`, `pmset`, `uptime`) and renders successfully.
4. **`-m mode` propagation**: The Rust client argv parser doesn't yet
   route `-m insert` through to the renderer's mode parameter; without
   an explicit mode, `mode_translations` colorscheme groups are inert
   (matching Python's behavior with no mode).

For everything else — markup, escaping (`#` → `##[]`, control chars
via `translate_np`), dividers (hard/soft/multi-char/empty/single-char),
colorscheme resolution (alias chains, fallback groups, gradients,
cterm/truecolor encoding with falsy-hex fallback), attrs (bold +
italics + underline bit-packed), `outer_padding`, `spaces`, left/right
side handling, empty sides, `before`/`after` wrapping, Unicode contents
— the byte stream is identical.

---

## `> LICENSE`

[MIT](https://opensource.org/licenses/MIT). Theme JSON files in `powerline/config/themes/` remain under their upstream licenses.

---

<p align="center">
<code>// END OF FILE // PROMPT LOCKED, NATIVE //</code>
</p>
