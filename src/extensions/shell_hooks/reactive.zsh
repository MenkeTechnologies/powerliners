# vim:ft=zsh
# powerliners — Reactive Prompt Push (zsh binding)
# ------------------------------------------------------------------------
# Opens a FIFO that the warm `powerline-daemon` writes to whenever the
# inputs backing THIS shell's current prompt change (its cwd, .git/HEAD,
# .git/index, or the active branch ref). A single wake byte fires
# `zle reset-prompt`, redrawing the prompt in place — so the branch name
# flips the instant you `git checkout` in another pane, between
# keystrokes, with no Enter and no interval/PROMPT_COMMAND timer.
#
# Requires: a running `powerline-daemon` built with the reactive
# extension (it reads $POWERLINE_RESET_FIFO out of each render request's
# environment and watches on this shell's behalf).
#
# Install: source AFTER powerline's own zsh binding in ~/.zshrc, then:
#
#     source /path/to/reactive.zsh
#     _powerline_reactive_setup
#
# Tear down (e.g. before `exec`-ing a new shell) with:
#
#     _powerline_reactive_teardown
#
# The fd, FIFO, and directory are per-shell ($$), so multiple concurrent
# shells never collide.

_powerline_reactive_setup() {
	emulate -L zsh
	# Re-entrant: drop any prior wiring first.
	(( ${+POWERLINE_RESET_FIFO} )) && _powerline_reactive_teardown

	local dir=${TMPDIR:-/tmp}/powerliners-reactive-$$-$RANDOM
	local fifo=$dir/reset.fifo
	command mkdir -p -- $dir || return 1
	command mkfifo -- $fifo || { command rmdir -- $dir 2>/dev/null; return 1; }

	# The daemon receives this via the render request's environment.
	export POWERLINE_RESET_FIFO=$fifo
	export POWERLINE_RESET_DIR=$dir

	# Open BOTH ends on one fd (<>, i.e. O_RDWR). This makes the open
	# non-blocking (no wait for a writer) and guarantees a live reader so
	# the daemon's non-blocking write never gets ENXIO. Then hand the fd
	# to ZLE: it calls the widget whenever the fd becomes readable.
	exec {POWERLINE_RESET_FD}<>$fifo
	zle -F $POWERLINE_RESET_FD _powerline_reactive_wake
}

# ZLE fd-watcher widget. Called with the ready fd as $1.
_powerline_reactive_wake() {
	# Drain the wake byte(s) — coalesced changes may leave several — then
	# redraw the current prompt in place.
	local discard
	while IFS= read -r -k 1 -t 0 -u $1 discard 2>/dev/null; do :; done
	zle reset-prompt
}

_powerline_reactive_teardown() {
	emulate -L zsh
	if (( ${+POWERLINE_RESET_FD} )); then
		zle -F $POWERLINE_RESET_FD 2>/dev/null
		exec {POWERLINE_RESET_FD}>&- 2>/dev/null
	fi
	[[ -n $POWERLINE_RESET_FIFO ]] && command rm -f -- $POWERLINE_RESET_FIFO
	[[ -n $POWERLINE_RESET_DIR ]] && command rmdir -- $POWERLINE_RESET_DIR 2>/dev/null
	unset POWERLINE_RESET_FD POWERLINE_RESET_FIFO POWERLINE_RESET_DIR
}

# Best-effort cleanup when the shell exits.
autoload -Uz add-zsh-hook 2>/dev/null && add-zsh-hook zshexit _powerline_reactive_teardown
