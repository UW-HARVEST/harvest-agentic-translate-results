# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/driver.c`

The mirror of `ERRORS.md`: every axis the C code actually branches on for
*accepted* input, and the combinations it treats differently.

## Axis 1 — runtime options / modes / flags

**There are none.** The mechanical sweep of `c_src/`:

| searched for | occurrences |
|--------------|-------------|
| `#ifdef` / `#if` / `#ifndef` other than the `DRIVER_H_` include guard | 0 |
| `switch` statements | 0 |
| global or `static` mutable state, setters, init/config functions | 0 |
| `getenv` / config file / compile-time tunables | 0 |
| `struct`/`enum`/`typedef` declarations | 0 |
| compiler-flag branches in `CMakeLists.txt` (no `CMAKE_BUILD_TYPE`, no `target_compile_definitions`) | 0 |

Correspondingly `translation/Cargo.toml` declares no `[features]` table, so
there is exactly one build configuration. Behaviour is a pure function of the
arguments, which makes Axes 2 and 3 the whole configuration surface.

## Axis 2 — entry points (all three, lowest level first)

| level | symbol | in public header? | why it must be driven directly |
|-------|--------|-------------------|--------------------------------|
| low  | `fma_array` | no, but externally linked and exported | the only place arithmetic and the loop bound live; `call_fma` reaches it with only all-ones/all-zeros vectors, so calling `fma_array` through `call_fma` alone never exercises general multiplicands, general addends, or overflow |
| mid  | `call_fma`  | no, but externally linked and exported | owns the `len == 0` guard and the VLA construction; `driver` only ever reaches it with `0 <= len <= 100` |
| top  | `driver`    | yes | owns the `sscanf` parse loop, the 100-element cap, and the `printf` output |

`driver` is the convenience/one-shot wrapper. Rows below deliberately cover
`fma_array` and `call_fma` on their own, plus the composed `driver` pipeline.

## Axis 3 — input shapes the C special-cases

- `fma_array` / `call_fma` `len`: `0` (guard / loop-never-runs), `1`
  (`out[len-1]` is `out[0]`, the element the C pre-sets to `0` before
  overwriting), `2`, small `3..8`, large (`1000`, `100000` — beyond any
  plausible unrolling/vectorisation width).
- element values: all zero, all one, small positives, negatives,
  `INT_MAX`/`INT_MIN` extremes, uniformly random full-`i32` range (drives
  wrapping multiply and wrapping add).
- `driver` input text, by parse shape: empty, whitespace-only, one integer,
  two, many, exactly 99 / 100 / 101 / 250 (the cap boundary), sign prefixes
  (`-`, `+`, none), leading zeros, separator kind (single space, multiple
  spaces, tab, newline, `\v`, `\f`, `\r`, mixed), trailing whitespace,
  trailing garbage, garbage mid-string, `%d`-out-of-range numerals, values
  that look like other bases (`0x10`, `1e5`), and the position of the value
  that ends up printed (only `data[i-1]` is observable).

## Configuration-surface table

