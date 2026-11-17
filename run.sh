#!/usr/bin/env bash
# Run lainify tasks inside the Nix dev shell (shell.nix).
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

usage() {
	cat <<'USAGE'
Usage: ./run.sh [command] [args...]

  run       build (release) and start lainify (default)
  debug     build (debug) and start lainify, with Qt logging on stderr
  build     release build into target/release/lainify
  check     clippy
  fmt       format the Rust code
  package   nix build (standalone binary in ./result)

Extra args are passed through to the command.
USAGE
}

cmd="${1:-run}"
[[ $# -gt 0 ]] && shift

case "$cmd" in
	help | -h | --help)
		usage
		exit 0
		;;
	package)
		# Builds through the flake; no dev shell needed.
		exec nix build "$@"
		;;
	run | debug | build | check | fmt) ;;
	*)
		echo "run.sh: unknown command '$cmd'" >&2
		usage >&2
		exit 1
		;;
esac

# Re-enter through nix-shell unless we're already in a Nix shell (nix-shell, nix develop, direnv).
if [[ -z "${IN_NIX_SHELL:-}" ]]; then
	exec nix-shell --run "$(printf '%q ' ./run.sh "$cmd" "$@")"
fi

case "$cmd" in
	run) exec cargo run --release -- "$@" ;;
	debug) QT_FORCE_STDERR_LOGGING=1 exec cargo run -- "$@" ;;
	build) exec cargo build --release "$@" ;;
	check) exec cargo clippy --release "$@" ;;
	fmt) exec cargo fmt "$@" ;;
esac
