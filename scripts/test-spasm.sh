#!/bin/sh
# Usage: test-spasm.sh [spasm.sh]
# Checks that spasm.sh fails on each class of error and warning spasm reports, removes the output
# when it fails, passes spasm's own exit status through, and accepts a clean source. The errors
# come from the real assembler in $SPASM; a stub assembler covers the exit status and colour codes.
set -eu

wrapper=${1:-$(cd "$(dirname "$0")" && pwd)/spasm.sh}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/with space"
cd "$tmp"

# run NAME SOURCE: assembles SOURCE (the text of a .z80 file) to out.bin and sets status.
run() {
	printf '%s\n' "$2" > "$tmp/with space/$1.z80"
	printf 'stale' > "$tmp/with space/out.bin"
	status=0
	sh "$wrapper" -N "$tmp/with space/$1.z80" "$tmp/with space/out.bin" > "$tmp/log" 2>&1 || status=$?
}

# refuses NAME SOURCE TEXT: the wrapper must fail, say TEXT, and leave no output file.
refuses() {
	run "$1" "$2"
	if [ "$status" -eq 0 ]; then
		echo "FAIL $1: succeeded" >&2
		cat "$tmp/log" >&2
		exit 1
	fi
	grep -q -e "$3" "$tmp/log" || { echo "FAIL $1: output was: $(cat "$tmp/log")" >&2; exit 1; }
	if [ -e "$tmp/with space/out.bin" ]; then
		echo "FAIL $1: left the output file" >&2
		exit 1
	fi
	echo "ok $1"
}

run clean '	ld b,3
	ret'
if [ "$status" -ne 0 ] || [ ! -s "$tmp/with space/out.bin" ] || [ "$(wc -c < "$tmp/with space/out.bin")" -ne 3 ]; then
	echo "FAIL clean: status $status: $(cat "$tmp/log")" >&2
	exit 1
fi
echo "ok clean (path with a space)"

refuses undefined-label '	ld b,nolabel' 'SE106'
refuses undefined-jump '	jp missing' 'SE106'
refuses undefined-macro '	nomacro 1,2' 'SE113'
refuses syntax '	ld a,,' 'SE110'
refuses unknown-opcode '	frob a' 'SE113'
refuses unknown-directive '	.frob 1' 'SE113'
refuses unknown-preop '#frob' 'SE111'
refuses unbalanced-if '#if 1' 'SE116'
refuses operand-as-label '	ld a,hl' 'SE106'
refuses missing-include '#include "nofile.inc"' 'SE001'
refuses jump-too-far '	jr far
	.fill 200,0
far:	nop' 'SE120'
refuses index-offset '	ld a,(ix+200)' 'SE121'
refuses divide-by-zero '	ld a,1/0' 'SE203'
refuses bad-digit '	ld a,$ZZ' 'SE103'
refuses explicit-error '	.error "boom"' 'SE600'
refuses truncation '	ld a,$1FF' 'SW800'
refuses macro-arguments '#macro M(a)
	nop
#endmacro
	M(1,2,3)' 'SW804'
refuses redefinition 'x	.equ 1
x	.equ 2' 'redefinition'

# A stub spasm: the first line of its source names what it prints and the status it returns.
cat > "$tmp/stub.sh" <<'STUB'
#!/bin/sh
eval "$(sed -n 1p "$1")"
printf '%b\n' "$SAY"
printf built > "$2"
exit "${CODE:-0}"
STUB
chmod +x "$tmp/stub.sh"

# stub NAME LINE: runs the wrapper over the stub assembler with LINE as the stub's script.
stub() {
	printf '%s\n' "$2" > "$tmp/$1.in"
	rm -f "$tmp/$1.out"
	status=0
	SPASM="$tmp/stub.sh" sh "$wrapper" "$tmp/$1.in" "$tmp/$1.out" > "$tmp/log" 2>&1 || status=$?
}

stub quiet 'SAY=fine'
[ "$status" -eq 0 ] && [ -e "$tmp/quiet.out" ] || { echo "FAIL stub-quiet" >&2; exit 1; }
stub exit3 'SAY=fine CODE=3'
[ "$status" -eq 3 ] && [ ! -e "$tmp/exit3.out" ] || { echo "FAIL stub-exit-3: status $status" >&2; exit 1; }
stub exit1 'SAY=fine CODE=1'
[ "$status" -eq 1 ] && [ ! -e "$tmp/exit1.out" ] || { echo "FAIL stub-exit-1: status $status" >&2; exit 1; }
echo "ok exit status passes through"

stub colour 'SAY="\033[1;31mf.z80:1: error SE106: Could not find label or macro x\033[0m"'
[ "$status" -ne 0 ] && [ ! -e "$tmp/colour.out" ] || { echo "FAIL stub-colour: status $status" >&2; exit 1; }
grep -q SE106 "$tmp/log" || { echo "FAIL stub-colour: output was dropped" >&2; exit 1; }
echo "ok colour codes do not hide an error"

stub words 'SAY="the error SE106 is mentioned but this is not a report"'
[ "$status" -eq 0 ] || { echo "FAIL stub-words: matched prose" >&2; exit 1; }
echo "ok prose that is not a report passes"

status=0
SPASM="$tmp/missing-assembler" sh "$wrapper" "$tmp/quiet.in" "$tmp/missing.out" > /dev/null 2>&1 || status=$?
[ "$status" -ne 0 ] || { echo "FAIL missing assembler succeeded" >&2; exit 1; }
echo "ok missing assembler fails"
echo "test-spasm: ok"
