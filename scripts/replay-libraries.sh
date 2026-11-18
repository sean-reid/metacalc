#!/bin/sh
# Usage: replay-libraries.sh libdir
# Rebuilds every proof of the prop, pred and ring libraries in libdir with the editor model's own
# operations, splices each export into the database in place of the original and checks it with
# the mcpack verifier. It prints "verified N of M" per library and fails unless N is M, the
# number of proofs build-libraries.sh recorded in libdir/NAME.proofs.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
MCEDIT_LIBRARIES=$1 sh scripts/require-tests.sh 1 cargo test --release --locked -p mcedit --test replay -- --ignored --nocapture --exact exported_libraries_replay
