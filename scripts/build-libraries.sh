#!/bin/sh
# Usage: build-libraries.sh database libdir pagedir
# Builds every library in libraries/ from the pinned database into libdir, then writes each
# library's reference page into pagedir.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
database=$1
libdir=$2
pagedir=$3

cargo build --release --locked -p mcpack
mcpack=$root/target/release/mcpack
mkdir -p "$libdir" "$pagedir"
for manifest in "$root"/libraries/*.toml; do
	name=$(basename "$manifest" .toml)
	[ "$name" = source ] && continue
	"$mcpack" build "$manifest" --database "$database" -o "$libdir/$name.mclib"
	"$mcpack" doc "$manifest" --library "$libdir/$name.mclib" -o "$pagedir/$name.md"
done
