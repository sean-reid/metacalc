# metacalc

A Metamath proof verifier and interactive proof editor for the monochrome TI-84 Plus Silver Edition.

metacalc has two halves. A desktop tool extracts a dependency-complete subset of a Metamath database such as set.mm and packs it into a compact binary library. A calculator application reads that library from flash, checks every proof with its own kernel, and lets you browse theorems and build proofs with the arrow keys and the five keys under the screen.

## Status

Early development. The repository holds the build scaffold; nothing runs on a calculator yet.

## Build

You need a Rust toolchain.

```sh
cargo build --release
cargo test
```

## Usage

```sh
cargo run -p mcpack -- --help
```

## Licence

MIT. Library data extracted from set.mm stays under set.mm's CC0 dedication.
