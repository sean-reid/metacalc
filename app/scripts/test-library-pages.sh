#!/bin/sh
# Usage: test-library-pages.sh spasm
# Builds the app around libraries with 0, 95, 96 and 97 free bytes in the last chunk and with
# short last chunks, and checks each.
set -eu
export LC_ALL=C

spasm=${1:?usage: test-library-pages.sh spasm}
app=$(cd "$(dirname "$0")/.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

# A library of $1 bytes in which no two chunks match.
make_library() {
	awk -v n="$1" 'BEGIN { for (i = 0; i < n; i++) printf "%c", (i * 7 + int(i / 16384)) % 120 + 1 }' > "$dir/lib"
}

# Library length, then the app's pages: page 0, the chunks, and one more when the last chunk
# leaves under 96 bytes.
for entry in 32768:4 32673:4 32672:3 32671:3 16385:3 17311:3 17312:3; do
	len=${entry%:*}
	want=${entry#*:}
	make_library "$len"
	rm -rf "$dir/build"
	make -s -C "$app" SPASM="$spasm" BUILD="$dir/build" LIBRARY="$dir/lib" > /dev/null
	out=$(sh "$app/scripts/check-8xk.sh" -l "$dir/lib" "$dir/build/metacalc.8xk")
	case $out in
		*" $want pages,"*) ;;
		*) echo "test-library-pages: $len bytes: want $want pages: $out" >&2; exit 1 ;;
	esac
	echo "test-library-pages: $len bytes: $want pages"
	case $len in
		32673 | 32671 | 17311) sh "$app/scripts/test-check-8xk.sh" -l "$dir/lib" "$dir/build/metacalc.8xk" ;;
	esac
done

mkdir "$dir/with space"
make_library 20000
mv "$dir/lib" "$dir/with space/my library.mclib"
make -s -C "$app" SPASM="$spasm" BUILD="$dir/with space/build dir" LIBRARY="$dir/with space/my library.mclib" > /dev/null
sh "$app/scripts/check-8xk.sh" -l "$dir/with space/my library.mclib" "$dir/with space/build dir/metacalc.8xk" > /dev/null
echo "test-library-pages: paths with spaces"

: > "$dir/lib"
if sh "$app/scripts/library-include.sh" "$dir/lib" 2> /dev/null; then
	echo "test-library-pages: accepted an empty library" >&2
	exit 1
fi
echo "test-library-pages: ok"
