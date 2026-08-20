# CONFIGS.md — Configuration surface table (Phase A, gate for Phase B)

## Public entry points (complete)

`c_src/include/lib.h` declares exactly one symbol, which is simultaneously the
lowest-level and the only entry point — there is no convenience wrapper, no
context/handle object, no init/finish pair:

```c
void normalize(float *dest, const float *src, int size);
```

So the configuration surface is not "which function", but the cross product of
the axes the C body actually branches on.

## Axes the C code actually distinguishes

**Axis 1 — pointer relationship** (drives `else if (dest != src)` and the
read-after-write ordering of loop 2, which reads `src[i]` *after* `dest[i-1]`
was written):

| id | mode | meaning |
|----|------|---------|
| A | `disjoint-sep` | `dest` and `src` in two separate allocations |
| B | `disjoint-same-dest-first` | one allocation, `dest` region entirely before `src` region |
| C | `disjoint-same-src-first` | one allocation, `src` region entirely before `dest` region |
| D | `in-place` | `dest == src` (the only case where the `memset` is suppressed) |
| E | `overlap-dest-after` | `dest = src + k`, `0 < k < size` (loop 2 reads already-written data) |
| F | `overlap-dest-before` | `dest = src - k`, `0 < k < size` (loop 2 reads not-yet-written data; `memset` clobbers `src`) |

**Axis 2 — `size` shape** (drives both loop guards; also the vectorisation
boundaries an optimiser may pick):
`0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129,
255, 256, 257, 1000` — swept inside every row below. Negative sizes are in
`ERRORS.md` rows 3–6.

**Axis 3 — element value class** (drives `sum > 0.0f`, the `sqrtf` argument,
overflow/underflow/NaN behaviour and the exact rounding of loop 1's ordered
accumulation):

| id | class | generator |
|----|-------|-----------|
| V1 | `uniform_pm1` | uniform in `[-1, 1)` |
| V2 | `random_finite_bits` | random 32-bit patterns, NaN/inf rejected (full exponent range incl. subnormals) |
| V3 | `random_any_bits` | arbitrary 32-bit patterns (NaN, inf, subnormals, signed zeros) |
| V4 | `zeros` | all `+0.0f` → `sum == 0` |
| V5 | `signed_zeros` | random mix of `+0.0f` / `-0.0f` → `sum == +0.0f` |
| V6 | `tiny` | `|x| ∈ [1e-30, 1e-20]` → squares underflow, `sum` often `0` |
| V7 | `huge` | `|x| ∈ [1e19, 3e38]` → `sum` overflows to `+inf` |
| V8 | `with_nan` | V1 plus a `NaN` (random sign/payload) at a random index |
| V9 | `with_inf` | V1 plus `±inf` at a random index |
| V10 | `single_nonzero` | all zero except one random element |
| V11 | `mixed_magnitudes` | alternating `~1e20` / `~1e-20` (accumulation-order sensitive) |
| V12 | `small_ints` | integers in `[-8, 8]` (exact; `sum` often a perfect square) |
| V13 | `boundary` | random draws from `{FLT_MIN, FLT_MAX, FLT_EPSILON, denormal_min, largest_denormal, ±1, ±0.5, ±2, ±0}` |
| V14 | `near_unit_sum` | V1 rescaled so `sum ≈ 1.0f` (`1/sqrtf(x)` near 1) |

Every row is executed for **all** sizes of axis 2, with **many randomized
inputs** per (row, size) pair from a fixed-seed SplitMix64 generator, and the
**entire** backing buffer (payload + surrounding guard elements + the `src`
buffer) is compared bit-for-bit between the C `.so` and the Rust `.so`, so
spurious or missing writes outside the nominal range are caught too.

