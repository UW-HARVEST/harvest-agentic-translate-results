# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis enumeration (what the C actually branches on)

**Runtime options / modes / flags:** there are **none**. The library exposes no
setter, no global, no context struct, no bitmask and no `#ifdef` (`grep -c
'#if\|#ifdef\|#ifndef' c_src/src/lib.c` → 0; the only preprocessor directives are
three `#include`s and the `STRINGIZE`/`TO_STRING` macro pair). So the only
configuration axes are **input shape** and **entry point**.

**Full set of public entry points (lowest-level included, not just the header):**

| entry point | declared in | level |
|---|---|---|
| `cleanup(int,int,int,int)` | `include/lib.h` **and** `lib.c:31` | composed operation (validate → accumulate → allocate → format → release) |
| `print_result(const char*,int)` | `lib.c:32` only (not in the public header, but exported `T`) | low-level output primitive |
| `cleanup_resources(char*)` | `lib.c:33` only (not in the public header, but exported `T`) | low-level release primitive |

`cleanup` is the convenience/one-shot wrapper; `print_result` and
`cleanup_resources` are the low-level entry points and are driven **directly**
below, not only through `cleanup`.

**Input shapes the code special-cases:**

* `cleanup`, per argument: the `switch` at `lib.c:48` distinguishes 5 classes —
  `10` (falls through into `20`, net `+30`), `20` (`+20`), `30` (falls through
  into `40`, net `+70`), `40` (`+40`), and `default` (`+value`). The loop
  `for (i = 0; i < 4; i++)` applies this to all four positions independently, so
  the true shape space is the **cross product `5^4 = 625`**, and *position*
  matters for reproducing the accumulation order.
* `default`-class sub-shapes that matter for value-dependent behaviour: `0`,
  small positive, small negative, off-by-one neighbours of the labels,
  `INT_MAX`, `INT_MIN`, and combinations that wrap the accumulator.
* `print_result`: label byte-shape (empty / short ASCII / long / high-byte /
  contains `%` / null pointer) × `result` magnitude & sign.
* `cleanup_resources`: pointer shape (null / live `malloc` result).
* Observable channels per call: the **return value** *and* the **bytes written to
  stdout** (`printf`/`snprintf` at `lib.c:43,67,71,72,80`). Both are compared.

There is no size/width/element-type/count/format/byte-order axis: every parameter
is a fixed-width `int` or `char*`, and the buffer size is the hard-coded `50`.

## Table

