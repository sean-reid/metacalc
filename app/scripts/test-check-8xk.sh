#!/bin/sh
# Usage: test-check-8xk.sh app.8xk [name]
#        test-check-8xk.sh -l library app.8xk [name]
# The first form breaks a one-page app each way the checker must notice. The second form does
# the same to an app built from the library, for the rules about pages and the library's bytes.
set -eu
export LC_ALL=C

library=
if [ "${1:-}" = -l ]; then
	library=${2:?usage: test-check-8xk.sh -l library app.8xk [name]}
	shift 2
fi
good=${1:?usage: test-check-8xk.sh [-l library] app.8xk [name]}
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

# refuses file reason [message]: the checker must fail, and say message if one is given.
refuses() {
	if [ -n "$library" ]; then
		sh "$check" -l "$library" "$dir/$1.8xk" "$name" 2> "$dir/err" && ran=yes || ran=
	else
		sh "$check" "$dir/$1.8xk" "$name" 2> "$dir/err" && ran=yes || ran=
	fi
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
	resum "$dir/$2.tmp" "$dir/$2.8xk"
}

if [ -z "$library" ]; then
	: > "$dir/empty.8xk"
	refuses empty "no bytes"

	sed "s/$name/${name%?}x/" "$good" > "$dir/file-name.8xk"
	refuses file-name "another name in the file header"

	mutate "s/$namefield$hexname/$namefield${hexname%??}78/" app-name
	refuses app-name "another name in the app header"

	mutate 's/807F00000000/807F00000001/' header
	refuses header "bytes between the header and 4080h"

	mutate 's/022D40/022D41/' signature
	refuses signature "no signature field"

	zero_last_byte "$good" "$dir/after-signature.tmp"
	resum "$dir/after-signature.tmp" "$dir/after-signature.8xk"
	refuses after-signature "a byte after the signature that is not padding" "no signature field"

	corrupt "$good" 0 1 "$dir/checksum.8xk"
	refuses checksum "a data byte changed under its checksum" "checksum is wrong"

	sh "$check" "$good" "$name" > /dev/null
	echo "test-check-8xk: ok"
	exit 0
fi

pages=$(sh "$check" -l "$library" "$good" "$name" | sed -n 's/.* \([0-9][0-9]*\) pages.*/\1/p')
[ "$pages" -ge 3 ] || { echo "test-check-8xk: $good has $pages pages, need at least 3" >&2; exit 1; }
len=$(wc -c < "$library" | tr -d ' ')
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
refuses long-page "a last page over 16384 bytes" "over 16384"

corrupt "$good" 1 1 "$dir/chunk.tmp"
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

sh "$check" -l "$library" "$good" "$name" > /dev/null
echo "test-check-8xk: ok: $good"
