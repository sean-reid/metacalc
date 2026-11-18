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
check=$(dirname "$0")/check-8xk.sh
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

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

# Changes the first nibble of the first data record of page $2, counted from 0. The first line
# holds the binary file header and the record that starts page 0, which awk would mangle.
corrupt_page() {
	{
		head -n 1 "$1"
		tail -n +2 "$1" | awk -v want="$2" '
			BEGIN { seen = 1; hit = (want == 0) }
			index($0, ":02000002") { seen++; hit = (seen - 1 == want); print; next }
			hit && /^:[0-9A-F]/ {
				first = substr($0, 10, 1)
				$0 = substr($0, 1, 9) (first == "0" ? "1" : "0") substr($0, 11)
				hit = 0
			}
			{ print }
		'
	} > "$3"
}

if [ -z "$library" ]; then
	: > "$dir/empty.8xk"
	refuses empty "no bytes"

	sed "s/$name/${name%?}x/" "$good" > "$dir/file-name.8xk"
	refuses file-name "another name in the file header"

	sed "s/8048$hexname/8048${hexname%??}78/" "$good" > "$dir/app-name.8xk"
	refuses app-name "another name in the app header"

	sed 's/807F00000000/807F00000001/' "$good" > "$dir/header.8xk"
	refuses header "bytes between the header and 4080h"

	sed 's/022D40/022D41/' "$good" > "$dir/signature.8xk"
	refuses signature "no signature field"

	sh "$check" "$good" "$name" > /dev/null
	echo "test-check-8xk: ok"
	exit 0
fi

pages=$(sh "$check" -l "$library" "$good" "$name" | sed -n 's/.* \([0-9][0-9]*\) pages.*/\1/p')
[ "$pages" -ge 3 ] || { echo "test-check-8xk: $good has $pages pages, need at least 3" >&2; exit 1; }

sed "2s/8081$(printf %02X "$pages")/8081$(printf %02X $((pages + 1)))/" "$good" > "$dir/more-pages.8xk"
refuses more-pages "a header that counts a page too many" "declares"

sed "2s/8081$(printf %02X "$pages")/8081$(printf %02X $((pages - 1)))/" "$good" > "$dir/fewer-pages.8xk"
refuses fewer-pages "a header that counts a page too few" "declares"

sed '100d' "$good" > "$dir/short-page.8xk"
refuses short-page "a first page of 16352 bytes" "page 0 is not 16384"

full=$(printf '%064d' 0 | tr 0 F)
{
	grep -a -v '^:00000001FF' "$good"
	i=0
	while [ "$i" -lt 512 ]; do
		printf ':20400000%sC0\r\n' "$full"
		i=$((i + 1))
	done
	printf ':00000001FF'
} > "$dir/long-page.8xk"
refuses long-page "a last page over 16384 bytes" "over 16384"

corrupt_page "$good" 1 "$dir/chunk.8xk"
refuses chunk "a byte of chunk 0 changed" "is not chunk 0"

corrupt_page "$good" $((pages - 1)) "$dir/tail.8xk"
refuses tail "a changed byte in the last page" "chunk\|zero padding"

sh "$check" -l "$library" "$good" "$name" > /dev/null
echo "test-check-8xk: ok: $good"
