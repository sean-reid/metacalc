#!/bin/sh
# Usage: check-8xk.sh [-p code-pages] [-l library] app.8xk [name]
# With -l, the pages after the app's code pages, 1 unless -p says otherwise, must hold the
# library's 16 KB chunks, then only zero padding and the signature.
set -eu
export LC_ALL=C

PAGE=16384
usage='usage: check-8xk.sh [-p code-pages] [-l library] app.8xk [name]'
library=
code=1
while :; do
	case ${1:-} in
		-p) code=${2:?$usage}; shift 2 ;;
		-l) library=${2:?$usage}; shift 2 ;;
		*) break ;;
	esac
done
case $code in
	'' | *[!0-9]* | 0) echo "$usage" >&2; exit 2 ;;
esac
f=${1:?$usage}
want=${2:-metacalc}
hexname=$(printf %s "$want" | od -An -tx1 | tr -d ' \n' | tr a-f A-F)
namefield=804$(printf %X "${#want}")
fail() { echo "check-8xk: $f: $*" >&2; exit 1; }

[ -s "$f" ] || fail "missing or empty"
[ "$(head -c 8 "$f")" = "**TIFL**" ] || fail "not a TIFL flash file"

name=$(dd if="$f" bs=1 skip=17 count=8 2>/dev/null)
[ "$name" = "$want" ] || fail "file header name is '$name', want '$want'"

# From the first page record on, every record line starts with a colon and ends in CR LF, and the
# file ends with the end record, whose own CR LF is optional. Header bytes before the first record
# may be anything, including 0Ah.
first=$(grep -a -b -o -m 1 ':02000002' "$f" | head -n 1 | cut -d: -f1)
[ -n "$first" ] || fail "no page record"
tail -c +$((first + 1)) "$f" | sed '$d' | awk '
	!/^:/ || !/\r$/ { bad = 1 }
	END { exit bad }
' || fail "a record line does not start with a colon or does not end in CR LF"
case $(tail -c 14 "$f" | tr '\r\n' 'RN') in
	*N:00000001FF | *N:00000001FFRN) ;;
	*) fail "the file does not end with the end record" ;;
esac

# Data bytes of every type-00 Intel hex record, as one hex string.
body=$(grep -a -E '^:[0-9A-F]{6}00' "$f" | tr -d '\r' |
	sed -E 's/^:[0-9A-F]{6}00(.*)..$/\1/' | tr -d '\n')

# Name field 80 4n <name> in the app header, n the length of the name.
case $body in
	800F*${namefield}${hexname}*) ;;
	*) fail "app header has no '$want' name field" ;;
esac

