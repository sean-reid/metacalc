#!/bin/sh
# Usage: fetch-metamath.sh [dir]
# Fetches metamath-exe at the pinned commit into dir (default .metamath), builds it there and
# verifies the commit. Prints the path of the metamath binary.
set -eu

rev=0e6fb2ebf0865fb90354839b2e586490b210d017
root=$(cd "$(dirname "$0")/.." && pwd)
dir=${1:-$root/.metamath}

if [ ! -d "$dir/.git" ]; then
	git init -q "$dir"
	git -C "$dir" remote add origin https://github.com/metamath/metamath-exe
fi
if [ "$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null || true)" != "$rev" ]; then
	git -C "$dir" fetch -q --depth 1 origin "$rev"
	git -C "$dir" checkout -q --detach FETCH_HEAD
fi
got=$(git -C "$dir" rev-parse HEAD)
[ "$got" = "$rev" ] || { echo "fetch-metamath: got $got, want $rev" >&2; exit 1; }
if [ ! -x "$dir/metamath" ]; then
	command -v cc >/dev/null 2>&1 || { echo "fetch-metamath: no C compiler" >&2; exit 1; }
	cc -O2 -o "$dir/metamath.part" "$dir"/*.c -lm >&2
	mv "$dir/metamath.part" "$dir/metamath"
fi
echo "$dir/metamath"
