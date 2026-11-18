#!/bin/sh
# Usage: worked-examples.sh check database
#        worked-examples.sh write database
#        worked-examples.sh images dir
# The worked examples page marks each goal tree with a line `<!-- outline: LABEL -->` followed by a
# fenced block. check regenerates every tree from the database with `mcpack outline` and fails when
# a block differs. write rewrites the blocks. images draws the statement at the top of each block
# in the calculator font into dir, one PNG per label; it needs no database.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
page=$root/docs/src/guide/worked-examples.md
command=${1:?usage: worked-examples.sh check|write database | images dir}
target=${2:?usage: worked-examples.sh check|write database | images dir}

labels() { sed -n 's/^<!-- outline: \(.*\) -->$/\1/p' "$page"; }

# Prints the lines of the block that follows the marker for $1.
block() {
	awk -v label="$1" '
		$0 == "<!-- outline: " label " -->" { want = 1; next }
		want && /^```/ { if (inside) exit; inside = 1; next }
		inside { print }
	' "$page"
}

outlines() {
	cargo build --release --locked -p mcpack >&2
	mkdir -p "$1"
	for label in $(labels); do
		"$root/target/release/mcpack" outline "$target" "$label" >"$1/$label"
	done
}

case $command in
check)
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	outlines "$tmp"
	status=0
	for label in $(labels); do
		block "$label" >"$tmp/$label.page"
		if ! diff -u "$tmp/$label.page" "$tmp/$label" >"$tmp/$label.diff"; then
			echo "worked-examples: the tree for $label in $page differs from the database" >&2
			cat "$tmp/$label.diff" >&2
			status=1
		fi
	done
	[ $status -eq 0 ] || echo "worked-examples: run scripts/worked-examples.sh write $target to rewrite the blocks" >&2
	exit $status
	;;
write)
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	outlines "$tmp"
	awk -v dir="$tmp" '
		/^<!-- outline: .* -->$/ { label = $0; sub(/^<!-- outline: /, "", label); sub(/ -->$/, "", label); want = 1 }
		want && /^```/ {
			if (!inside) { inside = 1; print; while ((getline line < (dir "/" label)) > 0) print line; next }
			inside = 0; want = 0
		}
		!inside { print }
	' "$page" >"$tmp/page"
	cp "$tmp/page" "$page"
	;;
images)
	cargo build --release --locked -p mcglyph >&2
	mkdir -p "$target"
	set -f
	for label in $(labels); do
		statement=$(block "$label" | head -n 1 | sed 's/  \[[^]]*\]$//')
		# shellcheck disable=SC2086
		"$root/target/release/mcglyph" -o "$target/$label.png" $statement
		if [ ! -f "$target/$label.png" ]; then
			echo "worked-examples: the statement of $label needs more than one screen, so $label.png was not written; choose another example or link the numbered files by hand" >&2
			exit 1
		fi
	done
	;;
*)
	echo "usage: worked-examples.sh check|write database | images dir" >&2
	exit 2
	;;
esac
