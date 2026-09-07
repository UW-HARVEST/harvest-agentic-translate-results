# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes derived mechanically from the C source

`c_src` is a single translation unit. Enumerating everything the code can branch
on or special-case:

### Runtime options / modes / flags
```
$ grep -nE '#if|#ifdef|switch|enum|extern .*(flag|mode|opt)' c_src/src/driver.c c_src/include/driver.h
(no matches)
```
**None.** There are no flags, no modes, no globals, no `#ifdef`s, no setters, no
init/teardown state. Behaviour is a pure function of the arguments. There is also
exactly one preprocessor conditional in the whole tree (the `DRIVER_H_` include
guard), so there is no compile-time configuration axis either.

### Public entry points (the FULL set, lowest-level included)
| entry point | linkage | notes |
|---|---|---|
| `fma_array(out, mul1, mul2, add, len)` | global (`T`) | **the low-level primitive.** Not in `driver.h`, but non-`static`, so it is directly callable by any consumer via `dlsym`. Must be tested directly, with all four pointers independent — a coverage hole that testing only `driver` leaves wide open, because `driver` only ever calls it fully aliased. |
| `driver(data, len)` | global (`T`) | the one-shot convenience wrapper: VLA + `memcpy` + `inner` (which = `fma_array` fully aliased, then a `printf("%d\n")` loop). |
| `inner(out, len)` | `static` (`t`) | not reachable across the ABI; covered transitively through `driver`. |

### Input shapes the code distinguishes
- **`len`**: `< 0` (no-op / UB), `0` (empty), `1` (single), `2`, small (2–8),
  many (64–1024). The loop guard `i < len` is the only size branch.
- **element value class**: zeros; small positives; small negatives; mixed sign;
  `INT_MAX` / `INT_MIN` / `-1` / `1` boundary values; values whose product
  overflows (`mul` wrap); values whose product+add overflows (`add` wrap);
  full-range random `i32`.
- **pointer aliasing relationship among `out`/`mul1`/`mul2`/`add`** (only
  meaningful for the low-level `fma_array`; no `restrict` anywhere, so every
  combination is legal C):
  all-distinct; `out == mul1`; `out == add`; `mul1 == mul2` (square);
  all-four-identical (the `inner` pattern); overlapping-at-an-offset
  (`mul1 == out + 1`), where the ascending iteration order is observable.
- **output channel**: `driver` writes to `stdout` via `printf("%d\n")`, so the
  formatting of negative numbers and of `INT_MIN` is part of the observable
  contract and must be compared as raw stdout bytes.

### Crate feature combinations
```
$ grep -n '\[features\]' translation/Cargo.toml
(no match)
```
**None** — `Cargo.toml` declares no `[features]` table and no optional
dependencies, so `--no-default-features` and the full feature powerset all
collapse to the single default configuration. `crate-type = ["cdylib"]` only, so
the project builds **no binary executable** → no stdout-of-driver-binary
comparison applies; stdout is instead compared per `driver` call inside the
tests by redirecting fd 1.

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is exercised against BOTH `.so`s through `libloading`, with many
randomized inputs per row (fixed seed, deterministic SplitMix64 PRNG).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|------------------------------------------|-----|
| 1 | `fma_array` | `len = 0`, all four pointers distinct and valid → must leave `out` bit-identical | [x] |
| 2 | `fma_array` | `len = 1`, all pointers distinct, random full-range `i32` | [x] |
| 3 | `fma_array` | `len = 2`, all pointers distinct, random full-range `i32` | [x] |
| 4 | `fma_array` | `len` small (2..=8), all pointers distinct, random full-range `i32` | [x] |
| 5 | `fma_array` | `len` many (64..=1024), all pointers distinct, random full-range `i32` | [x] |
| 6 | `fma_array` | `len` many, all elements zero | [x] |
| 7 | `fma_array` | `len` many, all elements small positive (0..=100) — no overflow | [x] |
| 8 | `fma_array` | `len` many, all elements small negative (-100..=0) | [x] |
| 9 | `fma_array` | `len` many, mixed-sign small values | [x] |
| 10 | `fma_array` | `len` many, elements drawn from the boundary set {`INT_MIN`, `INT_MIN+1`, `-2`, `-1`, `0`, `1`, `2`, `INT_MAX-1`, `INT_MAX`} → exercises `mul` wrap AND `add` wrap | [x] |
| 11 | `fma_array` | `len` many, `mul1`/`mul2` forced large (`>= 1<<16`) so every product overflows | [x] |
| 12 | `fma_array` | `len` many, product pinned at `INT_MAX` with `add = 1` so every `+` overflows | [x] |
| 13 | `fma_array` | aliasing `out == mul1`, `mul2`/`add` distinct, random values, `len` many | [x] |
| 14 | `fma_array` | aliasing `out == add`, `mul1`/`mul2` distinct, random values, `len` many | [x] |
| 15 | `fma_array` | aliasing `mul1 == mul2` (squaring), `out`/`add` distinct, random values | [x] |
| 16 | `fma_array` | aliasing all four identical (`out == mul1 == mul2 == add`) — the exact `inner` call pattern, random values, `len` many | [x] |
| 17 | `fma_array` | aliasing at an offset: `mul1 == out + 1`, `mul2 == add == out`, `len` many → result is order-dependent, pins ascending `i` | [x] |
| 18 | `fma_array` | aliasing at an offset: `out == mul1 + 1` (write-ahead of read), `len` many | [x] |
| 19 | `driver` | `len = 0` → no stdout output at all, returns normally | [x] |
| 20 | `driver` | `len = 1`, random full-range `i32` → one stdout line, compared as raw bytes | [x] |
| 21 | `driver` | `len` small (2..=8), random full-range `i32` → stdout compared byte-for-byte | [x] |
| 22 | `driver` | `len` many (64..=1024), random full-range `i32` → stdout compared byte-for-byte | [x] |
| 23 | `driver` | `len` many, all zeros → stdout is `len` copies of `"0\n"` | [x] |
| 24 | `driver` | `len` many, small positives → no wrap, plain decimal formatting | [x] |
| 25 | `driver` | `len` many, small negatives → exercises the `-` sign in `printf("%d\n")` | [x] |
| 26 | `driver` | `len` many, boundary set {`INT_MIN`..`INT_MAX`} → exercises wrapping AND `INT_MIN` decimal formatting (`-2147483648`) | [x] |
| 27 | `driver` | `len` many, values chosen so every product overflows | [x] |
| 28 | `driver` | back-to-back repeated calls with different `len` (state-freeness: the VLA must not leak state between calls) | [x] |
| 29 | `driver` + `fma_array` | composed/consistency: `driver(data, len)` stdout must equal the decimal rendering of `fma_array(t, data, data, data, len)`, i.e. the pipeline agrees with the primitive, for both C and Rust | [x] |
| 30 | `fma_array` | `len` many, `out` buffer pre-filled with a sentinel pattern → confirms exactly `len` elements are written and nothing beyond is touched (guards against off-by-one past the end) | [x] |
| 31 | `driver` | `len` = every power of two from `2^0` to `2^20` (VLA from 4 B up to 4 MiB), run on a thread with a pinned 8 MiB stack → the whole range in which the C's VLA is well defined | [x] |
| 32 | `fma_array` | all 16 NULL/non-NULL combinations of the four pointer arguments with `len <= 0`, plus `nullmask = 0` with `len > 0` → the valid half of the pointer-nullability axis | [x] |

