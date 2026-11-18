#!/bin/sh
# Usage: worked-examples.sh check database
#        worked-examples.sh write database
#        worked-examples.sh images dir
# The worked examples page marks each goal tree with a line `<!-- outline: LABEL -->` followed by a
# fenced block. check fails unless every numbered section has one library link, one image and one
# marker that name the same label, a block right after the marker, and hypotheses that appear in
# its tree. It then regenerates every tree from the database with `mcpack outline` and fails when
# a block differs. write rewrites the blocks. images draws the statement at the top of each block
# in the calculator font into dir, one PNG per label; it needs no database.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
page=$root/docs/src/guide/worked-examples.md
command=${1:?usage: worked-examples.sh check|write database | images dir}
target=${2:?usage: worked-examples.sh check|write database | images dir}

mcpack() { cargo run --release --locked -q --manifest-path "$root/Cargo.toml" -p mcpack -- "$@"; }
mcglyph() { cargo run --release --locked -q --manifest-path "$root/Cargo.toml" -p mcglyph -- "$@"; }

labels() { sed -n 's/^<!-- outline: \(.*\) -->$/\1/p' "$page"; }

# Prints the lines of the block that follows the marker for $1.
block() {
	awk -v label="$1" '
		$0 == "<!-- outline: " label " -->" { want = 1; next }
		want && /^```/ { if (inside) exit; inside = 1; next }
		inside { print }
	' "$page"
}

# Prints a message to stderr for each numbered section that lacks its link, image, marker or block,
# or whose label differs between them, and writes tab-separated "label hypothesis" and "FILE file"
# lines to the file $1.
structure() {
	awk -v out="$1" '
		function bad(msg) { print "worked-examples: " msg > "/dev/stderr"; failed = 1 }
		function flush(   n, i, list) {
			if (!title) return
			if (links != 1) bad(title ": expected one library link, found " links)
			if (markers != 1) bad(title ": expected one outline marker, found " markers)
			if (images != 1) bad(title ": expected one image, found " images)
			if (markers == 1 && links == 1 && marker != link) bad(title ": the link names " link " but the marker names " marker)
			if (markers == 1 && images == 1 && marker != image) bad(title ": the image names " image " but the marker names " marker)
			if (markers == 1 && !fenced) bad(title ": the marker for " marker " is not followed by a code block")
			n = split(hyps, list, "\n")
			for (i = 1; i <= n; i++) if (list[i] != "") print marker "\t" list[i] > out
		}
		/^## / {
			flush()
			title = ($0 ~ /^## [0-9]+\. /) ? $0 : ""
			links = markers = images = fenced = inh = 0
			hyps = ""
			examples += (title != "")
			next
		}
		/^<!-- outline: .* -->$/ {
			if (!title) bad("an outline marker is outside a numbered section: " $0)
			markers++
			marker = $0
			sub(/^<!-- outline: /, "", marker)
			sub(/ -->$/, "", marker)
			getline
			fenced = ($0 ~ /^```/)
			next
		}
		/^\[`[^`]*`\]\(\.\.\/libraries\/[a-z]+\.md#[^)]*\)/ {
			links++
			link = $0
			sub(/^\[`/, "", link)
			sub(/`\].*/, "", link)
			anchor = $0
			sub(/^[^#]*#/, "", anchor)
			sub(/\).*/, "", anchor)
			file = $0
			sub(/^[^(]*\(\.\.\/libraries\//, "", file)
			sub(/#.*/, "", file)
			if (anchor != link) bad(title ": the link text " link " and its anchor " anchor " differ")
			print "FILE\t" file > out
			next
		}
		/^!\[[^]]*\]\(examples\/[^)]*\.png\)$/ {
			images++
			image = $0
			sub(/^!\[/, "", image)
			sub(/\].*/, "", image)
			next
		}
		/^Hypotheses:$/ { inh = 1; next }
		inh && /^- / {
			line = $0
			sub(/^- */, "", line)
			sub(/^`+/, "", line)
			sub(/`+$/, "", line)
			hyps = hyps line "\n"
			next
		}
		inh && $0 != "" { inh = 0 }
		END {
			flush()
			if (!examples) bad("the page has no numbered example sections")
			exit failed
		}
	' "$page"
}

outlines() {
	mkdir -p "$1"
	for label in $(labels); do
		mcpack outline "$target" "$label" >"$1/$label"
	done
}

case $command in
check)
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	status=0
	: >"$tmp/structure"
	structure "$tmp/structure" || status=1
	outlines "$tmp"
	while IFS=$(printf '\t') read -r label statement; do
		if [ "$label" = FILE ]; then
			[ -f "$root/docs/src/libraries/$statement" ] ||
				{ echo "worked-examples: a link points at docs/src/libraries/$statement, which does not exist" >&2; status=1; }
		elif [ -f "$tmp/$label" ]; then
			sed 's/^ *//; s/  \[[^]]*\]$//' "$tmp/$label" | grep -qxF -e "$statement" ||
				{ echo "worked-examples: the hypothesis $statement of $label is not in its tree" >&2; status=1; }
		fi
	done <"$tmp/structure"
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
	mkdir -p "$target"
	set -f
	for label in $(labels); do
		statement=$(block "$label" | head -n 1 | sed 's/  \[[^]]*\]$//')
		# shellcheck disable=SC2086
		mcglyph -o "$target/$label.png" $statement
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
