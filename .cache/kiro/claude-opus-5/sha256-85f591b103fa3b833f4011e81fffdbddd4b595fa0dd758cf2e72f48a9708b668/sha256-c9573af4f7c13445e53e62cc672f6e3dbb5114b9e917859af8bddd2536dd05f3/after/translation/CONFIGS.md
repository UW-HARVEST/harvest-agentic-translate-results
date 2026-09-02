# CONFIGS.md — Configuration-surface table (Phase A)

Mirror of `ERRORS.md`, for **valid** inputs. Axes derived mechanically from the
C source, not guessed.

## Axis derivation

### Runtime options / modes / flags — none exist

```sh
grep -nE '#if|#ifdef|#ifndef|#else|switch|getenv|setlocale|static [a-z_]+ g_|extern' c_src/src/driver.c
# -> only the #ifndef DRIVER_H_ include guard in the header
```

There is **no** option struct, **no** setter, **no** mode enum, **no** global
mutable state, **no** environment variable and **no** conditional compilation.
The only `if`/`switch` in the whole library is the 4-conjunct guard in
`parse_val` (line 64) and the `if (parse_val(...))` in `driver` (line 75).
So the configuration surface is entirely **input shape + struct state +
call sequence**.

### Public entry points (both must be driven directly)

| entry point | level | signature |
|---|---|---|
| `run` | **low-level** — mutates a caller-owned `house_t`, prints 4 lines | `void run(house_t *, int)` |
| `driver` | high-level one-shot wrapper — parses, builds `house_t{2,5,2.5}`, calls `run` **twice on the same struct** | `void driver(const char *)` |

`house_t` is not in the public header but `run` takes it by pointer, so an
external caller must reproduce the layout `{ int floors; int bedrooms; double
bathrooms; }` (12 data bytes, 8-byte aligned, `sizeof == 16`). The tests
therefore also assert **struct layout/ABI agreement** by reading back the
mutated struct after `run` returns.

### Input-shape axes the C actually distinguishes

* `run`/`floors` (`int`): typical, 0, negative, `INT_MAX` (`++` wraps), `INT_MIN`
* `run`/`bedrooms` (`int`): typical, 0, negative, `INT_MAX`, `INT_MIN`
* `run`/`bathrooms` (`double`), because of `+= 1.0` then `%.1f`:
  `k.0`, `k.5`, **`%.1f` round-half-to-even tie values** (`0.05 0.15 0.25 0.45
  2.45 …`), negative, `-0.0`, `NaN`, `±inf`, magnitudes ≥ `1e17` where `+1.0`
  is a no-op, `1e300` (309-char output), subnormal, `f64::MIN`/`MAX`
* `run`/`extra_bedrooms` (`int`): 0, ±small, `INT_MAX`, `INT_MIN`, random
* **call sequence**: 1 call vs 2 calls vs N calls on the *same* struct — `run`
  is stateful in its argument, and `driver` exercises exactly the 2-call
  composition, so the accumulated pipeline must be checked, not just one call
* `driver`/input string: the strtol shape axes (sign, whitespace class, leading
  zeros, trailing garbage, digit count, magnitude band relative to `INT_MIN`,
  `INT_MAX`, `LONG_MIN`, `LONG_MAX`)

Comparison in every row is **byte-for-byte on captured stdout** (fd 1 is
redirected around each call) **plus** the post-call `house_t` bytes.
Every row uses many randomized inputs from a fixed-seed SplitMix64 generator.

## Table

