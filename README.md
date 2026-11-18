# metacalc

A Metamath proof verifier and interactive proof editor for the monochrome TI-84 Plus Silver Edition.

metacalc has two halves. A desktop tool extracts a dependency-complete subset of a Metamath database such as set.mm and packs it into a compact binary library. A check app on the calculator reads that library from flash and runs every proof through its own kernel; an app to browse theorems and build proofs is still to come.

## Status

Early development. The desktop tool parses, verifies and packs Metamath databases, and signed apps run the proof kernel over a whole library on the calculator.

## Build

You need a Rust toolchain.

```sh
cargo build --release
cargo test
```

## Usage

```sh
cargo run --release -p mcpack -- verify "$(sh scripts/fetch-setmm.sh)"
```

## Licence

MIT. Library data extracted from set.mm stays under set.mm's CC0 dedication.
