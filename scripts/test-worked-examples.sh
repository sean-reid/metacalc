#!/bin/sh
# Usage: test-worked-examples.sh
# Checks that worked-examples.sh check passes a page written from a small database and fails, naming
# the section, when a marker, the markers, a hypothesis, a link label, a link target, an image, a
# tree, the symbol table or an ASCII token in the prose is damaged. It runs the real mcpack over the
# database.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fake=$tmp/repo
mkdir -p "$fake/scripts" "$fake/docs/src/guide" "$fake/docs/src/libraries" "$fake/libraries"
for item in Cargo.toml Cargo.lock crates; do ln -s "$root/$item" "$fake/$item"; done
cp "$root/scripts/worked-examples.sh" "$fake/scripts/"
: >"$fake/docs/src/libraries/tiny.md"
cat >"$fake/libraries/tiny.toml" <<'TOML'
name = "tiny"
title = "Tiny"
description = "Three theorems."
order = 1
roots = ["th-g", "th-m", "th-k"]
TOML
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
  $( A comment may hold th-c.1 $e |- ps $. and ${ and
     run over lines. $)
  th-k.1 $e |- ( ( ps -> A. y ps )
    -> ph ) $.
  ${
    $d ps y $.
    th-k $p |- ph $= wps wps vy wal wi wph wps vy ax-g th-k.1 ax-m $.
  $}
$}
DB

cat >"$fake/libraries/source.toml" <<TOML
repository = "https://example.invalid/tiny"
commit = "0"
path = "tiny.mm"
sha256 = "$(if command -v sha256sum >/dev/null 2>&1; then sha256sum "$tmp/tiny.mm"; else shasum -a 256 "$tmp/tiny.mm"; fi | cut -d' ' -f1)"
TOML

good=$tmp/good.md
cat >"$good" <<'PAGE'
# Page

<!-- symbols -->

| Symbol | Read as | In a source file |
| --- | --- | --- |
| ⊢ | is provable | `\|-` |
| ( ) | grouping | `(` `)` |
| → | implies | `->` |
| ∀ | for all | `A.` |
| 𝜑 𝜓 | any statement | `ph` `ps` |
| 𝑦 | any set variable | `y` |

<!-- /symbols -->

## 1. Generalization

