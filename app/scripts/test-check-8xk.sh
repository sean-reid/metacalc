#!/bin/sh
# Usage: test-check-8xk.sh [-p code-pages] app.8xk [name]
#        test-check-8xk.sh [-p code-pages] -l library app.8xk [name]
# Either form takes -p code-pages first, the app's number of code pages, 1 by default. The first
# form breaks an app of only code pages each way the checker must notice. The second form does
# the same to an app built from the library, for the rules about pages and the library's bytes.
set -eu
export LC_ALL=C

usage='usage: test-check-8xk.sh [-p code-pages] [-l library] app.8xk [name]'
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
	'' | *[!0-9]* | 0*) echo "$usage" >&2; exit 2 ;;
esac
good=${1:?$usage}
name=${2:-metacalc}
hexname=$(printf %s "$name" | od -An -tx1 | tr -d ' \n' | tr a-f A-F)
namefield=804$(printf %X "${#name}")
check=$(dirname "$0")/check-8xk.sh
# With KEEP_FIXTURES set to a directory, the broken files are built there and left in place.
if [ -n "${KEEP_FIXTURES:-}" ]; then
	dir=$KEEP_FIXTURES
	mkdir -p "$dir"
else
	dir=$(mktemp -d)
	trap 'rm -rf "$dir"' EXIT
fi

# run_check args: the checker with the code page count and the library, if there is one.
run_check() {
	if [ -n "$library" ]; then
		sh "$check" -p "$code" -l "$library" "$@"
	else
		sh "$check" -p "$code" "$@"
	fi
}

# wrong_code_pages: the checker must refuse the good file when told another number of code pages.
wrong_code_pages() {
	for wrong in $((code + 1)) $((code - 1)); do
		[ "$wrong" -ge 1 ] || continue
		if [ -n "$library" ]; then
			sh "$check" -p "$wrong" -l "$library" "$good" "$name" > /dev/null 2>&1 && ran=yes || ran=
		else
			sh "$check" -p "$wrong" "$good" "$name" > /dev/null 2>&1 && ran=yes || ran=
		fi
		if [ -n "$ran" ]; then
			echo "test-check-8xk: accepted an app of $code code pages as one of $wrong" >&2
			exit 1
		fi
	done
}

# refuses file reason [message]: the checker must fail, and say message if one is given.
refuses() {
	run_check "$dir/$1.8xk" "$name" 2> "$dir/err" && ran=yes || ran=
	if [ -n "$ran" ]; then
		echo "test-check-8xk: accepted an app with $2" >&2
		exit 1
	fi
	if [ -n "${3:-}" ] && ! grep -q "$3" "$dir/err"; then
		echo "test-check-8xk: refused an app with $2 for another reason: $(cat "$dir/err")" >&2
		exit 1
	fi
}

# The first line holds the binary file header and the record that starts page 0, which awk
# would mangle, so awk only sees the lines after it.
HEX='
	function hex(s,   i, n) {
		n = 0
		for (i = 1; i <= length(s); i++) n = n * 16 + index("0123456789ABCDEF", substr(s, i, 1)) - 1
		return n
	}
	function checksum(body,   i, sum) {
		sum = 0
		for (i = 1; i < length(body); i += 2) sum += hex(substr(body, i, 2))
		return (256 - sum % 256) % 256
	}
'

# resum in out: rewrites every record's checksum, so a change to the data is not caught by it.
resum() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk "$HEX"'
			{ sub(/\r$/, ""); body = substr($0, 2, length($0) - 3); printf ":%s%02X\r\n", body, checksum(body) }
		'
	} > "$2"
}

# unlf file: drops the file's last byte when it is LF, which awk adds after the unterminated end
# record the signer writes.
unlf() {
	if [ "$(tail -c 1 "$1" | od -An -tx1 | tr -d ' ')" = 0a ]; then
		head -c $(($(wc -c < "$1") - 1)) "$1" > "$1.cut"
		mv "$1.cut" "$1"
	fi
}

# corrupt in page record out: changes the first nibble of data record $3 of page $2, both
# counted from 0.
corrupt() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk -v page="$2" -v rec="$3" '
			BEGIN { seen = 1; at = -1 }
			index($0, ":02000002") { seen++; at = -1; print; next }
			/^:[0-9A-F]/ && seen - 1 == page && ++at == rec {
				first = substr($0, 10, 1)
				$0 = substr($0, 1, 9) (first == "0" ? "1" : "0") substr($0, 11)
			}
			{ print }
		'
	} > "$4"
	unlf "$4"
}

