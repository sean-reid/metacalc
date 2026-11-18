#!/bin/sh
# Usage: require-tests.sh min command [args...]
# Runs a cargo test command and fails when it fails or when the "N passed" counts in its output add
# up to less than min, which a filter that matches no test makes cargo exit 0 with. The command
# runs with colour off and escape sequences are stripped before counting.
set -eu

usage() {
	echo "usage: require-tests.sh min command [args...]" >&2
	exit 2
}

[ $# -ge 2 ] || usage
min=$1
shift
case $min in '' | *[!0-9]*) usage ;; esac

command -v "$1" >/dev/null 2>&1 || { echo "require-tests: cannot run $1" >&2; exit 127; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
export CARGO_TERM_COLOR=never
{ "$@" 2>&1 || echo $? >"$tmp/status"; } | tee "$tmp/out"
if [ -f "$tmp/status" ]; then
	exit "$(cat "$tmp/status")"
fi
esc=$(printf '\033')
passed=$(sed "s/$esc\[[0-9;]*[A-Za-z]//g" "$tmp/out" | sed -n 's/.*test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' | awk '{ n += $1 } END { print n + 0 }')
if [ "$passed" -lt "$min" ]; then
	echo "require-tests: $passed tests passed, want at least $min" >&2
	exit 1
fi
