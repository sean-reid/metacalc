#!/bin/sh
# Usage: test-verify-exports.sh
# Checks that verify-exports.sh passes small databases and refuses each way it can fail, matching
# the script's own message. Faulty databases run through the real metamath-exe, and canned output
# from a stub verifier reaches the checks the real one never gets past.
# shellcheck disable=SC2016
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

write_tiny() {
	cat <<'DB'
$c ( ) -> wff |- A. setvar $.
$v ph ps ch x y $.
wph $f wff ph $.
wps $f wff ps $.
wch $f wff ch $.
vx $f setvar x $.
vy $f setvar y $.
wi $a wff ( ph -> ps ) $.
wal $a wff A. x ph $.
${
  ax-mp.1 $e |- ph $.
  ax-mp.2 $e |- ( ph -> ps ) $.
  ax-mp $a |- ps $.
$}
${
  $d ph x $.
  ax-g $a |- ( ph -> A. x ph ) $.
$}
${
  $d ps y $.
  th-g $p |- ( ps -> A. y ps ) $= wps vy ax-g $.
$}
${
  mp2.1 $e |- ph $.
  mp2.2 $e |- ps $.
  mp2.3 $e |- ( ph -> ( ps -> ch ) ) $.
  mp2 $p |- ch $= wps wch mp2.2 wph wps wch wi mp2.1 mp2.3 ax-mp ax-mp $.
$}
${
  use.1 $e |- ph $.
  use.2 $e |- ps $.
  use.3 $e |- ( ph -> ( ps -> ch ) ) $.
  use $p |- ch $= wph wps wch use.1 use.2 use.3 mp2 $.
$}
DB
}

# database DIR [SED]: writes DIR/tiny.mm, the tiny database with SED applied, and DIR/tiny.proofs.
database() {
	mkdir -p "$1"
	write_tiny | sed "${2:-}" >"$1/tiny.mm"
	echo 3 >"$1/tiny.proofs"
	echo tiny >"$1/libraries.list"
}

# run DIR [VERIFIER]: runs verify-exports.sh on DIR, with VERIFIER as the verifier if given.
run() {
	if [ -n "${2:-}" ]; then
		METAMATH=$2 sh "$root/scripts/verify-exports.sh" "$1" >"$tmp/out" 2>"$tmp/err"
	else
		sh "$root/scripts/verify-exports.sh" "$1" >"$tmp/out" 2>"$tmp/err"
	fi
}

