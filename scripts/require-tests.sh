#!/bin/sh
# Usage: require-tests.sh min command [args...]
# Runs a cargo test command and fails when it fails or when the "N passed" counts in its output add
# up to less than min. A filter that matches no test makes cargo exit 0 with 0 passed.
set -eu

usage() {
	echo "usage: require-tests.sh min command [args...]" >&2
	exit 2
}

[ $# -ge 2 ] || usage
min=$1
shift
case $min in '' | *[!0-9]*) usage ;; esac

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
{ "$@" 2>&1 || echo $? >"$tmp/status"; } | tee "$tmp/out"
if [ -f "$tmp/status" ]; then
	exit "$(cat "$tmp/status")"
fi
passed=$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed.*/\1/p' "$tmp/out" | awk '{ n += $1 } END { print n + 0 }')
if [ "$passed" -lt "$min" ]; then
	echo "require-tests: $passed tests passed, want at least $min" >&2
	exit 1
fi
