#!/bin/sh
# Usage: test-build-site.sh
# Checks that build-site.sh refuses an existing site directory, a missing or empty table, and an
# index without exactly one of each marker line. Each case stops before mdbook runs.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/pages" "$tmp/docs/src/libraries"
printf '| a |\n' >"$tmp/pages/libraries-table.md"
printf 'x\n' >"$tmp/pages/prop.md"
good='<!-- libraries-table -->\nrow\n<!-- /libraries-table -->\n'
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
printf '| a |\n' >"$tmp/pages/libraries-table.md"

printf '<!-- /libraries-table -->\n' >"$tmp/docs/src/libraries/index.md"
expect missing-start "exactly one line <!-- libraries-table -->" "$tmp/site"
printf '<!-- libraries-table -->\n' >"$tmp/docs/src/libraries/index.md"
expect missing-end "exactly one line <!-- /libraries-table -->" "$tmp/site"
printf "$good$good" >"$tmp/docs/src/libraries/index.md"
expect repeated-marker "exactly one line" "$tmp/site"
[ ! -e "$tmp/site" ] || { echo "FAIL: a refused run created the site" >&2; exit 1; }
