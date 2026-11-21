# Examples

Eight proofs of the kind an intro-to-proofs course sets, taken from the set.mm libraries metacalc builds and printed as goal trees.

## Reading a statement

A Metamath statement is a line of symbols separated by spaces. The first symbol, ⊢, is the turnstile and means "is provable". Everything after it is the claim. The last column shows the ASCII token that a Metamath source file uses for each symbol.

<!-- symbols -->

| Symbol | Read as | In a source file |
| --- | --- | --- |
| ⊢ | is provable | `\|-` |
| ( ) | grouping | `(` `)` |
| → | implies | `->` |
| ↔ | if and only if | `<->` |
| ¬ | not | `-.` |
| ∧ | and | `/\` |
| ∨ | or | `\/` |
| = | equals | `=` |
| ∈ | is an element of | `e.` |
| ⊆ | is a subset of | `C_` |
| ‘ | applied to, as in ( 𝐹 ‘ 𝐴 ) | `` ` `` |
| 𝜑 𝜓 𝜒 | any statement | `ph` `ps` `ch` |
| 𝐴 𝐵 𝐶 𝐺 𝑅 𝑋 | any class, which you can read as a set | `A` `B` `C` `G` `R` `X` |
| Grp Mnd Ring Rng | is a group, a monoid, a ring, a non-unital ring | `Grp` `Mnd` `Ring` `Rng` |
| Base +g 0g .r | the set, the operation, the identity and the multiplication of a structure | `Base` `+g` `0g` `.r` |
| + 0 · | an operation or element that the hypotheses name, underlined with dots | `.+` `.0.` `.x.` |

<!-- /symbols -->

A theorem can have hypotheses, written "given" on the library pages. To use such a theorem you first need a proof of each hypothesis. Where an example has hypotheses, they are listed before its tree.

## Reading a goal tree

A proof is written backward, from what you want to show. The root is the goal. Each line is a statement followed by a label, the theorem applied to it. A label that is a link opens that theorem on its library page.

Applying a theorem to a goal means choosing a theorem whose conclusion matches the goal once its variables are replaced by statements. The theorem's hypotheses, with the same replacements made, become new goals, and they are the lines indented under the goal. In the tree of example 1, `mpd` is applied to the first goal, and `ax-1` closes both goals under it.

A goal closes when the theorem applied to it has no hypotheses, as `ax-1` does. A label such as `syl.1` is a hypothesis of the theorem being proved, so that goal is already given and its label is not a link. Each theorem applied has its own proof, which the tree does not expand.

The root statement of each example is also drawn in the calculator's font. These images are renderings of the statement on a 96 by 64 pixel screen, enlarged four times.

## 1. Every statement implies itself

