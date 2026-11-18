#!/bin/sh
# Usage: library-include.sh library
# Prints the assembler lines that append a library after the app's first page. The signer needs
# 96 free bytes after the last used byte, so a library that leaves fewer gets zero padding onto a
# page of its own. It also exits 3 with a warning when under 928 bytes are used on the last page,
# so a short last page is padded up to 928.
set -eu

lib=${1:?usage: library-include.sh library}
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
