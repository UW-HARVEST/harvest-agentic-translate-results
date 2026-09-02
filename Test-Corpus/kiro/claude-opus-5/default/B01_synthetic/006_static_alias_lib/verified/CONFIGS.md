# CONFIGS.md — configuration surface table (Phase B gate)

## Axes derived mechanically from the C source

Public entry points (`include/staticalias.h`) — both are covered, the low-level
one **directly**, not only through the wrapper:

* `int *static_alias(int *outer)` — the low-level primitive; owns the
  function-local `static int inner = 1;` and the only branch in the library.
* `void driver(int initial_value, int iterations)` — the convenience wrapper; a
  loop that chains `static_alias`'s *return pointer* back into its own argument
  and prints `"%d\n"` through libc `printf` each iteration.

Runtime options / modes / flags: **none**. There is no options struct, no flag
argument, no global setter, no environment lookup and no `#ifdef` outside the
header's include guard. The library's entire "configuration" is therefore
(a) the *hidden persistent state* `inner`, (b) the *value class* of the `int`
arguments, and (c) the *call sequence / aliasing shape*. The axes the C code
actually branches on:

| axis | values the C distinguishes |
|------|----------------------------|
| A. branch in `static_alias` | `*outer >= inner` (then: mutate `inner`, return `&inner`) vs `*outer < inner` (else: mutate `*outer`, return `outer`) |
| B. pointer identity of the argument | fresh caller-owned `int` vs the previously returned `&inner` (self-aliasing) vs the previously returned `outer` (chained caller object) |
| C. value class of `*outer` / `initial_value` | `INT_MIN`, negative, `0`, `1`, small positive, `inner-1`, `inner`, `inner+1`, large positive, `INT_MAX` |
| D. state of `inner` | initial (`1`), grown positive, wrapped negative, back near boundary |
| E. `iterations` count shape | `INT_MIN`/negative, `0` (empty), `1` (one), `2`, many, oversized (`100_000`) |
| F. sequence length for direct `static_alias` driving | 1 call, 2 calls, long randomized chains |
| G. observable channel | returned pointer identity, `*returned`, caller object after the call, hidden `inner` (probed non-destructively with `INT_MIN`), and `stdout` bytes for `driver` |

Pruned cross-product — one row per combination the C treats differently:

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `static_alias` | A=then, B=fresh, C=`*outer == inner` exactly, D=initial | `cfg_01_then_equal_fresh` | [x] |
| 2 | `static_alias` | A=then, B=fresh, C=`*outer > inner` (randomized small/large positives) | `cfg_02_then_greater_fresh_random` | [x] |
| 3 | `static_alias` | A=else, B=fresh, C=`*outer == inner - 1` (boundary−1) | `cfg_03_else_boundary_minus_one` | [x] |
| 4 | `static_alias` | A=else, B=fresh, C=negative / `0` / `INT_MIN` (randomized below-`inner` values) | `cfg_04_else_below_random` | [x] |
| 5 | `static_alias` | A=then, B=**self-aliased** (`outer == &inner`, obtained by feeding the previous return value back) — the doubling path | `cfg_05_self_alias_chain` | [x] |
| 6 | `static_alias` | B=**chained caller object** (else-branch return fed back in: `outer` unchanged identity, value now `old + inner`, which is `>= inner`, so the next call flips to then) | `cfg_06_chained_outer_flip` | [x] |
| 7 | `static_alias` | F=long randomized chain (500 calls) always re-feeding the returned pointer — walks A/B/C/D through every state the wrapper can reach, incl. wraparound | `cfg_07_long_chain_random` | [x] |
| 8 | `static_alias` | D=`inner` wrapped **negative** (forced with `INT_MAX`), then randomized values on both sides of the now-negative boundary | `cfg_08_negative_inner_states` | [x] |
| 9 | `static_alias` | independent-object interleaving: several distinct caller `int`s used round-robin so `inner` advances between touches of each object | `cfg_09_multi_object_interleave` | [x] |
| 10 | `driver` | E=`0` iterations (empty) — no output, no state change | `cfg_10_driver_zero` | [x] |
| 11 | `driver` | E=`1` iteration (one), C randomized over all value classes | `cfg_11_driver_single_random` | [x] |
| 12 | `driver` | E=`2` iterations (many-boundary: first call decides then/else, second sees the mutated state) | `cfg_12_driver_two_random` | [x] |
| 13 | `driver` | E=many (3…64 randomized), C randomized over all value classes — full `%d\n` stdout stream compared byte for byte | `cfg_13_driver_many_random` | [x] |
| 14 | `driver` | E=many with C forced `< inner` so the *else* path dominates the first iterations, then flips | `cfg_14_driver_else_dominant` | [x] |
| 15 | `driver` | E=oversized (`100_000`) — long stream, `inner` overflows and wraps mid-stream | `cfg_15_driver_oversized` | [x] |
| 16 | `driver` + `static_alias` | **mixed composed pipeline**: alternate direct low-level calls and wrapper calls so the wrapper observes state left by direct calls and vice versa (the composed-pipeline bug class) | `cfg_16_mixed_pipeline_random` | [x] |
| 17 | `driver` | D=`inner` pre-driven to a wrapped-negative value by direct `static_alias` calls, then `driver` run with randomized args | `cfg_17_driver_after_negative_inner` | [x] |
| 18 | `static_alias` | G=hidden-state parity: non-destructive `INT_MIN` probe of `inner` after every one of the above rows, asserting C and Rust hold the *same* private static value | `cfg_18_hidden_inner_parity` (and the probe assertion embedded in every row) | [x] |

All 18 rows are checked off; each row uses many randomized inputs from a fixed
seed (`SEED = 0x5A11_A11A_5EED_0001`, xorshift64\*) so runs are reproducible.
See `tests/differential.rs`.

## Feature combinations

`Cargo.toml` has no `[features]` table, so the default (empty) feature set is the
only configuration; `tests/all_features.sh` enumerates it from `Cargo.toml` and
re-runs the whole suite for each combination found (default and
`--no-default-features`).
