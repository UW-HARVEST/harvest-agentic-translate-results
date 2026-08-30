# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C code actually takes, and from the
full public API surface in `c_src/include/simplestruct.h`.

## Axis enumeration (derived from the source, not guessed)

### Runtime options / modes / flags

`grep` of the public header and the implementation for options, flags, modes,
`switch`, or `#ifdef`-selected behaviour:

* public functions in `include/simplestruct.h`: **1** — `int smallestValue(struct ListNode *)`
* parameters other than the list itself: **none**
* global/static configuration state: **none**
* `switch` statements: **none**
* conditional-compilation branches affecting behaviour: **none**
  (the only `#ifndef` is the `SIMPLESTRUCT_H_` include guard)
* cargo features in `translation/Cargo.toml`: **none** (no `[features]` section)

=> The **option axis is a single point.** There is no configuration to vary, so
the cross-product below is driven entirely by input shape.

### Public entry points (complete set, including the lowest level)

There is exactly one entry point, and it *is* the lowest-level one — there are no
convenience wrappers, no init/teardown pair, and no separate one-shot API. So
"exercise the low-level entry points directly" is satisfied by calling
`smallestValue` directly through the `.so` export. There is no composed pipeline
to hide bugs in; the only caller-visible state is the list the caller builds, so
each test builds the node chain itself (the real consumer's job) rather than
using a helper the library provides.

### Input shapes the code special-cases

From the three branches in the implementation:

| branch | line | shape it distinguishes |
|--------|------|------------------------|
| `if (head)` | 27 | NULL vs non-NULL list |
| `while (head->next)` | 29 | length 1 (body never runs) vs length >= 2 |
| `if (head->value < smallest)` | 31 | whether a later node **strictly** beats the running min — so *position of the minimum* and *ties* are distinct shapes |

Plus the value-domain shapes implied by `int` arithmetic: sign, `INT_MIN`/`INT_MAX`
boundaries, and the `-1` collision with the error sentinel.

Resulting axes:
* **length**: 0 (NULL), 1, 2, 3, small (4-16), large (1e3-1e5)
* **position of minimum**: first (seed wins), middle, last, everywhere (all equal)
* **value domain**: all positive, all negative, mixed, `INT_MIN`, `INT_MAX`, full `i32` range, narrow range (forces ties)
* **ties**: unique minimum vs minimum repeated (exercises the strict `<`)

## Configuration-surface table

Every row is driven with **many randomized inputs** (seeded, deterministic
xorshift64* PRNG, seed fixed per row) except where the shape is a single fixed
value. Both `.so`s are called with the identical node chain and results compared
byte-for-byte (`i32` bit patterns).

| # | entry point(s) | configuration (options set + input shape) | iters | [ ] |
|---|----------------|--------------------------------------------|-------|-----|
| C1 | `smallestValue` | length 0 — NULL head (no options; only degenerate shape) | 1 | [x] |
| C2 | `smallestValue` | length 1, value random over full `i32` range; `while` body never executes | 2000 | [x] |
| C3 | `smallestValue` | length 1, value fixed at each of `INT_MIN`, `-1`, `0`, `1`, `INT_MAX` | 5 | [x] |
| C4 | `smallestValue` | length 2, both values random full range (min may be at either position) | 2000 | [x] |
| C5 | `smallestValue` | length 2, minimum forced at position 0 (seed wins, `<` never fires) | 500 | [x] |
| C6 | `smallestValue` | length 2, minimum forced at position 1 (`<` fires exactly once) | 500 | [x] |
| C7 | `smallestValue` | length 2, both values **equal** (tie: strict `<` must not fire) | 500 | [x] |
| C8 | `smallestValue` | length 3, all values random full range | 2000 | [x] |
| C9 | `smallestValue` | length 4-16 random, values random full range | 3000 | [x] |
| C10 | `smallestValue` | length 4-16, minimum forced at **first** position | 1000 | [x] |
| C11 | `smallestValue` | length 4-16, minimum forced at a random **middle** position | 1000 | [x] |
| C12 | `smallestValue` | length 4-16, minimum forced at **last** position (latest possible `<` fire) | 1000 | [x] |
| C13 | `smallestValue` | length 4-16, **all values identical** (no `<` ever fires) | 1000 | [x] |
| C14 | `smallestValue` | length 8-64, values from a **narrow range** (`-3..=3`) so the minimum repeats many times (ties throughout) | 2000 | [x] |
| C15 | `smallestValue` | length 4-32, **all positive** values (`1..=i32::MAX`) | 1000 | [x] |
| C16 | `smallestValue` | length 4-32, **all negative** values (`i32::MIN..=-1`) | 1000 | [x] |
| C17 | `smallestValue` | length 4-32, mixed sign, `INT_MIN` injected at a random position | 1000 | [x] |
| C18 | `smallestValue` | length 4-32, every value `INT_MAX` (upper boundary, all ties) | 500 | [x] |
| C19 | `smallestValue` | length 4-32, values drawn only from the sign-boundary set `{INT_MIN, -1, 0, 1, INT_MAX, 0x7FFFFFFF, -0x80000000}` (signed-vs-unsigned compare trap) | 2000 | [x] |
| C20 | `smallestValue` | length 4-32, values random, with `-1` guaranteed present (return collides with the error sentinel) | 1000 | [x] |
| C21 | `smallestValue` | length 1000 (large), values random full range | 50 | [x] |
| C22 | `smallestValue` | length 100000 (oversized), values random full range | 3 | [x] |
| C23 | `smallestValue` | **repeated calls on the same list** — the C function takes `head` by value and must not mutate the caller's chain; call both `.so`s 3x on one list and also verify every node's `value`/`next` is unchanged afterwards | 500 | [x] |
| C24 | `smallestValue` | **ABI/layout**: `sizeof(struct ListNode)`, `offsetof(value)`, `offsetof(next)` from a compiled C probe vs Rust `size_of`/`offset_of!` | 3 | [x] |

Rows C1 and C3 overlap with `ERRORS.md` rows E1/B3-B6 by construction; they are
kept in both tables so each gate is independently satisfied.

## Feature-combination coverage

`translation/Cargo.toml` declares no `[features]`, so the set of feature
combinations is the single default one. `scripts/check_all_features.sh`
enumerates features from `Cargo.toml` and loops over the powerset; with zero
features it runs the default build/test once, which is the complete combination
space. Verified: `--no-default-features` and default produce identical results.

## Gate status

- [x] Every row above passes across its randomized inputs, under every feature
      combination (the single default). Phase B complete.
