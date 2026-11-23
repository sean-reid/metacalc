#!/bin/sh
# Usage: stages-include.sh library [list]
# Prints the assembler lines of the app's stage table: the stages the Libraries screen shows, one
# row each. The list is stages.list, `name count title` per line, smallest first, as
# scripts/build-libraries.sh writes it beside the libraries. The build fails when the ends do not
# rise, when the last end is not the library's assertion count (the u16 at offset 14 of its
# header), or when the list has more than 6 stages, a name over 12 characters, a title over 48,
# or a character that is not printable ASCII.
# With no list there is one stage: the library's whole count, named like the library file without
# its extension and with no title. That stage's end is read from the library when the app runs,
# so STAGES_FROM_LIBRARY is defined and its end here is 0.
set -eu
export LC_ALL=C

usage='usage: stages-include.sh library [list]'
lib=${1:?$usage}
list=${2:-}

[ -f "$lib" ] || { echo "stages-include: $lib is not a file" >&2; exit 1; }

# One line per stage, name then count then title, from the list or from the library's file name.
if [ -n "$list" ]; then
	[ -f "$list" ] || { echo "stages-include: $list is not a file" >&2; exit 1; }
	stages=$(awk 'NF { name = $1; count = $2; title = $0; sub(/^[ \t]*[^ \t]+[ \t]+[^ \t]+[ \t]*/, "", title); print name "\t" count "\t" title }' "$list")
	[ -n "$stages" ] || { echo "stages-include: $list has no stage" >&2; exit 1; }
else
	file=${lib##*/}
	stages=$(printf '%s\t0\t' "${file%.*}")
fi

# The text of `$1` as glyph codes, comma separated; nothing for an empty one.
codes() {
	printf '%s' "$1" | od -An -tx1 -v | awk '{ for (i = 1; i <= NF; i++) printf "%sUI_FONT_CH_%s", (n++ ? ", " : ""), toupper($i) }'
}

count=0
if [ -n "$list" ]; then
	header=$(od -An -tu1 -j14 -N2 "$lib" | tr -s ' ')
	set -- $header
	[ "$#" -eq 2 ] || { echo "stages-include: $lib is too short for a library header" >&2; exit 1; }
	count=$(($1 + 256 * $2))
fi

tab=$(printf '\t')
n=0
previous=0
ends=
text=
names=
while IFS=$tab read -r name end title; do
	n=$((n + 1))
	case $end in
		'' | *[!0-9]*) echo "stages-include: stage $name: the count '$end' is not a number" >&2; exit 1 ;;
	esac
	[ "$n" -le 6 ] || { echo "stages-include: more than 6 stages" >&2; exit 1; }
	if [ "${#name}" -lt 1 ] || [ "${#name}" -gt 12 ]; then
		if [ -n "$list" ]; then
			echo "stages-include: stage '$name': a name is 1 to 12 characters" >&2
		else
			echo "stages-include: with no stage list the stage is named after the library file, '$name', which is not 1 to 12 characters: rename the file or give a stages.list" >&2
		fi
		exit 1
	fi
	[ "${#title}" -le 48 ] || { echo "stages-include: stage $name: a title is at most 48 characters" >&2; exit 1; }
	case $name$title in
		*[![:print:]]*) echo "stages-include: stage $name: only printable ASCII" >&2; exit 1 ;;
	esac
	if [ -n "$list" ]; then
		[ "$end" -gt "$previous" ] || { echo "stages-include: stage $name ends at $end, which does not rise above $previous" >&2; exit 1; }
		previous=$end
	fi
	ends="$ends$end, "
	names="$names	.dw stage_name_$n, stage_title_$n
"
	text="${text}stage_name_$n:
	.db $(codes "$name"), 0
stage_title_$n:
	.db $(codes "$title")${title:+, }0
"
done <<EOF
$stages
EOF

if [ -n "$list" ] && [ "$previous" -ne "$count" ]; then
	echo "stages-include: $list ends at $previous and $lib has $count assertions" >&2
	exit 1
fi

[ -n "$list" ] || echo '#define STAGES_FROM_LIBRARY'
echo "STAGE_COUNT .equ $n"
echo 'stage_list:'
echo "	.db $n"
echo "	.dw ${ends%, }"
echo 'stage_text:'
printf '%s' "$names"
printf '%s' "$text"
