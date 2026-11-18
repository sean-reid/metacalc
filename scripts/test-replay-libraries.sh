#!/bin/sh
# Usage: test-replay-libraries.sh
# Checks that check-replayed.sh, the metamath-exe stage of replay-libraries.sh, passes a spliced
# database that is right and refuses one with a wrong proof, one with a disjoint variable violation,
# one with fewer proofs than recorded, and a library the replay wrote no database for. Every case
# runs through the real metamath-exe.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

write_tiny() {
	cat <<'DB'
$c ( ) -> wff |- A. setvar $.
$v ph ps x y $.
wph $f wff ph $.
wps $f wff ps $.
vx $f setvar x $.
vy $f setvar y $.
wi $a wff ( ph -> ps ) $.
wal $a wff A. x ph $.
${
  $d ph x $.
  ax-g $a |- ( ph -> A. x ph ) $.
$}
${
  $d ps y $.
  th-g $p |- ( ps -> A. y ps ) $= wps vy ax-g $.
$}
${
  $d ph y $.
  th-h $p |- ( ph -> A. y ph ) $= wph vy ax-g $.
$}
DB
}

# case NAME SED COUNT: libdir NAME holds the original database and COUNT as the recorded count;
# spliced/NAME holds the database with SED applied.
case_dirs() {
	mkdir -p "$tmp/$1/lib" "$tmp/$1/spliced"
	write_tiny >"$tmp/$1/lib/tiny.mm"
	write_tiny | sed "$2" >"$tmp/$1/spliced/tiny.mm"
	echo "$3" >"$tmp/$1/lib/tiny.proofs"
	echo tiny >"$tmp/$1/lib/libraries.list"
}

run() { sh "$root/scripts/check-replayed.sh" "$tmp/$1/lib" "$tmp/$1/spliced" >"$tmp/out" 2>"$tmp/err"; }

expect() {
	if run "$1"; then
		echo "FAIL $1: succeeded" >&2
		exit 1
	fi
	grep -qF -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	if grep -q 'metamath-exe verified' "$tmp/out"; then echo "FAIL $1: reported a verified count" >&2; exit 1; fi
	echo "ok $1"
}

case_dirs good 's/^$//' 2
run good || { echo "FAIL good: $(cat "$tmp/err")" >&2; exit 1; }
[ "$(cat "$tmp/out")" = "metamath-exe verified 2 of 2" ] || { echo "FAIL good: stdout was: $(cat "$tmp/out")" >&2; exit 1; }
echo "ok good"

case_dirs wrong-proof 's/wps vy ax-g/wph vy ax-g/' 2
expect wrong-proof 'FAIL metamath-exe tiny: the verifier reported problems'

case_dirs disjoint-violation '/^  \$d ps y \$\.$/d' 2
expect disjoint-violation 'FAIL metamath-exe tiny: the verifier reported problems'

case_dirs short-count '/^  th-h /d' 2
expect short-count 'has 1 proofs, want 2'

case_dirs no-proofs 's/\$= wps vy ax-g \$\./$= ? $./;s/\$= wph vy ax-g \$\./$= ? $./' 2
expect no-proofs 'unproved statements or warnings'

case_dirs missing-database 's/^$//' 2
rm "$tmp/missing-database/spliced/tiny.mm"
expect missing-database 'the replay wrote no'

case_dirs no-final-newline 's/wps vy ax-g/wph vy ax-g/' 2
printf 'tiny' >"$tmp/no-final-newline/lib/libraries.list"
expect no-final-newline 'FAIL metamath-exe tiny: the verifier reported problems'