# Sets the last data byte of the file, which the signer pads with FF, to 00.
zero_last_byte() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk '
			{ line[NR] = $0 }
			END {
				for (i = 1; i <= NR; i++) {
					if (line[i + 1] ~ /^:00000001FF/) {
						sub(/\r$/, "", line[i])
						n = length(line[i])
						line[i] = substr(line[i], 1, n - 4) "00" substr(line[i], n - 1)
					}
					print line[i]
				}
			}
		'
	} > "$2"
}

# mutate sed-script name: applies the script to the good file and fixes the checksums.
mutate() {
	sed "$1" "$good" > "$dir/$2.tmp"
	if cmp -s "$good" "$dir/$2.tmp"; then
		echo "test-check-8xk: the mutation $2 changed nothing in $good" >&2
		exit 1
	fi
	resum "$dir/$2.tmp" "$dir/$2.8xk"
}

# read_field: sets n, t, sig and trailer from the 96-byte signature field of the good file, n
# signature bytes and a trailer of t bytes, and fieldpos to the field's offset in the file's data
# as hex digits.
read_field() {
	body=$(grep -a -E '^:[0-9A-F]{6}00' "$good" | tr -d '\r' | sed -E 's/^:[0-9A-F]{6}00(.*)..$/\1/' | tr -d '\n')
	n=$(run_check "$good" "$name" | sed -n 's/.*, \([0-9]*\)-byte signature.*/\1/p')
	stripped=$(printf '%s' "$body" | sed -E 's/F+$//')
	case $stripped in
		*010[123]) t=2 ;;
		*) t=1 ;;
	esac
	old=$(printf '%s' "$stripped" | tail -c $((2 * (3 + n + t))))
	sig=$(printf '%s' "$old" | cut -c7-$((6 + 2 * n)))
	trailer=$(printf '%s' "$old" | cut -c$((7 + 2 * n))-)
	fieldpos=$((${#stripped} - 2 * (3 + n + t)))
}

# set_field out hex: the good file with its signature field replaced by hex, which must be 192
# hex digits.
set_field() {
	{
		head -n 1 "$good"
		tail -n +2 "$good" | awk -v pos="$fieldpos" -v new="$2" "$HEX"'
			BEGIN { off = 0 }
			/^:[0-9A-F]{6}00/ {
				len = 2 * hex(substr($0, 2, 2))
				data = substr($0, 10, len)
				for (i = 1; i <= len; i++) {
					at = off + i - 1 - pos
					if (at >= 0 && at < length(new)) data = substr(data, 1, i - 1) substr(new, at + 1, 1) substr(data, i + 1)
				}
				off += len
				$0 = substr($0, 1, 9) data substr($0, 10 + len)
			}
			{ print }
		'
	} > "$1.tmp"
	resum "$1.tmp" "$1"
}

fill() { printf '%*s' "$1" '' | tr ' ' F; }
hexlen() { printf %02X "$1"; }

# rewrap in out: the file with every page's data in records of 32 bytes, but for the last page, whose
# first record is cut so that a record ends just after the first byte of the signature field, which
# then spans two records.
rewrap() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk "$HEX"'
			function flush(last,   n, pos, size, take, body) {
				n = length(data) / 2
				size = 32
				if (last && n > 95) { size = (n - 95) % 32; if (size == 0) size = 32 }
				for (pos = 0; pos < n; pos += take) {
					take = size < n - pos ? size : n - pos
					body = sprintf("%02X%04X00", take, 16384 + pos) substr(data, 2 * pos + 1, 2 * take)
					printf ":%s%02X\r\n", body, checksum(body)
					size = 32
				}
				data = ""
			}
			/^:02000002/ { flush(0); print; next }
			/^:00000001FF/ { flush(1); print; next }
			/^:[0-9A-F]{6}00/ { data = data substr($0, 10, 2 * hex(substr($0, 2, 2))); next }
			{ print }
		'
	} > "$2"
	unlf "$2"
}