## Row table (cross product, pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `normalize` | A `disjoint-sep` × V1 `uniform_pm1` × all sizes | [x] |
| 2 | `normalize` | A `disjoint-sep` × V2 `random_finite_bits` × all sizes | [x] |
| 3 | `normalize` | A `disjoint-sep` × V3 `random_any_bits` × all sizes | [x] |
| 4 | `normalize` | A `disjoint-sep` × V4 `zeros` × all sizes | [x] |
| 5 | `normalize` | A `disjoint-sep` × V5 `signed_zeros` × all sizes | [x] |
| 6 | `normalize` | A `disjoint-sep` × V6 `tiny` × all sizes | [x] |
| 7 | `normalize` | A `disjoint-sep` × V7 `huge` × all sizes | [x] |
| 8 | `normalize` | A `disjoint-sep` × V8 `with_nan` × all sizes | [x] |
| 9 | `normalize` | A `disjoint-sep` × V9 `with_inf` × all sizes | [x] |
| 10 | `normalize` | A `disjoint-sep` × V10 `single_nonzero` × all sizes | [x] |
| 11 | `normalize` | A `disjoint-sep` × V11 `mixed_magnitudes` × all sizes | [x] |
| 12 | `normalize` | A `disjoint-sep` × V12 `small_ints` × all sizes | [x] |
| 13 | `normalize` | A `disjoint-sep` × V13 `boundary` × all sizes | [x] |
| 14 | `normalize` | A `disjoint-sep` × V14 `near_unit_sum` × all sizes | [x] |
| 15 | `normalize` | B `disjoint-same-dest-first` × V1 × all sizes | [x] |
| 16 | `normalize` | B × V2 `random_finite_bits` × all sizes | [x] |
| 17 | `normalize` | B × V3 `random_any_bits` × all sizes | [x] |
| 18 | `normalize` | B × V4 `zeros` × all sizes | [x] |
| 19 | `normalize` | B × V5 `signed_zeros` × all sizes | [x] |
| 20 | `normalize` | B × V6 `tiny` × all sizes | [x] |
| 21 | `normalize` | B × V7 `huge` × all sizes | [x] |
| 22 | `normalize` | B × V8 `with_nan` × all sizes | [x] |
| 23 | `normalize` | B × V9 `with_inf` × all sizes | [x] |
| 24 | `normalize` | B × V10 `single_nonzero` × all sizes | [x] |
| 25 | `normalize` | B × V11 `mixed_magnitudes` × all sizes | [x] |
| 26 | `normalize` | B × V12 `small_ints` × all sizes | [x] |
| 27 | `normalize` | B × V13 `boundary` × all sizes | [x] |
| 28 | `normalize` | B × V14 `near_unit_sum` × all sizes | [x] |
| 29 | `normalize` | C `disjoint-same-src-first` × V1 × all sizes | [x] |
| 30 | `normalize` | C × V2 `random_finite_bits` × all sizes | [x] |
| 31 | `normalize` | C × V3 `random_any_bits` × all sizes | [x] |
| 32 | `normalize` | C × V4 `zeros` × all sizes | [x] |
| 33 | `normalize` | C × V5 `signed_zeros` × all sizes | [x] |
| 34 | `normalize` | C × V6 `tiny` × all sizes | [x] |
| 35 | `normalize` | C × V7 `huge` × all sizes | [x] |
| 36 | `normalize` | C × V8 `with_nan` × all sizes | [x] |
| 37 | `normalize` | C × V9 `with_inf` × all sizes | [x] |
| 38 | `normalize` | C × V10 `single_nonzero` × all sizes | [x] |
| 39 | `normalize` | C × V11 `mixed_magnitudes` × all sizes | [x] |
| 40 | `normalize` | C × V12 `small_ints` × all sizes | [x] |
| 41 | `normalize` | C × V13 `boundary` × all sizes | [x] |
| 42 | `normalize` | C × V14 `near_unit_sum` × all sizes | [x] |
| 43 | `normalize` | D `in-place` × V1 × all sizes | [x] |
| 44 | `normalize` | D × V2 `random_finite_bits` × all sizes | [x] |
| 45 | `normalize` | D × V3 `random_any_bits` × all sizes | [x] |
| 46 | `normalize` | D × V4 `zeros` × all sizes (memset suppressed) | [x] |
| 47 | `normalize` | D × V5 `signed_zeros` × all sizes (`-0.0f` must survive) | [x] |
| 48 | `normalize` | D × V6 `tiny` × all sizes (underflow, memset suppressed) | [x] |
| 49 | `normalize` | D × V7 `huge` × all sizes | [x] |
| 50 | `normalize` | D × V8 `with_nan` × all sizes (NaN payload must survive) | [x] |
| 51 | `normalize` | D × V9 `with_inf` × all sizes | [x] |
| 52 | `normalize` | D × V10 `single_nonzero` × all sizes | [x] |
| 53 | `normalize` | D × V11 `mixed_magnitudes` × all sizes | [x] |
| 54 | `normalize` | D × V12 `small_ints` × all sizes | [x] |
| 55 | `normalize` | D × V13 `boundary` × all sizes | [x] |
| 56 | `normalize` | D × V14 `near_unit_sum` × all sizes | [x] |
| 57 | `normalize` | E `overlap-dest-after` (`dest = src + k`) × V1 × all sizes × k ∈ {1, 2, 3, size/2, size-1} | [x] |
| 58 | `normalize` | E × V2 `random_finite_bits` × all sizes × all k | [x] |
| 59 | `normalize` | E × V3 `random_any_bits` × all sizes × all k | [x] |
| 60 | `normalize` | E × V4 `zeros` × all sizes × all k (memset overruns) | [x] |
| 61 | `normalize` | E × V5 `signed_zeros` × all sizes × all k | [x] |
| 62 | `normalize` | E × V6 `tiny` × all sizes × all k | [x] |
| 63 | `normalize` | E × V7 `huge` × all sizes × all k | [x] |
| 64 | `normalize` | E × V8 `with_nan` × all sizes × all k | [x] |
| 65 | `normalize` | E × V9 `with_inf` × all sizes × all k | [x] |
| 66 | `normalize` | E × V10 `single_nonzero` × all sizes × all k | [x] |
| 67 | `normalize` | E × V11 `mixed_magnitudes` × all sizes × all k | [x] |
| 68 | `normalize` | E × V12 `small_ints` × all sizes × all k | [x] |
| 69 | `normalize` | E × V13 `boundary` × all sizes × all k | [x] |
| 70 | `normalize` | E × V14 `near_unit_sum` × all sizes × all k | [x] |
| 71 | `normalize` | F `overlap-dest-before` (`dest = src - k`) × V1 × all sizes × all k | [x] |
| 72 | `normalize` | F × V2 `random_finite_bits` × all sizes × all k | [x] |
| 73 | `normalize` | F × V3 `random_any_bits` × all sizes × all k | [x] |
| 74 | `normalize` | F × V4 `zeros` × all sizes × all k (memset clobbers `src`) | [x] |
| 75 | `normalize` | F × V5 `signed_zeros` × all sizes × all k | [x] |
| 76 | `normalize` | F × V6 `tiny` × all sizes × all k | [x] |
| 77 | `normalize` | F × V7 `huge` × all sizes × all k | [x] |
| 78 | `normalize` | F × V8 `with_nan` × all sizes × all k | [x] |
| 79 | `normalize` | F × V9 `with_inf` × all sizes × all k | [x] |
| 80 | `normalize` | F × V10 `single_nonzero` × all sizes × all k | [x] |
| 81 | `normalize` | F × V11 `mixed_magnitudes` × all sizes × all k | [x] |
| 82 | `normalize` | F × V12 `small_ints` × all sizes × all k | [x] |
| 83 | `normalize` | F × V13 `boundary` × all sizes × all k | [x] |
| 84 | `normalize` | F × V14 `near_unit_sum` × all sizes × all k | [x] |

