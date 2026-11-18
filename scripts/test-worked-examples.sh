#!/bin/sh
# Usage: test-worked-examples.sh
# Checks that worked-examples.sh check passes a page written from a small database and fails, naming
# the section, when a marker, the markers, a hypothesis, a link label, a link target, an image or a
# block is damaged. It runs the real mcpack over the database.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fake=$tmp/repo
mkdir -p "$fake/scripts" "$fake/docs/src/guide" "$fake/docs/src/libraries"
for item in Cargo.toml Cargo.lock crates; do ln -s "$root/$item" "$fake/$item"; done
cp "$root/scripts/worked-examples.sh" "$fake/scripts/"
: >"$fake/docs/src/libraries/tiny.md"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"

cat >"$tmp/tiny.mm" <<'DB'
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
  ax-m.1 $e |- ph $.
  ax-m.2 $e |- ( ph -> ps ) $.
  ax-m $a |- ps $.
$}
${
  th-m.1 $e |- ph $.
  th-m.2 $e |- ( ph -> ps ) $.
  th-m $p |- ps $= wph wps th-m.1 th-m.2 ax-m $.
$}
DB

good=$tmp/good.md
cat >"$good" <<'PAGE'
# Page

## 1. Generalization

[`th-g`](../libraries/tiny.md#th-g), tiny library

![th-g](examples/th-g.png)

<!-- outline: th-g -->
```text
old
```

Prose.

## 2. Modus ponens

Hypotheses:

- `|- ph`
- `|- ( ph -> ps )`

[`th-m`](../libraries/tiny.md#th-m), tiny library

![th-m](examples/th-m.png)

<!-- outline: th-m -->
```text
old
```

Prose.
PAGE

page=$fake/docs/src/guide/worked-examples.md
run() { sh "$fake/scripts/worked-examples.sh" "$1" "$tmp/tiny.mm" 2>"$tmp/err"; }

cp "$good" "$page"
run write || { cat "$tmp/err" >&2; echo "FAIL write" >&2; exit 1; }
cp "$page" "$good"
run check || { cat "$tmp/err" >&2; echo "FAIL good-page: check refused the written page" >&2; exit 1; }
echo "ok good-page"

# damaged NAME MESSAGE SED-SCRIPT: check must fail with MESSAGE on stderr for the page edited by sed.
damaged() {
	sed "$3" "$good" >"$page"
	cmp -s "$good" "$page" && { echo "FAIL $1: the edit changed nothing" >&2; exit 1; }
	if run check; then
		echo "FAIL $1: check passed" >&2
		exit 1
	fi
	grep -q -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

damaged marker-removed "2. Modus ponens: expected one outline marker, found 0" '/^<!-- outline: th-m -->$/d'
damaged markers-removed "expected one outline marker, found 0" '/^<!-- outline: /d'
damaged marker-outside "outside a numbered section" '1i\
<!-- outline: th-g -->'
damaged marker-without-block "not followed by a code block" '/^<!-- outline: th-m -->$/a\
Text.'
damaged hypothesis-changed "the hypothesis |- ( ph -> ph ) of th-m is not in its tree" 's/^- `|- ( ph -> ps )`$/- `|- ( ph -> ph )`/'
damaged link-label-differs "the link names th-g but the marker names th-m" 's/^\[`th-m`\](\.\.\/libraries\/tiny\.md#th-m)/[`th-g`](..\/libraries\/tiny.md#th-g)/'
damaged link-anchor-differs "the link text th-m and its anchor th-g differ" 's/#th-m)/#th-g)/'
damaged link-target-missing "docs/src/libraries/gone.md, which does not exist" 's/tiny\.md#th-m/gone.md#th-m/'
damaged image-differs "the image names th-g but the marker names th-m" 's/^!\[th-m\]/![th-g]/'
damaged link-removed "expected one library link, found 0" '/^\[`th-m`\]/d'
damaged block-differs "the tree for th-g in .* differs from the database" "s/\[ax-g\]/[ax-x]/"
