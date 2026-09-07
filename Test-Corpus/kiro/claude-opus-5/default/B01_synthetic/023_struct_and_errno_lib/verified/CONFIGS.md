# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**Runtime options / modes / flags:** none. There is no setter, no global
config, no `#ifdef` (the only preprocessor directives are the `DRIVER_H_`
include guard and `#include`s). The library is stateless between calls; the
only mutable state is the caller-owned `house_t`.

**Public entry points (the full set, lowest level first):**

| entry point | linkage | declared in header? |
|---|---|---|
| `run(house_t *the_house, int extra_bedrooms)` | external | **no** (ABI-only, lower level) |
| `driver(const char *in)` | external | yes (one-shot convenience wrapper) |

`driver` composes: `parse_val` → construct `house_t{2, 5, 2.5}` → `run` → `run`
**again on the mutated house**. So the composed pipeline has state carry-over
that per-call tests of `run` cannot see; rows 20–22 target it directly.

**Input shapes the code special-cases:**

*For `run`* — the branch-free arithmetic/format axes:
* `floors` (`int`, `%d`): normal, `0`, negative, `INT_MAX` (`++` overflows), `INT_MIN`
* `bedrooms` (`int`, `%d`): normal, `0`, negative, `INT_MAX`/`INT_MIN` (`+=` overflows)
* `bathrooms` (`double`, `%.1f`, and `+= 1.0`): normal, `0.0`, `-0.0`, negative,
  half-way values where `%.1f` must round (`x.x5`), very large (`1e300` — where
  `+= 1.0` is a no-op), very small/subnormal, `NaN`, `±Inf`
* `extra_bedrooms` (`int`): `0`, positive, negative, `INT_MAX`, `INT_MIN`
* number of consecutive `run` calls on the same house: 1, 2, many

*For `driver`* — the `strtol` input-shape axes:
* sign: none / `+` / `-`
* leading whitespace: none / spaces / mixed `\t\r\n\v\f`
* digit run: 1 digit / many / leading zeros / 400 digits
* trailing bytes: none / alphanumeric / whitespace / punctuation
* magnitude class: in-`int` / `int`-boundary / out-of-`int`-but-in-`long` /
  out-of-`long` (`ERANGE`)
* base-10-specific: `"0x10"` parses as `0` (stops at `x`), `"010"` parses as `10`

## Rows (cross-product pruned to combinations the C distinguishes)

