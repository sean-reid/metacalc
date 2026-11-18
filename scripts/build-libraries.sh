#!/bin/sh
# Usage: build-libraries.sh database libdir pagedir
# Builds every library in libraries/ from the pinned database into libdir, then writes each
# library's reference page into pagedir, writes the number of proofs each export holds beside it as
# name.proofs and the names it built to libraries.list, and checks that each library exports to a database the
# reference verifier accepts and that every symbol in it has a Unicode spelling. Also writes
# libraries-table.md into pagedir, the index table in the order the manifests give.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
database=$1
libdir=$2
pagedir=$3

cargo build --release --locked -p mcpack
mcpack=$root/target/release/mcpack
mkdir -p "$libdir" "$pagedir"
rm -f "$libdir/libraries.list"
set --
names=
for manifest in "$root"/libraries/*.toml; do
	name=$(basename "$manifest" .toml)
	[ "$name" = source ] && continue
	set -- "$@" "$manifest"
	names="$names$name
"
	rm -f "$libdir/$name.mm" "$libdir/$name.proofs"
	"$mcpack" build "$manifest" --database "$database" -o "$libdir/$name.mclib"
	"$mcpack" doc "$manifest" --library "$libdir/$name.mclib" -o "$pagedir/$name.md" --require-unicode
	"$mcpack" export "$libdir/$name.mclib" -o "$libdir/$name.mm"
	checked=$("$mcpack" verify "$libdir/$name.mm")
	echo "$checked"
	proofs=$(echo "$checked" | sed -n 's/^\([0-9][0-9]*\) proofs checked, 0 failed$/\1/p')
	[ -n "$proofs" ] || { echo "build-libraries: no proof count in the verify output for $name" >&2; exit 1; }
	echo "$proofs" >"$libdir/$name.proofs"
done

printf '%s' "$names" >"$libdir/libraries.list"
"$mcpack" summary "$libdir" "$@" >"$pagedir/libraries-table.md"
