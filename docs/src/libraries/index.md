# Shipped libraries

The calculator ships libraries extracted from [set.mm](https://github.com/metamath/set.mm) as staged dependency closures. Each one contains the one before it.

- [Propositional calculus](prop.md)
- [Predicate calculus with equality](pred.md)
- [Ring theory](ring.md)

A manifest under `libraries/` names the root theorems of each library and where it stops, and `libraries/source.toml` pins the exact set.mm commit and its SHA-256. `mcpack build` refuses any other file. Each page lists every assertion in the packed library, decoded from the same bytes the calculator reads, with the library's size and the pinned source commit.
