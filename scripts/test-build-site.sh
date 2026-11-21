#!/bin/sh
# Usage: test-build-site.sh
# Checks that build-site.sh refuses an existing site directory, a missing or empty table, an index
# without exactly one of each marker line and a missing generated page, all before mdbook runs. Then
# builds a small book with mdbook and checks the generated page and table replace the placeholders
# and that a repeated id fails the build.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/pages" "$tmp/docs/src/libraries"
printf '| Library |\n| --- |\n| tablerow |\n' >"$tmp/pages/libraries-table.md"
printf '# Prop\n\ngenerated prop text\n' >"$tmp/pages/prop.md"
printf '[book]\ntitle = "t"\n' >"$tmp/docs/book.toml"
printf '# Summary\n\n- [Examples](guide/examples.md)\n- [Libraries](libraries/index.md)\n  - [Prop](libraries/prop.md)\n' >"$tmp/docs/src/SUMMARY.md"
mkdir -p "$tmp/docs/src/guide"
cp "$root/docs/src/guide/examples.md" "$tmp/docs/src/guide/"
printf '# Prop\n\nThis page is generated from the packed library when the site is built.\n' >"$tmp/docs/src/libraries/prop.md"
good='intro text\n\n<!-- libraries-table -->\nBuilt with the site\n<!-- /libraries-table -->\n\noutro text\n'
printf "$good" >"$tmp/docs/src/libraries/index.md"

# expect NAME MESSAGE SITEDIR: build-site.sh must exit nonzero with MESSAGE on stderr.
expect() {
	if sh "$root/scripts/build-site.sh" "$tmp/pages" "$3" "$tmp/docs" 2>"$tmp/err"; then
		echo "FAIL $1: succeeded" >&2
		exit 1
	fi
	grep -q -e "$2" "$tmp/err" || { echo "FAIL $1: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $1"
}

mkdir "$tmp/exists"
expect existing-site "already exists" "$tmp/exists"

: >"$tmp/pages/libraries-table.md"
expect empty-table "missing or empty" "$tmp/site"
rm "$tmp/pages/libraries-table.md"
expect missing-table "missing or empty" "$tmp/site"
printf '| Library |\n| --- |\n| tablerow |\n' >"$tmp/pages/libraries-table.md"

printf '<!-- /libraries-table -->\n' >"$tmp/docs/src/libraries/index.md"
expect missing-start "exactly one line <!-- libraries-table -->" "$tmp/site"
printf '<!-- libraries-table -->\n' >"$tmp/docs/src/libraries/index.md"
expect missing-end "exactly one line <!-- /libraries-table -->" "$tmp/site"
printf "$good$good" >"$tmp/docs/src/libraries/index.md"
expect repeated-marker "exactly one line" "$tmp/site"
[ ! -e "$tmp/site" ] || { echo "FAIL: a refused run created the site" >&2; exit 1; }

printf "$good" >"$tmp/docs/src/libraries/index.md"
mv "$tmp/pages/prop.md" "$tmp/prop.md"
expect missing-page "no generated page prop.md" "$tmp/site"
[ ! -e "$tmp/site" ] || { echo "FAIL: a refused run created the site" >&2; exit 1; }

mv "$tmp/prop.md" "$tmp/pages/prop.md"
printf '# Prop\n\n<a id="dup"></a>\n<a id="dup"></a>\n' >"$tmp/pages/prop.md"
expect repeated-id "repeats id=\"dup\"" "$tmp/site"
rm -rf "$tmp/site"

printf '# Prop\n\ngenerated prop text\n' >"$tmp/pages/prop.md"
sh "$root/scripts/build-site.sh" "$tmp/pages" "$tmp/site" "$tmp/docs" >"$tmp/out" 2>"$tmp/err" || { cat "$tmp/err" >&2; echo "FAIL build: exited nonzero" >&2; exit 1; }
grep -q "generated prop text" "$tmp/site/book/libraries/prop.html" || { echo "FAIL build: the generated page is not in the book" >&2; exit 1; }
! grep -q "generated from the packed library" "$tmp/site/book/libraries/prop.html" || { echo "FAIL build: the placeholder is in the book" >&2; exit 1; }
grep -q "tablerow" "$tmp/site/book/libraries/index.html" || { echo "FAIL build: the table is not in the index" >&2; exit 1; }
! grep -q "Built with the site" "$tmp/site/book/libraries/index.html" || { echo "FAIL build: the table placeholder is in the index" >&2; exit 1; }
grep -q "intro text" "$tmp/site/book/libraries/index.html" && grep -q "outro text" "$tmp/site/book/libraries/index.html" ||
	{ echo "FAIL build: the index lost the text around the table" >&2; exit 1; }
guide=$tmp/site/book/guide/examples.html
grep -q 'href="../libraries/prop.html#mpd"' "$guide" || { echo "FAIL build: the tree link to mpd is not ../libraries/prop.html#mpd" >&2; exit 1; }
! grep -q 'prop.md#' "$guide" || { echo "FAIL build: the guide page still links prop.md#" >&2; exit 1; }
echo "ok build"
rm -rf "$tmp/site"

guidepage=$tmp/docs/src/guide/examples.md
cp "$guidepage" "$tmp/guide.good"

sed 's|<a class="lb" href=|<a class="x" href=|' "$tmp/guide.good" >"$guidepage"
expect no-tree-links "has no tree link to a library page" "$tmp/site"
rm -rf "$tmp/site"

awk '!done && /<a class="lb" href="\.\.\/libraries\/prop\.md#/ {
	sub(/href="\.\.\/libraries\/prop\.md#[^"]*"/, "href=\"https://example.invalid/libraries/prop.md#a\"")
	done = 1
} { print }' "$tmp/guide.good" >"$guidepage"
cmp -s "$guidepage" "$tmp/guide.good" && { echo "FAIL md-tree-link: the edit changed nothing" >&2; exit 1; }
expect md-tree-link "links to a library .md file" "$tmp/site"
rm -rf "$tmp/site"

cp "$tmp/guide.good" "$guidepage"
sed '/guide\/examples/d' "$tmp/docs/src/SUMMARY.md" >"$tmp/summary"
cp "$tmp/summary" "$tmp/docs/src/SUMMARY.md"
expect missing-guide "examples.html was not built" "$tmp/site"
