# Verification report — C → Rust translation of `div_euclid`

Ground truth: `c_src/` (never modified). Subject: `translation/src/lib.rs`.
Every comparison is made through `dlopen`/`dlsym` on **both** shared objects; the
Rust function is never called directly, so the `#[no_mangle] extern "C"` export
and the C ABI are part of what is tested.

* C `.so`   : `c_src/build/libharvest-work-58LIFt.so`
* Rust `.so`: `translation/target/{release,debug}/libdiv_euclid_lib.so`

Build note: this environment has no crates.io egress, so all cargo commands use
`--offline` (`libloading 0.8.9` was already in the local registry cache).

## Outcome

**No divergence was found. The Rust translation is byte-identical to the C for
every input tested, and the heavy sweeps are exhaustive in one operand.
No fix to `translation/src/lib.rs` was required.**

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` on both objects. C exports exactly one symbol, `div_euclid`; Rust exports the same one. Symbol diff empty in both directions. No untranslated C module (the whole C library is `src/lib.c`, 32 lines, one function). No stubs in the Rust. |
| `ERRORS.md` | 15 rows, derived by grepping every `return` / `if` / sentinel constant in `lib.c`. The only *explicit* rejection is `v2 == 0 → return 0`; the other rows are the four `INT_MIN` guards (lines 11, 15, 18, 24), the two re-associated `INT_MIN` paths (lines 23, 25), and the sign-dependent epilogue at line 31. No pointers / lengths / enums exist in the signature, which is recorded as a derived fact, not an omission. |
| `CONFIGS.md` | 31 rows = the 10 mutually exclusive C paths (P0–P9) × epilogue branch × input shape, plus dense/exhaustive sweeps. |

## Phase B — valid-path differential tests

`tests/configs.rs` (27 tests, rows C1–C26) and `tests/exhaustive.rs` (5 tests,
rows C27–C31). Fixed seed `0x5EED1234`, SplitMix64; `DIFF_SAMPLES` (default
20 000) randomized inputs per row.

```
Running tests/configs.rs     -> 27 passed; 0 failed
Running tests/exhaustive.rs  ->  5 passed; 0 failed
```

Heaviest evidence (full-strength run, 287 s wall):

| row | coverage | divergences |
|-----|----------|-------------|
| C27 | exhaustive `v1, v2 ∈ [-300, 300]` = 361 201 pairs | 0 |
| C28 | **every** `i32` value of `v2` × 6 pinned `v1` = 25.8 G pairs | 0 |
| C29 | **every** `i32` value of `v1` × 10 pinned `v2` = 42.9 G pairs | 0 |
| C30 | 40 M-point coprime trajectory + coarse 2-D grid | 0 |
| C31 | 5 M uniform + 1 M boundary-biased random pairs | 0 |

Total for C28+C29 alone: **68 719 476 912 input pairs**, all matching.

**Binary / driver:** not applicable. `c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)` and no executable; `Cargo.toml` has only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`. There is
no stdout to compare.

## Phase C — error-path differential tests

`tests/differential.rs`, 20 tests: one per `ERRORS.md` row (`e1_…` … `e15_…`)
plus `generic_boundaries`, `out_of_range_integer_arguments`,
`no_pointer_arguments_in_api`, `shared_objects_are_distinct_files`, and
`independent_oracle_agrees_with_both`.

```
Running tests/differential.rs -> 20 passed; 0 failed
```

Every row asserts the *same exact* value from both sides, and where the C pins a
sentinel the test asserts that sentinel too (`check_eq`), not merely "both
failed":

| row | trigger | agreed result |
|-----|---------|---------------|
| E1–E4 | `v2 == 0` (incl. `v1 = INT_MIN`/`INT_MAX`, 200 k random `v1`) | `0` |
| E5 | `v1 >= 0`, `v2 == INT_MIN` (line 11 guard) | `0` |
| E6 | `v1 == INT_MIN` vs every kind of `v2` (line 15 guard) | agree |
| E7 | `v1 < 0` (`≠ INT_MIN`), `v2 == INT_MIN` (line 18 guard) | `1` |
| E8 | both `INT_MIN` (line 24 guard) | `1` |
| E9 | `v1 == INT_MIN`, `v2 > 0` (line 23 re-association) | agree |
| E10 | `v1 == INT_MIN`, `v2 < 0 ≠ INT_MIN` (line 25 re-association) | agree |
| E11 | `div_euclid(INT_MIN, -1)` — the classic trapping input | `INT_MIN`, no trap |
| E12 | `div_euclid(INT_MIN, 1)` | `INT_MIN`, no trap |
| E13 | negative `r`, `v2 > 0` → `q - 1` | agree + floor-division oracle |
| E14 | negative `r`, `v2 < 0` → `q + 1` | agree + oracle |
| E15 | `q ± 1` overflow at line 31 (13×13 extreme grid + `v2 = ±1` sweeps) | agree, wrapping |

