#!/usr/bin/env bash
# Run lainify tasks inside the Nix dev shell (shell.nix).
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"

usage() {
	cat <<'USAGE'
Usage: ./run.sh [command] [args...]

  dev       start the dev server on http://localhost:5173 (default)
  build     production build into build/
  preview   build, then serve build/ on http://localhost:4173
  check     type-check
  test      unit tests
  lint      check formatting
  format    format all files
  docker    build and start the Docker image (http://localhost:8080)

Extra args are passed through, e.g. ./run.sh dev --host
USAGE
}

cmd="${1:-dev}"
[[ $# -gt 0 ]] && shift

case "$cmd" in
	help | -h | --help)
		usage
		exit 0
		;;
	docker)
		# Runs on the host's Docker, no Nix shell needed.
		exec docker compose up --build "$@"
		;;
	dev | build | preview | check | test | lint | format) ;;
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

# Install dependencies on first run or when the lockfile changed since the last install.
if [[ ! -f node_modules/.modules.yaml || pnpm-lock.yaml -nt node_modules/.modules.yaml ]]; then
	pnpm install --frozen-lockfile
	# pnpm leaves the marker untouched when nothing changed; bump it so we don't reinstall every run.
	touch node_modules/.modules.yaml
fi

case "$cmd" in
	preview)
		pnpm build
		exec pnpm preview "$@"
		;;
	*) exec pnpm "$cmd" "$@" ;;
esac
