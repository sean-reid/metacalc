#!/bin/sh
# Usage: worked-examples.sh check database
#        worked-examples.sh write database
#        worked-examples.sh images dir
# The worked examples page marks each goal tree with a line `<!-- outline: LABEL -->` followed by an
# HTML list that `mcpack outline --html` writes. check fails unless every numbered section has one
# library link, one image and one marker that name the same label, a list right after the marker,
# and a Hypotheses list equal to the essential hypotheses the database gives the theorem. It then
# regenerates every list from the database and fails when one differs. It also fails when the symbol
# table between `<!-- symbols -->` and `<!-- /symbols -->` lacks a symbol the examples use, lists one
# they do not, or spells one differently from mcpack, and when inline code outside the table still
# holds an ASCII Metamath token. write rewrites the lists. images draws the statement at the top of
# each list in the calculator font into dir, one PNG per label; it needs no database.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
page=$root/docs/src/guide/worked-examples.md
command=${1:?usage: worked-examples.sh check|write database | images dir}
target=${2:?usage: worked-examples.sh check|write database | images dir}

mcpack() { cargo run --release --locked -q --manifest-path "$root/Cargo.toml" -p mcpack -- "$@"; }
mcglyph() { cargo run --release --locked -q --manifest-path "$root/Cargo.toml" -p mcglyph -- "$@"; }

labels() { sed -n 's/^<!-- outline: \(.*\) -->$/\1/p' "$page"; }

# Prints the lines of the list that follows the marker for $1.
block() {
	awk -v label="$1" '
		$0 == "<!-- outline: " label " -->" { want = 1; next }
		want && !inside { if ($0 !~ /^<ul class="tree"/) exit; inside = 1 }
		inside { print; if ($0 == "</ul>") exit }
	' "$page"
}

# Prints a message to stderr for each numbered section that lacks its link, image, marker or block,
# or whose label differs between them, and writes tab-separated "HYPS label", "HYP label hypothesis"
# and "FILE file" lines to the file $1.
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
			if (images == 1 && !described) bad(title ": the image alt text must read " marker ": followed by a description")
			if (markers == 1 && images == 1 && marker != imagefile) bad(title ": the image file is " imagefile ".png but the marker names " marker)
			if (markers == 1 && !fenced) bad(title ": the marker for " marker " is not followed by a tree")
			if (markers == 1 && seen[marker]++) bad(title ": the marker " marker " is used in another section")
			n = split(hyps, list, "\n")
			if (markers == 1) print "HYPS\t" marker > out
			for (i = 1; i <= n; i++) if (list[i] != "") print "HYP\t" marker "\t" list[i] > out
		}
		afterMarker { fenced = ($0 ~ /^<ul class="tree"/); afterMarker = 0 }
		inh && $0 != "" && $0 !~ /^- / { inh = 0 }
		inh && $0 == "" && hyps != "" { inh = 0 }
		/^## / {
			flush()
			title = ($0 ~ /^## [0-9]+\. /) ? $0 : ""
			links = markers = images = described = fenced = inh = afterMarker = 0
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
			afterMarker = 1
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
			described = (image ~ /^[^:]+: ./)
			sub(/: .*/, "", image)
			imagefile = $0
			sub(/^!\[[^]]*\]\(examples\//, "", imagefile)
			sub(/\.png\)$/, "", imagefile)
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
		END {
			flush()
			if (!examples) bad("the page has no numbered example sections")
			exit failed
		}
	' "$page"
}

# Prints "label<TAB>statement" for each essential hypothesis in scope of each theorem named after
# the database, read from the $e statements of the enclosing blocks.
essentials() {
	database=$1
	shift
	awk -v wanted="$*" '
		BEGIN { n = split(wanted, w, /[ \n]+/); for (i = 1; i <= n; i++) want[w[i]] = 1 }
		{
			for (i = 1; i <= NF; i++) {
				t = $i
				if (comment) { if (t == "$)") comment = 0; continue }
				if (t == "$(") { comment = 1; continue }
				if (!open) {
					if (t == "${") { depth++; mark[depth] = top }
					else if (t == "$}") { top = mark[depth]; depth-- }
					else if (t ~ /^\$[eapfcvd]$/) { open = 1; kind = substr(t, 2, 1); name = prev; body = ""; proof = 0 }
					else prev = t
				} else if (t == "$.") {
					if (kind == "e") hyp[++top] = body
					else if ((kind == "p" || kind == "a") && (name in want)) for (j = 1; j <= top; j++) print name "\t" hyp[j]
					open = 0
				} else if (kind == "p" && t == "$=") proof = 1
				else if (!proof) body = (body == "" ? t : body " " t)
			}
		}
	' "$database"
}

