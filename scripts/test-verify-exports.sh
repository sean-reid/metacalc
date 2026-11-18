#!/bin/sh
# Usage: test-verify-exports.sh
# Checks that verify-exports.sh passes a small database and refuses damaged copies of it, a missing
# directory, an empty directory and a verifier that prints nothing. Runs the real metamath-exe.
# shellcheck disable=SC2016
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/good"

cat >"$tmp/good/tiny.mm" <<'DB'
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

# run DIR: runs verify-exports.sh on DIR, stdout in $tmp/out and stderr in $tmp/err.
run() { sh "$root/scripts/verify-exports.sh" "$1" >"$tmp/out" 2>"$tmp/err"; }

# expect NAME MESSAGE DIR: verify-exports.sh must exit nonzero with MESSAGE on stderr.
expect() {
	if run "$3"; then
		echo "FAIL $1: succeeded" >&2
		exit 1
	fi
	grep -q -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

# damage NAME SED: copies the good database to a directory named NAME with the sed script applied.
damage() {
	mkdir "$tmp/$1"
	sed "$2" "$tmp/good/tiny.mm" >"$tmp/$1/tiny.mm"
	cmp -s "$tmp/good/tiny.mm" "$tmp/$1/tiny.mm" && { echo "FAIL $1: sed changed nothing" >&2; exit 1; }
	return 0
}

run "$tmp/good" || { echo "FAIL good: $(cat "$tmp/err")" >&2; exit 1; }
grep -q 'tiny.mm: 3 proofs verified' "$tmp/out" || { echo "FAIL good: stdout was: $(cat "$tmp/out")" >&2; exit 1; }
echo "ok good"

damage wrong-label 's/wph wps wch use.1 use.2 use.3 mp2 \$\./wph wps wch use.1 use.2 use.3 ax-mp $./'
expect wrong-label 'RPN stack' "$tmp/wrong-label"

damage missing-dot 's/^  use \$p \(.*\) \$\.$/  use $p \1/'
expect missing-dot 'Expected "\$\." here' "$tmp/missing-dot"

damage swapped-hypotheses '/mp2.2 \$e/{h;d;};/mp2.3 \$e/{G;}'
expect swapped-hypotheses 'cannot be unified' "$tmp/swapped-hypotheses"

damage missing-disjoint '/^  \$d ps y \$\.$/d'
expect missing-disjoint 'disjoint variable' "$tmp/missing-disjoint"

damage syntax-error 's/^wi \$a wff ( ph -> ps ) \$\./wi $a wff ( ph -> ps ) $a/'
expect syntax-error '^?Error' "$tmp/syntax-error"

expect missing-directory 'not a directory' "$tmp/nothing-here"
mkdir "$tmp/empty"
expect no-files 'no .mm files' "$tmp/empty"
printf '#!/bin/sh\nexit 0\n' >"$tmp/silent"
chmod +x "$tmp/silent"
if METAMATH="$tmp/silent" run "$tmp/good"; then echo "FAIL silent-verifier: succeeded" >&2; exit 1; fi
grep -q 'did not report a clean read' "$tmp/err" || { echo "FAIL silent-verifier: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
echo "ok silent-verifier"
if METAMATH="$tmp/absent" run "$tmp/good"; then echo "FAIL missing-verifier: succeeded" >&2; exit 1; fi
grep -q 'not an executable' "$tmp/err" || { echo "FAIL missing-verifier: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
echo "ok missing-verifier"
