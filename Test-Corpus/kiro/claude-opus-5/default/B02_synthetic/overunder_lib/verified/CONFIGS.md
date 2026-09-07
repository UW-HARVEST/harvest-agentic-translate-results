# CONFIGS.md — Configuration surface table (valid inputs)

## Axes actually branched on by the C code

There are **no runtime options, modes, flags, or `#ifdef`s** in
`c_src/src/lib.c` — grep confirms zero `#ifdef`/`#if`, zero global/static
state, zero setters. The library is a pure function of its four `int`
arguments (plus the two lower-level entry points' own arguments). The
configuration surface is therefore entirely **input shape**:

| axis | values the C distinguishes | source of the branch |
|------|----------------------------|----------------------|
| `A1` entry point | `overunder`, `safe_double_to_int`, `process_with_fallthrough`, `copy_data_block`, `handle_pointer_operations` | 5 exported symbols (`nm -D`) |
| `A2` `a % 6` | `0`,`1`,`2`,`3`,`4`,`5` and the negative mirrors `-1..-5` | `switch (code)` cases 0..5 + `default:` |
| `A3` sign of `a`,`b`,`c`,`d` | each independently `< 0`, `== 0`, `> 0` | `(double)` casts, `%`, `sqrt` domain |
| `A4` magnitude | small (no overflow), large enough that `a*a`/`d*d`/`a+b`/`value*2` overflow `int`, and the extremes `INT_MIN`/`INT_MAX` | `d*d + a*a`, `a+b`, `value*2`, `total +=` |
| `A5` `sqrt` argument sign | `d*d + a*a` non-negative vs. wrapped-negative (→ `NaN`) | `sqrt((double)(...))` |
| `A6` `safe_double_to_int` input class | `> INT_MAX`, `< INT_MIN`, `NaN`, exact boundaries, in-range positive, in-range negative, `±0.0`, subnormal, fractional (truncation) | 4-way `if/else if/else` chain |
| `A7` `c / 3.3` sign & truncation | positive truncation-down, negative truncation-up, exact-zero | `(double)c / 3.3` then `(int)` |
| `A8` `DataBlock` byte content | fully-initialised source, `value` = NaN/inf/large, arbitrary `label` bytes incl. embedded NUL and non-ASCII, and the 4 padding bytes after `id` | `memcpy(dest, src, sizeof(DataBlock))` copies padding too |
| `A9` stdout | the 8 `printf` call sites in `overunder` (`%d`, `%.2f`, `%s`, bare literals) must match byte-for-byte | `printf` / `PRINT_VAR` macro |

`sizeof(DataBlock)` is 32 on x86-64 (`int` @0, 4 pad, `double` @8, `char[20]`
@16, 4 trailing pad) — the padding is part of the copied payload.

## Rows (pruned cross-product — combinations the C treats differently)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) unless the row is a specific singleton boundary.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `safe_double_to_int` | randomized in-range positive doubles (fractional, truncation toward zero) | [x] |
| 2 | `safe_double_to_int` | randomized in-range negative doubles (fractional, truncation toward zero) | [x] |
| 3 | `safe_double_to_int` | randomized magnitudes spanning `1e-320 .. 1e300`, both signs (crosses all 4 branches) | [x] |
| 4 | `safe_double_to_int` | exact boundary singletons: `±0.0`, `±1.0`, `2147483646/7/8.0`, `-2147483647/8/9.0`, `nextafter` neighbours of both limits, `±inf`, NaN, `-NaN`, `f64::MIN_POSITIVE`, subnormal | [x] |
| 5 | `safe_double_to_int` | randomized **raw bit patterns** reinterpreted as `f64` (hits NaN payloads, subnormals, inf, huge exponents) | [x] |
| 6 | `process_with_fallthrough` | `code` = each of `0,1,2,3,4,5` × randomized `base_value` (all fall-through chains) | [x] |
| 7 | `process_with_fallthrough` | `code` outside `0..=5` (randomized, incl. negatives and `>= 6`) × randomized `base_value` (`default:`) | [x] |
| 8 | `process_with_fallthrough` | `code` in `1..=5` × `base_value` near `INT_MAX`/`INT_MIN` (signed overflow of `+= N`) | [x] |
| 9 | `process_with_fallthrough` | `code` at `INT_MIN`, `INT_MAX`, `6`, `-1`, `-6` (boundary of the case set) | [x] |
| 10 | `handle_pointer_operations` | randomized `value`, no overflow | [x] |
| 11 | `handle_pointer_operations` | `value` such that `value*2` and/or `+100` overflow: `INT_MAX`, `INT_MIN`, `INT_MAX/2 ± k`, `INT_MIN/2 ± k` | [x] |
| 12 | `copy_data_block` | fully randomized 32-byte `DataBlock` payloads (incl. padding bytes), compared byte-for-byte over all 32 bytes | [x] |
| 13 | `copy_data_block` | `label` containing embedded NUL, all-`0xFF`, non-ASCII high bytes; `value` = NaN / `±inf` / subnormal (bit-exact copy) | [x] |
| 14 | `copy_data_block` | pre-filled non-zero destination (proves the full 32 bytes incl. padding are overwritten, not just the 3 fields) | [x] |
| 15 | `overunder` | `a % 6 == 0` (positive `a`) — switch `case 0`, `switch_result == 0` | [x] |
| 16 | `overunder` | `a % 6 == 1,2,3,4,5` (positive `a`) — each fall-through chain, randomized `b,c,d` | [x] |
| 17 | `overunder` | `a < 0` with `a % 6 != 0` — negative `code`, `default:` → `-1` | [x] |
| 18 | `overunder` | all of `a,b,c,d` zero; and each one individually zero (division `0/3.3`, `sqrt(0)`) | [x] |
| 19 | `overunder` | small positive `a,b,c,d` (no overflow anywhere) — randomized in `1..=1000` | [x] |
| 20 | `overunder` | small negative `a,b,c,d` — randomized in `-1000..=-1`, mixed signs | [x] |
| 21 | `overunder` | `a`,`d` large enough that `d*d + a*a` overflows to **positive** (still a valid `sqrt` domain) | [x] |
| 22 | `overunder` | `a`,`d` large enough that `d*d + a*a` overflows to **negative** → `sqrt(NaN)` → `conv4 == 0` | [x] |
| 23 | `overunder` | `b` large so `b * 2.7` exceeds `INT_MAX` → `conv2 == INT_MAX` clamp | [x] |
| 24 | `overunder` | `b` very negative so `b * 2.7 < INT_MIN` → `conv2 == INT_MIN` clamp | [x] |
| 25 | `overunder` | `a` large so `a * 1.5` exceeds `INT_MAX` → `conv1` clamp, and `%.2f` of a huge `value` | [x] |
| 26 | `overunder` | `c` at `INT_MIN`/`INT_MAX` — `c/3.3` sign & truncation, plus `handle_pointer_operations(c)` overflow | [x] |
| 27 | `overunder` | `a + b` overflows (`a`,`b` both near `INT_MAX` / both near `INT_MIN`) — `array1[4]` wrap | [x] |
| 28 | `overunder` | all four args at the extremes: full `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` sweep combinations | [x] |
| 29 | `overunder` | fully randomized `i32` quadruples over the whole 32-bit range (property-style, 2000+ cases) | [x] |
| 30 | `overunder` | **stdout byte-for-byte**: captured `printf` output compared for every configuration above (rows 15–29) | [x] |
| 31 | composed pipeline | `overunder` return value cross-checked against the low-level exports called individually in the same order (proves the composition, not just each wrapper) | [x] |

## Row → test mapping

Rows 1–31 map one-to-one onto the tests in `tests/phase_b_valid.rs`, named
`rowNN_*` (e.g. row 22 → `row22_overunder_sqrt_arg_overflows_negative_nan`).
Every `overunder` row asserts the return value **and** the byte-exact `printf`
output; the `.so` `printf` writes are captured by temporarily redirecting
file descriptor 1 (see `tests/common/mod.rs`).

On top of the per-row tests, `tests/phase_d_sweep.rs` re-drives the same
surface at much higher volume:

| sweep | coverage |
|-------|----------|
| `sweep_sdti_structured_exponents` | all 2048 exponents × 12 mantissas × both signs (≈49k doubles) |
| `sweep_sdti_every_integer_boundary_region` | ±4096 ULP around `INT_MAX`, `INT_MIN`, `0`, `±1`, plus a strided band over ±3e9 |
| `sweep_pwf_exhaustive_code_band` | every `code` in `-2000..=2000` × 17 `base_value`s, plus the `i32` extremes |
| `sweep_hpo_dense_i32` | strided sweep across the entire `i32` range (prime stride 65521) |
| `sweep_copy_data_block_volume` | 20 000 random 40-byte payloads × 256 destination fill patterns |
| `sweep_overunder_return_values_high_volume` | 40 000 quadruples (uniform + banded toward the clamp/overflow thresholds) |
| `sweep_overunder_stdout_byte_exact_subset` | 7 000 quadruples compared byte-for-byte on stdout |
| `parity_datablock_layout_matches_c` | `size`/`align`/field offsets, plus proof the Rust `.so` copies exactly 40 bytes (guard bytes) |

## Harness validation (negative controls)

The differential harness was validated by injecting deliberate mutants into
`src/lib.rs`, rebuilding the `.so`, and confirming the suite fails:

| mutant | detected by |
|--------|-------------|
| `safe_double_to_int` NaN branch returns `1` instead of `0` | rows 4, 5, 22 + Phase C row 3 |
| `case 5` adds `51` instead of `50` | rows 6, 8, 9, 16 |
| `"value=%.2f"` → `"value=%.3f"` (stdout only) | rows 15–30 |
| `copy_data_block` copies 36 bytes instead of 40 | rows 12, 13, 14 |

All mutants were reverted; `verify.sh` reports `ALL CHECKS PASSED` on the
final source.
