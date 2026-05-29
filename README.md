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

[![Status](https://img.shields.io/badge/status-EARLY-ff2a6d.svg)](#status)
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

This is an **early port stub**. The repo currently contains only the README announcing intent. Active development tracks the upstream powerline-status segment grammar; per-segment implementation lands incrementally.

Watch the repo for tagged milestones.

---

## `> LICENSE`

[MIT](https://opensource.org/licenses/MIT). Theme JSON files in `powerline/config/themes/` remain under their upstream licenses.

---

<p align="center">
<code>// END OF FILE // PROMPT LOCKED, NATIVE //</code>
</p>
