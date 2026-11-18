#!/bin/sh
# Usage: compare-frames.sh export database
# Compares every assertion of an exported library with the same label in the database it was built
# from, through metamath-exe (METAMATH, default .metamath/metamath at the repo root). The frame of
# an assertion is its statement, the number of its mandatory hypotheses, the hypotheses in order
# with hypothesis labels left out, and its mandatory disjoint variable pairs. Prints each
# difference and exits 1 if there is one. A line starting ?Error in the output of metamath-exe is a
# failure.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
export=$1
database=$2
metamath=${METAMATH:-$root/.metamath/metamath}

[ -x "$metamath" ] || { echo "compare-frames: $metamath is not an executable, run scripts/fetch-metamath.sh" >&2; exit 1; }
for file in "$export" "$database"; do
	[ -f "$file" ] && [ -r "$file" ] || { echo "compare-frames: $file is not a readable file" >&2; exit 1; }
	case $file in *\"*) echo "compare-frames: the name $file has a quote" >&2; exit 1 ;; esac
done
tmp=$(mktemp -d)
case $tmp in *\"*) echo "compare-frames: the temporary directory $tmp has a quote" >&2; exit 1 ;; esac
trap 'rm -rf "$tmp"' EXIT

# The frame of each statement in the output of `show statement ... /full`, one line per part.
# Takes a name for the files in $tmp where it leaves the count of statements it parsed and any
# line of the output it could not place.
frames() {
	awk -v errfile="$tmp/$1.errors" -v countfile="$tmp/$1.count" '
	function bad(why) { print why > errfile }
	function squeeze(text) {
		gsub(/[ \t]+/, " ", text)
		sub(/^ /, "", text)
		sub(/ $/, "", text)
		return text
	}
	function flush(   n, i, pair, text) {
		if (state == 0) {
			bad("the header of statement " headers " is followed by no statement")
			return
		}
		text = squeeze(stmt)
		label = text
		sub(/ .*/, "", label)
		if (text ~ /^[^ ]+ \$[fe] .* \$\.$/) {
			skipped++
			return
		}
		if (text !~ /^[^ ]+ \$[ap] .* \$\.$/) {
			bad("statement " label " did not parse: " text)
			return
		}
		parsed++
		label = text
		sub(/ .*/, "", label)
		print label "\tstatement\t" text
		print label "\thypotheses\t" nhyp
		for (i = 1; i <= nhyp; i++) {
			text = squeeze(hyp[i])
			if (text !~ /^[^ ]+ \$[fe] .* \$\.$/) {
				bad("a hypothesis of " label " did not parse: " text)
				continue
			}
			sub(/^[^ ]+ /, "", text)
			print label "\thypothesis " i "\t" text
		}
		n = split(dv, pairs, " ")
		for (i = 1; i <= n; i++) {
			sub(/,$/, "", pairs[i])
			if (pairs[i] !~ /^<[^,<>]+,[^,<>]+>$/) {
				bad("a disjoint pair of " label " did not parse: " pairs[i])
				continue
			}
			split(substr(pairs[i], 2, length(pairs[i]) - 2), pair, ",")
			print label "\tdisjoint\t" (pair[1] < pair[2] ? pair[1] "," pair[2] : pair[2] "," pair[1])
		}
	}
	/^\?Error/ { bad("metamath-exe reported: " $0); next }
	/^Statement [0-9]+ is located/ {
		if (headers > 0) flush()
		headers++
		state = 0; dv = ""; nhyp = 0; label = ""
		next
	}
	state == 0 && headers > 0 && /^[0-9]+( |$)/ {
		stmt = $0
		sub(/^[0-9]+/, "", stmt)
		state = 1
		next
	}
	/^Its mandatory hypotheses in RPN order are:/ { state = 2; next }
	/^Its mandatory disjoint variable pairs are:/ {
		state = 3
		dv = $0
		sub(/^[^:]*:/, "", dv)
		next
	}
	/^(Its optional hypotheses|The statement and its hypotheses|These additional variables|The variables it contains)/ { state = 4; next }
	/^-+$/ { next }
	state == 1 { stmt = stmt " " $0; next }
	state == 2 && /^  \(None\)$/ { next }
	state == 2 && /^  [^ ]/ { hyp[++nhyp] = substr($0, 3); next }
	state == 2 && /^   / { hyp[nhyp] = hyp[nhyp] " " $0; next }
	state == 2 && /[^ \t]/ { bad("a line among the hypotheses of statement " headers " was not understood: " $0); next }
	state == 3 && /^ / { dv = dv " " $0; next }
	END {
		if (headers > 0) flush()
		print headers " " parsed + 0 " " skipped + 0 > countfile
	}
	' | awk -F'\t' '
	{
		text = $3
		sub(/ \$= \.\.\. \$\.$/, " $.", text)
		print $1 "\t" $2 "\t" text
	}' | LC_ALL=C sort
}

# Fails unless frames parsed every statement of the output: no line it could not place, a
# statement after every header, and as many statements as assertion labels in the export.
parsed_all() {
	if [ -s "$tmp/$1.errors" ]; then
		echo "compare-frames: the output for $2 has lines that were not understood:" >&2
		head -n 10 "$tmp/$1.errors" >&2
		exit 1
	fi
	read -r headers parsed skipped <"$tmp/$1.count"
	[ "$headers" = $((parsed + skipped)) ] || { echo "compare-frames: $2 has $headers statements and $parsed assertions and $skipped hypotheses were parsed" >&2; exit 1; }
	[ "$parsed" = "$labels" ] || { echo "compare-frames: $2 has $labels assertions in the export and $parsed were compared" >&2; exit 1; }
}

labels=$(tr -s ' \t\r\n' '\n' <"$export" | grep -c '^\$[ap]$' || true)
[ "$labels" -gt 0 ] || { echo "compare-frames: no assertions found in $export" >&2; exit 1; }
"$metamath" "read \"$export\"" 'show statement * /full' exit </dev/null 2>&1 | frames export >"$tmp/export"
parsed_all export "$export"
cut -f1 "$tmp/export" | LC_ALL=C sort -u >"$tmp/labels"
sed 's/.*/show statement & \/full/' "$tmp/labels" >"$tmp/commands"
"$metamath" "read \"$database\"" "submit \"$tmp/commands\"" exit </dev/null 2>&1 | frames source >"$tmp/source"
parsed_all source "$database"

if cmp -s "$tmp/export" "$tmp/source"; then
	echo "ok $export: $labels assertions have the frames of $(basename "$database")"
else
	echo "the frames of $export differ from those in $database (export <, database >):"
	diff "$tmp/export" "$tmp/source" | head -40
	exit 1
fi
