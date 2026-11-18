#!/bin/sh
# Usage: replay-libraries.sh libdir
# Rebuilds the proofs of the prop, pred and ring libraries in libdir with the editor model's own
# operations, writes each export back into the database and checks it with the reference verifier.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
MCEDIT_LIBRARIES=$1 cargo test --release --locked -p mcedit --test replay exported_libraries -- --ignored --nocapture
