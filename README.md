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

[![Status](https://img.shields.io/badge/status-134%2F137%20DONE-39ff14.svg)](#-status)
[![Tests](https://img.shields.io/badge/lib%20tests-2473%20passing-39ff14.svg)](#-status)
[![Parity](https://img.shields.io/badge/parity%20tests-462%20vs%20upstream-05d9e8.svg)](#-status)
[![Bugs Fixed](https://img.shields.io/badge/port%20bugs%20fixed-11-d300c5.svg)](#-status)
[![Source](https://img.shields.io/badge/port_of-powerline--status-05d9e8.svg)](https://github.com/powerline/powerline)
[![Language](https://img.shields.io/badge/lang-rust-d300c5.svg)](https://www.rust-lang.org/)
[![Target](https://img.shields.io/badge/target-tmux%20%7C%20zsh%20%7C%20vim-39ff14.svg)](#-targets)
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
[remaining]       3 lipstick-only py files — zero-fn class shells, see Status
[partial/sparse]  0 / 0 — no degraded files
[lib tests]       2473 passing, 0 failing, 0 ignored
[parity tests]    462 against live upstream Python — every assertion runs the
                  Python interpreter on the upstream powerline source and
                  compares byte/value identical with the Rust port
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

The 3 remaining NEAR files are **Python lipstick** — there is no
behavior left to port:

- `renderers/shell/readline.py` (14 LOC) — a class shell with two string
  constants (`escape_hl_start = '\x01'`, `escape_hl_end = '\x02'`) and
  one module-level alias. `py_fn_total == 0`.
- `renderers/shell/zsh.py` (16 LOC) — same shape, different escape
  constants. `py_fn_total == 0`.
- `bindings/i3/powerline-i3.py` (52 LOC) — one `render` function which
  is already **100% ported** (see `docs/port_report.html`); the
  classifier still tags it NEAR because the file structure trips its
  class-only branch.

These exist in upstream because Python's renderer registry needs a
class per shell flavour to subclass `ShellRenderer`; the Rust port
holds the same escape-marker constants directly on the equivalent
renderer struct without ceremony. The classifier's `py_fn_total == 0`
denominator simply can't promote 0/0 → DONE — a classifier amendment
would resolve it cosmetically, but there is no real port gap. The
remaining behavioral surface (every Python function with a body) is
at DONE.

### What's wired end-to-end

| Binary | Mirrors | What it does |
|---|---|---|
| `powerline` | `client/powerline.c` | Native Rust client — forwards `argv + cwd + env` to the daemon over a Unix socket via the upstream wire format, falls back to `powerline-render` exec if the daemon is unreachable |
| `powerline-config` | `scripts/powerline-config` | tmux / shell known-function dispatch |
| `powerline-lint` | `scripts/powerline-lint` | argparse + full check pipeline (markedjson loader + Spec checks + orchestrator integration) |
| `powerline-render` | `scripts/powerline-render` | argparse + ext lookup + full direct-render path through the `Powerline` orchestrator (used as daemon-less fallback) |
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

54 segment adapters wired in `src/bin/shared/render_runtime.rs` (55
`ADAPTERS` keys including the bare `exec` alias; shared
between `powerline-daemon` and `powerline-render`): `battery`, `branch`,
`clementine`, `cmus`, `containers`, `cpu_load_percent`, `cwd`, `date`,
`dbus_player`, `disk_io`, `disk_usage`, `disk_usage_percent`,
`email_imap_alert`, `environment`, `exec`, `external_ip`, `fuzzy_time`,
`gpu_usage_percent`, `gpu_vram`, `hostname`, `internal_ip`, `itunes`,
`jobnum`, `kubecontext`, `last_pipe_status`, `last_status`, `mem_swap`,
`mem_swap_percentage`, `mem_usage`, `mem_usage_percent`, `mocp`, `mpd`,
`network_load`, `process_count`, `rhythmbox`, `spotify`, `stash`,
`system_load`, `thermal`, `uptime`, `user`, `virtualenv`, `weather`,
plus `git_status`, `ci_status`, `aws.context`, `gcp.context`,
`fusevm.jit_cache`, and the `rkyv_cache` / `version` adapters for
`zshrs`, `stryke`, and `awkrs`.

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
same `EOF\0\0` shutdown, same socket path (`/tmp/powerline-ipc-$UID`
on macOS / BSD, abstract `\0powerline-ipc-$UID` on Linux).

### Step 1: Install or build

Fastest path — Homebrew tap (auto-bumped by each release):

```sh
brew tap MenkeTechnologies/menketech
brew install powerliners
# installs powerline, powerline-daemon, powerline-config, powerline-render, powerline-lint
```

Or build from source (entire 5-binary suite):

```sh
git clone https://github.com/MenkeTechnologies/powerliners
cd powerliners
cargo build --release --locked \
  --bin powerline --bin powerline-daemon \
  --bin powerline-config --bin powerline-render --bin powerline-lint
```

Release binaries land at `target/release/{powerline,powerline-daemon,powerline-config,powerline-render,powerline-lint}`.

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
proceed. If they don't, file an issue with the divergence — the
`daemon_parity` suite covers 45 byte-for-byte scenarios but real
configs hit combinations we haven't asserted on.

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
canonical lines:

```tmux
run-shell "powerline-config tmux setup"
run-shell -b "powerline-daemon -q &>/dev/null || exit 0"
```

`powerline-config tmux setup` is **install-method-agnostic** as of
0.2.3 — the 8 tmux conf files (`powerline-base.conf` plus 7
version-specific variants) are embedded into the binary via
`include_str!` and extracted to `$XDG_CACHE_HOME/powerliners/tmux/`
(default `~/.cache/powerliners/tmux/`) on first call. Works
identically for `cargo install`, `brew install`, and manual `cp`
into `$PATH` — no compile-time path baking required.

Kill and reattach tmux to confirm:

```sh
tmux kill-server
tmux new-session
```

The status bar should look identical. The `powerline-daemon` process
in `ps aux` should now be a Rust-port invocation rather than the
Python shebang.

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

---

## `> VIM SETUP`

Vim statusline + tabline rendering ships in the same `powerline` binary
— no `+python3` requirement, no `pip install powerline-status`. The
bundled `powerline.vim` plugin is embedded in the binary via
`include_str!` and extracted to `~/.cache/powerliners/vim/powerline.vim`
on first source.

Works on vim 7.4+, vim 9, and neovim — the plugin uses vim8-compatible
syntax only (no `vim9script` lock-in).

### Step 1: Source the plugin from `.vimrc`

The one-line install that extracts and sources the plugin on every vim
launch:

```vim
if executable('powerline-config')
  execute 'source' trim(system('powerline-config vim source-path'))
endif
```

`powerline-config vim source-path` extracts the bundled plugin and
prints its path; the `:execute 'source' …` then sources it. Caches in
`$XDG_CACHE_HOME/powerliners/vim/` (default `~/.cache/powerliners/vim/`)
so subsequent vim launches re-source from the already-extracted file.

Or the manual two-step (pin the path, skip the per-launch fork):

```sh
# one-time extraction
powerline-config vim source-path
# /Users/<you>/.cache/powerliners/vim/powerline.vim
```

```vim
" .vimrc
set runtimepath+=~/.cache/powerliners/vim
source ~/.cache/powerliners/vim/powerline.vim
```

### Step 2: Pre-start the daemon (recommended)

The plugin shells out via `system('powerline vim left …')` on every
statusline refresh. With the daemon running, that's a microsecond
Unix-socket round-trip; without it, every refresh fork-execs
`powerline-render` which is ~30 ms cold per call. Start the daemon at
shell login (zsh `.zshrc` or bash `.bash_profile`):

```sh
powerline-daemon -q
```

The `-q` flag double-forks and detaches; vim sees the socket
immediately. No per-vim daemon — one process per UID handles tmux,
shell prompts, and every running vim simultaneously.

### Step 3: Verify

Open a fresh vim session and check:

```vim
:echo &statusline
" expected: a %#Pl_…# markup string ending in segment chunks

:PowerlinersRefresh
" manually re-runs the refresh; useful for debugging

:messages
" no errors should appear; if 'powerline: write() to daemon failed'
" shows up, the daemon isn't reachable — re-run `powerline-daemon -q`
```

The plugin sets `laststatus=2` automatically so the statusline shows
even in single-window sessions.

### What the plugin wires up

Triggered autocmds (`augroup powerliners`):

| Event | When it fires |
|---|---|
| `VimEnter` | initial render on launch |
| `WinEnter` / `BufWinEnter` / `BufEnter` / `TabEnter` | refresh on context switch |
| `ModeChanged` (vim ≥ 8.2.2871) | mode transitions (normal → insert etc) |
| `CursorMoved` / `CursorMovedI` (legacy fallback) | every cursor move on ancient vim |

Per-request keys sent to the renderer (matches upstream's
`powerline.bindings.vim` so theme JSON written for Python upstream
renders identically here):

- `mode` — current vim mode (`n` / `i` / `v` / `R` / …)
- `bufnr` — `bufnr('%')`
- `winnr` — `winnr()`
- `buf_name` — `expand('%:p')` (omitted when empty)

The bundled `powerline.vim` shows the actual wiring at the source
path you printed above.

### Override the binary name

If you've installed under a non-default name or want to test a build
from `target/release/`, set the global before sourcing the plugin:

```vim
let g:powerliners_binary = expand('~/code/powerliners/target/release/powerline')
if executable(g:powerliners_binary)
  execute 'source' trim(system(g:powerliners_binary . '-config vim source-path'))
endif
```

### Customize the theme

The vim statusline pulls from
`~/.config/powerline/themes/vim/default.json` plus
`~/.config/powerline/colorschemes/vim/default.json`. The bundled
defaults live under `src/ported/config_files/themes/vim/` and
`src/ported/config_files/colorschemes/vim/` for reference; copy any
of them into `~/.config/powerline/` and edit.

Same JSON shape as upstream powerline — segments listed under
`segments.left` / `segments.right`, theme inheritance via
`extends`, per-mode highlight overrides via `mode_translations`.

### Troubleshooting

| Symptom | Cause / fix |
|---|---|
| empty statusline | `set laststatus=2` got stomped by something else in `.vimrc`; re-set after sourcing |
| garbled escape codes | terminal doesn't support truecolor; renderer falls back to cterm but vim must be in a 256-color tty (`$TERM=xterm-256color`) |
| colors don't match terminal | colorscheme JSON missing your custom palette — copy `colorschemes/vim/default.json` into `~/.config/powerline/colorschemes/vim/` |
| `powerline: write() to daemon failed` | daemon isn't running; run `powerline-daemon -q` |
| refresh stutters / flickers | the legacy `CursorMoved` fallback is firing on every keystroke (vim < 8.2.2871) — upgrade vim or pin a `let g:powerliners_no_cursormoved = 1` patch |
| `E121: Undefined variable: g:powerliners_binary` before source line | `g:powerliners_binary` is set BY the plugin; reference it only inside autocmds that fire after sourcing |

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

For everything else — markup, escaping (`#` → `##[]`, control chars
via `translate_np`), dividers (hard/soft/multi-char/empty/single-char),
colorscheme resolution (alias chains, fallback groups, gradients,
cterm/truecolor encoding with falsy-hex fallback), attrs (bold +
italics + underline bit-packed), `outer_padding`, `spaces`, left/right
side handling, empty sides, `before`/`after` wrapping, Unicode contents
— the byte stream is identical.

---

## `> CUSTOM SEGMENTS`

Upstream Python powerline lets you drop a `.py` file in
`~/.config/powerline/segments/` and reference it by dotted-path in
theme JSON; `__import__` makes it callable. The Rust binary has no
dynamic import, so the dispatch layer in
`src/extensions/exec_segment.rs` (`src/bin/shared/render_runtime.rs`
adapter wiring) provides two equivalent surfaces that fall through
to a subprocess.

### Pattern A — explicit `exec` adapter

Reference the built-in `exec` adapter directly in theme JSON. Use
when you want the script path, args, and format string spelled out
inline:

```json
{
  "function": "exec",
  "args": {
    "command": "/usr/local/bin/cpu_temp.sh",
    "args": ["--unit", "C"],
    "format": "%s°C",
    "highlight_groups": ["cpu_load"]
  }
}
```

### Pattern B — dotted-path filesystem dispatch

Reference your segment by dotted path and drop the script under
`<config_path>/segments/`. The daemon resolves
`myseg.cpu_temp` →
`<config_path>/segments/myseg/cpu_temp.{sh,py,rb,pl,lua,js,executable}`
(first hit wins; order documented in
`src/extensions/exec_segment.rs::SCRIPT_EXTENSIONS`):

```json
{
  "function": "myseg.cpu_temp"
}
```

```sh
mkdir -p ~/.config/powerline/segments/myseg
cat > ~/.config/powerline/segments/myseg/cpu_temp.sh <<'EOF'
#!/bin/sh
echo "$(osx-cpu-temp | sed 's/°C//')"
EOF
chmod +x ~/.config/powerline/segments/myseg/cpu_temp.sh
```

The dotted-path resolution honors the daemon's full config-path
cascade (`POWERLINE_CONFIG_PATHS`, `--config-path`, `~/.config/`),
so segments dropped in any config root that's on the cascade
become available.

### Output protocol

Both patterns parse the script's stdout on the first non-whitespace
byte:

| Stdout starts with | Treated as | Wrapped how |
|---|---|---|
| `[` (valid JSON array) | Verbatim segment list | Used directly — full control over `highlight_groups`, `gradient_level`, `divider_highlight_group`, multi-chunk output |
| anything else | Plain text | `[{"contents": <trimmed-stdout>, "highlight_groups": [...]}]` — `format` template applies (`%s` → contents, `%%` → literal `%`) |

Plain-text scripts:

```sh
#!/bin/sh
echo "CPU $(cat /proc/loadavg | cut -d' ' -f1)"
```

JSON scripts (control gradient color, attach divider group, emit
multiple chunks):

```python
#!/usr/bin/env python3
import json, psutil
print(json.dumps([
    {"contents": "🌡 ", "highlight_groups": ["thermal_icon"]},
    {"contents": f"{psutil.cpu_percent():.0f}%",
     "highlight_groups": ["thermal_gradient", "background"],
     "gradient_level": psutil.cpu_percent()}
]))
```

### Highlight-group caveat (Rust-port divergence)

The current Rust port's `gen_segment_getter` (at
`src/ported/segment.rs:1011-1023`) uses the segment's `function_name`
as the only highlight group for `"type": "function"` segments — it
**ignores** the theme's `"highlight_groups"` override. Upstream Python
respects the override.

Practical impact: name your colorscheme group to match the function
name (`"exec"` for pattern A, `"cpu_temp"` for pattern B), e.g.

```json
"groups": {
  "exec": { "fg": "white", "bg": "gray0", "attrs": [] },
  "cpu_temp": { "fg": "yellow", "bg": "black", "attrs": [] }
}
```

Pattern B sidesteps this nicely because the dotted path's trailing
component (the function name) is also a natural colorscheme group
name. Pattern A requires either a literal `"exec"` group, or pinning
the highlight via a JSON-array stdout that emits explicit
`highlight_groups` inline.

### Bundled extensions

`src/extensions/` ships several net-new segments above what upstream
powerline-status offers, dispatched as standard built-ins (no
filesystem lookup needed):

| Dotted path | What |
|---|---|
| `powerlinemem.mem_usage.mem_usage` | 1:1 port of `mKaloer/powerline_mem_segment`'s `USED/TOTAL` formatted bytes |
| `powerlinemem.mem_usage.mem_usage_percent` | Percentage variant |
| `powerlinemem.mem_usage.mem_swap` | Swap `USED/TOTAL` |
| `powerlinemem.mem_usage.mem_swap_percentage` | Swap percent |
| `powerliners.disk.disk_usage` | Filesystem `USED/TOTAL` for any mount |
| `powerliners.disk.disk_usage_percent` | Filesystem percent-used |
| `powerliners.disk.disk_io` | Live read/write throughput for any device |
| `powerliners.gpu.gpu_usage_percent` | Vendor-dispatched GPU compute percent (nvidia-smi → rocm-smi → intel_gpu_top → ioreg fallback) |
| `powerliners.gpu.gpu_vram` | GPU VRAM `USED/TOTAL` via same dispatch chain |
| `powerliners.thermal.thermal` | CPU/GPU temp + fan RPM (`/sys/class/hwmon` on Linux, `powermetrics`/`istats` on macOS) |
| `powerliners.vcs.git_status` | p10k-style single-chunk VCS segment — branch glyph + name + count badges for unstaged/untracked/staged/conflicts/ahead/behind/stashed from one `git status --porcelain=v2 --branch` fork; detached HEAD falls back to short SHA / tag; omitted outside a git work tree |
| `powerliners.docker.containers` | Docker / OCI container counts (`{running}`/`{total}`/`{images}`/`{stopped}` tokens; probes via `docker ps`, falls through silently when the daemon is unreachable) |
| `powerliners.k8s.kubecontext` | Current kubectl context + active namespace (honors `$KUBECONFIG` cascade and in-context `kubectl config set-context --namespace` overrides; `hide_default` arg suppresses the namespace when it equals the configured default) |
| `powerliners.proc.process_count` | POSIX process tally via `ps -eo stat=` (`{total}`/`{running}`/`{sleeping}`/`{zombie}`/`{dwait}`/`{stopped}` tokens; `warn_zombie` flips the highlight group when defunct processes are present) |
| `powerliners.github.ci_status` | Current branch's HEAD check-runs via `gh api repos/:o/:r/commits/:sha/check-runs`, cached on disk by SHA (`ttl_secs` default 30). Tokens: `{icon}`/`{state}`/`{passed}`/`{failed}`/`{running}`/`{total}`. Highlight groups split into `github_ci_success` / `github_ci_failure` / `github_ci_pending`. Falls through silently outside a GitHub work tree or when `gh` is missing |
| `powerliners.aws.context` | Active AWS profile + region. Pure-fs probe of `$AWS_PROFILE`/`$AWS_REGION`/`$AWS_DEFAULT_REGION` then `~/.aws/config` (honors the `[profile NAME]` quirk). Tokens: `{icon}`/`{profile}`/`{region}`. `hide_default_profile` drops `{profile}` + adjacent separator when profile is `default` |
| `powerliners.gcp.context` | Active gcloud configuration's project + account. Pure-fs probe of `~/.config/gcloud/active_config` + `configurations/config_<NAME>` (`[core]` section). Env overrides `$CLOUDSDK_ACTIVE_CONFIG_NAME` / `$CLOUDSDK_CORE_PROJECT` / `$CLOUDSDK_CORE_ACCOUNT` win. Tokens: `{icon}`/`{project}`/`{account}`/`{config}`. `hide_account` strips the account fragment |
| `powerliners.fusevm.jit_cache` | fusevm Cranelift JIT cache stats (entry count + bytes) for the zshrs/stryke runtime. Recursively walks `path` (theme arg) or `$FUSEVM_JIT_CACHE` / `$XDG_CACHE_HOME/fusevm-jit` / `~/.cache/fusevm-jit`. Tokens: `{icon}`/`{entries}`/`{size}` (`du -sh` block allocation, human K/M/G)/`{bytes}` (raw disk bytes)/`{logical_size}`/`{logical_bytes}` (stat-style content sum). `show_when_empty` controls whether a missing/cold cache still renders |
| `powerliners.zshrs.rkyv_cache` | Single-file stat of the zshrs authoritative rkyv archive at `$ZSHRS_RKYV_CACHE` / `$ZSHRS_HOME/scripts.rkyv` / `$XDG_DATA_HOME/zshrs/scripts.rkyv` / `~/.zshrs/scripts.rkyv`. Same `{size}` / `{bytes}` / `{logical_*}` token surface as `fusevm.jit_cache`. Disk-bytes default matches `du -sh` |
| `powerliners.stryke.rkyv_cache` | Single-file stat of `~/.stryke/scripts.rkyv` (stryke's authoritative bytecode store, Cranelift-JIT'd via the shared fusevm runtime). Same token surface |
| `powerliners.awkrs.rkyv_cache` | Single-file stat of `~/.awkrs/scripts.rkyv`. Same token surface |
| `powerliners.zshrs.version` | Latest installed zshrs version (parsed from `<bin> --version`). In-process TTL cache (default 300 s) so the daemon doesn't fork on every prompt tick. Tokens: `{icon}`/`{version}` |
| `powerliners.stryke.version` | Same for stryke (handles the `This is stryke vX.Y.Z — ...` prefix shape). |
| `powerliners.awkrs.version` | Same for awkrs. |
| `powerliners.exec.exec` | The explicit `exec` adapter (also resolves via bare `"function": "exec"`) |

These each live in `src/extensions/<module>.rs` and are wired into
the daemon's `ADAPTERS` table — adding more follows the same pattern
(no new fn-name rules apply per `docs/PORT.md`'s `src/extensions/`
carve-out).

### `> Cache-size segments — shared resolution chain`

The four cache-size segments (`fusevm.jit_cache`, `zshrs.rkyv_cache`,
`stryke.rkyv_cache`, `awkrs.rkyv_cache`) share a uniform 4-level
resolution chain and an identical `{size}` / `{bytes}` / `{logical_size}` /
`{logical_bytes}` token surface so the same theme JSON works across all
four. Each is a pure filesystem probe — no subprocess, no daemon RPC.

For the rkyv segments, `<NAME> ∈ {ZSHRS, STRYKE, AWKRS}`:

1. `$<NAME>_RKYV_CACHE` — explicit override, used verbatim
2. `$<NAME>_HOME/scripts.rkyv`
3. `$XDG_DATA_HOME/<name>/scripts.rkyv` — used only when the file exists
4. `~/.<name>/scripts.rkyv` — final fallback

`fusevm.jit_cache` uses the analogous 3-level chain
(`$FUSEVM_JIT_CACHE` → `$XDG_CACHE_HOME/fusevm-jit` →
`~/.cache/fusevm-jit`) but recursively walks a cache directory rather
than stat-ing a single archive, and also tracks an `{entries}` token
for the recursive file count. Symlinks count as a single entry and are
never followed (avoids infinite loops across re-symlinked cache dirs).

The resolution chain is unit-tested via a pure-functional
`default_path_with(get_env, path_exists)` seam in
`src/extensions/{zshrs,stryke,awkrs}_rkyv.rs` and a matching
`default_root_with(get_env)` seam in `src/extensions/fusevm_jit.rs` —
no env-var mutation in tests, no thread-safety hazard, every
precedence level pinned.

Missing-archive behavior: returns `None` by default (no chunk renders).
Set `"show_when_empty": true` to render zeroed stats (`0B`/`0`) instead.

Highlight-group chain (3 levels): `<segment>_rkyv_cache` →
`<segment>` → `information:regular`. The trailing `information:regular`
is a neutral fallback so the chunk renders in any colorscheme.

---

## `> LICENSE`

[MIT](https://opensource.org/licenses/MIT). Theme JSON files in `powerline/config/themes/` remain under their upstream licenses.

---

<p align="center">
<code>// END OF FILE // PROMPT LOCKED, NATIVE //</code>
</p>
