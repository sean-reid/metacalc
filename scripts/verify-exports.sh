#!/bin/sh
# Usage: verify-exports.sh libdir
# Checks every libdir/*.mm with metamath-exe (METAMATH, default .metamath/metamath at the repo root).
# metamath-exe exits 0 whatever it finds, so a file passes only if its output has no line starting
# with "?" (every error and warning does), says the database was read without errors, reports at
# least one $p statement and ends with the line saying all proofs were verified.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
libdir=$1
metamath=${METAMATH:-$root/.metamath/metamath}

[ -x "$metamath" ] || { echo "verify-exports: $metamath is not an executable, run scripts/fetch-metamath.sh" >&2; exit 1; }
[ -d "$libdir" ] || { echo "verify-exports: $libdir is not a directory" >&2; exit 1; }
out=$(mktemp)
trap 'rm -f "$out"' EXIT

status=0
count=0
for file in "$libdir"/*.mm; do
	[ -e "$file" ] || break
	count=$((count + 1))
	case $file in *\"*) echo "verify-exports: $file has a quote in its name" >&2; exit 1 ;; esac
	"$metamath" "read \"$file\"" 'verify proof *' 'exit' </dev/null >"$out" 2>&1 || true
	proofs=$(sed -n 's/^The source has [0-9]* statements; [0-9]* are \$a and \([0-9]*\) are \$p\.$/\1/p' "$out")
	problem=
	if grep -q '^?' "$out"; then
		problem="the verifier reported problems"
	elif ! grep -q '^No errors were found\.' "$out"; then
		problem="the verifier did not report a clean read"
	elif [ -z "$proofs" ] || [ "$proofs" -eq 0 ]; then
		problem="the database has no proofs"
	elif ! grep -q '^All proofs in the database were verified in ' "$out"; then
		problem="the verifier did not report every proof verified"
	fi
	if [ -n "$problem" ]; then
		echo "FAIL $file: $problem" >&2
		grep -A8 '^?' "$out" | head -40 >&2 || true
		status=1
	else
		echo "ok $file: $proofs proofs verified"
	fi
done
if [ "$count" -eq 0 ]; then
	echo "verify-exports: no .mm files in $libdir" >&2
	exit 1
fi
exit $status
