#!/bin/sh
# Usage: verify-exports.sh libdir
# Checks every libdir/*.mm with metamath-exe (METAMATH, default .metamath/metamath at the repo root).
# Each NAME.mm needs a NAME.proofs file holding the number of $p statements it must have.
# metamath-exe exits 0 whatever it finds, so a file passes only if its output has no line starting
# with "?" (every error and warning does), no unproved statements, exactly one source summary,
# a clean read, as many $p statements as expected and the line saying all proofs were verified.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
libdir=$1
metamath=${METAMATH:-$root/.metamath/metamath}

[ -x "$metamath" ] || { echo "verify-exports: $metamath is not an executable, run scripts/fetch-metamath.sh" >&2; exit 1; }
[ -d "$libdir" ] || { echo "verify-exports: $libdir is not a directory" >&2; exit 1; }
out=$(mktemp)
trap 'rm -f "$out"' EXIT

fail() {
	echo "FAIL $1: $2" >&2
	status=1
}

status=0
databases=0
for file in "$libdir"/*.mm; do
	[ "$file" = "$libdir/*.mm" ] && [ ! -e "$file" ] && [ ! -L "$file" ] && break
	databases=$((databases + 1))
	if [ ! -f "$file" ] || [ ! -r "$file" ]; then
		fail "$file" "not a readable file"
		continue
	fi
	case $file in *\"*) fail "$file" "the name has a quote"; continue ;; esac
	want=
	[ -f "${file%.mm}.proofs" ] && want=$(cat "${file%.mm}.proofs")
	case $want in
	'' | *[!0-9]*) fail "$file" "no expected proof count in ${file%.mm}.proofs"; continue ;;
	esac
	"$metamath" "read \"$file\"" 'verify proof *' 'exit' </dev/null >"$out" 2>&1 || true
	summaries=$(grep -c '^The source has ' "$out" || true)
	proofs=$(sed -n 's/^The source has [0-9]* statements; [0-9]* are \$a and \([0-9]*\) are \$p\.$/\1/p' "$out" | head -n 1)
	problem=
	if grep -q '^?' "$out"; then
		problem="the verifier reported problems"
	elif ! grep -q '^No errors were found\.' "$out"; then
		problem="the verifier did not report a clean read"
	elif [ "$summaries" -ne 1 ] || [ -z "$proofs" ]; then
		problem="the verifier printed $summaries source summaries, want 1"
	elif [ "$proofs" -eq 0 ]; then
		problem="the database has no proofs"
	elif [ "$proofs" -ne "$want" ]; then
		problem="the database has $proofs proofs, want $want"
	elif grep -qi '^warning\|were not proved' "$out"; then
		problem="the database has unproved statements or warnings"
	elif ! grep -q '^All proofs in the database were verified in ' "$out"; then
		problem="the verifier did not report every proof verified"
	fi
	if [ -n "$problem" ]; then
		fail "$file" "$problem"
		grep -A8 '^?' "$out" | head -40 >&2 || true
	else
		echo "ok $file: $proofs proofs verified"
	fi
done

if [ "$databases" -eq 0 ]; then
	echo "verify-exports: no .mm files in $libdir" >&2
	exit 1
fi
counts=0
for file in "$libdir"/*.proofs; do
	[ -e "$file" ] || [ -L "$file" ] && counts=$((counts + 1))
done
if [ "$counts" -ne "$databases" ]; then
	fail "$libdir" "$databases .mm files but $counts .proofs files"
fi
exit $status