# append_ff in out: the file with one FF byte after the signature field, in a record of its own.
append_ff() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk "$HEX"'
			/^:[0-9A-F]{6}00/ { next_address = hex(substr($0, 4, 4)) + hex(substr($0, 2, 2)) }
			/^:00000001FF/ {
				body = sprintf("01%04X00FF", next_address)
				printf ":%s%02X\r\n", body, checksum(body)
				printf "%s", $0
				next
			}
			{ print }
		'
	} > "$2"
}

# The rules that hold for the app of any library: the signature covers every byte, the header
# says shareware, the records end in CR LF and the file ends with the end record.
common_tests() {
	corrupt "$good" 0 4 "$dir/code.tmp"
	resum "$dir/code.tmp" "$dir/code.8xk"
	refuses code "a code byte changed under fixed checksums" "does not verify"

	mutate 's/80120104/80120105/' key-id
	refuses key-id "key ID 0105" "key ID"

	# 80 12 01 04 elsewhere in the header is not the key ID field.
	mutate 's/80120104/80120105/;s/A16B99F6/80120104/' key-id-elsewhere
	refuses key-id-elsewhere "the key ID 0104 somewhere other than after the length" "key ID"

	{ cat "$good"; printf '\n'; } > "$dir/lf-end.8xk"
	refuses lf-end "an end record that ends in LF alone" "end record"

	sed '$d' "$good" > "$dir/no-end.8xk"
	refuses no-end "no end record" "end record"

	sed "s/$(printf '\r')\$//" "$good" > "$dir/lf.8xk"
	refuses lf "LF line endings" "CR LF"

	# A signature of fewer than 64 bytes with a zero byte added at the top is the same number.
	read_field
	# The signature plus the modulus squares to the same number, and fits in 64 bytes when the
	# signature is below 2^512 minus the modulus.
	bumped=$(python3 -I -c '
import sys
n = int.from_bytes(bytes.fromhex(
    "AD2431DA2297E4175EAC61A3154FA3D847115794DD330AB7FF36BA59FEDA195F"
    "EA7C16743BD7BCED8A0DA885E5E5C34D5BF20D0AB3EF9181ED39BA2C4D898E87"), "little")
s = int.from_bytes(bytes.fromhex(sys.argv[1]), "little") + n
print(s.to_bytes(64, "little").hex().upper() if s < 1 << 512 else "")
' "$sig")
	if [ -n "$bumped" ]; then
		set_field "$dir/above-modulus.8xk" "022D40${bumped}${trailer}$(fill $((192 - 6 - 128 - 2 * t)))"
		refuses above-modulus "the signature plus the modulus" "not below the modulus"
	fi
	if [ "$n" -lt 64 ]; then
		leading="022D$(hexlen $((n + 1)))${sig}00${trailer}$(fill $((192 - 6 - 2 * (n + 1) - 2 * t)))"
		set_field "$dir/leading-zero.8xk" "$leading"
		refuses leading-zero "a zero byte at the top of the signature" "zero byte at the top"
	fi
}

# resign in out form delta claim: the good file with the code byte 100 bytes before the signature
# field changed until the signer's arithmetic gives trailer form `form` for the MD5 of the data up
# to the field plus delta bytes, signed with the factors of the modulus of key 0104 as published in
# spasm-ng (export.cpp) and with the trailer written as form `claim`. Delta 0 is what the checker
# hashes.
RESIGN='
import hashlib, re, sys

P = int.from_bytes(bytes.fromhex(
    "5B2E54E9B5C1FE26CE93261478D3873F3FC41BFFF1F5F934D7A5793A43C1C21C"), "little")
Q = int.from_bytes(bytes.fromhex(
    "97F7707B94079B73858720BF6D4909AB3BEDA1BA9B93112B041340A16ED597B604"), "little")
N = P * Q
src, out, form, delta, claim = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])
raw = open(src, "rb").read()
records = [m for m in re.finditer(rb":[0-9A-F]{8}[0-9A-F]*", raw) if m.group()[7:9] == b"00"]
spans = [(m.start(), 11 + 2 * int(m.group()[1:3], 16)) for m in records]
body = bytes.fromhex(b"".join(m.group()[9:-2] for m in records).decode())
at = len(body) - 96


