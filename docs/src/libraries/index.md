# Shipped libraries

A library is a set of Metamath statements and the proofs of its theorems, packed into the flash pages the calculator reads. Each library below contains the one before it: ring theory includes all of predicate calculus, and predicate calculus includes all of propositional calculus.

<!-- libraries-table -->
| Library | Covers | Axioms | Theorems | Size (bytes) | Flash pages |
| --- | --- | ---: | ---: | ---: | ---: |
| Built with the site | | | | | |
<!-- /libraries-table -->

Syntax rules and definitions are listed on each page but not counted here. A flash page holds 16,384 bytes.

Every page starts with a notation key. It then lists the syntax rules, axioms, definitions and theorems in the order the library stores them, each statement in mathematical notation with its ASCII spelling under it.

## How the libraries are built

The libraries are extracted from [set.mm](https://github.com/metamath/set.mm). A manifest under `libraries/` names the root theorems of each library and where it stops, and takes in everything they depend on.

`libraries/source.toml` pins the exact set.mm commit and its SHA-256, and `mcpack build` refuses any other file. Each page links the pinned commit.

The pages and the table above are generated when the site is built. They are decoded from the packed library, the same bytes the calculator reads, so the counts and sizes cannot differ from what ships.