[`id`](../libraries/prop.md#id), propositional calculus library

![id: the statement drawn in the calculator font](examples/id.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: id -->
<ul class="tree" role="list" data-statement="|- ( ph -&gt; ph )">
  <li><span class="st">⊢ ( 𝜑 → 𝜑 )</span> <a class="lb" href="../libraries/prop.md#mpd">mpd</a>
    <ul role="list">
      <li><span class="st">⊢ ( 𝜑 → ( 𝜑 → 𝜑 ) )</span> <a class="lb" href="../libraries/prop.md#ax-1">ax-1</a></li>
      <li><span class="st">⊢ ( 𝜑 → ( ( 𝜑 → 𝜑 ) → 𝜑 ) )</span> <a class="lb" href="../libraries/prop.md#ax-1">ax-1</a></li>
    </ul>
  </li>
</ul>

`mpd` combines `( 𝜑 → 𝜓 )` and `( 𝜑 → ( 𝜓 → 𝜒 ) )` into `( 𝜑 → 𝜒 )`. The statement `𝜓` does not appear in the goal, so you choose it. With `𝜓` as `( 𝜑 → 𝜑 )`, both subgoals are instances of the axiom `ax-1`, `( 𝜑 → ( 𝜓 → 𝜑 ) )`.

## 2. Chaining implications

If `𝜑` implies `𝜓` and `𝜓` implies `𝜒`, then `𝜑` implies `𝜒`.

Hypotheses:

- `⊢ ( 𝜑 → 𝜓 )`
- `⊢ ( 𝜓 → 𝜒 )`

[`syl`](../libraries/prop.md#syl), propositional calculus library

![syl: the statement drawn in the calculator font](examples/syl.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: syl -->
<ul class="tree" role="list" data-statement="|- ( ph -&gt; ch )">
  <li><span class="st">⊢ ( 𝜑 → 𝜒 )</span> <a class="lb" href="../libraries/prop.md#mpd">mpd</a>
    <ul role="list">
      <li><span class="st">⊢ ( 𝜑 → 𝜓 )</span> <span class="lb">syl.1</span></li>
      <li><span class="st">⊢ ( 𝜑 → ( 𝜓 → 𝜒 ) )</span> <a class="lb" href="../libraries/prop.md#a1i">a1i</a>
        <ul role="list">
          <li><span class="st">⊢ ( 𝜓 → 𝜒 )</span> <span class="lb">syl.2</span></li>
        </ul>
      </li>
    </ul>
  </li>
</ul>

This is the hypothetical syllogism. The proof applies `mpd` to the goal. The first subgoal is the first hypothesis. For the second, `a1i` turns the second hypothesis `( 𝜓 → 𝜒 )` into `( 𝜑 → ( 𝜓 → 𝜒 ) )`, which says that a proven statement still holds when `𝜑` is assumed.

## 3. The contrapositive

If `𝜑` implies `𝜓`, then not `𝜓` implies not `𝜑`.

Hypotheses:

- `⊢ ( 𝜑 → 𝜓 )`

[`con3i`](../libraries/prop.md#con3i), propositional calculus library

![con3i: the statement drawn in the calculator font](examples/con3i.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: con3i -->
<ul class="tree" role="list" data-statement="|- ( -. ps -&gt; -. ph )">
  <li><span class="st">⊢ ( ¬ 𝜓 → ¬ 𝜑 )</span> <a class="lb" href="../libraries/prop.md#nsyl">nsyl</a>
    <ul role="list">
      <li><span class="st">⊢ ( ¬ 𝜓 → ¬ 𝜓 )</span> <a class="lb" href="../libraries/prop.md#id">id</a></li>
      <li><span class="st">⊢ ( 𝜑 → 𝜓 )</span> <span class="lb">con3i.a</span></li>
    </ul>
  </li>
</ul>

The proof applies `nsyl`, which chains an implication whose conclusion is a negation, `( 𝜑 → ¬ 𝜓 )`, with an implication `( 𝜒 → 𝜓 )`. Here the first one is `( ¬ 𝜓 → ¬ 𝜓 )`, which `id` from the first example proves, and the second one is your hypothesis.

## 4. Proof by cases

If `𝜓` follows from `𝜑` and also follows from not `𝜑`, then `𝜓` holds.

Hypotheses:

- `⊢ ( 𝜑 → 𝜓 )`
- `⊢ ( ¬ 𝜑 → 𝜓 )`

[`pm2.61i`](../libraries/prop.md#pm2.61i), propositional calculus library

![pm2.61i: the statement drawn in the calculator font](examples/pm2.61i.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: pm2.61i -->
<ul class="tree" role="list" data-statement="|- ps">
  <li><span class="st">⊢ 𝜓</span> <a class="lb" href="../libraries/prop.md#pm2.18i">pm2.18i</a>
    <ul role="list">
      <li><span class="st">⊢ ( ¬ 𝜓 → 𝜓 )</span> <a class="lb" href="../libraries/prop.md#nsyl4">nsyl4</a>
        <ul role="list">
          <li><span class="st">⊢ ( 𝜑 → 𝜓 )</span> <span class="lb">pm2.61i.1</span></li>
          <li><span class="st">⊢ ( ¬ 𝜑 → 𝜓 )</span> <span class="lb">pm2.61i.2</span></li>
        </ul>
      </li>
    </ul>
  </li>
</ul>

`nsyl4` combines the two hypotheses into `( ¬ 𝜓 → 𝜓 )`, and `pm2.18i` turns that into `𝜓`. The cases are `𝜑` and `¬ 𝜑`, and the library assumes classical logic, so one of them always holds.

## 5. De Morgan's law

Not both `𝜑` and `𝜓` holds exactly when `𝜑` fails or `𝜓` fails.

[`ianor`](../libraries/prop.md#ianor), propositional calculus library

![ianor: the statement drawn in the calculator font](examples/ianor.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: ianor -->
<ul class="tree" role="list" data-statement="|- ( -. ( ph /\ ps ) &lt;-&gt; ( -. ph \/ -. ps ) )">
  <li><span class="st">⊢ ( ¬ ( 𝜑 ∧ 𝜓 ) ↔ ( ¬ 𝜑 ∨ ¬ 𝜓 ) )</span> <a class="lb" href="../libraries/prop.md#bitr3i">bitr3i</a>
    <ul role="list">
      <li><span class="st">⊢ ( ( 𝜑 → ¬ 𝜓 ) ↔ ¬ ( 𝜑 ∧ 𝜓 ) )</span> <a class="lb" href="../libraries/prop.md#imnan">imnan</a></li>
      <li><span class="st">⊢ ( ( 𝜑 → ¬ 𝜓 ) ↔ ( ¬ 𝜑 ∨ ¬ 𝜓 ) )</span> <a class="lb" href="../libraries/prop.md#pm4.62">pm4.62</a></li>
    </ul>
  </li>
</ul>

The proof chains two equivalences. `imnan` says `( 𝜑 → ¬ 𝜓 )` is equivalent to `¬ ( 𝜑 ∧ 𝜓 )`, and `pm4.62` says it is equivalent to `( ¬ 𝜑 ∨ ¬ 𝜓 )`. `bitr3i` joins them through the shared middle statement.

## 6. Subset is transitive

If `𝐴` is a subset of `𝐵` and `𝐵` is a subset of `𝐶`, then `𝐴` is a subset of `𝐶`.

Hypotheses:

- `⊢ 𝐴 ⊆ 𝐵`
- `⊢ 𝐵 ⊆ 𝐶`

[`sstri`](../libraries/ring.md#sstri), ring theory library

![sstri: the statement drawn in the calculator font](examples/sstri.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: sstri -->
<ul class="tree" role="list" data-statement="|- A C_ C">
  <li><span class="st">⊢ 𝐴 ⊆ 𝐶</span> <a class="lb" href="../libraries/prop.md#mp2">mp2</a>
    <ul role="list">
      <li><span class="st">⊢ 𝐴 ⊆ 𝐵</span> <span class="lb">sstri.1</span></li>
      <li><span class="st">⊢ 𝐵 ⊆ 𝐶</span> <span class="lb">sstri.2</span></li>
      <li><span class="st">⊢ ( 𝐴 ⊆ 𝐵 → ( 𝐵 ⊆ 𝐶 → 𝐴 ⊆ 𝐶 ) )</span> <a class="lb" href="../libraries/ring.md#sstr2">sstr2</a></li>
    </ul>
  </li>
</ul>

The proof applies `mp2`, which takes two proven statements `𝜑` and `𝜓` and a proof of `( 𝜑 → ( 𝜓 → 𝜒 ) )` and concludes `𝜒`. The implication is `sstr2`, the same fact stated as one implication, so the proof only has to feed the two hypotheses into it.

## 7. A group's identity works on the left

In a group `𝐺` with underlying set `𝐵`, operation `+` and identity element `0`, adding `0` on the left of any element `𝑋` of `𝐵` gives `𝑋`.

Hypotheses:

- ``⊢ 𝐵 = ( Base ‘ 𝐺 )``
- ``⊢ + = ( +g ‘ 𝐺 )``
- ``⊢ 0 = ( 0g ‘ 𝐺 )``

[`grplid`](../libraries/ring.md#grplid), ring theory library

![grplid: the statement drawn in the calculator font](examples/grplid.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: grplid -->
<ul class="tree" role="list" data-statement="|- ( ( G e. Grp /\ X e. B ) -&gt; ( .0. .+ X ) = X )">
  <li><span class="st">⊢ ( ( 𝐺 ∈ Grp ∧ 𝑋 ∈ 𝐵 ) → ( <span class="s">0</span> <span class="s">+</span> 𝑋 ) = 𝑋 )</span> <a class="lb" href="../libraries/prop.md#sylan">sylan</a>
    <ul role="list">
      <li><span class="st">⊢ ( 𝐺 ∈ Grp → 𝐺 ∈ Mnd )</span> <a class="lb" href="../libraries/ring.md#grpmnd">grpmnd</a></li>
      <li><span class="st">⊢ ( ( 𝐺 ∈ Mnd ∧ 𝑋 ∈ 𝐵 ) → ( <span class="s">0</span> <span class="s">+</span> 𝑋 ) = 𝑋 )</span> <a class="lb" href="../libraries/ring.md#mndlid">mndlid</a>
        <ul role="list">
          <li><span class="st">⊢ 𝐵 = ( Base ‘ 𝐺 )</span> <span class="lb">grpbn0.b</span></li>
          <li><span class="st">⊢ <span class="s">+</span> = ( +g ‘ 𝐺 )</span> <span class="lb">grplid.p</span></li>
          <li><span class="st">⊢ <span class="s">0</span> = ( 0g ‘ 𝐺 )</span> <span class="lb">grplid.o</span></li>
        </ul>
      </li>
    </ul>
  </li>
</ul>

In the statement, `𝐺 ∈ Grp` says `𝐺` is a group, and `( 0 + 𝑋 )` adds the identity to `𝑋`, where `+` and `0` are the operation and identity the hypotheses name. The proof applies `sylan` to reduce the goal to the same fact for monoids. `grpmnd` shows that every group is a monoid, and `mndlid` is the monoid theorem.

## 8. Zero times anything is zero in a ring

In a ring `𝑅` with underlying set `𝐵`, multiplication `·` and zero `0`, `0 · 𝑋 = 0` for every element `𝑋` of `𝐵`.

Hypotheses:

- ``⊢ 𝐵 = ( Base ‘ 𝑅 )``
- ``⊢ · = ( .r ‘ 𝑅 )``
- ``⊢ 0 = ( 0g ‘ 𝑅 )``

[`ringlz`](../libraries/ring.md#ringlz), ring theory library

![ringlz: the statement drawn in the calculator font](examples/ringlz.png)

*A rendering of the statement in the calculator's font, not a screen capture.*

<!-- outline: ringlz -->
<ul class="tree" role="list" data-statement="|- ( ( R e. Ring /\ X e. B ) -&gt; ( .0. .x. X ) = .0. )">
  <li><span class="st">⊢ ( ( 𝑅 ∈ Ring ∧ 𝑋 ∈ 𝐵 ) → ( <span class="s">0</span> <span class="s">·</span> 𝑋 ) = <span class="s">0</span> )</span> <a class="lb" href="../libraries/prop.md#sylan">sylan</a>
    <ul role="list">
      <li><span class="st">⊢ ( 𝑅 ∈ Ring → 𝑅 ∈ Rng )</span> <a class="lb" href="../libraries/ring.md#ringrng">ringrng</a></li>
      <li><span class="st">⊢ ( ( 𝑅 ∈ Rng ∧ 𝑋 ∈ 𝐵 ) → ( <span class="s">0</span> <span class="s">·</span> 𝑋 ) = <span class="s">0</span> )</span> <a class="lb" href="../libraries/ring.md#rnglz">rnglz</a>
        <ul role="list">
          <li><span class="st">⊢ 𝐵 = ( Base ‘ 𝑅 )</span> <span class="lb">ringz.b</span></li>
          <li><span class="st">⊢ <span class="s">·</span> = ( .r ‘ 𝑅 )</span> <span class="lb">ringz.t</span></li>
          <li><span class="st">⊢ <span class="s">0</span> = ( 0g ‘ 𝑅 )</span> <span class="lb">ringz.z</span></li>
        </ul>
      </li>
    </ul>
  </li>
</ul>

In the statement, `𝑅 ∈ Ring` says `𝑅` is a ring, and `( 0 · 𝑋 )` multiplies `𝑋` by zero, where `·` and `0` are the multiplication and zero the hypotheses name. The proof has the same shape as example 7, with `ringrng` and `rnglz` in place of `grpmnd` and `mndlid`. `ringrng` shows that a ring is a non-unital ring, which is a ring without the requirement of a multiplicative identity, and `rnglz` proves the fact there.
