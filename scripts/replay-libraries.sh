#!/bin/sh
# Usage: replay-libraries.sh libdir
# Rebuilds every proof of the prop, pred and ring libraries in libdir with the editor model's own
# operations. Each export is spliced into the database in place of the original and checked with
# the mcpack verifier, and all exports are spliced into one database per library, which
# metamath-exe verifies. It prints "verified N of M" and "metamath-exe verified N of M" per
# library and fails unless N is M, the number of proofs build-libraries.sh recorded in
# libdir/NAME.proofs. metamath-exe comes from scripts/fetch-metamath.sh.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
spliced=$(mktemp -d)
trap 'rm -rf "$spliced"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
MCEDIT_SPLICED=$spliced MCEDIT_LIBRARIES=$1 sh scripts/require-tests.sh 1 cargo test --release --locked -p mcedit --test replay -- --ignored --nocapture --exact exported_libraries_replay
sh scripts/check-replayed.sh "$1" "$spliced"
