#!/bin/sh
# Usage: fetch-tilem.sh [dir]
# Fetches TilEm at the pinned commit into dir (default .tilem at the repo root) and verifies it.
set -eu

rev=f56ad637d0524ee841dd381be6ecbaf5b8975600
root=$(cd "$(dirname "$0")/.." && pwd)
dir=${1:-$root/.tilem}

if [ ! -d "$dir/.git" ]; then
	git init -q "$dir"
	git -C "$dir" remote add origin https://github.com/debrouxl/tilem
fi
if [ "$(git -C "$dir" rev-parse -q --verify HEAD 2>/dev/null || true)" != "$rev" ]; then
	git -C "$dir" fetch -q --depth 1 origin "$rev"
	git -C "$dir" checkout -q --detach FETCH_HEAD
fi
got=$(git -C "$dir" rev-parse HEAD)
[ "$got" = "$rev" ] || { echo "fetch-tilem: got $got, want $rev" >&2; exit 1; }
echo "$dir: TilEm $rev"
