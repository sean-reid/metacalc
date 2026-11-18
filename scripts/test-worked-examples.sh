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
${
  $d ps y $.
  th-k.1 $e |- ( ( ps -> A. y ps ) -> ph ) $.
  th-k $p |- ph $= wps wps vy wal wi wph wps vy ax-g th-k.1 ax-m $.
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

## 3. Through an intermediate goal

Hypotheses:

- `|- ( ( ps -> A. y ps ) -> ph )`

[`th-k`](../libraries/tiny.md#th-k), tiny library

![th-k](examples/th-k.png)

<!-- outline: th-k -->
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

# harmless NAME SED-SCRIPT: check must pass for the page edited by sed.
harmless() {
	sed "$2" "$good" >"$page"
	cmp -s "$good" "$page" && { echo "FAIL $1: the edit changed nothing" >&2; exit 1; }
	run check || { cat "$tmp/err" >&2; echo "FAIL $1: check failed" >&2; exit 1; }
	echo "ok $1"
}

# page_fails NAME MESSAGE: check must fail with MESSAGE for the page already written to $page.
page_fails() {
	if run check; then
		echo "FAIL $1: check passed" >&2
		exit 1
	fi
	grep -q -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

# damaged NAME MESSAGE SED-SCRIPT: check must fail with MESSAGE on stderr for the page edited by sed.
damaged() {
	sed "$3" "$good" >"$page"
	cmp -s "$good" "$page" && { echo "FAIL $1: the edit changed nothing" >&2; exit 1; }
	page_fails "$1" "$2"
}

damaged marker-removed "2. Modus ponens: expected one outline marker, found 0" '/^<!-- outline: th-m -->$/d'
damaged markers-removed "expected one outline marker, found 0" '/^<!-- outline: /d'
damaged marker-outside "outside a numbered section" '1i\
<!-- outline: th-g -->'
damaged marker-without-block "not followed by a code block" '/^<!-- outline: th-m -->$/a\
Text.'
damaged marker-before-heading "9. Extra: expected one library link, found 0" '/^<!-- outline: th-g -->$/a\
## 9. Extra'
damaged marker-repeated "the marker th-g is used in another section" '/^## 3\./,$s/th-k/th-g/g'
damaged hypothesis-changed "the hypotheses listed for th-m differ from the database" 's/^- `|- ( ph -> ps )`$/- `|- ( ph -> ph )`/'
damaged hypothesis-deleted "the hypotheses listed for th-m differ from the database" '/^- `|- ph`$/d'
damaged hypotheses-block-deleted "the hypotheses listed for th-m differ from the database" '/^## 2\./,/^## 3\./{/^Hypotheses:$/d;/^- `/d;}'
damaged hypothesis-is-intermediate-goal "the hypotheses listed for th-k differ from the database" '/^- `|- ( ( ps -> A. y ps ) -> ph )`$/a\
- `|- ( ps -> A. y ps )`'
harmless bullet-after-text-ignored '/^- `|- ( ph -> ps )`$/a\
Then:\
- `|- ( ph -> ph )`'
harmless bullet-after-blank-ignored '/^- `|- ( ph -> ps )`$/a\
\
- `|- ( ph -> ph )`'
harmless bullet-after-link-ignored '/^\[`th-m`\]/a\
\
- `|- ( ph -> ph )`'
damaged link-label-differs "the link names th-g but the marker names th-m" 's/^\[`th-m`\](\.\.\/libraries\/tiny\.md#th-m)/[`th-g`](..\/libraries\/tiny.md#th-g)/'
damaged link-anchor-differs "the link text th-m and its anchor th-g differ" 's/#th-m)/#th-g)/'
damaged link-target-missing "docs/src/libraries/gone.md, which does not exist" 's/tiny\.md#th-m/gone.md#th-m/'
damaged image-alt-differs "the image names th-g but the marker names th-m" 's/^!\[th-m\]/![th-g]/'
damaged image-path-differs "the image file is wrong.png but the marker names th-m" 's|examples/th-m\.png|examples/wrong.png|'
damaged link-removed "expected one library link, found 0" '/^\[`th-m`\]/d'
damaged block-differs "the tree for th-g in .* differs from the database" "s/\[ax-g\]/[ax-x]/"
damaged block-whitespace-differs "the tree for th-g in .* differs from the database" 's/\[ax-g\]$/[ax-g] /'

printf '# Page\n\nNo examples here.\n' >"$page"
page_fails no-numbered-sections "the page has no numbered example sections"
