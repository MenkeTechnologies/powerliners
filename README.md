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
[![Tests](https://img.shields.io/badge/lib%20tests-2152%20passing-39ff14.svg)](#-status)
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
[remaining]       3 NEAR — class-only Python sources at classifier ceiling
[partial/sparse]  0 / 0 — no degraded files
[lib tests]       2152 passing, 0 failing, 0 ignored
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

43 segment adapters wired in `src/bin/shared/render_runtime.rs` (shared
between `powerline-daemon` and `powerline-render`): `battery`, `branch`,
`clementine`, `cmus`, `containers`, `cpu_load_percent`, `cwd`, `date`,
`dbus_player`, `disk_io`, `disk_usage`, `disk_usage_percent`,
`email_imap_alert`, `environment`, `exec`, `external_ip`, `fuzzy_time`,
`gpu_usage_percent`, `gpu_vram`, `hostname`, `internal_ip`, `itunes`,
`jobnum`, `kubecontext`, `last_pipe_status`, `last_status`, `mem_swap`,
`mem_swap_percentage`, `mem_usage`, `mem_usage_percent`, `mocp`, `mpd`,
`network_load`, `process_count`, `rhythmbox`, `spotify`, `stash`,
`system_load`, `thermal`, `uptime`, `user`, `virtualenv`, `weather`.

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
| `powerliners.docker.containers` | Docker / OCI container counts (`{running}`/`{total}`/`{images}`/`{stopped}` tokens; probes via `docker ps`, falls through silently when the daemon is unreachable) |
| `powerliners.k8s.kubecontext` | Current kubectl context + active namespace (honors `$KUBECONFIG` cascade and in-context `kubectl config set-context --namespace` overrides; `hide_default` arg suppresses the namespace when it equals the configured default) |
| `powerliners.proc.process_count` | POSIX process tally via `ps -eo stat=` (`{total}`/`{running}`/`{sleeping}`/`{zombie}`/`{dwait}`/`{stopped}` tokens; `warn_zombie` flips the highlight group when defunct processes are present) |
| `powerliners.github.ci_status` | Current branch's HEAD check-runs via `gh api repos/:o/:r/commits/:sha/check-runs`, cached on disk by SHA (`ttl_secs` default 30). Tokens: `{icon}`/`{state}`/`{passed}`/`{failed}`/`{running}`/`{total}`. Highlight groups split into `github_ci_success` / `github_ci_failure` / `github_ci_pending`. Falls through silently outside a GitHub work tree or when `gh` is missing |
| `powerliners.aws.context` | Active AWS profile + region. Pure-fs probe of `$AWS_PROFILE`/`$AWS_REGION`/`$AWS_DEFAULT_REGION` then `~/.aws/config` (honors the `[profile NAME]` quirk). Tokens: `{icon}`/`{profile}`/`{region}`. `hide_default_profile` drops `{profile}` + adjacent separator when profile is `default` |
| `powerliners.gcp.context` | Active gcloud configuration's project + account. Pure-fs probe of `~/.config/gcloud/active_config` + `configurations/config_<NAME>` (`[core]` section). Env overrides `$CLOUDSDK_ACTIVE_CONFIG_NAME` / `$CLOUDSDK_CORE_PROJECT` / `$CLOUDSDK_CORE_ACCOUNT` win. Tokens: `{icon}`/`{project}`/`{account}`/`{config}`. `hide_account` strips the account fragment |
| `powerliners.fusevm.jit_cache` | fusevm Cranelift JIT cache stats (entry count + bytes) for the zshrs/stryke runtime. Recursively walks `path` (theme arg) or `$FUSEVM_JIT_CACHE` / `$XDG_CACHE_HOME/fusevm/jit` / `~/.cache/fusevm/jit`. Tokens: `{icon}`/`{entries}`/`{size}` (human K/M/G)/`{bytes}` (raw). `show_when_empty` controls whether a cold cache still renders |
| `powerliners.exec.exec` | The explicit `exec` adapter (also resolves via bare `"function": "exec"`) |

These each live in `src/extensions/<module>.rs` and are wired into
the daemon's `ADAPTERS` table — adding more follows the same pattern
(no new fn-name rules apply per `docs/PORT.md`'s `src/extensions/`
carve-out).

---

## `> LICENSE`

[MIT](https://opensource.org/licenses/MIT). Theme JSON files in `powerline/config/themes/` remain under their upstream licenses.

---

<p align="center">
<code>// END OF FILE // PROMPT LOCKED, NATIVE //</code>
</p>
