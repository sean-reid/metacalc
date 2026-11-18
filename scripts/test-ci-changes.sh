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
	got=$(printf '%s\n' "$@" | sh "$root/scripts/ci-changes.sh" | sed -n 's/=true$//p' | tr '\n' ' ' | sed 's/ $//')
	if [ "$got" = "$want" ]; then
		echo "ok $name"
	else
		echo "FAIL $name: ran '$got', want '$want'"
		status=1
	fi
}

every="rust emu audit docs app libraries"

expect nothing "" ""
expect readme "" README.md
expect favicon "docs" docs/theme/favicon.svg
expect guide-page "docs libraries" docs/src/guide/worked-examples.md
expect screens "rust emu libraries" crates/mcui/src/data.rs
expect editor "rust emu libraries" crates/mcedit/src/ops.rs
expect packer "rust emu libraries" crates/mcpack/src/library/pack.rs
expect font "rust emu libraries" crates/mcglyph/src/z80.rs
expect harness "rust emu libraries" crates/mcemu/src/lib.rs
expect new-crate "rust audit" crates/mcsend/src/main.rs
expect kernel "emu app libraries" kernel/init.inc
expect reader "emu app libraries" lib/lib.z80
expect drawing "emu app libraries" ui/blit.inc
expect shell "emu app libraries" app/src/metacalc.z80
expect check-app "emu app libraries" probe/kerndriver.inc
expect library-list "rust libraries" libraries/ring.toml
expect release-config "libraries" release-please-config.json
expect lockfile "$every" Cargo.lock
expect manifest "$every" Cargo.toml
expect deny "$every" deny.toml
expect script "$every" scripts/spasm.sh
expect workflow "$every" .github/workflows/ci.yml
expect unknown "$every" somewhere/new.txt
expect union "rust emu docs libraries" docs/theme/favicon.svg crates/mcui/src/data.rs
expect one-unknown-among-known "$every" README.md tools/rebuild-rom.sh
expect docs-prefix-only "$every" docsx/file

exit $status