# Writes "label page" for each label of the database the shipped libraries hold, the page being that
# of the first library, in manifest order, that holds it, to the file $2. Builds in the directory $1.
links() {
	: >"$1/manifests" || return 1
	for manifest in "$root"/libraries/*.toml; do
		[ "$(basename "$manifest")" = source.toml ] && continue
		order=$(sed -n 's/^order = \([0-9][0-9]*\)$/\1/p' "$manifest") || return 1
		[ -n "$order" ] || { echo "worked-examples: $manifest has no order line" >&2; return 1; }
		echo "$order $manifest" >>"$1/manifests" || return 1
	done
	sort -n "$1/manifests" >"$1/manifests.sorted" || return 1
	: >"$1/links.all" || return 1
	while read -r _ manifest; do
		name=$(basename "$manifest" .toml)
		mcpack build "$manifest" --database "$target" -o "$1/$name.mclib" >/dev/null ||
			{ echo "worked-examples: building the library $name failed" >&2; return 1; }
		mcpack export "$1/$name.mclib" -o "$1/$name.mm" >/dev/null ||
			{ echo "worked-examples: exporting the library $name failed" >&2; return 1; }
		awk -v page="../libraries/$name.md" '
			{
				for (i = 1; i <= NF; i++) {
					t = $i
					if (comment) { if (t == "$)") comment = 0; continue }
					if (t == "$(") { comment = 1; continue }
					if (open) { if (t == "$.") open = 0; continue }
					if (t == "${" || t == "$}") continue
					if (t ~ /^\$[eapfcvd]$/) { open = 1; if (t == "$a" || t == "$p") print prev " " page; continue }
					prev = t
				}
			}
		' "$1/$name.mm" >>"$1/links.all" || { echo "worked-examples: reading the export of $name failed" >&2; return 1; }
	done <"$1/manifests.sorted"
	awk '!seen[$1]++' "$1/links.all" >"$2" || return 1
}

# Callers run these under ||, which turns set -e off, so every command here returns its own failure.
outlines() {
	mkdir -p "$1" || return 1
	links "$1" "$1/links" || return 1
	labels >"$1/labels" || return 1
	while read -r label; do
		mcpack outline "$target" "$label" --html --links "$1/links" >"$1/$label" || return 1
		mcpack outline "$target" "$label" >"$1/$label.ascii" || return 1
	done <"$1/labels"
}

# Prints the ASCII tokens of the statements on the lines of the files, one per line, sorted.
tokens() {
	sed 's/  \[[^]]*\]$//' "$@" | tr -s ' \t' '\n\n' | sed '/^$/d' | sort -u
}

# Prints the lines of the symbol table of the page: Unicode cell, then the ASCII tokens its code
# spans hold with the surrounding spans removed, separated by a tab.
symbol_rows() {
	awk '
		$0 == "<!-- symbols -->" { inside = 1; next }
		$0 == "<!-- /symbols -->" { inside = 0 }
		inside && /^\| / && $0 !~ /^\| (Symbol|---) / {
			row = $0
			sub(/^\| /, "", row)
			sub(/ \|$/, "", row)
			n = split(row, cell, / \| /)
			ascii = cell[n]
			gsub(/`` ` ``/, "BACKTICK", ascii)
			gsub(/`/, "", ascii)
			gsub(/\\\|/, "|", ascii)
			gsub(/BACKTICK/, "`", ascii)
			print cell[1] "\t" ascii
		}
	' "$page"
}

# Prints "line:token" for each ASCII Metamath token left in inline code outside the symbol table and
# the trees. The tokens are those of the file $1, one per line.
leftovers() {
	awk -v banned="$1" '
		BEGIN { while ((getline t < banned) > 0) ban[t] = 1 }
		$0 == "<!-- symbols -->" { skip = 1 }
		$0 == "<!-- /symbols -->" { skip = 0; next }
		/^<ul class="tree"/ { skip = 1 }
		/^<\/ul>$/ { skip = 0; next }
		/^```/ { fenced = !fenced; next }
		skip || fenced { next }
		{
			line = $0
			if (line ~ /^- ``.*``$/) { sub(/^- ``/, "", line); sub(/``$/, "", line); spans = line "\001" }
			else {
				k = split(line, part, "`")
				spans = ""
				for (i = 2; i <= k; i += 2) spans = spans part[i] "\001"
			}
			count = split(spans, span, "\001")
			for (j = 1; j < count; j++) {
				m = split(span[j], tok, " ")
				for (i = 1; i <= m; i++) {
					if (!(tok[i] in ban)) continue
					printf "%d:%s\n", NR, tok[i]
				}
			}
		}
	' "$page"
}

case $command in
check)
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	status=0
	: >"$tmp/structure"
	structure "$tmp/structure" || status=1
	outlines "$tmp" || exit 1
	wanted=$(labels | tr '\n' ' ')
	essentials "$target" "$wanted" >"$tmp/essentials"
	cut -f1 "$tmp/essentials" >"$tmp/essential.names"
	cut -f2 "$tmp/essentials" | mcpack spell >"$tmp/essential.statements"
	paste "$tmp/essential.names" "$tmp/essential.statements" >"$tmp/essentials.unicode"
	while IFS=$(printf '\t') read -r kind label statement; do
		case $kind in
		FILE)
			[ -f "$root/docs/src/libraries/$label" ] ||
				{ echo "worked-examples: a link points at docs/src/libraries/$label, which does not exist" >&2; status=1; }
			;;
		HYPS)
			awk -F'\t' -v l="$label" '$1 == "HYP" && $2 == l { print $3 }' "$tmp/structure" | sort >"$tmp/listed"
			awk -F'\t' -v l="$label" '$1 == l { print $2 }' "$tmp/essentials.unicode" | sort >"$tmp/expected"
			cmp -s "$tmp/listed" "$tmp/expected" || {
				echo "worked-examples: the hypotheses listed for $label differ from the database" >&2
				diff -u "$tmp/listed" "$tmp/expected" >&2 || true
				status=1
			}
			;;
		esac
	done <"$tmp/structure"
	for label in $(labels); do
		block "$label" >"$tmp/$label.page"
		if ! diff -u "$tmp/$label.page" "$tmp/$label" >"$tmp/$label.diff"; then
			echo "worked-examples: the tree for $label in $page differs from the database" >&2
			cat "$tmp/$label.diff" >&2
			status=1
		fi
	done
	for marker in '<!-- symbols -->' '<!-- /symbols -->'; do
		[ "$(grep -cx -e "$marker" "$page")" = 1 ] ||
			{ echo "worked-examples: the page must have exactly one line $marker" >&2; status=1; }
	done
	symbol_rows >"$tmp/rows"
	{ tokens "$tmp"/*.ascii; cut -f2 "$tmp/essentials" | tr -s ' ' '\n'; } | sed '/^$/d' | sort -u >"$tmp/used"
	cut -f2 "$tmp/rows" | tr -s ' ' '\n' | sed '/^$/d' | sort -u >"$tmp/tabled"
	sort -u "$tmp/used" "$tmp/tabled" >"$tmp/all"
	: >"$tmp/spellings"
	spell_status=0
	mcpack spell <"$tmp/all" >"$tmp/spelled" 2>"$tmp/spell.err" || spell_status=$?
	if [ $spell_status -eq 0 ]; then
		paste "$tmp/all" "$tmp/spelled" >"$tmp/spellings"
		awk -F'\t' '
			function bad(msg) { print "worked-examples: " msg > "/dev/stderr"; failed = 1 }
			function same(a, b) { return length(a) == length(b) && index(a, b) == 1 }
			FILENAME == ARGV[1] { spelling[$1] = $2; next }
			FILENAME == ARGV[2] { used[$1] = 1; next }
			{
				n = split($1, uni, " ")
				m = split($2, asc, " ")
				if (n != m) { bad("the symbol table row " $1 " has " n " symbols and " m " tokens"); next }
				for (i = 1; i <= n; i++) {
					listed[asc[i]] = 1
					if (!same(spelling[asc[i]], uni[i])) bad("the symbol table shows " asc[i] " as " uni[i] " but mcpack spells it " spelling[asc[i]])
				}
			}
			END {
				for (t in used) if (!(t in listed)) bad("the symbol table lacks " t ", which the examples use")
				for (t in listed) if (!(t in used)) bad("the symbol table lists " t ", which no example uses")
				exit failed
			}
		' "$tmp/spellings" "$tmp/used" "$tmp/rows" || status=1
	elif [ $spell_status -eq 1 ]; then
		cat "$tmp/spell.err" >&2
		echo "worked-examples: a symbol of the page or the examples has no Unicode spelling" >&2
		status=1
	else
		cat "$tmp/spell.err" >&2
		echo "worked-examples: mcpack spell failed with status $spell_status" >&2
		status=1
	fi
	[ -s "$tmp/rows" ] || { echo "worked-examples: the page has no symbol table" >&2; status=1; }
	awk -F'\t' 'function same(a, b) { return length(a) == length(b) && index(a, b) == 1 } !same($1, $2) { print $1 }' "$tmp/spellings" >"$tmp/banned"
	leftovers "$tmp/banned" >"$tmp/leftovers"
	while IFS=: read -r number token; do
		echo "worked-examples: line $number of the page has the ASCII token $token in inline code" >&2
		status=1
	done <"$tmp/leftovers"
	[ $status -eq 0 ] || echo "worked-examples: run scripts/worked-examples.sh write $target to rewrite the trees" >&2
	exit $status
	;;
write)
	tmp=$(mktemp -d)
	trap 'rm -rf "$tmp"' EXIT
	outlines "$tmp" || exit 1
	awk -v dir="$tmp" '
		/^<!-- outline: .* -->$/ { label = $0; sub(/^<!-- outline: /, "", label); sub(/ -->$/, "", label); want = 1; print; next }
		want && !inside {
			want = 0
			if ($0 ~ /^<ul class="tree"/) { inside = 1; while ((getline line < (dir "/" label)) > 0) print line; next }
		}
		inside { if ($0 == "</ul>") inside = 0; next }
		{ print }
	' "$page" >"$tmp/page"
	cp "$tmp/page" "$page"
	;;
images)
	mkdir -p "$target"
	set -f
	for label in $(labels); do
		statement=$(block "$label" | sed -n '1s/^<ul class="tree" role="list" data-statement="\(.*\)">$/\1/p' |
			sed 's/&lt;/</g; s/&gt;/>/g; s/&quot;/"/g; s/&amp;/\&/g')
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