All rows call **both** `.so` files through `libloading` and compare the return
value **and** the captured stdout bytes. Rows marked *randomised* use a fixed
seed (`0x5EED_C0DE_1234_5678`, SplitMix64) so failures reproduce.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `cleanup` | all four args in `default` class, randomised in `[-1000, 1000]` (2 000 cases) | [x] |
| 2 | `cleanup` | **exhaustive cross product** of the 5 switch classes over all 4 positions (`5^4 = 625` rows), `default` slots filled with randomised values | [x] |
| 3 | `cleanup` | exactly one arg `= 10` (fall-through `10→20`), each of the 4 positions, others `default` | [x] |
| 4 | `cleanup` | exactly one arg `= 20`, each of the 4 positions | [x] |
| 5 | `cleanup` | exactly one arg `= 30` (fall-through `30→40`), each of the 4 positions | [x] |
| 6 | `cleanup` | exactly one arg `= 40`, each of the 4 positions | [x] |
| 7 | `cleanup` | all four args `= 10` (maximum fall-through, `4 × 30`) | [x] |
| 8 | `cleanup` | all four args `= 30` (`4 × 70`); also all `20`, all `40` | [x] |
| 9 | `cleanup` | `INT_MAX` in each position (accumulator overflow / wrap) | [x] |
| 10 | `cleanup` | `INT_MIN` in each position (accumulator underflow / wrap) | [x] |
| 11 | `cleanup` | all args `= 0` (the "empty" shape) | [x] |
| 12 | `cleanup` | all-negative args, randomised in `[INT_MIN, -1]` | [x] |
| 13 | `cleanup` | off-by-one neighbours of every label: `{9,11,19,21,29,31,39,41}` in all 4 positions (`8^4 = 4096`) | [x] |
| 14 | `cleanup` | *randomised* over the **full** `i32` range, 20 000 cases (hits `default` with wrapping sums; also the out-of-range-"variant" class G4) | [x] |
| 15 | `cleanup` | *randomised* biased sampler: each arg drawn from `{10,20,30,40} ∪ full-range i32` with 50 % label probability, 20 000 cases (mixes fall-through and `default` in one call) | [x] |
| 16 | `cleanup` | stdout side-effect equality for a representative spread (the `Processed numbers: numbers` line produced by `TO_STRING(numbers)`, plus absence of both diagnostics) | [x] |
| 17 | `print_result` (low-level, direct) | short ASCII label × `result ∈ {0, 1, -1, 42, INT_MAX, INT_MIN}` and randomised `result` (2 000 cases) | [x] |
| 18 | `print_result` | empty label (`""`) × randomised `result` | [x] |
| 19 | `print_result` | 64 KiB label (oversized) × randomised `result` | [x] |
| 20 | `print_result` | label containing `%d %s %n %%` (label is an argument, not the format) | [x] |
| 21 | `print_result` | label of non-UTF-8 high bytes `0x80..0xFF` | [x] |
| 22 | `print_result` | `NULL` label (glibc prints `(null)`) | [x] |
| 23 | `cleanup_resources` (low-level, direct) | `NULL` pointer — null-guard no-op | [x] |
| 24 | `cleanup_resources` | live `malloc(n)` pointer, randomised `n ∈ [1, 4096]`, 2 000 cases — pointer is freed | [x] |
| 25 | `cleanup_resources` | `malloc(50)` pointer, i.e. exactly the size `cleanup` itself allocates, filled with the same `snprintf` payload | [x] |
| 26 | composed pipeline (all three entry points) | `cleanup(...)` → feed its return into `print_result(label, r)` → `cleanup_resources(malloc(...))`, randomised, 2 000 iterations; compares the *concatenated* stdout of the whole sequence so ordering/buffering differences surface | [x] |
| 27 | repeated invocation | `cleanup` called 1 000 times in a row on one library handle (state leakage / allocator reuse across calls) | [x] |
| 28 | interleaved libraries | alternate C-call / Rust-call within one captured stdout region (both libraries share one process `stdout` FILE; proves no divergent buffering) | [x] |

Feature combinations: `Cargo.toml` declares no `[features]`, so the default
(empty) set is the only combination. `scripts/check_features.sh` re-derives this
from `Cargo.toml` and runs the whole suite for each combination it finds.

## Result

Every row above is checked off. Tests live in `tests/phase_b_valid.rs`, named
`rowNN_*` to match the row numbers one-to-one:
`phase_b_valid` → **28 passed, 0 failed**, under the release profile, the debug
profile (overflow checks on), and every feature combination.

Total differential call pairs exercised by Phase B: roughly 1.1 × 10^5 — each
one a C call and a Rust call compared on both return value and stdout bytes.

Randomisation is seeded from `SEED = 0x5EED_C0DE_1234_5678` (SplitMix64) with a
per-row salt (`SEED ^ row`), so any failure reproduces exactly.

## Note on running the suite

`crate-type = ["cdylib"]` is **not** rebuilt by `cargo test`, and the suite
`dlopen`s the `.so` from disk. Always build first:

```sh
cd translation && cargo build --release && cargo test --release -- --test-threads=1
```

`tests/common/mod.rs::assert_so_is_fresh` fails the run with `STALE ARTIFACT` if
this is skipped. `--test-threads=1` is required because stdout capture swaps
process-global fd 1.
