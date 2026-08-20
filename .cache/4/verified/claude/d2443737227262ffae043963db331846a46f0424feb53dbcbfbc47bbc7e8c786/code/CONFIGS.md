# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE (valid inputs)

## Build-time configuration

`c_src/CMakeLists.txt` has **no** options, no `target_compile_definitions`, no
`option()`; `c_src/src/lib.c` contains **no** `#ifdef`. `Cargo.toml` therefore
declares no features. The complete set of valid feature combinations is:

| # | feature combination | cargo invocation |
|---|--------------------|------------------|
| F1 | *(empty)* = `default` | `cargo test --no-default-features` |
| F2 | `default` (also empty) | `cargo test` / `cargo test --features default` |

Both are the same code; both are run by `./run_verification.sh`.

## Runtime configuration axes (derived from the C branches)

`div_euclid` has no options/flags/modes struct — its *configuration* is the
shape of its two `int` operands. The axes the C code actually branches on:

* **A1 — zero divisor**: `v2 == 0` (`lib.c:4`).
* **A2 — sign of `v1`**: `v1 >= 0` vs `v1 < 0` (`lib.c:8`).
* **A3 — `v1 == INT_MIN`**: `v1 != (-0x7fffffff-1)` (`lib.c:15`).
* **A4 — sign of `v2`**: `v2 >= 0` vs `v2 < 0` (`lib.c:9`, `:16`, `:22`).
* **A5 — `v2 == INT_MIN`**: `v2 != (-0x7fffffff-1)` (`lib.c:11`, `:18`, `:24`).
* **A6 — sign of the computed remainder**: `r >= 0` vs `r < 0` (`lib.c:28`),
  i.e. *divisibility* of the operands.
* **A7 — sign of `v2` in the tail adjust**: `v2 > 0 ? -1 : 1` (`lib.c:31`).
* **A8 — magnitude relation** `|v1|` vs `|v2|` (value-dependent: decides
  `q == 0` vs `q != 0`, and whether the tail adjust turns `0` into `±1`).
* **A9 — early return**: only the `v1>=0 && v2>0` leaf returns before the
  `r`-check; every other leaf goes through the tail adjust.

Cross product of A1–A5 gives the 10 reachable leaf paths of the function:

| path | condition | body |
|------|-----------|------|
| P1 | `v2 == 0` | `return 0` |
| P2 | `v1>=0`, `v2>0` | `return v1/v2` (early return) |
| P3 | `v1>=0`, `v2<0`, `v2!=INT_MIN` | `q=-(v1/-v2)`, `r=v1%-v2` |
| P4 | `v1>=0`, `v2==INT_MIN` | `q=0`, `r=v1` |
| P5 | `v1<0`, `v1!=INT_MIN`, `v2>0` | `q=-((-v1)/v2)`, `r=-((-v1)%v2)` |
| P6 | `v1<0`, `v1!=INT_MIN`, `v2<0`, `v2!=INT_MIN` | `q=(-v1)/(-v2)`, `r=-((-v1)%(-v2))` |
| P7 | `v1<0`, `v1!=INT_MIN`, `v2==INT_MIN` | `q=1`, `r=v1-q*v2` |
| P8 | `v1==INT_MIN`, `v2>0` | `q=-((-(v1+v2))/v2)-1`, `r=-((-(v1+v2))%v2)` |
| P9 | `v1==INT_MIN`, `v2<0`, `v2!=INT_MIN` | `q=((-(v1-v2))/(-v2))+1`, `r=-((-(v1-v2))%(-v2))` |
| P10 | `v1==INT_MIN`, `v2==INT_MIN` | `q=1`, `r=0` |

## Public entry points

`div_euclid` is the *only* public entry point (no convenience wrapper, no
lower-level helper is exported: `cdiv`/`crem` are private in the Rust side and
are `/` and `%` in C). Every row below drives it through the `.so` export.
Rows C34–C38 additionally drive it through the raw ABI (`extern "C" fn(i64,
i64) -> i64`) and through dense/exhaustive sweeps, which is the "lowest level"
the surface admits.