Generic boundaries also covered: full 9×9 extreme grid
`{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}`; ±64 neighbourhoods
of `INT_MIN`, `-1`, `0`, `1`, `INT_MAX` against every extreme; `u32`-reinterpreted
patterns (`0x80000000`, `0xffffffff`, `0xdeadbeef`, …). Null-pointer and
length tests are inapplicable and `no_pointer_arguments_in_api` asserts the
header still has no `*` so the omission cannot silently become wrong.

## Phase D — symbol parity, feature combos, completion gate

`tests/feature_sweep.sh` enumerates features mechanically from `Cargo.toml`,
rebuilds the C `.so`, and for each combination × each Rust profile
(`release`, `debug`) diffs `nm -D` and runs all three test binaries against that
exact `.so` via `RUST_SO`.

`Cargo.toml` declares **no `[features]`**, so the complete combination set is
`{default}` = `{--no-default-features}`. Both are run, against both profiles:

```
############ features: <default> ############            (release .so) symbol diff EMPTY; 27 + 20 + 5 passed
############ features: <default> ############            (debug   .so) symbol diff EMPTY; 27 + 20 + 5 passed
############ features: --no-default-features ############ (release .so) symbol diff EMPTY; 27 + 20 + 5 passed
############ features: --no-default-features ############ (debug   .so) symbol diff EMPTY; 27 + 20 + 5 passed
===== ALL FEATURE COMBINATIONS PASSED =====
```

12 test-binary runs, 12 × `test result: ok`, 0 failures.

## Negative control (proof the suite is not vacuous)

`tests/mutation_check.sh`:

```
PASS  mutant 1  (6 tests caught it)   -- P8 quotient adjustment +1 -> +2
PASS  mutant 2  (12 tests caught it)  -- epilogue sign adjustment flipped
PASS  mutant 3  (4 tests caught it)   -- P3 q = 0 -> q = 1
PASS  mutant 4  (4 tests caught it)   -- v2 == 0 sentinel 0 -> -1
PASS  mutant 5  (4 tests caught it)   -- P6 drops the q*v2 term from r
PASS  mutant 6  (undetected, as expected: semantically equivalent)
===== NEGATIVE CONTROL PASSED: suite is not vacuous =====
```

Mutant 6 replaces the whole hand-rolled algorithm with `i32::wrapping_div_euclid`
and is *correctly* undetected: the C function computes exactly wrapping Euclidean
division, returning `0` for `v2 == 0`. That equivalence is itself asserted as a
third independent oracle over 23.7 M pairs, so the suite pins the behaviour from
two directions rather than only comparing two possibly-co-buggy implementations.

## Reproducing

```bash
# 1. C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + full-strength suite (~290 s, exhaustive C28/C29)
cd translation && cargo build --offline --release \
  && cargo test --offline --release -- --nocapture

# 3. quick run (strided sweeps, a few seconds)
EXHAUSTIVE_STRIDE=65521 cargo test --offline --release

# 4. all feature combinations x both profiles, with nm -D parity checks
bash tests/feature_sweep.sh

# 5. negative control
bash tests/mutation_check.sh
```

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust; symbol diff empty both directions.
- [x] Phase B: every one of the 31 `CONFIGS.md` rows passes across randomized inputs (C28/C29 exhaustive in one operand — 68.7 G pairs, 0 divergences).
- [x] Binary stdout comparison: **N/A**, no executable target in either build.
- [x] Phase C: all 15 `ERRORS.md` rows have a passing differential test asserting the same exact value, plus null/length/range/enum-class generic boundaries.
- [x] All of the above hold under every feature combination (`{default}`, `{--no-default-features}`) and both Rust profiles (`release`, `debug`).
- [x] Suite verified non-vacuous by mutation testing.