[`th-g`](../libraries/tiny.md#th-g), tiny library

![th-g: the statement drawn in the calculator font](examples/th-g.png)

<!-- outline: th-g -->
<ul class="tree">
</ul>

Prose.

## 2. Modus ponens

Hypotheses:

- `⊢ 𝜑`
- `⊢ ( 𝜑 → 𝜓 )`

[`th-m`](../libraries/tiny.md#th-m), tiny library

![th-m: the statement drawn in the calculator font](examples/th-m.png)

<!-- outline: th-m -->
<ul class="tree">
</ul>

Prose.

## 3. Through an intermediate goal

Hypotheses:

- `⊢ ( ( 𝜓 → ∀ 𝑦 𝜓 ) → 𝜑 )`

[`th-k`](../libraries/tiny.md#th-k), tiny library

![th-k: the statement drawn in the calculator font](examples/th-k.png)

<!-- outline: th-k -->
<ul class="tree">
</ul>

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
damaged marker-without-block "not followed by a tree" '/^<!-- outline: th-m -->$/a\
Text.'
damaged marker-before-heading "9. Extra: expected one library link, found 0" '/^<!-- outline: th-g -->$/a\
## 9. Extra'
damaged marker-repeated "the marker th-g is used in another section" '/^## 3\./,$s/th-k/th-g/g'
damaged hypothesis-changed "the hypotheses listed for th-m differ from the database" 's/^- `⊢ ( 𝜑 → 𝜓 )`$/- `⊢ ( 𝜑 → 𝜑 )`/'
damaged hypothesis-deleted "the hypotheses listed for th-m differ from the database" '/^- `⊢ 𝜑`$/d'
damaged hypotheses-block-deleted "the hypotheses listed for th-m differ from the database" '/^## 2\./,/^## 3\./{/^Hypotheses:$/d;/^- `/d;}'
damaged hypothesis-is-intermediate-goal "the hypotheses listed for th-k differ from the database" '/^- `⊢ ( ( 𝜓 → ∀ 𝑦 𝜓 ) → 𝜑 )`$/a\
- `⊢ ( 𝜓 → ∀ 𝑦 𝜓 )`'
harmless bullet-after-text-ignored '/^- `⊢ ( 𝜑 → 𝜓 )`$/a\
Then:\
- `⊢ ( 𝜑 → 𝜑 )`'
harmless bullet-after-blank-ignored '/^- `⊢ ( 𝜑 → 𝜓 )`$/a\
\
- `⊢ ( 𝜑 → 𝜑 )`'
damaged hypothesis-spurious "the hypotheses listed for th-g differ from the database" '/^!\[th-g:/a\
\
Hypotheses:\
\
- `⊢ 𝜑`'
damaged link-label-differs "the link names th-g but the marker names th-m" 's/^\[`th-m`\](\.\.\/libraries\/tiny\.md#th-m)/[`th-g`](..\/libraries\/tiny.md#th-g)/'
damaged link-anchor-differs "the link text th-m and its anchor th-g differ" 's/#th-m)/#th-g)/'
damaged link-target-missing "docs/src/libraries/gone.md, which does not exist" 's/tiny\.md#th-m/gone.md#th-m/'
damaged image-alt-differs "the image names th-g but the marker names th-m" 's/^!\[th-m:/![th-g:/'
damaged image-path-differs "the image file is wrong.png but the marker names th-m" 's|examples/th-m\.png|examples/wrong.png|'
damaged link-removed "expected one library link, found 0" '/^\[`th-m`\]/d'
damaged tree-label-stale "the tree for th-g in .* differs from the database" 's/>ax-g</>ax-x</'
damaged tree-statement-stale "the tree for th-m in .* differs from the database" 's/<span class="st">⊢ 𝜓<\/span>/<span class="st">⊢ 𝜑<\/span>/'
damaged tree-link-stale "the tree for th-g in .* differs from the database" 's|tiny\.md#ax-g|tiny.md#th-g|'
damaged tree-whitespace-differs "the tree for th-g in .* differs from the database" 's/^  <li>/   <li>/'
damaged symbol-missing-from-table "the symbol table lacks A\., which the examples use" '/^| ∀ /d'
damaged symbol-unused-in-table "the symbol table lists <->, which no example uses" '/^| ∀ /a\
| ↔ | if and only if | `<->` |'
damaged symbol-spelled-differently "the symbol table shows -> as ← but mcpack spells it →" 's/^| → /| ← /'
damaged symbol-row-miscounted "the symbol table row 𝜑 𝜓 has 2 symbols and 1 tokens" 's/`ph` `ps`/`ph`/'
damaged symbol-markers-missing "the page must have exactly one line <!-- /symbols -->" '/^<!-- \/symbols -->$/d'
damaged ascii-token-in-prose "line .* of the page has the ASCII token ph in inline code" 's/^Prose\.$/Prose with `ph`./'
damaged ascii-statement-in-prose "has the ASCII token -> in inline code" '/^## 1\./,/^## 2\./s/^Prose\.$/Prose with `( 𝜑 -> 𝜓 )`./'
damaged image-alt-without-description "the image alt text must read th-m: followed by a description" 's/^!\[th-m: [^]]*\]/![th-m]/'
damaged single-letter-in-prose "has the ASCII token y in inline code" 's/^Prose\.$/Prose with `y`./'
damaged lone-capital-in-prose "has the ASCII token A in inline code" '/^| 𝑦 /a\
| 𝐴 | any statement | `A` |
s/^Prose\.$/Prose with `A`./'
damaged ascii-statement-with-a-letter "has the ASCII token y in inline code" 's/^Prose\.$/Prose with `A. y ps`./'
damaged ascii-token-in-hypothesis "has the ASCII token |- in inline code" 's/^- `⊢ 𝜑`$/- `|- ph`/'

sed 's/^<!-- outline: th-m -->$/<!-- outline: no-such -->/' "$good" >"$page"
cp "$page" "$tmp/unknown-label.md"
if run write; then
	echo "FAIL write-unknown-label: write succeeded" >&2
	exit 1
fi
cmp -s "$page" "$tmp/unknown-label.md" || { echo "FAIL write-unknown-label: the page changed" >&2; exit 1; }
echo "ok write-unknown-label"

cp "$good" "$page"
cat >"$fake/libraries/aaa.toml" <<'TOML'
name = "aaa"
title = "Broken"
description = "Cannot build."
order = 0
roots = ["no-such-theorem"]
TOML
page_fails library-build-fails "building the library aaa failed"
printf 'name = "aaa"\ntitle = "Broken"\ndescription = "No order."\nroots = ["th-g"]\n' >"$fake/libraries/aaa.toml"
page_fails library-without-order "aaa.toml has no order line"
rm "$fake/libraries/aaa.toml"

printf '# Page\n\nNo examples here.\n' >"$page"
page_fails no-numbered-sections "the page has no numbered example sections"
