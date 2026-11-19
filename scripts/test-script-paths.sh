#!/bin/sh
# Usage: test-script-paths.sh
# Checks that build-libraries.sh, worked-examples.sh and fetch-tilem.sh behave the same from any
# directory and under a CARGO_TARGET_DIR override. The scripts run in a copy of the repo whose
# target/release holds stale binaries that log and fail, over a one-theorem database, so a run that
# reaches them fails the case.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

fake=$tmp/repo
mkdir -p "$fake/scripts" "$fake/libraries" "$fake/docs/src/guide" "$fake/docs/src/libraries" "$fake/target/release" "$tmp/elsewhere"
for item in Cargo.toml Cargo.lock crates; do ln -s "$root/$item" "$fake/$item"; done
cp "$root"/scripts/build-libraries.sh "$root"/scripts/worked-examples.sh "$root"/scripts/fetch-tilem.sh "$fake/scripts/"

export STALE_LOG="$tmp/stale.log"
for bin in mcpack mcglyph; do
	cat >"$fake/target/release/$bin" <<'STALE'
#!/bin/sh
echo stale >>"$STALE_LOG"
exit 1
STALE
	chmod +x "$fake/target/release/$bin"
done

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
DB
sed "s/^sha256 = .*/sha256 = \"$(shasum -a 256 "$tmp/tiny.mm" | cut -d' ' -f1)\"/" "$root/libraries/source.toml" >"$fake/libraries/source.toml"
printf 'name = "tiny"\ntitle = "Tiny"\ndescription = "One theorem."\norder = 1\nroots = ["th-g"]\n' >"$fake/libraries/tiny.toml"
cat >"$fake/docs/src/guide/worked-examples.md" <<'PAGE'
# Page

<!-- symbols -->

| Symbol | Read as | In a source file |
| --- | --- | --- |
| ⊢ | is provable | `\|-` |
| ( ) | grouping | `(` `)` |
| → | implies | `->` |
| ∀ | for all | `A.` |
| 𝜓 | any statement | `ps` |
| 𝑦 | any set variable | `y` |

<!-- /symbols -->

## 1. Example

[`th-g`](../libraries/tiny.md#th-g)

![th-g](examples/th-g.png)

<!-- outline: th-g -->
<ul class="tree">
</ul>
PAGE

: >"$fake/docs/src/libraries/tiny.md"

fail() {
	echo "FAIL $1" >&2
	exit 1
}

# The cases run from another directory with a target directory that holds nothing yet.
export CARGO_TARGET_DIR="$tmp/target"
cd "$tmp/elsewhere"

sh "$fake/scripts/build-libraries.sh" "$tmp/tiny.mm" "$tmp/libs" "$tmp/pages" >"$tmp/out" 2>"$tmp/err" || { cat "$tmp/err" >&2; fail build-libraries; }
[ -s "$tmp/libs/tiny.mclib" ] && [ "$(cat "$tmp/libs/tiny.proofs")" = 1 ] || fail "build-libraries wrote no library"
[ -s "$tmp/pages/tiny.md" ] || fail "build-libraries wrote no page"
echo "ok build-libraries"

sh "$fake/scripts/worked-examples.sh" write "$tmp/tiny.mm" 2>"$tmp/err" || { cat "$tmp/err" >&2; fail worked-examples-write; }
grep -qF '⊢ ( 𝜓 → ∀ 𝑦 𝜓 )</span> <a class="lb" href="../libraries/tiny.md#ax-g">ax-g</a>' "$fake/docs/src/guide/worked-examples.md" || fail "worked-examples write left the old block"
sh "$fake/scripts/worked-examples.sh" check "$tmp/tiny.mm" 2>"$tmp/err" || { cat "$tmp/err" >&2; fail worked-examples-check; }
echo "ok worked-examples-check"
sh "$fake/scripts/worked-examples.sh" images "$tmp/images" 2>"$tmp/err" || { cat "$tmp/err" >&2; fail worked-examples-images; }
[ -s "$tmp/images/th-g.png" ] || fail "worked-examples drew no image"
echo "ok worked-examples-images"

[ ! -e "$STALE_LOG" ] || fail "a stale binary under target/release ran: $(cat "$STALE_LOG")"
echo "ok no-stale-binary"
[ -x "$CARGO_TARGET_DIR/release/mcpack" ] || fail "the build did not use CARGO_TARGET_DIR"
echo "ok target-dir-used"

rev=$(sed -n 's/^rev=//p' "$fake/scripts/fetch-tilem.sh")
mkdir "$tmp/bin"
cat >"$tmp/bin/git" <<'GIT'
#!/bin/sh
echo "$*" >>"$GIT_LOG"
case $* in *rev-parse*) echo "$REV" ;; esac
GIT
chmod +x "$tmp/bin/git"
GIT_LOG=$tmp/git.log REV=$rev PATH=$tmp/bin:$PATH sh "$fake/scripts/fetch-tilem.sh" >/dev/null || fail fetch-tilem
grep -qxF "init -q $fake/.tilem" "$tmp/git.log" || fail "fetch-tilem did not default to the repo root: $(cat "$tmp/git.log")"
echo "ok fetch-tilem-default"
