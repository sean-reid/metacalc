# Worked examples

Eight proofs of the kind an intro-to-proofs course sets, taken from the set.mm libraries metacalc builds and printed as goal trees.

## Reading a statement

A Metamath statement is a line of tokens separated by spaces. The first token, `|-`, is the turnstile and means "is provable". Everything after it is the claim, written in ASCII:

- `->` is "implies", `<->` is "if and only if", `-.` is "not", `/\` is "and", `\/` is "or".
- `A C_ B` is "A is a subset of B", `x e. A` is "x is an element of A", and `=` is equality.
- `ph`, `ps` and `ch` stand for any statements. `A`, `B` and `C` stand for any classes, which you can read as sets.

A theorem can have hypotheses, written "given" on the library pages. To use such a theorem you first need a proof of each hypothesis. Where an example has hypotheses, they are listed before its tree.

## Reading a goal tree

A proof is written backward, from what you want to show. The root is the goal. Each line is a statement followed by a label in brackets:

```text
|- ( ph -> ph )  [mpd]
  |- ( ph -> ( ph -> ph ) )  [ax-1]
  |- ( ph -> ( ( ph -> ph ) -> ph ) )  [ax-1]
```

Applying a theorem to a goal means choosing a theorem whose conclusion matches the goal once its variables are replaced by statements. The theorem's hypotheses, with the same replacements made, become new goals, and they are the lines indented under the goal. In this tree the bracketed `mpd` is applied to the first goal, and `ax-1` closes both goals under it.

A goal closes when the theorem applied to it has no hypotheses, as `ax-1` does. A label such as `syl.1` is a hypothesis of the theorem being proved, so that goal is already given. Each theorem applied has its own proof, which the tree does not expand.

The root statement of each example is also drawn in the calculator's font. These images are renderings of the statement on a 96 by 64 pixel screen, enlarged four times.

## 1. Every statement implies itself

[`id`](../libraries/prop.md#id), propositional calculus library

![id](examples/id.png)

<!-- outline: id -->
```text
|- ( ph -> ph )  [mpd]
  |- ( ph -> ( ph -> ph ) )  [ax-1]
  |- ( ph -> ( ( ph -> ph ) -> ph ) )  [ax-1]
```

`mpd` combines `( ph -> ps )` and `( ph -> ( ps -> ch ) )` into `( ph -> ch )`. The statement `ps` does not appear in the goal, so you choose it. With `ps` as `( ph -> ph )`, both subgoals are instances of the axiom `ax-1`, `( ph -> ( ps -> ph ) )`.

## 2. Chaining implications

If `ph` implies `ps` and `ps` implies `ch`, then `ph` implies `ch`.

Hypotheses:

- `|- ( ph -> ps )`
- `|- ( ps -> ch )`

[`syl`](../libraries/prop.md#syl), propositional calculus library

![syl](examples/syl.png)

<!-- outline: syl -->
```text
|- ( ph -> ch )  [mpd]
  |- ( ph -> ps )  [syl.1]
  |- ( ph -> ( ps -> ch ) )  [a1i]
    |- ( ps -> ch )  [syl.2]
```

This is the hypothetical syllogism. The proof applies `mpd` to the goal. The first subgoal is the first hypothesis. For the second, `a1i` turns the second hypothesis `( ps -> ch )` into `( ph -> ( ps -> ch ) )`, which says that a proven statement still holds when `ph` is assumed.

## 3. The contrapositive

If `ph` implies `ps`, then not `ps` implies not `ph`.

Hypotheses:

- `|- ( ph -> ps )`

[`con3i`](../libraries/prop.md#con3i), propositional calculus library

![con3i](examples/con3i.png)

<!-- outline: con3i -->
```text
|- ( -. ps -> -. ph )  [nsyl]
  |- ( -. ps -> -. ps )  [id]
  |- ( ph -> ps )  [con3i.a]
```

The proof applies `nsyl`, which chains an implication whose conclusion is a negation, `( ph -> -. ps )`, with an implication `( ch -> ps )`. Here the first one is `( -. ps -> -. ps )`, which `id` from the first example proves, and the second one is your hypothesis.

## 4. Proof by cases

If `ps` follows from `ph` and also follows from not `ph`, then `ps` holds.

Hypotheses:

- `|- ( ph -> ps )`
- `|- ( -. ph -> ps )`

[`pm2.61i`](../libraries/prop.md#pm2.61i), propositional calculus library

![pm2.61i](examples/pm2.61i.png)

<!-- outline: pm2.61i -->
```text
|- ps  [pm2.18i]
  |- ( -. ps -> ps )  [nsyl4]
    |- ( ph -> ps )  [pm2.61i.1]
    |- ( -. ph -> ps )  [pm2.61i.2]
