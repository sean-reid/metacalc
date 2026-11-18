# Worked examples

These are eight small proofs of the kind an intro-to-proofs course sets, each taken from a library that ships with metacalc. Every proof below is a real proof from set.mm, printed as a goal tree.

The calculator screens for building these proofs by hand do not exist yet. This page will gain the key-by-key steps and screenshots when they do.

## Reading a statement

A Metamath statement is a line of tokens separated by spaces. The first token, `|-`, is the turnstile and means "is provable". Everything after it is the claim, written in ASCII:

- `->` is "implies", `<->` is "if and only if", `-.` is "not", `/\` is "and", `\/` is "or".
- `A C_ B` is "A is a subset of B", `x e. A` is "x is an element of A", and `=` is equality.
- `ph`, `ps` and `ch` stand for any statements. `A`, `B` and `C` stand for any classes, which you can read as sets.

A theorem can have hypotheses, written "given" on the library pages. To use such a theorem you first need a proof of each hypothesis. Where an example has hypotheses, they are listed before its statement.

## Reading a goal tree

A proof is written backward, from what you want to show. The root is the goal. Each line is a statement followed by a label in brackets:

```text
|- ( ph -> ph )  [mpd]
  |- ( ph -> ( ph -> ph ) )  [ax-1]
  |- ( ph -> ( ( ph -> ph ) -> ph ) )  [ax-1]
```

Applying a theorem to a goal means choosing a theorem whose conclusion matches the goal once its variables are replaced by statements. The theorem's hypotheses, with the same replacements made, become new goals, and they are the lines indented under the goal. The label in brackets names the theorem applied. In this tree `mpd` is applied to the first goal, and `ax-1` closes both goals under it.

A goal closes when the theorem applied to it has no hypotheses, as `ax-1` does. A label such as `syl.1` is a hypothesis of the theorem being proved, so that goal is already given. The trees show only provable statements, and each theorem applied has a proof of its own that the tree does not expand.

Each statement is also drawn in the calculator's font. These images are renderings of the statement on a 96 by 64 pixel screen, enlarged four times. They are not screenshots of the calculator.

## 1. Every statement implies itself

For any statement `ph`, if `ph` holds then `ph` holds.

Statement: `|- ( ph -> ph )`

The label is [`id`](../libraries/prop.md#id), in the propositional calculus library.

![The statement of id drawn in the calculator's font](examples/id.png)

A rendering of `id` in the calculator's font.

<!-- outline: id -->
```text
|- ( ph -> ph )  [mpd]
  |- ( ph -> ( ph -> ph ) )  [ax-1]
  |- ( ph -> ( ( ph -> ph ) -> ph ) )  [ax-1]
```

The proof applies `mpd`, which combines `( ph -> ps )` and `( ph -> ( ps -> ch ) )` into `( ph -> ch )`. The statement `ps` does not appear in the goal. Here you choose `ps` to be `( ph -> ph )`, and both subgoals are then instances of the axiom `ax-1`, `( ph -> ( ps -> ph ) )`. This is a direct proof from the axioms.

## 2. Chaining implications

If `ph` implies `ps` and `ps` implies `ch`, then `ph` implies `ch`.

Hypotheses:

- `|- ( ph -> ps )`
- `|- ( ps -> ch )`

Statement: `|- ( ph -> ch )`

The label is [`syl`](../libraries/prop.md#syl), in the propositional calculus library.

![The statement of syl drawn in the calculator's font](examples/syl.png)

A rendering of `syl` in the calculator's font.

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

Statement: `|- ( -. ps -> -. ph )`

The label is [`con3i`](../libraries/prop.md#con3i), in the propositional calculus library.

![The statement of con3i drawn in the calculator's font](examples/con3i.png)

A rendering of `con3i` in the calculator's font.

<!-- outline: con3i -->
```text
|- ( -. ps -> -. ph )  [nsyl]
  |- ( -. ps -> -. ps )  [id]
  |- ( ph -> ps )  [con3i.a]
```

The proof applies `nsyl`, which chains an implication whose conclusion is a negation, `( ph -> -. ps )`, with an implication `( ch -> ps )`. Here the first one is `( -. ps -> -. ps )`, which `id` from the first example proves, and the second one is your hypothesis. The `id` line closes its goal because `id` has no hypotheses.

## 4. Proof by cases

If `ps` follows from `ph` and also follows from not `ph`, then `ps` holds.

Hypotheses:

- `|- ( ph -> ps )`
- `|- ( -. ph -> ps )`

Statement: `|- ps`

The label is [`pm2.61i`](../libraries/prop.md#pm2.61i), in the propositional calculus library.

![The statement of pm2.61i drawn in the calculator's font](examples/pm2.61i.png)

A rendering of `pm2.61i` in the calculator's font.

<!-- outline: pm2.61i -->
```text
|- ps  [pm2.18i]
  |- ( -. ps -> ps )  [nsyl4]
    |- ( ph -> ps )  [pm2.61i.1]
    |- ( -. ph -> ps )  [pm2.61i.2]