Every row is exercised through **both** `.so` files via `libloading`, stdout is
captured by `dup2`-ing a temp file over fd 1 and `fflush`ed, and the captured
bytes (plus, for `run`, the mutated `house_t` bytes) are compared
**byte-for-byte**. Every row uses **many randomized inputs** (seeded
`SEED = 0x5EED_1234_ABCD_0001`, SplitMix64) unless the row is a fixed boundary.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `run` | default house `{2, 5, 2.5}`, `extra_bedrooms = 0` (fixed) | `cfg_01_run_default_house_zero_extra` | [x] |
| 2 | `run` | default house, 512 randomized `extra_bedrooms` over the full `i32` range | `cfg_02_run_default_house_random_extra` | [x] |
| 3 | `run` | fully randomized `floors`/`bedrooms` (full `i32`) × randomized finite `bathrooms` × randomized `extra_bedrooms`, 2048 cases | `cfg_03_run_fully_random` | [x] |
| 4 | `run` | `floors ∈ {INT_MAX, INT_MAX-1, INT_MIN, -1, 0}` (the `house->floors++` overflow boundary) × randomized rest | `cfg_04_run_floors_boundaries` | [x] |
| 5 | `run` | `bedrooms ∈ {INT_MAX, INT_MIN, 0, ±1}` × `extra_bedrooms ∈ {INT_MAX, INT_MIN, 0, ±1}` full 25-combination cross-product (`+=` overflow) | `cfg_05_run_bedrooms_extra_cross_product` | [x] |
| 6 | `run` | `bathrooms = 0.0` and `-0.0` (sign of zero through `%.1f` after `+= 1.0`) | `cfg_06_run_bathrooms_signed_zero` | [x] |
| 7 | `run` | `bathrooms` at `%.1f` round-to-even half-way points: `0.05, 0.15, …, 2.45, 2.55`, and the values one ULP either side of each | `cfg_07_run_bathrooms_rounding_halfway` | [x] |
| 8 | `run` | `bathrooms` huge: `1e15, 1e16, 1e300, DBL_MAX`, negatives thereof (`+= 1.0` becomes a no-op; `%.1f` emits hundreds of digits) | `cfg_08_run_bathrooms_huge` | [x] |
| 9 | `run` | `bathrooms` tiny: `DBL_MIN`, subnormal `5e-324`, `1e-300` | `cfg_09_run_bathrooms_tiny_subnormal` | [x] |
| 10 | `run` | `bathrooms` non-finite: `NaN`, `-NaN`, signalling-NaN bit pattern, `+Inf`, `-Inf` | `cfg_10_run_bathrooms_non_finite` | [x] |
| 11 | `run` | `bathrooms` from 1024 randomized **raw 64-bit patterns** (covers every double class incl. NaN payloads) | `cfg_11_run_bathrooms_random_bitpatterns` | [x] |
| 12 | `driver` | plain positive decimal, 512 randomized values in `0..=INT_MAX` | `cfg_12_driver_random_positive` | [x] |
| 13 | `driver` | plain negative decimal, 512 randomized values in `INT_MIN..0` | `cfg_13_driver_random_negative` | [x] |
| 14 | `driver` | explicit `+` sign, randomized in-range values | `cfg_14_driver_explicit_plus` | [x] |
| 15 | `driver` | leading whitespace (randomized mix of `" \t\n\r\v\f"`) + randomized in-range value | `cfg_15_driver_leading_whitespace` | [x] |
| 16 | `driver` | trailing garbage after a valid number (randomized suffix bytes) — accepted by C | `cfg_16_driver_trailing_garbage` | [x] |
| 17 | `driver` | leading zeros (`"0000042"`, randomized zero-run lengths) and base-10 reading of `"0x1A"` / `"010"` | `cfg_17_driver_leading_zeros_and_base10` | [x] |
| 18 | `driver` | `int` boundaries as decimal text: `"2147483647"`, `"-2147483648"`, `"0"`, `"-0"`, `"+0"` | `cfg_18_driver_int_boundaries` | [x] |
| 19 | `driver` | out-of-`int` but in-`long`, and out-of-`long` (`ERANGE`) — 512 randomized magnitudes (rejection path, cross-checked against ERRORS.md rows 6–9) | `cfg_19_driver_out_of_int_range_random` | [x] |
| 20 | `driver` | 1024 fully randomized **arbitrary byte strings** (printable + punctuation + digits, length 0..24) — hits accept and reject paths unpredictably | `cfg_20_driver_random_fuzz_strings` | [x] |
| 21 | `run` composed | `run` called **twice in a row** on the same house with the same `extra_bedrooms` (exactly what `driver` does) — verifies state carry-over across calls, randomized start states | `cfg_21_run_twice_state_carryover` | [x] |
| 22 | `run` composed | `run` called **16 times** in a row on the same house, randomized start states (accumulating `floors++`, `bathrooms += 1.0`, `bedrooms +=`) | `cfg_22_run_many_times_accumulation` | [x] |
| 23 | `driver` + `run` | interleaved: `driver(s)` then `run(h, n)` then `driver(s)` in one process, to catch any hidden global state | `cfg_23_interleaved_driver_and_run` | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
— there is **no `add_executable`**, no `main()` anywhere in `c_src`, and
`translation/Cargo.toml` has no `[[bin]]` and no `src/main.rs`. The
"compare the C and Rust binaries' stdout" clause therefore has no subject.
The Phase D test `sym_04_project_builds_no_binary_target` asserts this
mechanically (no `add_executable`, no `int main(`, no `[[bin]]`, no
`src/main.rs`) so the conclusion cannot silently go stale.
