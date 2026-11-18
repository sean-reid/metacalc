#!/bin/sh
# Usage: test-release-config.sh [config]
# Checks that release-please-config.json rewrites the workspace version in Cargo.toml and the
# version of every crate under crates/ in Cargo.lock, so a new crate cannot keep an old version.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
config=${1:-$root/release-please-config.json}
status=0

grep -qF '"path": "Cargo.toml", "jsonpath": "$.workspace.package.version"' "$config" || {
	echo "test-release-config: $config does not update the workspace version in Cargo.toml" >&2
	status=1
}
for manifest in "$root"/crates/*/Cargo.toml; do
	name=$(sed -n 's/^name = "\(.*\)"$/\1/p' "$manifest" | head -n 1)
	[ -n "$name" ] || { echo "test-release-config: no package name in $manifest" >&2; status=1; continue; }
	grep -qF "\"path\": \"Cargo.lock\", \"jsonpath\": \"\$.package[?(@.name.value=='$name')].version\"" "$config" || {
		echo "test-release-config: $config does not update the version of $name in Cargo.lock" >&2
		status=1
	}
done
[ $status -ne 0 ] || echo "ok release-config"
exit $status