# expect NAME MESSAGE DIR: verify-exports.sh must exit nonzero with MESSAGE on stderr.
expect() {
	if run "$3"; then
		echo "FAIL $1: succeeded" >&2
		exit 1
	fi
	grep -qF -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

# expect_ok NAME DIR PROOFS: verify-exports.sh must succeed and report PROOFS proofs.
expect_ok() {
	run "$2" || { echo "FAIL $1: $(cat "$tmp/err")" >&2; exit 1; }
	grep -qF ": $3 proofs verified" "$tmp/out" || { echo "FAIL $1: stdout was: $(cat "$tmp/out")" >&2; exit 1; }
	echo "ok $1"
}

# stub NAME OUTPUT: a verifier that ignores its arguments and prints OUTPUT, in $tmp/NAME.
stub() {
	printf '#!/bin/sh\ncat "$0.out"\n' >"$tmp/$1"
	printf '%s\n' "$2" >"$tmp/$1.out"
	chmod +x "$tmp/$1"
}

# expect_stub NAME MESSAGE OUTPUT: with a verifier that prints OUTPUT the good database must fail.
expect_stub() {
	stub "$1" "$3"
	if run "$tmp/good" "$tmp/$1"; then echo "FAIL $1: succeeded" >&2; exit 1; fi
	grep -qF -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

database "$tmp/good"
expect_ok good "$tmp/good" 3

summary='The source has 5927 statements; 23 are $a and 3 are $p.'
clean='No errors were found.  However, proofs were not checked.'
verified='All proofs in the database were verified in 0.00 s.'
stub canned "$clean
$summary
$verified"
run "$tmp/good" "$tmp/canned" || { echo "FAIL canned: $(cat "$tmp/err")" >&2; exit 1; }
echo "ok canned"
expect_stub no-clean-read 'did not report a clean read' "$summary
$verified"
expect_stub no-verified-line 'did not report every proof verified' "$clean
$summary"
expect_stub zero-proofs 'has no proofs' "$clean
The source has 5927 statements; 23 are \$a and 0 are \$p.
$verified"
expect_stub proof-count-differs 'has 4 proofs, want 3' "$clean
The source has 5927 statements; 23 are \$a and 4 are \$p.
$verified"
expect_stub two-summaries 'printed 2 source summaries' "$clean
$summary
$summary
$verified"
expect_stub no-summary 'printed 0 source summaries' "$clean
$verified"
expect_stub warning-line 'unproved statements or warnings' "$clean
$summary
Warning: The following \$p statement(s) were not proved:  th-g
$verified"
expect_stub question-line 'reported problems' "$clean
$summary
?Error: canned
$verified"
expect_stub unproved-without-warning 'unproved statements or warnings' "$clean
$summary
The following \$p statement(s) were not proved:  th-g
$verified"
expect_stub lowercase-warning 'unproved statements or warnings' "$clean
$summary
warning: canned
$verified"
expect_stub unreadable-summary 'missing or has more than 9 digits' "$clean
The source has 5927 statements; 23 are \$a and many are \$p.
$verified"
expect_stub huge-summary 'missing or has more than 9 digits' "$clean
The source has 5927 statements; 23 are \$a and 99999999999999999999 are \$p.
$verified"
expect_stub silent 'did not report a clean read' ""

database "$tmp/wrong-label" 's/wph wps wch use.1 use.2 use.3 mp2 \$\./wph wps wch use.1 use.2 use.3 ax-mp $./'
expect wrong-label 'reported problems' "$tmp/wrong-label"
database "$tmp/missing-dot" 's/^  use \$p \(.*\) \$\.$/  use $p \1/'
expect missing-dot 'reported problems' "$tmp/missing-dot"
database "$tmp/swapped-hypotheses" '/mp2.2 \$e/{h;d;};/mp2.3 \$e/{G;}'
expect swapped-hypotheses 'reported problems' "$tmp/swapped-hypotheses"
database "$tmp/missing-disjoint" '/^  \$d ps y \$\.$/d'
expect missing-disjoint 'reported problems' "$tmp/missing-disjoint"
database "$tmp/syntax-error" 's/^wi \$a wff ( ph -> ps ) \$\./wi $a wff ( ph -> ps ) $a/'
expect syntax-error 'reported problems' "$tmp/syntax-error"

database "$tmp/incomplete" 's/\$= wps vy ax-g \$\./$= ? $./'
expect incomplete 'unproved statements or warnings' "$tmp/incomplete"

mkdir "$tmp/compressed"
cat >"$tmp/compressed/tiny.mm" <<'DB'
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
  th-g $p |- ( ps -> A. y ps ) $= ( ax-g ) ABC $.
$}
DB
echo 1 >"$tmp/compressed/tiny.proofs"
echo tiny >"$tmp/compressed/libraries.list"
expect_ok compressed "$tmp/compressed" 1

database "$tmp/second-bad"
cp "$tmp/second-bad/tiny.mm" "$tmp/second-bad/a.mm"
cp "$tmp/second-bad/tiny.proofs" "$tmp/second-bad/a.proofs"
mv "$tmp/second-bad/tiny.mm" "$tmp/second-bad/b.mm"
mv "$tmp/second-bad/tiny.proofs" "$tmp/second-bad/b.proofs"
sed '/^  \$d ps y \$\.$/d' "$tmp/second-bad/a.mm" >"$tmp/second-bad/b.mm"
expect second-bad 'reported problems' "$tmp/second-bad"
grep -qF 'a.mm: 3 proofs verified' "$tmp/out" || { echo "FAIL second-bad: a.mm did not pass" >&2; exit 1; }

printf 'a\nb\n' >"$tmp/second-bad/libraries.list"

database "$tmp/dangling"
mv "$tmp/dangling/tiny.mm" "$tmp/dangling/b.mm"
mv "$tmp/dangling/tiny.proofs" "$tmp/dangling/b.proofs"
ln -s "$tmp/nowhere.mm" "$tmp/dangling/a.mm"
printf 'a\nb\n' >"$tmp/dangling/libraries.list"
expect dangling-symlink 'not a readable file' "$tmp/dangling"
grep -qF 'b.mm: 3 proofs verified' "$tmp/out" || { echo "FAIL dangling-symlink: b.mm was not verified" >&2; exit 1; }

database "$tmp/no-count"
rm "$tmp/no-count/tiny.proofs"
expect no-count 'no expected proof count' "$tmp/no-count"
database "$tmp/bad-count"
echo three >"$tmp/bad-count/tiny.proofs"
expect bad-count 'no expected proof count' "$tmp/bad-count"
database "$tmp/extra-count"
echo 3 >"$tmp/extra-count/other.proofs"
expect extra-count '1 .mm files but 2 .proofs files' "$tmp/extra-count"
database "$tmp/wrong-count"
echo 4 >"$tmp/wrong-count/tiny.proofs"
expect wrong-count 'has 3 proofs, want 4' "$tmp/wrong-count"

expect missing-directory 'not a directory' "$tmp/nothing-here"
mkdir "$tmp/empty"
database "$tmp/empty-count"
: >"$tmp/empty-count/tiny.proofs"
expect empty-count 'no expected proof count' "$tmp/empty-count"
database "$tmp/huge-count"
echo 99999999999999999999 >"$tmp/huge-count/tiny.proofs"
expect huge-count 'more than 9 digits' "$tmp/huge-count"
database "$tmp/zero-padded"
echo 0003 >"$tmp/zero-padded/tiny.proofs"
expect_ok zero-padded "$tmp/zero-padded" 3
database "$tmp/missing-export"
echo other >>"$tmp/missing-export/libraries.list"
expect missing-export 'other was built but other.mm is missing' "$tmp/missing-export"
database "$tmp/no-list"
rm "$tmp/no-list/libraries.list"
expect no-list 'no libraries.list' "$tmp/no-list"
database "$tmp/empty-list"
: >"$tmp/empty-list/libraries.list"
expect empty-list 'libraries.list is empty' "$tmp/empty-list"

# frames NAME MESSAGE EXPORT_SED EXPORT_EXTRA SOURCE_EXTRA: the export is the tiny database with
# EXPORT_SED applied and EXPORT_EXTRA appended, the source is the tiny database with SOURCE_EXTRA
# appended, both read by the real metamath-exe. With a MESSAGE the comparison must fail with it on
# stderr after the export verifies; with an empty one it must pass.
frames() {
	database "$tmp/$1" "${3:-}"
	printf '%s\n' "${4:-}" >>"$tmp/$1/tiny.mm"
	{ write_tiny; printf '%s\n' "${5:-}"; } >"$tmp/$1.source.mm"
	if sh "$root/scripts/verify-exports.sh" "$tmp/$1" "$tmp/$1.source.mm" >"$tmp/out" 2>"$tmp/err"; then
		[ -z "$2" ] || { echo "FAIL $1: succeeded" >&2; exit 1; }
		grep -qF 'assertions have the frames of' "$tmp/out" || { echo "FAIL $1: stdout was: $(cat "$tmp/out")" >&2; exit 1; }
	else
		[ -n "$2" ] || { echo "FAIL $1: $(cat "$tmp/err")" >&2; exit 1; }
		grep -qF -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
		grep -qF 'proofs verified' "$tmp/out" || { echo "FAIL $1: the export did not verify first" >&2; exit 1; }
	fi
	echo "ok $1"
}

frames same-frames ''
frames renamed-hypotheses '' 's/ax-mp\.\([12]\)/ax-mp.e\1/g'
frames reordered-disjoint-pair '' 's/^  \$d ph x \$\.$/  $d x ph $./'
frames dropped-disjoint-pair '> ax-g	disjoint	ph,x' '/^  \$d ph x \$\.$/d'
frames changed-hypothesis '< keep	hypothesis 2	$e |- ps $.' '' '${ keep.1 $e |- ps $. keep $a |- ps $. $}' '${ keep.1 $e |- ph $. keep $a |- ps $. $}'
frames changed-statement '< keep	statement	keep $a |- ph $.' '' 'keep $a |- ph $.' 'keep $a |- ps $.'
frames dropped-hypothesis "< keep	hypotheses	4" '' '${ keep.1 $e |- ph $. keep.2 $e |- ps $. keep $a |- ps $. $}' '${ keep.1 $e |- ph $. keep $a |- ps $. $}'
frames added-hypothesis "< keep	hypotheses	3" '' '${ keep.1 $e |- ph $. keep $a |- ps $. $}' '${ keep.1 $e |- ph $. keep.2 $e |- ps $. keep $a |- ps $. $}'
frames error-line-in-the-database 'metamath-exe reported: ?Error' '' '' 'oops oops oops'
frames extra-assertion 'assertions in the export and' '' 'extra $a |- ph $.'
frames missing-from-the-database 'assertions in the export and' 's/^  th-g \$p/  th-x $p/' '' ''

# A statement of over 100 characters that differs from the database only after the point where
# metamath-exe wraps it, and one whose label is long enough to push its keyword to the next line.
nest() { awk -v last="$1" 'BEGIN { s = last; for (i = 0; i < 12; i++) s = "( ph -> " s " )"; print s }'; }
frames wrapped-statement '< wrapped	statement' '' "wrapped \$a |- $(nest ps) \$." "wrapped \$a |- $(nest ph) \$."
long_label=$(printf 'label-%.0s' 1 2 3 4 5 6 7 8 9 10 11 12 13 14)
frames wrapped-label "< $long_label	statement" '' "$long_label \$a |- ps \$." "$long_label \$a |- ph \$."
frames wrapped-label-same '' '' "$long_label \$a |- ph \$." "$long_label \$a |- ph \$."

# A verifier whose output the script cannot place must fail the comparison, not shrink it.
printf '' >"$tmp/stubbed.mm"
compare_stub() {
	stub "$1" "$3"
	if METAMATH=$tmp/$1 sh "$root/scripts/compare-frames.sh" "$tmp/good/tiny.mm" "$tmp/stubbed.mm" >"$tmp/out" 2>"$tmp/err"; then
		echo "FAIL $1: succeeded" >&2
		exit 1
	fi
	grep -qF -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}
compare_stub header-without-statement 'is followed by no statement' 'Statement 12 is located on line 3 of the file "x".
Its statement number for HTML pages is 1.'
compare_stub unparsed-statement 'did not parse' 'Statement 12 is located on line 3 of the file "x".
12 th-x $p |- ps'
compare_stub fewer-statements-than-labels 'assertions in the export and' 'Statement 12 is located on line 3 of the file "x".
12 wi $a wff ( ph -> ps ) $.'
compare_stub error-line 'metamath-exe reported: ?Error: canned' '?Error: canned
Statement 12 is located on line 3 of the file "x".
12 wi $a wff ( ph -> ps ) $.'
compare_stub unplaced-hypothesis-line 'among the hypotheses of statement 1 was not understood: stray' 'Statement 12 is located on line 3 of the file "x".
12 wi $a wff ( ph -> ps ) $.
Its mandatory hypotheses in RPN order are:
  wph $f wff ph $.
stray'

for source in "" "$tmp/no-such-database.mm"; do
	if sh "$root/scripts/verify-exports.sh" "$tmp/good" "$source" >"$tmp/out" 2>"$tmp/err"; then
		echo "FAIL missing-source: succeeded with '$source'" >&2
		exit 1
	fi
	grep -qF 'is not a file' "$tmp/err" || { echo "FAIL missing-source: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
done
echo "ok missing-source"

expect no-files 'no .mm files' "$tmp/empty"
if run "$tmp/good" "$tmp/absent"; then echo "FAIL missing-verifier: succeeded" >&2; exit 1; fi
grep -qF 'not an executable' "$tmp/err" || { echo "FAIL missing-verifier: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
echo "ok missing-verifier"