## Row-to-test mapping

Rows 1–30 are implemented in `tests/phase_b_configs.rs`; rows 31–32 fall out of
the isolated-subprocess tests in `tests/phase_c_errors.rs` (they need a pinned
stack size, which only the subprocess worker provides).

| rows | test(s) |
|------|---------|
| 1–5 | `row01_fma_len0_distinct_leaves_out_untouched` … `row05_fma_len_many_distinct_random` |
| 6–12 | `row06_fma_all_zeros` … `row12_fma_every_add_overflows` |
| 13–18 | `row13_…` … `row18_…`, plus `rows13_18_all_alias_schemes_across_all_lens` (full aliasing × value-class × length cross-product) |
| 19–28 | `row19_driver_len0_prints_nothing` … `row28_driver_repeated_calls_are_stateless` |
| 29 | `row29_driver_stdout_matches_fma_array_primitive` |
| 30 | `row30_fma_writes_exactly_len_elements` |
| 31 | `e14_driver_vla_within_the_stack_matches_exactly` |
| 32 | `e17_fma_individual_null_arguments` |

## Randomization

Every row is driven with many inputs, not one hand-picked value. Each test seeds
its own SplitMix64 generator with a fixed constant (`Rng::new(0x…)`) so runs are
reproducible. Per-row iteration counts are 200 for the cheap `fma_array` rows,
64 for the `driver` rows (each of which prints up to 1024 lines), and the
boundary rows additionally run the **exhaustive** 17 × 17 cross-product of the
`BOUNDARY` value set (`i32::MIN`, `i32::MIN+1`, `i32::MIN/2`, ±65536, ±65535,
±3, ±2, ±1, 0, `i32::MAX/2`, `i32::MAX-1`, `i32::MAX`) for both the multiply and
the add.

## How `driver`'s stdout is compared

`driver` reports its results only by calling libc `printf("%d\n", …)`, so the
comparison has to be on the raw bytes of file descriptor 1. Doing that in-process
is unsound under `cargo test`: libtest writes its own progress lines to fd 1 from
the main thread while worker threads run, and those bytes land in the capture.
The harness therefore re-execs the test binary as a single-threaded **worker
subprocess** (`common::run_driver_batch`), which redirects fd 1 to one file per
(case, implementation) with nothing else writing there. One subprocess handles a
whole test's batch of cases, so this costs one `fork`/`exec` per test rather than
per call.

## No binary target

`Cargo.toml` declares `crate-type = ["cdylib"]` and no `[[bin]]`, and
`c_src/CMakeLists.txt` declares only `add_library(driver SHARED …)` — neither
side builds a driver executable, so there is no binary-stdout comparison to
make. The equivalent end-to-end check is row 29, which compares the full
`driver` pipeline's stdout against the low-level `fma_array` primitive for both
libraries.