## Table — one row per meaningful combination (all via `.so` exports)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `div_euclid` | P1: `v2 == 0`, `v1` uniformly random over all `int` | [x] |
| C2 | `div_euclid` | P2: `v1 > 0`, `v2 > 0`, non-divisible (`r != 0`), random | [x] |
| C3 | `div_euclid` | P2: `v1 > 0`, `v2 > 0`, exact multiple (`r == 0`), random | [x] |
| C4 | `div_euclid` | P2: `0 <= v1 < v2` (quotient 0), random | [x] |
| C5 | `div_euclid` | P2: `v1 == 0`, `v2 > 0` random | [x] |
| C6 | `div_euclid` | P2: extremes `v1 == INT_MAX` × `v2 ∈ {1,2,3,INT_MAX-1,INT_MAX}`, and `v2 == 1` × random `v1 >= 0` | [x] |
| C7 | `div_euclid` | P3: `v1 > 0`, `INT_MIN < v2 < 0`, non-divisible, random | [x] |
| C8 | `div_euclid` | P3: `v1 > 0`, `v2 < 0`, exact multiple (`r == 0`), random | [x] |
| C9 | `div_euclid` | P3: `v1 >= 0`, `v2 == -1` (r always 0), random `v1` | [x] |
| C10 | `div_euclid` | P3: `v1 == 0`, `INT_MIN < v2 < 0` random | [x] |
| C11 | `div_euclid` | P3: `0 <= v1 < -v2` (quotient 0, `r >= 0` ⇒ no tail adjust), random | [x] |
| C12 | `div_euclid` | P4: `v1 > 0` random, `v2 == INT_MIN` | [x] |
| C13 | `div_euclid` | P4: `v1 == 0`, `v2 == INT_MIN` | [x] |
| C14 | `div_euclid` | P4: `v1 == INT_MAX`, `v2 == INT_MIN` | [x] |
| C15 | `div_euclid` | P5: `INT_MIN < v1 < 0`, `v2 > 0`, non-divisible (`r < 0` ⇒ tail `q-1`), random | [x] |
| C16 | `div_euclid` | P5: `v1 < 0`, `v2 > 0`, exact multiple (`r == 0` ⇒ no adjust), random | [x] |
| C17 | `div_euclid` | P5: `v1 < 0` random, `v2 == 1` | [x] |
| C18 | `div_euclid` | P5: `-v1 < v2` (`q == 0`, `r < 0` ⇒ returns `-1`), random | [x] |
| C19 | `div_euclid` | P6: `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, non-divisible (`r < 0` ⇒ tail `q+1`), random | [x] |
| C20 | `div_euclid` | P6: `v1 < 0`, `v2 < 0`, exact multiple, random | [x] |
| C21 | `div_euclid` | P6: `INT_MIN < v1 < 0` random, `v2 == -1` (incl. `v1 == INT_MIN+1` ⇒ `INT_MAX`) | [x] |
| C22 | `div_euclid` | P6: `-v1 < -v2` (`q == 0`, `r < 0` ⇒ returns `+1`), random | [x] |
| C23 | `div_euclid` | P7: `INT_MIN < v1 < 0` random, `v2 == INT_MIN` | [x] |
| C24 | `div_euclid` | P7: `v1 ∈ {-1, -2, INT_MIN+1, INT_MIN+2}`, `v2 == INT_MIN` (boundaries of `r = v1 - q*v2`) | [x] |
| C25 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 > 0` random, non-divisible | [x] |
| C26 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 == 2^k` for every `k ∈ 0..31` (exact multiples, `r == 0`) | [x] |
| C27 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 == 1` (quotient `INT_MIN`, boundary of representability) | [x] |
| C28 | `div_euclid` | P8: `v1 == INT_MIN`, `v2 ∈ {INT_MAX, INT_MAX-1, 2^30, 2^30+1}` | [x] |
| C29 | `div_euclid` | P9: `v1 == INT_MIN`, `INT_MIN < v2 < 0` random, non-divisible | [x] |
| C30 | `div_euclid` | P9: `v1 == INT_MIN`, `v2 == -2^k` for every `k ∈ 0..30` (`r == 0`) | [x] |
| C31 | `div_euclid` | P9: `v1 == INT_MIN`, `v2 == -1` (C signed-overflow wrap in `q+1`) | [x] |
| C32 | `div_euclid` | P9: `v1 == INT_MIN`, `v2 ∈ {INT_MIN+1, INT_MIN+2, -2^30, -(2^30+1)}` | [x] |
| C33 | `div_euclid` | P10: `v1 == INT_MIN`, `v2 == INT_MIN` | [x] |
| C34 | `div_euclid` | Full cross product of the curated boundary value set (`|S| ≈ 260` ⇒ ≈ 67 600 pairs) — every (A2..A9) combination the set can express | [x] |
| C35 | `div_euclid` | Dense contiguous sweeps: `v1 ∈ [INT_MIN, INT_MIN+512] ∪ [-512,512] ∪ [INT_MAX-512, INT_MAX]` × `v2 ∈ {±1,±2,±3,±7,±INT_MAX,INT_MIN}` | [x] |
| C36 | `div_euclid` | Uniform random `int` × `int` (2 000 000 pairs, fixed seed) — unconstrained shape | [x] |
| C37 | `div_euclid` | Structured random: `v2 ∈ {±2^k, ±(2^k±1), INT_MIN, INT_MAX}` × random `v1`, plus `v1 = m*v2 (+δ)` constructed multiples | [x] |
| C38 | `div_euclid` (raw ABI) | Same symbol invoked through `extern "C" fn(i64,i64)->i64` with dirty upper 32 bits in both argument registers; low 32 bits of both returns compared | [x] |
| C39 | `div_euclid` | Whole-`i32`-range deterministic stride sweep of the dividend (stride 65 537, ≈65 534 dividends spanning `INT_MIN..=INT_MAX`) × 22 divisors incl. `0`, `±1`, `±2^k`, `INT_MIN`, `±INT_MAX` | [x] |
| C40 | `div_euclid` | Reference-build axis: the same C source rebuilt as `gcc -O0/-O1/-O2/-O3`, `gcc -O2 -fwrapv`, `gcc -O2 -fno-strict-overflow`, `clang -O2` — Rust must agree with **every** build (the C has one UB signed overflow, so this pins the behaviour down independently of the C build configuration) | [x] |