One row per combination the C actually distinguishes. Each row is exercised
with many randomised inputs under a fixed seed (`SEED = 0x5EED_1234_ABCD_0001`),
not a single hand-picked value.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| C1  | `fma_array` | `len == 1`, randomised full-range `i32` for `mul1`/`mul2`/`add` | `c1_fma_array_len1_random` | [x] |
| C2  | `fma_array` | `len == 2`, randomised full-range `i32` | `c2_fma_array_len2_random` | [x] |
| C3  | `fma_array` | `len` random in `3..=8` (small, sub-vector-width), randomised full-range `i32` | `c3_fma_array_small_len_random` | [x] |
| C4  | `fma_array` | `len == 1000` (large, multi-vector), randomised full-range `i32` | `c4_fma_array_large_len_random` | [x] |
| C5  | `fma_array` | `len == 100_000` (very large), randomised full-range `i32` | `c5_fma_array_very_large_len_random` | [x] |
| C6  | `fma_array` | `len` random `1..=64`, values drawn from the extremes set `{0, ±1, ±2, ±46341, ±65536, INT_MAX, INT_MIN, ...}` — maximises overflow of both the multiply and the add | `c6_fma_array_extreme_values` | [x] |
| C7  | `fma_array` | `len` random `1..=64`, `mul1` all ones and `add` all zeros — the exact shape `call_fma` produces, verified against the general path | `c7_fma_array_ones_zeros_shape` | [x] |
| C8  | `fma_array` | `len` random `1..=64`, `mul2` all zeros (result is purely `add`) and separately `add` all zeros (result is purely the product) | `c8_fma_array_degenerate_operands` | [x] |
| C9  | `fma_array` | `out` pre-filled with a randomised sentinel pattern, `len` shorter than the buffer — checks the C writes exactly `len` elements and leaves the tail untouched | `c9_fma_array_writes_exactly_len` | [x] |
| C10 | `call_fma` | `len == 1`, randomised `data` — the `out[0] = 0`-then-overwrite path | `c10_call_fma_len1_random` | [x] |
| C11 | `call_fma` | `len` random `2..=8`, randomised full-range `i32` `data` | `c11_call_fma_small_len_random` | [x] |
| C12 | `call_fma` | `len == 100` (the largest `driver` can produce), randomised `data` | `c12_call_fma_len100_random` | [x] |
| C13 | `call_fma` | `len == 4096` (large VLA), randomised `data` | `c13_call_fma_large_len_random` | [x] |
| C14 | `call_fma` | `len` random `1..=64`, `data` from the extremes set incl. `INT_MAX`/`INT_MIN` | `c14_call_fma_extreme_values` | [x] |
| C15 | `call_fma` | `data` buffer longer than `len` — confirms only `data[len-1]` is observable and the tail is not read into the result | `c15_call_fma_ignores_tail` | [x] |
| C16 | `driver` | single integer, randomised full-range `i32` rendered as decimal, no surrounding whitespace, plus the exact `0`/`±1`/`INT_MAX`/`INT_MIN` boundary literals | `c16_driver_single_int_random` | [x] |
| C17 | `driver` | 2..=10 integers, single-space separated, randomised full-range `i32` | `c17_driver_few_ints_space_random` | [x] |
| C18 | `driver` | 2..=100 integers, separator randomly chosen per gap from `{" ", "  ", "\t", "\n", "\r", "\v", "\f", " \t\n", "\n\n  "}` — exercises `%d`'s whitespace skipping and the `%zn` byte count together | `c18_driver_mixed_whitespace_random` | [x] |
| C19 | `driver` | randomised leading whitespace before the first integer | `c19_driver_leading_whitespace_random` | [x] |
| C20 | `driver` | randomised trailing whitespace after the last integer — the final `sscanf` returns EOF rather than 0 | `c20_driver_trailing_whitespace_random` | [x] |
| C21 | `driver` | explicit `+` sign on a random subset of the integers, mixed with `-` and unsigned | `c21_driver_explicit_plus_signs_random` | [x] |
| C22 | `driver` | randomised leading zeros (`0`..`8` of them) on each integer — `%d` is decimal, so `007` is `7`, and the `%zn` count includes the zeros | `c22_driver_leading_zeros_random` | [x] |
| C23 | `driver` | exactly 99 integers (one below the cap), randomised | `c23_driver_exactly_99` | [x] |
| C24 | `driver` | exactly 100 integers (at the cap; the loop exits on `i < 100`, not on a parse failure) | `c24_driver_exactly_100` | [x] |
| C25 | `driver` | 101..=250 integers (past the cap; only the first 100 are read and the 100th is printed) | `c25_driver_past_cap_random` | [x] |
| C26 | `driver` | `k` random integers followed by randomised non-numeric garbage — the mid-stream `break` path with `1 <= k <= 100` | `c26_driver_ints_then_garbage_random` | [x] |
| C27 | `driver` | integer immediately followed by letters with no separator (`"5abc"`), randomised value and suffix | `c27_driver_int_glued_to_garbage_random` | [x] |
| C28 | `driver` | values that look like another base or a float (`0x10`, `1e5`, `1.5`, `0b1`) — `%d` takes the leading decimal run and the next `sscanf` then fails | `c28_driver_baseish_tokens` | [x] |
| C29 | `driver` | comma / semicolon / `\|` separated integers — the first parses, then a matching failure ends the loop | `c29_driver_nonspace_separators_random` | [x] |
| C30 | `driver` | integers whose decimal text is out of `int` range, at randomised positions among valid ones (accepted by `%d` with saturation, so parsing continues) | `c30_driver_out_of_range_numerals_random` | [x] |
| C31 | `driver` | the printed value is `data[i-1]`: randomised sequences where the last element is deliberately `INT_MIN`, `INT_MAX`, `0`, `-0`, `+0` or `000` | `c31_driver_last_value_extremes` | [x] |
| C32 | `driver` | fully randomised fuzz — random token stream mixing valid integers, signs, whitespace runs, garbage words, base-ish tokens and over-range numerals, 1..=300 tokens, 400 iterations | `c32_driver_fuzz_random` | [x] |
| C33 | `driver` -> `call_fma` -> `fma_array` | the composed pipeline: `driver` output cross-checked against an independent direct `call_fma` on the same parsed prefix, so a bug that cancels out inside one wrapper is still caught | `c33_pipeline_cross_check` | [x] |
| C34 | `driver` | arbitrary RAW BYTES (any byte but NUL is legal for a `const char *`), including 0x80..0xFF and control bytes, plus UTF-8 space-like sequences. `%d`'s leading-whitespace skip goes through the locale's `isspace`, which high bytes can reach; this is the broadest check that the Rust delegates to libc instead of parsing anything itself | `c34_driver_raw_bytes_fuzz` | [x] |

## Where the tests live

Split across two binaries for a mechanical reason, not a stylistic one:

| file | rows | why |
|------|------|-----|
| `tests/phase_b_configs.rs` | C1..C15 | compare return values and output buffers; safe on libtest's default thread pool, so each row is its own `#[test]` |
| `tests/phase_b_driver.rs` | C16..C34 | `driver` reports via `printf`, so verifying it means redirecting fd 1 and diffing the bytes. fd 1 is process-global: if libtest writes a progress line from another thread while the redirect is up, it lands in the capture and shows as a phantom diff. Every row is therefore a plain function called from a SINGLE `#[test]` in its own binary |

## Beyond the table

`check_feature_combos.sh` re-runs the whole suite across further axes:

- both cargo feature combinations (`default`, `--no-default-features`);
- both Rust profiles — including the release cdylib with `panic = "abort"`,
  reached via `DRIVER_RUST_SO` because `cargo test --release` cannot run a test
  harness under `panic = "abort"`;
- the C reference rebuilt out-of-tree at `-O0`, `-O1`, `-O2`, `-O3`, `-Os`,
  `-O3 -march=native` and `-Ofast`. This matters because the wrapping-arithmetic
  choice in `fma_array` relies on gcc emitting a plain two's-complement
  `imul`/`add` for what is formally signed-overflow UB; an optimiser is entitled
  to assume that never happens. All seven levels agree with the translation.

## Checklist

All 34 rows are exercised against both `.so`s and pass across their randomised
inputs, under every configuration listed above.

