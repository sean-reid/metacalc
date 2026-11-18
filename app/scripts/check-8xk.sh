#!/bin/sh
# Usage: check-8xk.sh app.8xk
set -eu
export LC_ALL=C

f=${1:?usage: check-8xk.sh app.8xk}
fail() { echo "check-8xk: $f: $*" >&2; exit 1; }

[ -s "$f" ] || fail "missing or empty"
[ "$(head -c 8 "$f")" = "**TIFL**" ] || fail "not a TIFL flash file"

name=$(dd if="$f" bs=1 skip=17 count=8 2>/dev/null)
[ "$name" = "metacalc" ] || fail "file header name is '$name', want 'metacalc'"

# Data bytes of every type-00 Intel hex record, as one hex string.
body=$(grep -a -E '^:[0-9A-F]{6}00' "$f" | tr -d '\r' |
	sed -E 's/^:[0-9A-F]{6}00(.*)..$/\1/' | tr -d '\n')

# Name field 80 48 "metacalc" in the app header.
case $body in
	800F*80486D65746163616C63*) ;;
	*) fail "app header has no 'metacalc' name field" ;;
esac

# The signer appends 02 2D 40, 64 signature bytes, a 00 or 01 xx trailer, then FF padding.
sig=$(printf '%s' "$body" | grep -oE '022D40[0-9A-F]{128}(00|01[0-9A-F]{2})F*$' | cut -c7-134 || true)
[ -n "$sig" ] || fail "no signature field"
[ -n "$(printf '%s' "$sig" | tr -d '0')" ] || fail "signature is all zero"

echo "check-8xk: ok: $f ($(wc -c < "$f" | tr -d ' ') bytes, name $name, 64-byte signature)"