Rows C1–C38 are `#[test]`s in `tests/phase_b_configs.rs`; rows C39–C40 are in
`tests/phase_e_robustness.rs`. Each row uses many randomized inputs from a
fixed-seed PRNG (`SplitMix64`) plus its boundary values, and asserts C and Rust
results are byte-identical.

## Harness validation (mutation testing)

A differential harness that cannot fail is worthless, so the harness was
mutation-tested: 15 deliberate bugs were injected into `src/lib.rs`, one at a
time, and the suite was re-run (`$TMPDIR/mutate*.sh`, source restored
afterwards).

| mutant | injected bug | result |
|--------|--------------|--------|
| M1 | P3 negates `r` | CAUGHT (7 tests) |
| M2 | P9 `saturating_add(1)` instead of `wrapping_add(1)` | CAUGHT (6) |
| M3 | P4 uses `-v1` as `r` | CAUGHT (6) |
| M4 | `v2 == 0` returns `1` | CAUGHT (4) |
| M5 | P5 forgets to negate `r` | CAUGHT (7) |
| M6 | P8 drops the `- 1` | CAUGHT (8) |
| M9 | P7 `r = v1` (drops `- q*v2`) | CAUGHT (6) |
| M10 | P6 swaps dividend/divisor | CAUGHT (9) |
| M11 | single boundary point `(INT_MIN+3, -3)` returns `42` | CAUGHT (3) |
| M12 | tail uses `r > 0` instead of `r >= 0` | CAUGHT (19) |
| M14 | P10 `r = -1` instead of `0` | CAUGHT (5) |
| M16 | P2 early return `+ 1` | CAUGHT (10) |
| M7 | tail `v2 >= 0` instead of `v2 > 0` | not caught — **equivalent** mutant: `v2 != 0` is guaranteed at `lib.c:28` by the early return at `lib.c:4` |
| M8 | `cdiv` uses `div_euclid` instead of truncating `/` | not caught — **equivalent** mutant, see invariant below |
| M15 | `crem` uses `wrapping_rem_euclid` | not caught — **equivalent** mutant, see invariant below |

The equivalence of M8/M15 was *proved empirically*, not assumed: `cdiv`/`crem`
were temporarily instrumented with `assert!(a >= 0 && b > 0)` and the entire
corpus (~3 M inputs, all boundary cross products) ran without a single
violation. With a non-negative dividend and a positive divisor, truncating and
Euclidean division/remainder coincide, so those mutants cannot change any
result. 12/12 non-equivalent mutants were caught.
