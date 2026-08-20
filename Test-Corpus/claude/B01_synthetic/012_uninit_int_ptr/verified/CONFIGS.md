# CONFIGS.md — configuration-surface table

## Build-time configuration axes (enumerated mechanically)

**Rust `[features]`:** `Cargo.toml` has **no `[features]` section**, so the only
valid feature combination is the empty one:

```
$ grep -n features Cargo.toml   ->  (no match)
```

| # | feature combo | command | status |
|---|---------------|---------|--------|
| 1 | *(none — the only one)* | `cargo check --no-default-features` | clean, 0 warnings |

`--no-default-features` and the default build are the *same* configuration; there
is no `#[cfg(feature = ...)]` anywhere in `src/`, so no backend-specific code
needs gating.

**C `#ifdef` / CMake options:** `CMakeLists.txt` defines no `option()`, no
`target_compile_definitions`, and `main.c` contains no `#ifdef`. `CMAKE_BUILD_TYPE`
is empty and `CMAKE_C_FLAGS` is empty (verified in `CMakeCache.txt`), i.e. the
reference is an unoptimised `-O0` build. **One C configuration.**

Both Rust profiles are covered anyway (`debug` matters because Rust panics on
integer overflow there but wraps in `release`; the `u128` accumulator in
`scan_int` must not overflow in either).

## Runtime configuration axes (derived from the C branches)

The program takes **no arguments, no env vars, no options** — `main()` has no
`argc`/`argv`. The only runtime input is **stdin bytes**. The axes the C actually
branches on:

- **A1 `if (x)`** (line 51) — the single conditional: `x == 0` -> `bad()`,
  `x != 0` -> `good()`.
- **A2 `scanf("%d")` outcome** — conversion performed / matching failure /
  input failure (EOF). Return value discarded, so it only reaches A1 via `x`.
- **A3 leading-whitespace skip** — the 6 C-locale space bytes vs any other byte.
- **A4 optional sign** — `+`, `-`, or absent; sign followed by non-digit.
- **A5 digit accumulation & `strtol` saturation** — in-range / `>LONG_MAX` /
  `<LONG_MIN`.
- **A6 assignment truncation** to `int` — low 32 bits; interacts with A1 because
  a nonzero 64-bit value can truncate to `0` and flip the branch.
- **A7 trailing bytes after the number** — left unconsumed (unobservable, single
  conversion, but must not change the result).
- **A8 stdout/stdin fd state** — open / closed / pipe closed early.

Rows below are the pruned cross-product of these axes — every combination the C
treats differently.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `main`→`bad`→`printIntPtrLine` | A1=`x==0` via plain `"0"` | `cfg_zero_plain` | [x] |
| 2 | `main`→`bad` | A1=0, A5 many leading zeros (`"0000"`, 500 zeros) | `cfg_zero_many_leading` | [x] |
| 3 | `main`→`bad` | A1=0, A4 sign present (`"+0"`, `"-0"`) | `cfg_signed_zero` | [x] |
| 4 | `main`→`bad` | A1=0, A3 leading ws forms (space/tab/nl/vtab/ff/cr, mixed runs) | `cfg_zero_leading_whitespace` | [x] |
| 5 | `main`→`bad` | A1=0, A7 trailing junk (`"0abc"`, `"0 "`, `"0\n9"`) | `cfg_zero_trailing_junk` | [x] |
| 6 | `main`→`good`→`printIntPtrLine` | A1=`x!=0`, smallest nonzero (`"1"`) | `cfg_nonzero_one` | [x] |
| 7 | `main`→`good` | A1≠0, negative (`"-1"`, `"-3"`) | `cfg_nonzero_negative` | [x] |
| 8 | `main`→`good` | A1≠0, A4 explicit `+` (`"+9"`) | `cfg_nonzero_plus_sign` | [x] |
| 9 | `main`→`good` | A1≠0, `INT_MAX`/`INT_MIN` exactly | `cfg_int_boundaries` | [x] |
| 10 | `main`→`good` | A6 truncation: `2^31` (`"2147483648"`) -> `INT_MIN`, nonzero | `cfg_trunc_int_min` | [x] |
| 11 | `main`→`bad` | A6 truncation to **zero**: `2^32`, `2^33`, `m·2^32` for m=1..39 | `cfg_trunc_low32_zero` | [x] |
| 12 | `main`→`good` | A5 positive saturation `>LONG_MAX` -> `LONG_MAX`, low32=`0xFFFFFFFF` | `cfg_saturate_positive` | [x] |
| 13 | `main`→`bad` | A5 negative saturation `<LONG_MIN` -> `LONG_MIN`, low32=`0` | `cfg_saturate_negative` | [x] |
| 14 | `main`→`bad` | A2 matching failure (leading alpha/punct) | `cfg_matching_failure` | [x] |
| 15 | `main`→`bad` | A2 input failure (empty stdin, EOF) | `cfg_input_failure_eof` | [x] |
| 16 | `main`→`bad` | A4 sign then non-digit (`"+"`, `"-"`, `"- 5"`, `"--5"`, `"+a"`) | `cfg_sign_then_nondigit` | [x] |
| 17 | `main`→`bad` | A3 non-locale "blank" bytes (`0x85`,`0xA0`,`0x00`,`0x1C`,`0x1F`) not skipped | `cfg_non_locale_blank_bytes` | [x] |
| 18 | `main`→`bad`/`good` | A5 oversized digit runs: 100/500/1000/4096/20000 digits, signed & unsigned | `cfg_oversized_digit_runs` | [x] |
| 19 | `main`→`bad` | A3 oversized whitespace run (10 000 bytes) then digit / then EOF | `cfg_oversized_whitespace` | [x] |
| 20 | `main`→`bad`/`good` | A5 every power of two `2^0..2^67`, positive and negative | `cfg_all_powers_of_two` | [x] |
| 21 | `main`→`bad` | A2/A8 stdin fd closed | `cfg_stdin_closed` | [x] |
| 22 | `main`→`printIntPtrLine` | A8 stdout closed / pipe closed early (EPIPE vs SIGPIPE) | `cfg_stdout_closed`, `cfg_stdout_epipe_sigpipe_parity` | [x] |
| 23 | `main` | A5 non-decimal prefixes (`"0x10"`,`"0b1"`,`"1e5"`,`"1,5"`,`"1_0"`) | `cfg_non_decimal_prefix` | [x] |
| 24 | whole pipeline | **randomised property sweep**, fixed seed, 8 generators × 6000 inputs (digit runs, ws+sign+digits, arbitrary bytes, ±2^e±δ, long runs) | `prop_random_differential` | [x] |
| 25 | whole pipeline | **exhaustive** small-integer sweep, every decimal `-1000..=1000` | `cfg_exhaustive_small_ints` | [x] |
| 26 | whole pipeline | rows 1–25 replayed against the **debug** Rust build (overflow panics vs wraps) | `--profile dev` run of the suite | [x] |

All 26 rows: **checked and passing** across randomised inputs.