```

The proof has two steps. `nsyl4` combines the two hypotheses into `( -. ps -> ps )`, and `pm2.18i` turns that into `ps`. The cases are `ph` and `-. ph`, and the library assumes classical logic, so one of them always holds.

## 5. De Morgan's law

Not both `ph` and `ps` holds exactly when `ph` fails or `ps` fails.

Statement: `|- ( -. ( ph /\ ps ) <-> ( -. ph \/ -. ps ) )`

The label is [`ianor`](../libraries/prop.md#ianor), in the propositional calculus library.

![The statement of ianor drawn in the calculator's font](examples/ianor.png)

A rendering of `ianor` in the calculator's font.

<!-- outline: ianor -->
```text
|- ( -. ( ph /\ ps ) <-> ( -. ph \/ -. ps ) )  [bitr3i]
  |- ( ( ph -> -. ps ) <-> -. ( ph /\ ps ) )  [imnan]
  |- ( ( ph -> -. ps ) <-> ( -. ph \/ -. ps ) )  [pm4.62]
```

The statement is a biconditional, and the proof is a chain of two equivalences. `imnan` says `( ph -> -. ps )` is equivalent to `-. ( ph /\ ps )`, and `pm4.62` says it is equivalent to `( -. ph \/ -. ps )`. `bitr3i` joins them through the shared middle statement, which is the usual way to prove a law by rewriting one side into the other.

## 6. Subset is transitive

If `A` is a subset of `B` and `B` is a subset of `C`, then `A` is a subset of `C`.

Hypotheses:

- `|- A C_ B`
- `|- B C_ C`

Statement: `|- A C_ C`

The label is [`sstri`](../libraries/ring.md#sstri), in the ring theory library.

![The statement of sstri drawn in the calculator's font](examples/sstri.png)

A rendering of `sstri` in the calculator's font.

<!-- outline: sstri -->
```text
|- A C_ C  [mp2]
  |- A C_ B  [sstri.1]
  |- B C_ C  [sstri.2]
  |- ( A C_ B -> ( B C_ C -> A C_ C ) )  [sstr2]
```

The proof applies `mp2`, which takes two proven statements `ph` and `ps` and a proof of `( ph -> ( ps -> ch ) )` and concludes `ch`. The implication is `sstr2`, the same fact stated as one implication, so the proof only has to feed the two hypotheses into it. The subset relation belongs to set theory, which is part of the ring library because the ring theorems depend on it.

## 7. A group's identity works on the left

In a group `G` with underlying set `B`, operation `+` and identity element `0`, adding `0` on the left of any element `X` of `B` gives `X`.

Hypotheses:

- ``|- B = ( Base ` G )``
- ``|- .+ = ( +g ` G )``
- ``|- .0. = ( 0g ` G )``

Statement: `|- ( ( G e. Grp /\ X e. B ) -> ( .0. .+ X ) = X )`

The label is [`grplid`](../libraries/ring.md#grplid), in the ring theory library.

![The statement of grplid drawn in the calculator's font](examples/grplid.png)

A rendering of `grplid` in the calculator's font.

<!-- outline: grplid -->
```text
|- ( ( G e. Grp /\ X e. B ) -> ( .0. .+ X ) = X )  [sylan]
  |- ( G e. Grp -> G e. Mnd )  [grpmnd]
  |- ( ( G e. Mnd /\ X e. B ) -> ( .0. .+ X ) = X )  [mndlid]
    |- B = ( Base ` G )  [grpbn0.b]
    |- .+ = ( +g ` G )  [grplid.p]
    |- .0. = ( 0g ` G )  [grplid.o]
```

In the statement, `G e. Grp` says `G` is a group, `( .0. .+ X )` is `0 + X`, and the three hypotheses say that `B` is the underlying set of `G`, `.+` is its operation and `.0.` is its identity element. The proof applies `sylan` to reduce the goal to the same fact for monoids. `grpmnd` shows that every group is a monoid, and `mndlid` is the monoid theorem. This technique is the one to use when a more general theorem already exists.

## 8. Zero times anything is zero in a ring

In a ring `R` with underlying set `B`, multiplication `x` and zero `0`, `0 * X = 0` for every element `X` of `B`.

Hypotheses:

- ``|- B = ( Base ` R )``
- ``|- .x. = ( .r ` R )``
- ``|- .0. = ( 0g ` R )``

Statement: `|- ( ( R e. Ring /\ X e. B ) -> ( .0. .x. X ) = .0. )`

The label is [`ringlz`](../libraries/ring.md#ringlz), in the ring theory library.

![The statement of ringlz drawn in the calculator's font](examples/ringlz.png)

A rendering of `ringlz` in the calculator's font.

<!-- outline: ringlz -->
```text
|- ( ( R e. Ring /\ X e. B ) -> ( .0. .x. X ) = .0. )  [sylan]
  |- ( R e. Ring -> R e. Rng )  [ringrng]
  |- ( ( R e. Rng /\ X e. B ) -> ( .0. .x. X ) = .0. )  [rnglz]
    |- B = ( Base ` R )  [ringz.b]
    |- .x. = ( .r ` R )  [ringz.t]
    |- .0. = ( 0g ` R )  [ringz.z]
```

In the statement, `R e. Ring` says `R` is a ring and `( .0. .x. X )` is `0 * X`. The tree above is the complete proof, six lines, and it has the same shape as the previous example. The three hypotheses say that `B` is the underlying set of `R`, `.x.` is its multiplication and `.0.` is its zero. `ringrng` shows that a ring is a non-unital ring, which is a ring without the requirement of a multiplicative identity, and `rnglz` proves the fact there. The proof of `rnglz` is not expanded here.
