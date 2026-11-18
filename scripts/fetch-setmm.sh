#!/bin/sh
# Usage: fetch-setmm.sh [dir]
# Downloads set.mm at the commit pinned in libraries/source.toml into dir (default .setmm)
# and verifies its SHA-256. Prints the path of the file.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
pin=$root/libraries/source.toml
dir=${1:-.setmm}

field() { sed -n "s/^$1 = \"\(.*\)\"\$/\1/p" "$pin"; }
repository=$(field repository)
commit=$(field commit)
path=$(field path)
want=$(field sha256)

digest() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | cut -d' ' -f1
	else
		shasum -a 256 "$1" | cut -d' ' -f1
	fi
}

file=$dir/$path
if [ ! -f "$file" ] || [ "$(digest "$file")" != "$want" ]; then
	mkdir -p "$dir"
	url=$(echo "$repository" | sed 's,://github.com/,://raw.githubusercontent.com/,')/$commit/$path
	curl -fsSL --retry 3 -o "$file.part" "$url"
	mv "$file.part" "$file"
fi
got=$(digest "$file")
[ "$got" = "$want" ] || { echo "fetch-setmm: $file has sha256 $got, want $want" >&2; exit 1; }
echo "$file"
