#!/bin/sh
# Usage: test-stages.sh spasm
# Builds the app around a library with a stage list beside it, and checks that the build refuses a
# list whose last end is not the library's assertion count, ends that do not rise, a list that
# cannot be shown, and a list named with STAGES that is missing; and that a build with no list has
# one stage named like the library file.
set -eu
export LC_ALL=C

spasm=${1:?usage: test-stages.sh spasm}
app=$(cd "$(dirname "$0")/.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

# A library file of 64 bytes whose header holds $1 as its assertion count.
make_library() {
	lo=$(($1 % 256))
	hi=$(($1 / 256))
	{
		printf 'MCL1'
		printf '\000\000\000\000\000\000\000\000\000\000'
		printf "$(printf '\\%03o\\%03o' "$lo" "$hi")"
		i=0
		while [ "$i" -lt 48 ]; do
			printf 'x'
			i=$((i + 1))
		done
	} > "$dir/$2"
}

# Builds into a directory of its own with the make arguments after the first.
build() {
	out=$1
	shift
	rm -rf "$dir/$out"
	make -s -C "$app" SPASM="$spasm" BUILD="$dir/$out" "$@"
}

# The build must fail and say $1.
refuses() {
	want=$1
	shift
	if got=$(build refused "$@" 2>&1); then
		echo "test-stages: the build passed, wanted a refusal naming '$want'" >&2
		exit 1
	fi
	case $got in
		*"$want"*) ;;
		*) echo "test-stages: wanted '$want' in: $got" >&2; exit 1 ;;
	esac
}

make_library 3326 ring.mclib
printf 'prop 779 Propositional calculus\npred 1171 Predicate calculus\nring 3326 Ring theory\n' > "$dir/stages.list"
build listed LIBRARY="$dir/ring.mclib" > /dev/null
inc=$dir/listed/stages.inc
grep -q '^STAGE_COUNT .equ 3$' "$inc" || { echo "test-stages: no stage count of 3" >&2; exit 1; }
grep -q '^	\.dw 779, 1171, 3326$' "$inc" || { echo "test-stages: the ends are not 779, 1171, 3326" >&2; exit 1; }
if grep -q STAGES_FROM_LIBRARY "$inc"; then
	echo "test-stages: a list was given and the end is the library's" >&2
	exit 1
fi
echo "test-stages: three stages"

rm "$dir/stages.list"
build unlisted LIBRARY="$dir/ring.mclib" > /dev/null
inc=$dir/unlisted/stages.inc
grep -q '^#define STAGES_FROM_LIBRARY$' "$inc" || { echo "test-stages: no list and the end is not the library's" >&2; exit 1; }
grep -q '^STAGE_COUNT .equ 1$' "$inc" || { echo "test-stages: no list and not one stage" >&2; exit 1; }
# "ring", the library file's name without .mclib, as glyph codes: r i n g.
grep -q 'UI_FONT_CH_72, UI_FONT_CH_69, UI_FONT_CH_6E, UI_FONT_CH_67, 0$' "$inc" || { echo "test-stages: the stage is not named ring" >&2; exit 1; }
echo "test-stages: no list, one stage"

printf 'prop 779 Propositional calculus\npred 1171 Predicate calculus\nring 3325 Ring theory\n' > "$dir/stages.list"
refuses 'ends at 3325 and' LIBRARY="$dir/ring.mclib"
printf 'prop 779 Propositional calculus\npred 1171 Predicate calculus\nring 3327 Ring theory\n' > "$dir/stages.list"
refuses 'ends at 3327 and' LIBRARY="$dir/ring.mclib"
echo "test-stages: a last end that is not the library's count"

printf 'prop 779 P\npred 779 Q\nring 3326 R\n' > "$dir/stages.list"
refuses 'does not rise above 779' LIBRARY="$dir/ring.mclib"
printf 'prop 1171 P\npred 779 Q\nring 3326 R\n' > "$dir/stages.list"
refuses 'does not rise above 1171' LIBRARY="$dir/ring.mclib"
printf 'prop 0 P\nring 3326 R\n' > "$dir/stages.list"
refuses 'does not rise above 0' LIBRARY="$dir/ring.mclib"
echo "test-stages: ends that do not rise"

printf 'prop many P\nring 3326 R\n' > "$dir/stages.list"
refuses 'is not a number' LIBRARY="$dir/ring.mclib"
printf 'averyverylongname 3326 R\n' > "$dir/stages.list"
refuses 'a name is 1 to 12 characters' LIBRARY="$dir/ring.mclib"
printf 'ring 3326 %s\n' "$(printf 'x%.0s' $(seq 1 49))" > "$dir/stages.list"
refuses 'a title is at most 48 characters' LIBRARY="$dir/ring.mclib"
seq 1 7 | awk '{ printf "s%d %d T\n", $1, $1 * 100 }' | sed '$s/700/3326/' > "$dir/stages.list"
refuses 'more than 6 stages' LIBRARY="$dir/ring.mclib"
printf 'ring 3326 caf\303\251\n' > "$dir/stages.list"
refuses 'only printable ASCII' LIBRARY="$dir/ring.mclib"
echo "test-stages: lists that cannot be shown"

rm "$dir/stages.list"
refuses 'is not a file' LIBRARY="$dir/ring.mclib" STAGES="$dir/missing.list"
printf 'ring 3326 Ring theory\n' > "$dir/elsewhere.list"
build elsewhere LIBRARY="$dir/ring.mclib" STAGES="$dir/elsewhere.list" > /dev/null
grep -q '^STAGE_COUNT .equ 1$' "$dir/elsewhere/stages.inc" || { echo "test-stages: STAGES was not read" >&2; exit 1; }
if grep -q STAGES_FROM_LIBRARY "$dir/elsewhere/stages.inc"; then
	echo "test-stages: STAGES was given and the end is the library's" >&2
	exit 1
fi
echo "test-stages: a list given as STAGES"
echo "test-stages: ok"
