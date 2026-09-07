# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

Full public API (`c_src/include/driver.h`, digraphs resolved):

```c
#ifndef DRIVER_H_
#define DRIVER_H_
void driver(int x, int y);
#endif
```

* **Public entry points:** exactly one — `driver`. It is simultaneously the
  lowest-level and the highest-level entry point; there is no convenience
  wrapper and no lower layer to reach past. `nm -D` confirms `driver` is the
  only exported symbol (see `SYMBOLS.md`).
* **Runtime options / modes / flags:** none. Greps for `if`, `switch`,
  `#ifdef`, and for any global/static state in `c_src/` return 0 matches; the
  function has no configuration parameters, no setters, and no persistent
  state.
* **Compile-time configuration:** none in `CMakeLists.txt` beyond
  `add_library(driver SHARED src/driver.c)`; no `option()`, no
  `target_compile_definitions`. `translation/Cargo.toml` has no `[features]`
  section, so the Rust side has one build configuration only.

The remaining axes are therefore the **input data shapes**, which is where all
behaviour variation lives. `int result = x | ~y;` then `printf("%d", result)`
is data-dependent in these ways the code/libc distinguish:

* **A. sign of `result`** — controls whether `%d` emits a `-`.
* **B. decimal width of `result`** — 1..10 digits (11 bytes with the sign),
  controls the emitted byte count.
* **C. sign bit of each operand** (`x` and `y` independently) — `~y` moves the
  sign bit, and `|` with a negative operand forces `result` negative.
* **D. extreme values** `INT_MIN` / `INT_MAX` / `0` / `-1` — `-1` is the
  all-ones identity for `|`, `0` is the identity, `INT_MIN` is the only
  magnitude with no positive `int` counterpart.
* **E. algebraic relationships between the operands** — `y == x` (forces
  `result == -1`), `y == ~x` (forces `result == x`), `y == -1` (forces
  `result == x`), `x == 0` (forces `result == ~y`), disjoint vs overlapping
  bit sets.
* **F. bit position** — single-bit operands across all 32 positions, incl. the
  sign bit.
* **G. call multiplicity** — one call vs many back-to-back calls in one
  process, since output goes through the shared `stdout` `FILE` buffer and
  `printf`/`puts` are split across two calls (a per-call comparison would miss
  a wrong flush or a swapped ordering of the number and the newline).

Every row below is a combination of these axes that the C treats differently
(cross-product pruned to distinguishable cases). Rows marked *(random)* are
property-style: many randomized inputs per row from a fixed-seed xorshift PRNG,
implemented in `tests/differential.rs`.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x = 0, y = 0` — both identities, result `-1` (all ones, negative, 2 bytes) | [x] |
| 2 | `driver` | `x = 0, y = -1` — the unique pair giving `result == 0` (1 digit, no sign) | [x] |
| 3 | `driver` | `x = -1, y` arbitrary *(random)* — `x` all-ones saturates the `\|`, result always `-1` | [x] |
| 4 | `driver` | `y = -1`, `x` arbitrary *(random)* — `~y == 0`, so `result == x`: exercises the full `x` range incl. both signs | [x] |
| 5 | `driver` | `x = 0`, `y` arbitrary *(random)* — `result == ~y`: exercises the complement alone over the full range | [x] |
| 6 | `driver` | `y = x`, both arbitrary *(random)* — `x \| ~x == -1` for every `x` | [x] |
| 7 | `driver` | `y = !x` (bitwise complement of `x`), `x` arbitrary *(random)* — `x \| x == x` | [x] |
| 8 | `driver` | result forced positive: `x ≥ 0` and `y < 0` *(random)* — sign bit clear in both `x` and `~y`, so `%d` emits no `-` | [x] |
| 9 | `driver` | result forced negative: `y ≥ 0` *(random)* — `~y` has the sign bit set, so result is always negative | [x] |
| 10 | `driver` | both operands non-negative: `x, y ∈ [0, INT_MAX]` *(random)* | [x] |
| 11 | `driver` | both operands negative: `x, y ∈ [INT_MIN, -1]` *(random)* | [x] |
| 12 | `driver` | mixed signs: `x < 0 ≤ y` *(random)* | [x] |
| 13 | `driver` | mixed signs, other way: `y < 0 ≤ x` *(random)* | [x] |
| 14 | `driver` | unconstrained full range: `x, y ∈ [INT_MIN, INT_MAX]` *(random, 4000 pairs)* | [x] |
| 15 | `driver` | `result == INT_MAX` (`x = INT_MAX, y = -1`) — widest positive, 10 digits | [x] |
| 16 | `driver` | `result == INT_MIN` (`x = 0, y = INT_MAX`) — widest negative, 11 bytes, magnitude unrepresentable as positive `int` | [x] |
| 17 | `driver` | extreme-value cross-product: `x, y ∈ {INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` — all 81 combinations | [x] |
| 18 | `driver` | single-bit operand sweep: `x = 1<<i`, `y = 1<<j`, all `i, j ∈ 0..32` (1024 pairs, sign bit included) — axis F | [x] |
| 19 | `driver` | single-bit-complement sweep: `x = !(1<<i)`, `y = !(1<<j)`, all `i, j ∈ 0..32` — one clear bit in a field of ones | [x] |
| 20 | `driver` | disjoint bit sets: `x` and `y` chosen so `x & ~y == 0` *(random)* — `\|` contributes nothing from `x` | [x] |
| 21 | `driver` | decimal-width sweep, positive results: `result` with exactly 1,2,…,10 digits (`x = 10^k-ish`, `y = -1`) — axis B | [x] |
| 22 | `driver` | decimal-width sweep, negative results: `result` with 1..10 digits plus the `-` sign | [x] |
| 23 | `driver` | small-magnitude dense sweep: every `x, y ∈ [-40, 40]` (6561 pairs) — dense coverage around zero | [x] |
| 24 | `driver` | many back-to-back calls in one process, outputs accumulated without an intervening flush: 512 randomized pairs replayed as a single output stream — axis G (catches wrong flush points, swapped number/newline order, missing or extra newline) | [x] |
| 25 | `driver` | one call in a freshly loaded library (first-use `stdout` buffer initialisation), `x, y` randomized — checks nothing is emitted at load time and no `printf` state is assumed | [x] |
| 26 | `driver` | stdout redirected to a pipe vs to a regular file (glibc chooses line- vs full-buffering from the fd type) for the same input — verifies buffering mode cannot change the emitted bytes | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(driver SHARED …)` and no
`add_executable`, so the project builds **no driver binary**; there is no
stdout-of-two-programs comparison to run. `translation/Cargo.toml` likewise
declares only `[lib] crate-type = ["cdylib"]` and has no `src/main.rs`
(verified: `ls translation/src` → `lib.rs` only). The stdout comparison is
instead performed at the FFI level in rows 24–26, where the captured stdout of
each library is compared byte-for-byte.
