#!/bin/sh
# Usage: test-require-tests.sh
# Checks that require-tests.sh passes when enough tests ran and fails on a filter that matches
# nothing, too few passes, a failing test and bad arguments. The tests run under a real cargo.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
export CARGO_TARGET_DIR="$tmp/target"

mkdir -p "$tmp/crate/src"
cat >"$tmp/crate/Cargo.toml" <<'T'
[package]
name = "sample"
version = "0.1.0"
edition = "2021"
T
cat >"$tmp/crate/src/lib.rs" <<'T'
#[cfg(test)]
mod tests {
    #[test]
    fn works() {}
    #[test]
    #[ignore]
    fn also_works() {}
    #[test]
    #[ignore]
    fn breaks() {
        panic!("broken");
    }
}
T
manifest=$tmp/crate/Cargo.toml

# expect NAME MESSAGE ARGS...: require-tests.sh ARGS must exit nonzero with MESSAGE in its output.
expect() {
	name=$1
	message=$2
	shift 2
	if sh "$root/scripts/require-tests.sh" "$@" >"$tmp/out" 2>"$tmp/err"; then
		echo "FAIL $name: succeeded" >&2
		exit 1
	fi
	grep -qF -e "$message" "$tmp/out" "$tmp/err" || { echo "FAIL $name: stderr was: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $name"
}

# expect_ok NAME ARGS...: require-tests.sh ARGS must succeed.
expect_ok() {
	name=$1
	shift
	sh "$root/scripts/require-tests.sh" "$@" >"$tmp/out" 2>"$tmp/err" || { echo "FAIL $name: $(cat "$tmp/err")" >&2; exit 1; }
	echo "ok $name"
}

expect_ok one-test 1 cargo test --offline --manifest-path "$manifest" -- --exact tests::works
expect_ok ignored-test 1 cargo test --offline --manifest-path "$manifest" -- --ignored --exact tests::also_works
expect_ok zero-needed 0 cargo test --offline --manifest-path "$manifest" -- nothing_matches_this
expect no-match "0 tests passed, want at least 1" 1 cargo test --offline --manifest-path "$manifest" -- nothing_matches_this
expect inexact-name "0 tests passed, want at least 1" 1 cargo test --offline --manifest-path "$manifest" -- --exact works
expect too-few "1 tests passed, want at least 2" 2 cargo test --offline --manifest-path "$manifest" -- --exact tests::works

# expect_status NAME STATUS ARGS...: require-tests.sh ARGS must exit with STATUS.
expect_status() {
	name=$1
	want=$2
	shift 2
	got=0
	sh "$root/scripts/require-tests.sh" "$@" >"$tmp/out" 2>"$tmp/err" || got=$?
	[ "$got" -eq "$want" ] || { echo "FAIL $name: exit $got, want $want" >&2; exit 1; }
	echo "ok $name"
}

expect_status failing-test 101 1 cargo test --offline --manifest-path "$manifest" -- --ignored --exact tests::breaks
expect_status status-kept 7 1 sh -c 'exit 7'
expect missing-command "No such file" 1 "$tmp/no-such-command"
expect_status no-command 2
expect_status no-number 2 many cargo test --offline --manifest-path "$manifest"
