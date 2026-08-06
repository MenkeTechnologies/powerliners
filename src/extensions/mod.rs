// vim:fileencoding=utf-8:noet
//! Features powerliners ships beyond the strict 1:1 port of
//! `powerline-status`. Sanctioned non-port location per `docs/PORT.md`.
//!
//! Currently houses:
//!
//! - [`mem_usage`] — 1:1 port of mKaloer's
//!   [`powerline_mem_segment`](https://github.com/mKaloer/powerline_mem_segment)
//!   plugin (`powerlinemem.mem_usage` Python module). Provides the 4
//!   adapter entrypoints the user's theme references by name:
//!     - `mem_usage`            → `USED/TOTAL` formatted bytes
//!     - `mem_usage_percent`    → `NN%` of total memory
//!     - `mem_swap`             → `USED/TOTAL` formatted swap bytes
//!     - `mem_swap_percentage`  → `NN%` of total swap
//!
//! These names are wired into `ADAPTERS` in
//! `src/bin/shared/render_runtime.rs` so the daemon's segment lookup
//! finds them without a Python `__import__` round-trip.
//!
//! - [`gpu`]        — `powerliners.gpu.{gpu_usage_percent,gpu_vram}`
//!   (vendor-dispatched: nvidia-smi → rocm-smi → ioreg → intel_gpu_top)
//! - [`disk`]       — `powerliners.disk.{disk_usage,disk_usage_percent,disk_io}`
//! - [`thermal`]    — `powerliners.thermal.thermal` (CPU/GPU temp + fan RPM)
//! - [`docker`]     — `powerliners.docker.containers` (running/total/images
//!   via `docker ps` / OCI-compatible CLI)
//! - [`k8s`]        — `powerliners.k8s.kubecontext` (current kubectl
//!   context + active namespace, honors KUBECONFIG cascade)
//! - [`proc_count`] — `powerliners.proc.process_count` (POSIX `ps -eo
//!   stat=` tally by state code: running/sleeping/zombie/dwait/stopped)
//! - [`wthr_extensions`] — IP-based geolocation + on-disk
//!   location cache for the weather segment. Upstream powerline takes
//!   `location_query` only; these helpers let the segment render
//!   without an explicit query and survive transient lookup failures.
//!   Imported by `src/ported/segments/common/wthr.rs`.
//! - [`github_ci`]  — `powerliners.github.ci_status` (current branch's
//!   HEAD check-runs via `gh api`, cached on disk by SHA)
//! - [`aws_ctx`]    — `powerliners.aws.context` (active AWS profile +
//!   region, pure-fs read of env + `~/.aws/config`)
//! - [`gcp_ctx`]    — `powerliners.gcp.context` (active gcloud config's
//!   project + account, pure-fs read of `~/.config/gcloud`)
//! - [`fusevm_jit`] — `powerliners.fusevm.jit_cache` (entry count +
//!   bytes under the fusevm Cranelift JIT cache root)
//! - [`tmux_exe`]   — resolves the multiplexer binary the tmux
//!   bindings drive (tmux vs `ztmux`), keyed off `$TMUX`'s socket path
//!   so `powerline-config tmux setup` configures the server the user is
//!   actually running.
//! - [`ipc_socket`] — Unix-socket `bind`/`connect` that routes the
//!   Linux default address (`\0powerline-ipc-<uid>`) into the abstract
//!   namespace, which Rust's path-based socket API rejects. Shared by
//!   the daemon and the `powerline` client.
//! - [`watch`]      — Reactive Prompt Push: the warm daemon watches each
//!   client's prompt inputs (cwd, `.git/HEAD`, `.git/index`, branch ref)
//!   with real OS events (`notify`: kqueue/FSEvents/inotify) and writes a
//!   one-byte wake to a per-client FIFO on a real change. Registered from
//!   the daemon binary's `render_fn`; the pull-only ported path is
//!   untouched.
//! - [`shell_hooks`] — shell-side bindings that create the wake FIFO and
//!   wire it into the line editor. `shell_hooks/reactive.zsh` uses
//!   `zle -F <fd>` → `zle reset-prompt` so the branch flips between
//!   keystrokes with no Enter and no interval timer.

pub mod awkrs_rkyv;
pub mod awkrs_version;
pub mod aws_ctx;
pub mod bin_version;
pub mod bundled_config;
pub mod diag_log;
pub mod disk;
pub mod docker;
pub mod elisprs_rkyv;
pub mod elisprs_version;
pub mod exec_segment;
pub mod fusevm_jit;
pub mod gcp_ctx;
pub mod git_status;
pub mod github_ci;
pub mod gpu;
pub mod icons;
pub mod ipc_socket;
pub mod k8s;
pub mod mem_usage;
pub mod proc_count;
pub mod shell_hooks;
pub mod stryke_rkyv;
pub mod stryke_version;
pub mod thermal;
pub mod tmux_exe;
pub mod vimlrs_rkyv;
pub mod vimlrs_version;
pub mod watch;
pub mod wthr_extensions;
pub mod zshrs_rkyv;
pub mod zshrs_version;
