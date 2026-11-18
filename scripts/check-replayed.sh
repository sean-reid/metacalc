#!/bin/sh
# Usage: check-replayed.sh libdir spliced
# Checks with metamath-exe, through verify-exports.sh, the databases the replay wrote to spliced:
# for each library in libdir/libraries.list, spliced/NAME.mm holds the library with every proof
# replaced by the editor's rebuilt export. It prints "metamath-exe verified N of M" per library,
# M being the count in libdir/NAME.proofs, and fails unless every proof of every library is
# verified and the database holds exactly M of them.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
libdir=$1
spliced=$2

[ -f "$libdir/libraries.list" ] || { echo "check-replayed: no libraries.list in $libdir" >&2; exit 1; }
[ -s "$libdir/libraries.list" ] || { echo "check-replayed: libraries.list is empty" >&2; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

status=0
while read -r name; do
	if [ ! -f "$spliced/$name.mm" ]; then
		echo "FAIL $name: the replay wrote no $spliced/$name.mm" >&2
		status=1
		continue
	fi
	[ -f "$libdir/$name.proofs" ] || { echo "FAIL $name: no $libdir/$name.proofs" >&2; status=1; continue; }
	mkdir "$tmp/$name"
	cp "$spliced/$name.mm" "$tmp/$name/$name.mm"
	cp "$libdir/$name.proofs" "$tmp/$name/$name.proofs"
	echo "$name" >"$tmp/$name/libraries.list"
	if sh "$root/scripts/verify-exports.sh" "$tmp/$name" >"$tmp/out" 2>"$tmp/err"; then
		echo "metamath-exe verified $(sed -n 's/^ok .*: \([0-9]*\) proofs verified$/\1/p' "$tmp/out") of $(cat "$libdir/$name.proofs")"
	else
		sed "s/^FAIL [^ ]*:/FAIL metamath-exe $name:/" "$tmp/err" >&2
		status=1
	fi
done <"$libdir/libraries.list"
exit $status
