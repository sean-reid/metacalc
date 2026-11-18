#!/bin/sh
# Usage: test-check-8xk.sh app.8xk
set -eu
export LC_ALL=C

good=${1:?usage: test-check-8xk.sh app.8xk}
check=$(dirname "$0")/check-8xk.sh
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT

refuses() {
	if sh "$check" "$dir/$1.8xk" 2>/dev/null; then
		echo "test-check-8xk: accepted an app with $2" >&2
		exit 1
	fi
}

: > "$dir/empty.8xk"
refuses empty "no bytes"

sed 's/metacalc/metacalx/' "$good" > "$dir/file-name.8xk"
refuses file-name "another name in the file header"

sed 's/80486D65746163616C63/80486D65746163616C78/' "$good" > "$dir/app-name.8xk"
refuses app-name "another name in the app header"

sed 's/807F00000000/807F00000001/' "$good" > "$dir/header.8xk"
refuses header "bytes between the header and 4080h"

sed 's/022D40/022D41/' "$good" > "$dir/signature.8xk"
refuses signature "no signature field"

sh "$check" "$good" > /dev/null
echo "test-check-8xk: ok"