def sign(data):
    m = int.from_bytes(hashlib.md5(data).digest(), "little") * 256 + 1
    lp = 0 if pow(m, (P - 1) // 2, P) == 1 else 1
    lq = 1 if pow(m, (Q - 1) // 2, Q) == 1 else 0
    f = lp + lq + lq
    if lp == lq:
        m *= 2
    if lq == 0:
        m = N - m
    r, s = pow(m, (P + 1) // 4, P), pow(m, (Q + 1) // 4, Q)
    root = ((r - s) * pow(Q, -1, P) % P) * Q + s
    assert root * root % N == m
    return f, root.to_bytes((root.bit_length() + 7) // 8, "little")


for value in range(256):
    data = body[:at - 100] + bytes([value]) + body[at - 99:at]
    f, sig = sign((data + body[at:])[:at + delta])
    if f == form:
        break
else:
    sys.exit("no byte value gives form %d" % form)
trailer = b"\x00" if claim == 0 else bytes([1, claim])
field = (b"\x02\x2d" + bytes([len(sig)]) + sig + trailer).ljust(96, b"\xff")
hexed = (data + field).hex().upper().encode()
pieces, last, used = [], 0, 0
for m, (start, size) in zip(records, spans):
    rec = m.group()[1:9] + hexed[used:used + size - 11]
    used += size - 11
    total = sum(bytes.fromhex(rec.decode()))
    pieces += [raw[last:start], b":" + rec + b"%02X" % ((256 - total % 256) % 256)]
    last = start + size
open(out, "wb").write(b"".join(pieces) + raw[last:])
'

# trailer_tests: for each of the four trailer forms, a file signed the way the signer signs passes,
# and one whose signed range is a byte short or a byte long, or whose trailer names another form,
# is refused.
trailer_tests() {
	form=0
	while [ "$form" -le 3 ]; do
		python3 -I -c "$RESIGN" "$good" "$dir/form$form.8xk" "$form" 0 "$form"
		run_check "$dir/form$form.8xk" "$name" > /dev/null || {
			echo "test-check-8xk: refused a correct file with trailer form $form" >&2
			exit 1
		}
		python3 -I -c "$RESIGN" "$good" "$dir/short$form.8xk" "$form" -1 "$form"
		refuses "short$form" "a form $form signature over one byte too few" "does not verify"
		python3 -I -c "$RESIGN" "$good" "$dir/long$form.8xk" "$form" 1 "$form"
		refuses "long$form" "a form $form signature over one byte too many" "does not verify"
		other=$(((form + 1) % 4))
		python3 -I -c "$RESIGN" "$good" "$dir/claim$form.8xk" "$form" 0 "$other"
		refuses "claim$form" "a form $form signature with the trailer of form $other" "does not verify"
		form=$((form + 1))
	done
}

if [ -z "$library" ]; then
	common_tests
	trailer_tests

	append_ff "$good" "$dir/after-field.8xk"
	refuses after-field "a byte after the signature field" "no signature field"

	: > "$dir/empty.8xk"
	refuses empty "no bytes"

	sed "s/$name/${name%?}x/" "$good" > "$dir/file-name.8xk"
	refuses file-name "another name in the file header"

	mutate "s/$namefield$hexname/$namefield${hexname%??}78/" app-name
	refuses app-name "another name in the app header"

	mutate 's/807F00000000/807F00000001/' header
	refuses header "bytes between the header and 4080h"

	zero_last_byte "$good" "$dir/after-signature.tmp"
	resum "$dir/after-signature.tmp" "$dir/after-signature.8xk"
	refuses after-signature "a byte after the signature that is not padding" "no signature field"

	corrupt "$good" 0 1 "$dir/checksum.8xk"
	refuses checksum "a data byte changed under its checksum" "checksum is wrong"

	read_field
	pad=$((192 - 6 - 2 * n - 2 * t))
	short="022D$(hexlen $((n - 1)))$(printf '%s' "$sig" | cut -c1-$((2 * n - 2)))$trailer$(fill $((pad + 2)))"
	set_field "$dir/short-signature.8xk" "$short"
	refuses short-signature "a signature one byte short of the signed number" "does not verify"
	liar="022D$(hexlen $((n - 1)))$sig$trailer$(fill $pad)"
	set_field "$dir/length-lies.8xk" "$liar"
	refuses length-lies "a length byte one under the signature bytes present" "no signature field"
	long="022D$(hexlen $((n + 1)))$sig$trailer$(fill $pad)"
	set_field "$dir/length-over.8xk" "$long"
	refuses length-over "a length byte one over the signature bytes present" "no signature field"
	rewrap "$good" "$dir/split.8xk"
	run_check "$dir/split.8xk" "$name" > /dev/null || {
		echo "test-check-8xk: refused a correct file with the signature field across two records" >&2
		exit 1
	}
	if grep -aq "022D$(hexlen "$n")" "$dir/split.8xk"; then
		echo "test-check-8xk: the signature field is not across two records in split.8xk" >&2
		exit 1
	fi
	saved=$good
	good=$dir/split.8xk
	read_field
	set_field "$dir/split-length-over.8xk" "$long"
	good=$saved
	refuses split-length-over "a length byte one over the signature bytes present, the field across two records" "no signature field"
	empty="022D00${trailer}$(fill $((192 - 6 - 2 * t)))"
	set_field "$dir/zero-length.8xk" "$empty"
	refuses zero-length "a zero-length signature" "no signature field"
	too_long="022D41${sig}00$(fill $((pad - 2)))"
	set_field "$dir/sixty-five.8xk" "$too_long"
	refuses sixty-five "a length byte of 65, which the signer cannot emit" "no signature field"

	wrong_code_pages
	run_check "$good" "$name" > /dev/null
	echo "test-check-8xk: ok"
	exit 0
fi

pages=$(run_check "$good" "$name" | sed -n 's/.* \([0-9][0-9]*\) pages.*/\1/p')
[ "$pages" -ge 3 ] || { echo "test-check-8xk: $good has $pages pages, need at least 3" >&2; exit 1; }
len=$(wc -c < "$library" | tr -d ' ')
common_tests
field() { printf '2s/8081%02X/8081%02X/' "$1" "$2"; }

mutate "$(field "$pages" $((pages + 1)))" more-pages
refuses more-pages "a header that counts a page too many" "declares"

mutate "$(field "$pages" $((pages - 1)))" fewer-pages
refuses fewer-pages "a header that counts a page too few" "declares"

mutate '513d' short-page
refuses short-page "a first page of 16352 bytes" "page 0 is not 16384"

sed '100d' "$good" > "$dir/gap.8xk"
refuses gap "a missing record in the middle of a page" "not contiguous"

{
	head -n 1 "$good"
	tail -n +2 "$good" | awk "$HEX"'
		index($0, ":02000002") { used = 0 }
		/^:[0-9A-F]{8}/ && substr($0, 8, 2) == "00" { used += hex(substr($0, 2, 2)) }
		/^:00000001FF/ {
			for (i = 0; i < 512; i++) {
				body = sprintf("20%04X00", 16384 + used + 32 * i) "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
				printf ":%s%02X\r\n", body, checksum(body)
			}
		}
		{ print }
	'
} > "$dir/long-page.8xk"
unlf "$dir/long-page.8xk"
refuses long-page "a last page over 16384 bytes" "over 16384"

corrupt "$good" "$code" 1 "$dir/chunk.tmp"
resum "$dir/chunk.tmp" "$dir/chunk.8xk"
refuses chunk "a byte of chunk 0 changed" "is not chunk 0"

corrupt "$good" 1 1 "$dir/checksum.8xk"
refuses checksum "a data byte changed under its checksum" "checksum is wrong"

# A last chunk with under 96 free bytes puts the signature on a page of its own after zero padding.
if [ $(((16384 - len % 16384) % 16384)) -lt 96 ]; then
	corrupt "$good" $((pages - 1)) 1 "$dir/padding.tmp"
	resum "$dir/padding.tmp" "$dir/padding.8xk"
	refuses padding "a changed byte inside the zero padding" "zero padding"
fi

zero_last_byte "$good" "$dir/after-signature.tmp"
resum "$dir/after-signature.tmp" "$dir/after-signature.8xk"
refuses after-signature "a byte after the signature that is not padding" "no signature field"

cp "$good" "$dir/same.8xk"
real=$library
{ cat "$real"; head -c 16384 /dev/zero; } > "$dir/longer.lib"
library=$dir/longer.lib
refuses same "one chunk more in the library than in the app" "pages for a"
head -c $((len - 16384)) "$real" > "$dir/shorter.lib"
library=$dir/shorter.lib
refuses same "one chunk less in the library than in the app" "pages for a"
library=$real

wrong_code_pages
run_check "$good" "$name" > /dev/null
echo "test-check-8xk: ok: $good"
