#!/bin/sh
# Usage: test-ci-changes.sh
# Checks which CI jobs ci-changes.sh turns on for sets of changed paths.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
status=0

# expect NAME JOBS PATH...: the jobs turned on for the paths are exactly JOBS, in the script's order.
expect() {
	name=$1
	want=$2
	shift 2
	out=$(printf '%s\n' "$@" | sh "$root/scripts/ci-changes.sh") || { echo "FAIL $name: the script failed"; status=1; return; }
	[ "$(echo "$out" | grep -cE '^[a-z]+=(true|false)$')" = 7 ] || { echo "FAIL $name: printed '$out'"; status=1; return; }
	got=$(echo "$out" | sed -n 's/=true$//p' | tr '\n' ' ' | sed 's/ $//')
	if [ "$got" = "$want" ]; then
		echo "ok $name"
	else
		echo "FAIL $name: ran '$got', want '$want'"
		status=1
	fi
}

every="rust emu audit docs app libraries site"
crate="rust emu libraries site"
z80="emu app libraries site"

expect nothing "" ""
expect readme "" README.md
expect gitignore "" .gitignore
expect favicon "docs site" docs/theme/favicon.svg
expect book-config "docs site" docs/book.toml
expect library-page "docs site" docs/src/libraries/prop.md
expect guide-page "docs libraries site" docs/src/guide/worked-examples.md
expect screens "$crate" crates/mcui/src/data.rs
expect editor "$crate" crates/mcedit/src/ops.rs
expect packer "$crate" crates/mcpack/src/library/pack.rs
expect font "$crate" crates/mcglyph/src/z80.rs
expect harness "$crate" crates/mcemu/src/lib.rs
expect crate-manifest "rust emu audit libraries site" crates/mcui/Cargo.toml
expect sender "rust audit app" crates/mcsend/src/tifile.rs
expect new-crate "rust audit" crates/mcnew/src/main.rs
expect kernel "$z80" kernel/init.inc
expect reader "$z80" lib/lib.z80
expect drawing "$z80" ui/blit.inc
expect shell "$z80" app/src/metacalc.z80
expect check-app "$z80" probe/kerndriver.inc
expect library-list "rust libraries site" libraries/ring.toml
expect library-source "rust libraries site" libraries/source.toml
expect release-config "site" release-please-config.json
expect release-manifest "site" .release-please-manifest.json
expect lockfile "$every" Cargo.lock
expect manifest "$every" Cargo.toml
expect deny "$every" deny.toml
expect script "$every" scripts/spasm.sh
expect workflow "$every" .github/workflows/ci.yml
expect unknown "$every" somewhere/new.txt
expect quoted-path "$every" '"docs/caf\303\251.md"'
expect path-with-space "docs site" 'docs/src/a page.md'
expect union "rust emu docs libraries site" docs/theme/favicon.svg crates/mcui/src/data.rs
expect one-unknown-among-known "$every" README.md tools/rebuild-rom.sh
expect docs-prefix-only "$every" docsx/file

exit $status
