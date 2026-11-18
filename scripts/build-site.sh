#!/bin/sh
# Usage: build-site.sh pagedir sitedir [docsdir]
# Copies docs/ (or docsdir) into sitedir, which must not exist, fills in every library page and the
# index table that build-libraries.sh wrote into pagedir, draws the worked example images, and
# builds the book into sitedir/book. Refuses to run when the table or a marker line of the index is
# missing, and fails when a page repeats an id.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
pagedir=${1:?usage: build-site.sh pagedir sitedir [docsdir]}
sitedir=${2:?usage: build-site.sh pagedir sitedir [docsdir]}
docs=${3:-$root/docs}
table=$pagedir/libraries-table.md
index=$sitedir/src/libraries/index.md

die() {
	echo "build-site: $*" >&2
	exit 1
}

[ ! -e "$sitedir" ] || die "$sitedir already exists"
[ -s "$table" ] || die "$table is missing or empty"
for marker in '<!-- libraries-table -->' '<!-- /libraries-table -->'; do
	[ "$(grep -cx -e "$marker" "$docs/src/libraries/index.md")" = 1 ] ||
		die "the index must have exactly one line $marker"
done

cp -r "$docs" "$sitedir"
for page in "$pagedir"/*.md; do
	[ "$page" = "$table" ] || cp "$page" "$sitedir/src/libraries/"
done
awk -v table="$table" '
	$0 == "<!-- libraries-table -->" { print; while ((getline line < table) > 0) print line; skip = 1; next }
	$0 == "<!-- /libraries-table -->" { skip = 0 }
	!skip { print }
' "$index" >"$index.new"
mv "$index.new" "$index"
sh "$root/scripts/worked-examples.sh" images "$sitedir/src/guide/examples"
mdbook build "$sitedir"

status=0
for html in "$sitedir"/book/libraries/*.html; do
	repeated=$(grep -o ' id="[^"]*"' "$html" | sort | uniq -d)
	[ -z "$repeated" ] || { echo "build-site: $html repeats$repeated" >&2; status=1; }
done
exit $status
