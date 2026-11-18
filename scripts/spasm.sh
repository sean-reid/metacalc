#!/bin/sh
# Usage: spasm.sh [spasm options] source output
# Runs $SPASM (default spasm) and passes its output through. spasm reports an undefined label,
# an unknown operand and other errors without failing, so the run fails when spasm exits
# nonzero or when any output line is an error or warning. On failure the output file is removed.
set -u

out=
for arg in "$@"; do out=$arg; done

log=$(mktemp) || exit 1
trap 'rm -f "$log"' EXIT

"${SPASM:-spasm}" "$@" > "$log" 2>&1
status=$?
cat "$log"

esc=$(printf '\033')
if [ "$status" -eq 0 ] && sed "s/$esc\\[[0-9;]*m//g" "$log" | grep -Eq ': (error|warning)( S[EW][0-9A-F]{3})?: '; then
	echo "spasm.sh: spasm reported an error or warning" >&2
	status=1
fi
if [ "$status" -ne 0 ]; then
	[ "$#" -ge 2 ] && rm -f "$out"
fi
exit "$status"
