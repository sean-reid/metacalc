#!/bin/sh
# Usage: library-include.sh library
# Prints the assembler lines that append a library after the app's first page, with the library's
# absolute path. The signer needs 96 free bytes after the last used byte and warns, with a nonzero
# exit, under 928 used bytes on the last page. A library that leaves fewer than 96 free gets zero
# padding onto a page of its own, and a short last page is padded up to 928 bytes.
set -eu

lib=${1:?usage: library-include.sh library}
case $lib in
	/*) ;;
	*) lib=$PWD/$lib ;;
esac
size=$(wc -c < "$lib" | tr -d ' ')
[ "$size" -gt 0 ] || { echo "library-include: $lib is empty" >&2; exit 1; }
used=$((size % 16384))
free=$(((16384 - used) % 16384))
pad=0
if [ "$free" -lt 96 ]; then
	pad=$((free + 928))
elif [ "$used" -lt 928 ]; then
	pad=$((928 - used))
fi
printf '#import "%s"\n' "$lib"
if [ "$pad" -gt 0 ]; then
	printf '.fill %d, 0\n' "$pad"
fi