`[x]` = differential test exists, drives BOTH `.so`s in that configuration
over its randomized inputs, and passes (Phase B).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `run` | single call; all-typical small values: `floors∈[0,20]`, `bedrooms∈[0,20]`, `bathrooms` = `k` or `k+0.5` for `k∈[0,20]`, `extra_bedrooms∈[0,20]` | `c1_run_typical` | [x] |
| C2 | `run` | single call; `extra_bedrooms == 0` (no-op add) with random typical struct | `c2_run_zero_extra` | [x] |
| C3 | `run` | single call; **negative** `floors`/`bedrooms`/`extra_bedrooms` (random in `[-1000,-1]`) | `c3_run_negative` | [x] |
| C4 | `run` | single call; fully random `int` `floors`, `bedrooms`, `extra_bedrooms` over the whole `i32` range (hits both wrap directions) | `c4_run_full_i32_range` | [x] |
| C5 | `run` | single call; `floors == INT_MAX` (and `INT_MAX-1`, `INT_MIN`) → `++` overflow path, random other fields | `c5_run_floors_boundary` | [x] |
| C6 | `run` | single call; `bedrooms == INT_MAX` with `extra_bedrooms > 0`, and `bedrooms == INT_MIN` with `extra_bedrooms < 0` → `+=` overflow both directions | `c6_run_bedrooms_overflow` | [x] |
| C7 | `run` | single call; `extra_bedrooms ∈ {INT_MAX, INT_MIN}` with random `bedrooms` | `c7_run_extreme_extra` | [x] |
| C8 | `run` | single call; `bathrooms` = `%.1f` **rounding-tie** values: `n/20` for `n∈[-400,400]` (i.e. `…05`, `…15`, `…25`, `…45`) and `n/16`, `n/32` dyadics | `c8_run_rounding_ties` | [x] |
| C9 | `run` | single call; `bathrooms` = uniformly random `f64` bit patterns filtered to finite (all exponents, incl. subnormals and huge magnitudes) | `c9_run_random_finite_f64` | [x] |
| C10 | `run` | single call; `bathrooms ∈ {0.0, -0.0, f64::MIN_POSITIVE, subnormal 5e-324, 1e16, 1e17, 1e300, f64::MAX, f64::MIN, -1.0}` (precision-loss and long-output shapes) | `c10_run_special_finite` | [x] |
| C11 | `run` | single call; `bathrooms ∈ {NaN (quiet, both signs, payload-carrying), +inf, -inf}` | `c11_run_non_finite` | [x] |
| C12 | `run` | **2 sequential calls** on the same struct (the `driver` composition), random typical values — checks accumulated `floors+2`, `bathrooms+2.0`, `bedrooms+2*extra` | `c12_run_twice_same_struct` | [x] |
| C13 | `run` | **N=8 sequential calls** on the same struct, random values incl. extremes — long stateful pipeline | `c13_run_eight_calls` | [x] |
| C14 | `run` | struct-ABI readback: after `run`, compare the 16 bytes of `house_t` (incl. padding-free field values) between C and Rust | `c14_struct_abi_readback` | [x] |
| C15 | `driver` | plain non-negative decimal, random in `[0, 2147483647]`, no whitespace/sign | `c15_driver_plain_nonneg` | [x] |
| C16 | `driver` | plain negative decimal, random in `[-2147483648, -1]` | `c16_driver_plain_neg` | [x] |
| C17 | `driver` | explicit `'+'` sign, random in-range magnitude | `c17_driver_plus_sign` | [x] |
| C18 | `driver` | random **leading whitespace** run drawn from `{' ','\t','\n','\v','\f','\r'}` (length 1–8) followed by an optional sign and digits | `c18_driver_leading_whitespace` | [x] |
| C19 | `driver` | random **leading zeros** (1–30 of them) before an in-range value, with/without sign | `c19_driver_leading_zeros` | [x] |
| C20 | `driver` | **trailing garbage** after a valid in-range prefix: random suffix from `{letters, punctuation, '.', ' ', '-', '+', 'x', 'e', high bytes}` — accepted by this C | `c20_driver_trailing_garbage` | [x] |
| C21 | `driver` | in-`int`-range **boundary values**: `INT_MIN`, `INT_MIN+1`, `INT_MAX`, `INT_MAX-1`, `0`, `-0`, `-1`, `1`, and `±(2^k)`/`±(2^k−1)` for `k∈[0,31]` | `c21_driver_boundaries` | [x] |
| C22 | `driver` | combined shape: whitespace + sign + leading zeros + digits + trailing garbage in one string (cross-product, randomized) | `c22_driver_combined_shapes` | [x] |
| C23 | `driver` | digit-count axis: values rendered with 1..10 digits, and the same value written with padding so total length ∈ {1..64} | `c23_driver_digit_counts` | [x] |
| C24 | `driver` | **repeated `driver` calls** in one process with alternating accept/reject inputs — asserts no `errno`/state leakage across calls (each call must rebuild `house_t{2,5,2.5}` from scratch) | `c24_driver_repeated_alternating` | [x] |
| C25 | `driver` + `run` | mixed sequence: `driver(valid)` then `run(caller struct)` then `driver(invalid)` then `run(...)` — interleaving the two entry points in one process | `c25_interleaved_entry_points` | [x] |

## Row counts actually exercised

Every row runs 400 randomized cases by default (`N` in `tests/valid_path.rs`),
except C8 (1 200 randomized + a 5 607-case exhaustive tie sweep), C10/C11
(20 × 24 and 8 × 24 hand-picked × randomized structs), C21 (~130 exact boundary
values × 2 sign forms) and C23 (10 digit-widths × 40 × 2 + 64 padded lengths).
Roughly 22 000 differential comparisons per full run; each compares captured
stdout byte-for-byte **and**, for `run`, the resulting 16 `house_t` bytes.

## Why the tests fork

`capture()` redirects **fd 1**, which is process-global. libtest writes its own
progress lines to fd 1 from other threads, so under the default
`--test-threads=N` those lines land inside a concurrent capture and corrupt the
comparison (observed, then fixed). Each test body therefore runs in a forked
single-threaded child (`common::isolated`), leaving the parent's fd 1 untouched.
The suite is correct under plain `cargo test` and under `--test-threads=1`.
