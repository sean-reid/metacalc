#!/bin/sh
# Usage: test-release-config.sh [config]
# Checks that release-please-config.json rewrites the workspace version in Cargo.toml and the
# version of every crate under crates/ in Cargo.lock, so a new crate cannot keep an old version.
# Also checks that the release workflow builds what install.sh downloads: the same platforms, the
# same package and app names, SHA256SUMS, and every action pinned by commit.
# shellcheck disable=SC2016
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

workflow=$root/.github/workflows/release-please.yml
install=$root/scripts/install.sh
package=$root/scripts/package-mcsend.sh
failed=0
bad() { echo "test-release-config: $*" >&2; failed=1; }

built=$(sed -n 's/^ *- { runner: [^,]*, os: \([a-z]*\), arch: \([a-z0-9_]*\) }$/\1-\2/p' "$workflow" | sort | tr '\n' ' ')
asked=$(sed -n 's/^builds="\(.*\)"$/\1/p' "$install" | tr ' ' '\n' | sort | tr '\n' ' ')
[ -n "$built" ] && [ "$built" = "$asked" ] || bad "the workflow builds '$built' but install.sh asks for '$asked'"
grep -qF "mcsend-\$tag-\$os-\$arch" "$install" || bad "install.sh does not name the package mcsend-\$tag-\$os-\$arch"
grep -qF "name=mcsend-\$tag-\$os-\$arch" "$package" || bad "package-mcsend.sh does not name the package mcsend-\$tag-\$os-\$arch"
for asset in metacalc.8xk SHA256SUMS; do
	for file in "$install" "$workflow"; do
		grep -qF "$asset" "$file" || bad "$asset is not named in $file"
	done
done
grep -qF 'cargo build --release --locked -p mcsend' "$workflow" || bad "the release workflow does not build mcsend with --release --locked"
grep -qF 'scripts/package-mcsend.sh' "$workflow" || bad "the release workflow does not call package-mcsend.sh"
grep -E 'uses: [^.]' "$workflow" | grep -qvE 'uses: [^@ ]+@[0-9a-f]{40} # [^ ]+$' && bad "an action in the release workflow is not pinned by commit with a version comment"
jobs=$(sed -n '/^jobs:/,$s/^  \([a-z-]*\):$/\1/p' "$workflow" | grep -vx release-please | tr '\n' ' ')
count() { grep -cF -e "$1" "$workflow" || true; }
njobs=$(echo "$jobs" | wc -w)
[ "$(count "needs.release-please.outputs.release_created == 'true'")" -eq "$njobs" ] || bad "every job after release-please must run when a release was created"
[ "$(count "github.event_name == 'workflow_dispatch'")" -eq "$njobs" ] || bad "every job after release-please must also run when the workflow is dispatched"
grep -qF "if: github.event_name == 'push'" "$workflow" || bad "the release-please job must run only on push"
grep -qF 'workflow_dispatch:' "$workflow" || bad "the release workflow has no workflow_dispatch trigger"
grep -qF 'required: true' "$workflow" || bad "the dispatch tag input is not required"
[ "$(count 'TAG: ${{ inputs.tag || needs.release-please.outputs.tag_name }}')" -eq "$njobs" ] || bad "every job after release-please must take its tag from the same expression"
[ "$(grep -cE 'needs\.release-please\.outputs\.tag_name|inputs\.tag' "$workflow")" -eq "$njobs" ] || bad "a tag is read somewhere other than the shared TAG expression"
grep -qF 'gh release upload "$TAG" --clobber' "$workflow" || bad "the upload does not use --clobber"
awk '
	/^ *(- )?run:/ { match($0, /^ */); indent = RLENGTH; if ($0 ~ /\$\{\{/) bad = 1; inrun = ($0 ~ /\|$/); next }
	inrun { match($0, /^ */); if (RLENGTH <= indent) inrun = 0; else if ($0 ~ /\$\{\{/) bad = 1 }
	END { exit bad }
' "$workflow" || bad "a run step interpolates an expression; pass it through env"
[ $failed -ne 0 ] && status=1
[ $failed -ne 0 ] || echo "ok release-assets"
exit $status