# TI-OS enters an app at 4080h, so the header's last field ends 16 bytes before it. A name shorter
# than 8 characters moves the field one byte back per missing character.
last=$(printf '%s' "$body" | cut -c1-256 | grep -oE '807F(00)*$' || true)
[ "${#last}" -eq $((44 + 2 * (8 - ${#want}))) ] || fail "app header is not 128 bytes"

# The key ID field 80 12 01 04 follows the length field, and 0104 is the key the signer uses.
case $body in
	800F????????80120104*) ;;
	*) fail "app header has no 0104 key ID field after the length field" ;;
esac


# Pages 8081 nn field of the header against the 02 records that start each page.
declared=$(printf '%s' "$body" | sed -nE "s/^800F[0-9A-F]{0,80}${namefield}${hexname}8081(..).*/\1/p")
[ -n "$declared" ] || fail "app header has no page count"
declared=$((0x$declared))

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
grep -a -o -E ':[0-9A-F]{8}[0-9A-F]*' "$f" | awk -v dir="$dir" -v f="$f" '
	function hex(s,   i, n) {
		n = 0
		for (i = 1; i <= length(s); i++) n = n * 16 + index("0123456789ABCDEF", substr(s, i, 1)) - 1
		return n
	}
	function bad(why) { print "check-8xk: " f ": record " NR ": " why > "/dev/stderr"; exit 1 }
	{
		len = hex(substr($0, 2, 2)); type = substr($0, 8, 2)
		if (length($0) != 11 + 2 * len) bad("the length does not match the data")
		sum = 0
		for (i = 2; i < length($0); i += 2) sum += hex(substr($0, i, 2))
		if (sum % 256 != 0) bad("the checksum is wrong")
		if (type == "02") {
			if (hex(substr($0, 10, 4)) != pages) bad("page records are out of order")
			pages++
			file = dir "/page." pages
			used = 0
			printf "" > file
		} else if (type == "00") {
			if (!pages) bad("data before the first page record")
			if (hex(substr($0, 4, 4)) != 16384 + used) bad("the address is not contiguous from 4000h in page " (pages - 1))
			used += len
			printf "%s", substr($0, 10, 2 * len) >> file
			close(file)
		}
	}
	END { print pages + 0 > (dir "/count") }
' || exit 1
pages=$(cat "$dir/count")
[ "$pages" = "$declared" ] || fail "header declares $declared pages, the file has $pages"

n=1
while [ "$n" -lt "$pages" ]; do
	[ "$(wc -c < "$dir/page.$n" | tr -d ' ')" = $((PAGE * 2)) ] || fail "page $((n - 1)) is not $PAGE bytes"
	n=$((n + 1))
done
[ "$(wc -c < "$dir/page.$pages" | tr -d ' ')" -le $((PAGE * 2)) ] || fail "the last page is over $PAGE bytes"

if [ -z "$library" ]; then
	[ "$pages" = "$code" ] || fail "$pages pages, want the $code code pages"
else
	len=$(wc -c < "$library" | tr -d ' ')
	[ "$len" -gt 0 ] || fail "library is empty"
	want_pages=$(((code * PAGE + len + 96 + PAGE - 1) / PAGE))
	[ "$pages" = "$want_pages" ] || fail "$pages pages for a $len byte library, want $want_pages"
	c=0
	while [ $((c * PAGE)) -lt "$len" ]; do
		want_chunk=$(dd if="$library" bs=$PAGE skip=$c count=1 2>/dev/null | od -An -v -tx1 | tr -d ' \n' | tr a-f A-F)
		got=$(cut -c1-${#want_chunk} "$dir/page.$((c + code + 1))")
		[ "$got" = "$want_chunk" ] || fail "page $((c + code)) is not chunk $c of the library"
		c=$((c + 1))
	done
	n=1
	while [ "$n" -le "$pages" ]; do
		cat "$dir/page.$n"
		n=$((n + 1))
	done > "$dir/all"
	tail=$(cut -c$(((code * PAGE + len) * 2 + 1))- "$dir/all")
	pad=$((${#tail} - 192))
	[ "$pad" -ge 0 ] || fail "bytes after the library are not zero padding then the signature"
	[ "$pad" -eq 0 ] || [ -z "$(printf '%s' "$tail" | cut -c1-"$pad" | tr -d 0)" ] ||
		fail "bytes after the library are not zero padding then the signature"
fi

# The last 96 bytes of the data are the signature field: 02 2D n, then the n bytes of a number below
# the signer's 512-bit modulus, least significant byte first, with no zero byte at the top, so n is
# 1 to 64 and below the modulus. A trailer follows, 00 or 01 f with f 1 to 3, and FF fills the field to 96 bytes. The
# number is a Rabin signature: its square, mod the modulus, is the MD5 of the data before the field
# read as a little-endian number, times 256 plus 1, then doubled if f is 0 or 3 and subtracted from
# the modulus if f is 0 or 1.
command -v python3 > /dev/null || fail "python3 is needed to verify the signature"
verdict=$(printf '%s' "$body" | python3 -I -c '
import hashlib, sys

# The public modulus of the TI application key 0104, published in WabbitSign and spasm-ng (export.cpp).
MODULUS = int.from_bytes(bytes.fromhex(
    "AD2431DA2297E4175EAC61A3154FA3D847115794DD330AB7FF36BA59FEDA195F"
    "EA7C16743BD7BCED8A0DA885E5E5C34D5BF20D0AB3EF9181ED39BA2C4D898E87"), "little")


def verdict(data):
    if len(data) < 96:
        return "field"
    data, field = data[:-96], data[-96:]
    n = field[2]
    if field[:2] != b"\x02\x2d" or not 1 <= n <= 64:
        return "field"
    sig, rest = field[3:3 + n], field[3 + n:]
    f = 0
    if rest[:1] == b"\x01" and rest[1:2] in (b"\x01", b"\x02", b"\x03"):
        f, rest = rest[1], rest[2:]
        trailer = "01 0%d" % f
    elif rest[:1] == b"\x00":
        rest = rest[1:]
        trailer = "00"
    else:
        return "field"
    if rest.strip(b"\xff"):
        return "field"
    if sig[-1] == 0:
        return "leading"
    if int.from_bytes(sig, "little") >= MODULUS:
        return "range"
    m = int.from_bytes(hashlib.md5(data).digest(), "little") * 256 + 1
    if f in (0, 3):
        m *= 2
    if f in (0, 1):
        m = MODULUS - m
    if pow(int.from_bytes(sig, "little"), 2, MODULUS) != m:
        return "verify"
    return "%d:%s" % (n, trailer)


print(verdict(bytes.fromhex(sys.stdin.read())))
')
case $verdict in
	field) fail "no signature field" ;;
	leading) fail "the signature has a zero byte at the top, which the signer never writes" ;;
	range) fail "the signature is not below the modulus, so the signer cannot have written it" ;;
	verify) fail "the signature does not verify against the data" ;;
	[0-9]*) siglen=${verdict%%:*}; trailer=${verdict#*:} ;;
	*) fail "the signature check failed: $verdict" ;;
esac

echo "check-8xk: ok: $f ($(wc -c < "$f" | tr -d ' ') bytes, $pages pages, name $name, $siglen-byte signature, trailer $trailer)"
