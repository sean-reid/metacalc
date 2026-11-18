#!/bin/sh
# Usage: build-site.sh pagedir sitedir
# Copies docs/ into sitedir (which must not exist), fills in the library pages and the index table that build-libraries.sh
# wrote into pagedir, draws the worked example images, and builds the book into sitedir/book.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
pagedir=$1
sitedir=$2
index=$sitedir/src/libraries/index.md

cp -r "$root/docs" "$sitedir"
cp "$pagedir"/prop.md "$pagedir"/pred.md "$pagedir"/ring.md "$sitedir/src/libraries/"
awk -v table="$pagedir/libraries-table.md" '
	$0 == "<!-- libraries-table -->" { print; while ((getline line < table) > 0) print line; skip = 1; next }
	$0 == "<!-- /libraries-table -->" { skip = 0 }
	!skip { print }
' "$index" >"$index.new"
mv "$index.new" "$index"
sh "$root/scripts/worked-examples.sh" images "$sitedir/src/guide/examples"
mdbook build "$sitedir"