### Additional shape/state axes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 85 | `normalize` | element offset / alignment sweep: `dest`/`src` start at element offsets 0..15 of a 64-byte-aligned block (16B/32B/64B boundaries), V2/V3 values | [x] |
| 86 | `normalize` | repeated invocation on the same buffer (idempotence of the composed pipeline: `normalize` twice, then compare — catches state leaks between calls) | [x] |
| 87 | `normalize` | `sum` exactly representable / exact-`sqrt` inputs (`src` = one-hot, all-ones, `[3,4]`, `[1,1,1,1]`) where `1/sqrtf(sum)` is exact | [x] |
| 88 | `normalize` | `sum` at the overflow edge: values chosen so `sum` is the largest finite `f32` and so `sum` overflows on the *last* accumulation only | [x] |
| 89 | `normalize` | `sum` at the underflow edge: values whose squares are denormal (`1e-20f`) so `sum > 0` is true but tiny → huge `1/sqrtf(sum)` → `dest` overflows to `±inf` | [x] |
| 90 | `normalize` | accumulation-order sensitivity: same multiset of values in different permutations (`sum` differs by rounding; ordered accumulation must match C exactly) | [x] |
| 91 | `normalize` | large size (`size = 1000`) with V1/V11 so hundreds of roundings compound | [x] |
| 92 | `normalize` | Rust `.so` built with `dev` profile (`opt-level = 0`) vs the same C `.so` | [x] |
| 93 | `normalize` | Rust `.so` built with `release` profile (`opt-level = 3`, vectorisation enabled) vs the same C `.so` | [x] |
| 94 | `normalize` | feature combination `--no-default-features` (the only one that exists; `default = []`) | [x] |
| 95 | `normalize` | systematic stride sweep of the **entire** `f32` bit-pattern space (65 536 patterns x 9 layouts) as the `sqrtf` argument — every exponent, subnormals, `±inf`, every NaN class | [x] |
| 96 | `normalize` | randomized sampling of the FULL cross product (aliasing mode x value class x size x head offset x poisoned element), 120 000 cases | [x] |
| 97 | `normalize` | randomized `size` including `0`, random negative, `INT_MIN..INT_MIN+3`, random positive, x random pointer relation, 20 000 cases | [x] |
| 98 | `normalize` | random call SEQUENCES (1..8 chained `normalize` calls with random `dest`/`src`/`size` inside one buffer), 2 000 sequences | [x] |