```

`nsyl4` combines the two hypotheses into `( -. ps -> ps )`, and `pm2.18i` turns that into `ps`. The cases are `ph` and `-. ph`, and the library assumes classical logic, so one of them always holds.

## 5. De Morgan's law

Not both `ph` and `ps` holds exactly when `ph` fails or `ps` fails.

[`ianor`](../libraries/prop.md#ianor), propositional calculus library

![ianor](examples/ianor.png)

<!-- outline: ianor -->
```text
|- ( -. ( ph /\ ps ) <-> ( -. ph \/ -. ps ) )  [bitr3i]
  |- ( ( ph -> -. ps ) <-> -. ( ph /\ ps ) )  [imnan]
  |- ( ( ph -> -. ps ) <-> ( -. ph \/ -. ps ) )  [pm4.62]
```

The proof chains two equivalences. `imnan` says `( ph -> -. ps )` is equivalent to `-. ( ph /\ ps )`, and `pm4.62` says it is equivalent to `( -. ph \/ -. ps )`. `bitr3i` joins them through the shared middle statement.

## 6. Subset is transitive

If `A` is a subset of `B` and `B` is a subset of `C`, then `A` is a subset of `C`.

Hypotheses:

- `|- A C_ B`
- `|- B C_ C`

[`sstri`](../libraries/ring.md#sstri), ring theory library

![sstri](examples/sstri.png)

<!-- outline: sstri -->
```text
|- A C_ C  [mp2]
  |- A C_ B  [sstri.1]
  |- B C_ C  [sstri.2]
  |- ( A C_ B -> ( B C_ C -> A C_ C ) )  [sstr2]
```

The proof applies `mp2`, which takes two proven statements `ph` and `ps` and a proof of `( ph -> ( ps -> ch ) )` and concludes `ch`. The implication is `sstr2`, the same fact stated as one implication, so the proof only has to feed the two hypotheses into it.

## 7. A group's identity works on the left

In a group `G` with underlying set `B`, operation `+` and identity element `0`, adding `0` on the left of any element `X` of `B` gives `X`.

Hypotheses:

- ``|- B = ( Base ` G )``
- ``|- .+ = ( +g ` G )``
- ``|- .0. = ( 0g ` G )``

[`grplid`](../libraries/ring.md#grplid), ring theory library

![grplid](examples/grplid.png)

<!-- outline: grplid -->
```text
|- ( ( G e. Grp /\ X e. B ) -> ( .0. .+ X ) = X )  [sylan]
  |- ( G e. Grp -> G e. Mnd )  [grpmnd]
  |- ( ( G e. Mnd /\ X e. B ) -> ( .0. .+ X ) = X )  [mndlid]
    |- B = ( Base ` G )  [grpbn0.b]
    |- .+ = ( +g ` G )  [grplid.p]
    |- .0. = ( 0g ` G )  [grplid.o]
```

In the statement, `G e. Grp` says `G` is a group and `( .0. .+ X )` is `0 + X`. The proof applies `sylan` to reduce the goal to the same fact for monoids. `grpmnd` shows that every group is a monoid, and `mndlid` is the monoid theorem.

## 8. Zero times anything is zero in a ring

In a ring `R` with underlying set `B`, multiplication `*` and zero `0`, `0 * X = 0` for every element `X` of `B`.

Hypotheses:

- ``|- B = ( Base ` R )``
- ``|- .x. = ( .r ` R )``
- ``|- .0. = ( 0g ` R )``

[`ringlz`](../libraries/ring.md#ringlz), ring theory library

![ringlz](examples/ringlz.png)

<!-- outline: ringlz -->
```text
|- ( ( R e. Ring /\ X e. B ) -> ( .0. .x. X ) = .0. )  [sylan]
  |- ( R e. Ring -> R e. Rng )  [ringrng]
  |- ( ( R e. Rng /\ X e. B ) -> ( .0. .x. X ) = .0. )  [rnglz]
    |- B = ( Base ` R )  [ringz.b]
    |- .x. = ( .r ` R )  [ringz.t]
    |- .0. = ( 0g ` R )  [ringz.z]
```

In the statement, `R e. Ring` says `R` is a ring and `( .0. .x. X )` is `0 * X`. The proof has the same shape as example 7, with `ringrng` and `rnglz` in place of `grpmnd` and `mndlid`. `ringrng` shows that a ring is a non-unital ring, which is a ring without the requirement of a multiplicative identity, and `rnglz` proves the fact there.
