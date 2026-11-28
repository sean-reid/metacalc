#!/bin/sh
# Usage: test-library-pages.sh spasm
#        test-library-pages.sh --find spasm first last
# Builds the app around libraries with 0, 95, 96 and 97 free bytes in the last chunk and with
# short last chunks, and checks each.
# The signature lengths in the table below, the second `for entry` loop, belong to the app's bytes
# and go stale with every change to the Z80 under app/, ui/, fm/, lib/ or kernel/. Then run
# --find: it builds the app around a library of each length from first to last and prints the
# length, the page count, the signature's length and its trailer, so that a length for each of
# the four trailers and for a 63-byte signature can be copied into the table. A 63-byte signature
# with trailer 00 may not exist in the range; any trailer will do for the 63-byte entry.
set -eu
export LC_ALL=C

usage='usage: test-library-pages.sh spasm | --find spasm first last'
find=
if [ "${1:-}" = --find ]; then
	find=yes
	shift
fi
spasm=${1:?$usage}
# The lengths this script expects belong to the bytes of an app of this version, whatever the
# workspace's is.
version=0.1.0
app=$(cd "$(dirname "$0")/.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

# A library of $1 bytes in which no two chunks match.
make_library() {
	awk -v n="$1" 'BEGIN { for (i = 0; i < n; i++) printf "%c", (i * 7 + int(i / 16384)) % 120 + 1 }' > "$dir/lib"
}

if [ -n "$find" ]; then
	first=${2:?$usage}
	last=${3:?$usage}
	len=$first
	while [ "$len" -le "$last" ]; do
		make_library "$len"
		rm -rf "$dir/build"
		make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" LIBRARY="$dir/lib" VERSION="$version" > /dev/null
		out=$(sh "$app/scripts/check-8xk.sh" -p 3 -l "$dir/lib" "$dir/build/metacalc.8xk")
		echo "$len $(echo "$out" | sed 's/.*bytes, //')"
		len=$((len + 1))
	done
	exit 0
fi

# Library length, then the app's pages: the three code pages, the chunks, and one more when the
# last chunk leaves under 96 bytes.
for entry in 32768:6 32673:6 32672:5 32671:5 16385:5 17311:5 17312:5; do
	len=${entry%:*}
	want=${entry#*:}
	make_library "$len"
	rm -rf "$dir/build"
	make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" LIBRARY="$dir/lib" > /dev/null
	out=$(sh "$app/scripts/check-8xk.sh" -p 3 -l "$dir/lib" "$dir/build/metacalc.8xk")
	case $out in
		*" $want pages,"*) ;;
		*) echo "test-library-pages: $len bytes: want $want pages: $out" >&2; exit 1 ;;
	esac
	echo "test-library-pages: $len bytes: $want pages"
	case $len in
		32673 | 32671 | 17311) sh "$app/scripts/test-check-8xk.sh" -p 3 -l "$dir/lib" "$dir/build/metacalc.8xk" ;;
	esac
done

# Libraries whose apps hash to each signature trailer form (01 01 to 01 03, then 00), and to a
# signature of 63 bytes, the signer's output being a function of the app's bytes alone. Length,
# signature bytes, trailer.
for entry in 611:63:01\ 02 5:64:01\ 01 1:64:01\ 02 3:64:01\ 03 6:64:00; do
	len=${entry%%:*}
	rest=${entry#*:}
	want=${rest%%:*}
	form=${rest#*:}
	make_library "$len"
	rm -rf "$dir/build"
	make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" LIBRARY="$dir/lib" VERSION="$version" > /dev/null
	out=$(sh "$app/scripts/check-8xk.sh" -p 3 -l "$dir/lib" "$dir/build/metacalc.8xk")
	case $out in
		*", $want-byte signature, trailer $form)") ;;
		*) echo "test-library-pages: $len bytes: want a $want-byte signature and trailer $form: $out" >&2; exit 1 ;;
	esac
	echo "test-library-pages: $len bytes: $want-byte signature, trailer $form"
done
# A library of 935 bytes puts 0Ah in the file header's length field, which is not a line ending.
make_library 935
rm -rf "$dir/build"
make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" LIBRARY="$dir/lib" VERSION="$version" > /dev/null
[ "$(head -c 78 "$dir/build/metacalc.8xk" | tail -c 4 | od -An -tx1 | tr -d ' ')" = 0ad80100 ] || {
	echo "test-library-pages: 935 bytes: the header has no 0Ah byte" >&2
	exit 1
}
sh "$app/scripts/check-8xk.sh" -p 3 -l "$dir/lib" "$dir/build/metacalc.8xk" > /dev/null
echo "test-library-pages: 935 bytes: 0Ah in the header"

rm -rf "$dir/build"
make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" > /dev/null
sh "$app/scripts/test-check-8xk.sh" -p 3 "$dir/build/metacalc.8xk"

mkdir "$dir/with space"
make_library 20000
mv "$dir/lib" "$dir/with space/my library.mclib"
make -s -C "$app" SPASM="$spasm" BUILD="$dir/with space/build dir" LIBRARY="$dir/with space/my library.mclib" > /dev/null
sh "$app/scripts/check-8xk.sh" -p 3 -l "$dir/with space/my library.mclib" "$dir/with space/build dir/metacalc.8xk" > /dev/null
echo "test-library-pages: paths with spaces"

# Both scripts refuse a code page count that is not a positive number, and say how to call them.
for bad in 0 00 08 09 x 1x ''; do
	for script in check-8xk.sh test-check-8xk.sh; do
		if out=$(sh "$app/scripts/$script" -p "$bad" "$dir/build/metacalc.8xk" 2>&1); then
			echo "test-library-pages: $script accepted -p '$bad'" >&2
			exit 1
		fi
		case $out in
			*usage:*) ;;
			*) echo "test-library-pages: $script refused -p '$bad' without its usage: $out" >&2; exit 1 ;;
		esac
	done
done
echo "test-library-pages: -p is checked"

: > "$dir/lib"
if sh "$app/scripts/library-include.sh" "$dir/lib" 2> /dev/null; then
	echo "test-library-pages: accepted an empty library" >&2
	exit 1
fi
echo "test-library-pages: ok"