### Test-map

| rows | test |
|------|------|
| 1–84 | `tests/valid_paths.rs::cfg_rows_*` (one test per aliasing mode) |
| 85–91 | `tests/valid_paths.rs::cfg_row_085..091` |
| 92–94 | `scripts/verify_all.sh` (runs the whole suite for every feature combination x `dev`/`release`) |
| 95–98 | `tests/fuzz_paths.rs::cfg_row_095..098` |

`tests/meta.rs` additionally proves the suite is meaningful rather than
vacuous: a negative control (an intentionally perturbed implementation —
`x / sqrt(sum)` instead of `x * (1/sqrt(sum))`, and NaN propagation instead of
zero-filling) MUST be rejected by the very comparison used above, and the
generators are shown to reach all three C branches (`sum > 0` ≈ 2 000 cases,
`memset` ≈ 400, `dest == src` no-op ≈ 400, `sum == +inf` ≈ 1 100,
`sum == NaN` ≈ 230, underflowed `sum` ≈ 60).

### C compiler configuration (informational)

`c_src/CMakeLists.txt` has a single configuration (no `CMAKE_BUILD_TYPE`, so
`-O0`). The whole suite was additionally run against a C `.so` built with
`-O3` (`HARVEST_C_SO=<path> cargo test`) and still passes bit-for-bit, i.e. the
translation does not depend on the C optimisation level.
