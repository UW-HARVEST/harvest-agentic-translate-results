# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

## How the axes were derived

From `c_src/include/driver.h` and `c_src/src/driver.c` only.

**Public entry points (from the header).** Exactly one:

```c
void driver(double f);
```

There is no init/teardown, no context object, no setter, no lower-level variant,
and no convenience wrapper — so "exercise the low-level entry points, not just
the wrappers" collapses to: `driver` *is* the low-level entry point, and it is
the only symbol the `.so` exports (see `SYMBOLS.md`). The three units the C
composes internally are the three conversions of the single `printf` call, and
the only way to reach them is through `driver`. The Rust helpers
(`format_llx`, `format_hex_double`, `format_fixed_4`, `render`, `write_stdout`)
are private and deliberately **not** called directly by the tests — everything
goes through the `.so` export.

**Runtime options / modes / flags.** None. `grep` for `if` / `switch` / `#if` in
the code region returns 0 hits (see `ERRORS.md`), and the function takes no flag
parameter. So the configuration surface is *entirely* the shape of the single
`double` argument, plus the two ambient process settings glibc's conversions
genuinely branch on.

**Axes the C actually distinguishes** (all branching lives in the glibc
conversions the C selects with its format string `"%llx %a %.4f\n"`):

| axis | values the code treats differently |
|------|------------------------------------|
| `sign` | sign bit clear / set (affects `%llx` digit count, `%a` leading `-`, `%.4f` leading `-` — including on zero and NaN) |
| `expfield` | `0` (zero + subnormal: `%a` leading digit `0`, exponent pinned to `-1022`) · `1` (first normal, also `p-1022`) · `2..0x3fe` (`p-` exponents) · `0x3ff` (`p+0`) · `0x400..0x7fd` (`p+` exponents) · `0x7fe` (`p+1023`, max normal, must not be read as inf) · `0x7ff` (inf when mantissa 0, NaN otherwise) |
| `mantissa` | `0` (`%a` emits **no** radix point) · trailing zero nibbles (trimmed) · low nibble set (all 13 digits kept) · all ones · arbitrary |
| `llx width` | 1 … 16 significant hex digits; no padding, no leading zeroes; `0` prints as `0` |
| `%.4f` magnitude | `0` · `(0, 5e-5)` → `0.0000` · `[5e-5, 1)` · `[1, 1e4)` · `[1e4, 1e15)` · `[1e15, 1e17)` (fraction always exactly `.0000`) · `[1e100, 1e200)` · `[1e300, DBL_MAX]` (309 integer digits) |
| `%.4f` rounding | below the 4-digit boundary · above it · exactly-representable tie with **even** 4th digit · exactly-representable tie with **odd** 4th digit · carry propagation into the integer part |
| stream state | fresh buffer vs. mid-line buffered `FILE*` across repeated calls |
| ambient (not settable through `driver.h`) | `LC_NUMERIC` decimal point; FP rounding mode — glibc's `%a`/`%.4f` read both |

Rows below are the pruned cross-product: one row per combination the C treats
differently. **Every row is exercised with many randomized inputs from a fixed
seed** (`SEED = 0x5EED_D1FF_2025`, splitmix64), not a single hand-picked value;
the fixed special values that define a row's boundary are included in that row's
input set. Total values compared: see the test output summary.

## Table

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | `expfield=0, mantissa=0`, both signs — `+0.0` / `-0.0` (exhaustive, 2 values) | [x] |
| 2 | `driver` | `expfield=0, mantissa≠0` random, sign `+` — positive subnormals (4096 values) | [x] |
| 3 | `driver` | `expfield=0, mantissa≠0` random, sign `-` — negative subnormals (4096 values) | [x] |
| 4 | `driver` | `expfield=0, mantissa` = exactly one bit set, all 52 positions × both signs (exhaustive, 104) | [x] |
| 5 | `driver` | `expfield=0, mantissa` with 1..12 trailing zero nibbles → `%a` fraction trimming inside the subnormal branch | [x] |
| 6 | `driver` | `expfield=1` (min normal, `p-1022`), `mantissa ∈ {0, random}`, both signs — same exponent as rows 2–5 but leading digit `1` | [x] |
| 7 | `driver` | `expfield ∈ [1,0x7fe]` uniform random, `mantissa` uniform random, random sign — broad normal-range property sweep (200 000 values) | [x] |
| 8 | `driver` | `mantissa=0`, `expfield` exhaustive `1..0x7fe` × both signs — every power of two; `%a` has no radix point (4092 values) | [x] |
| 9 | `driver` | `mantissa=0xF_FFFF_FFFF_FFFF` (all ones), `expfield` exhaustive `1..0x7fe` × both signs (4092 values) | [x] |
| 10 | `driver` | `mantissa` low nibble nonzero (no trimming, interior zeroes preserved), random `expfield`, both signs | [x] |
| 11 | `driver` | `mantissa` with exactly *k* trailing zero nibbles for k = 1..12, random high part, random `expfield`, both signs — trimming boundary, exhaustive in k | [x] |
| 12 | `driver` | `expfield=0x3ff` (`p+0`), `mantissa` random — values in `[1,2)`; exponent sign must print as `+0` | [x] |
| 13 | `driver` | `expfield=0x3fe` (`p-1`), `mantissa` random — values in `[0.5,1)`; the `p+`/`p-` branch boundary | [x] |
| 14 | `driver` | `expfield=0x7fe` (`p+1023`, max normal), `mantissa ∈ {0, all-ones, random}`, both signs | [x] |
| 15 | `driver` | `expfield=0x7ff, mantissa=0` — `±inf` (exhaustive, 2 values) | [x] |
| 16 | `driver` | `expfield=0x7ff, mantissa≠0` — NaN: quiet, signalling, min payload `1`, max payload, random payloads, both signs | [x] |
| 17 | `driver` | `%.4f` magnitude `(0, 5e-5)` — everything rounds to `0.0000` / `-0.0000` | [x] |
| 18 | `driver` | `%.4f` magnitude `[5e-5, 1)` random | [x] |
| 19 | `driver` | `%.4f` magnitude `[1, 1e4)` random | [x] |
| 20 | `driver` | `%.4f` magnitude `[1e4, 1e15)` random | [x] |
| 21 | `driver` | `%.4f` magnitude `[1e15, 1e17)` — integral crossover, fraction always exactly `.0000` | [x] |
| 22 | `driver` | `%.4f` magnitude `[1e100, 1e200)` — long exact digit strings | [x] |
| 23 | `driver` | `%.4f` magnitude `[1e300, DBL_MAX]` — up to 309 integer digits, both signs | [x] |
| 24 | `driver` | `%.4f` exactly-representable ties `k/2^n` whose 5th fraction digit is `5` and rest zero — **even** 4th digit (round down) and **odd** 4th digit (round up), exhaustive over the generated family | [x] |
| 25 | `driver` | `%.4f` near-ties: `nextafter` neighbours (±1 ulp, ±2 ulp) of the nearest double to each of many `k/10000 + 5/100000` boundaries | [x] |
| 26 | `driver` | `%.4f` nearest doubles to `k/10000` for random `k` — 4-digit fractions that are *almost* exact | [x] |
| 27 | `driver` | integral doubles `(double)i`, `i` random in `[-2^53, 2^53]` — fraction exactly `.0000`, wide `%llx` | [x] |
| 28 | `driver` | `%llx` width axis: bit patterns with exactly 1..16 significant hex digits (boundaries `0x0`, `0xF`, `0x10`, `0xFF`, … `0xFFFF_FFFF_FFFF_FFFF`) plus randoms within each width — exhaustive in width | [x] |
| 29 | `driver` | `%llx` with the sign bit set (leading nibble `8..f`, always 16 digits) vs. clear, at otherwise identical low bits | [x] |
| 30 | `driver` | whole-domain unbiased fuzz: uniform random `u64` reinterpreted as `double` (500 000 values) — hits every class above at its natural density, including patterns no hand-written case would pick | [x] |
| 31 | `driver` | `expfield` swept exhaustively `0..0x7ff` × both signs × mantissa ∈ {0, 1, all-ones, random} — every exponent value including both specials (16 384 values) | [x] |
| 32 | `driver` | stream-state axis: N successive `driver` calls inside **one** buffered stdout capture (no intervening flush), values drawn from all classes — verifies the composed output stream, not just one call | [x] |
| 33 | `driver` | ambient: default `C` locale + `FE_TONEAREST` — the specified configuration under which rows 1–32 run | [x] |
| 34 | `driver` | ambient axis: `LC_NUMERIC` switched to a comma-decimal locale (glibc `%a`/`%.4f` use the locale decimal point) | [x] |
| 35 | `driver` | ambient axis: FP rounding mode set to `FE_UPWARD` / `FE_DOWNWARD` / `FE_TOWARDZERO` (glibc `%.4f` and `%a` honour it) | [x] |

## Notes on rows 34–35

Neither `LC_NUMERIC` nor the FP rounding mode is reachable through
`driver.h` — they are process-wide ambient state, not parameters of the API.
They are listed because glibc's conversions *do* branch on them, so a caller
that changes them can observe a difference.

**Both were real divergences, and both were fixed in the Rust.** They are the
only divergences this verification found:

1. **Locale decimal point.** glibc uses the `LC_NUMERIC` decimal point as the
   radix character for BOTH `%a` and `%.4f`. Under `de_DE.utf8` the C prints
   `400921fb54442d11 0x1,921fb54442d11p+1 3,1416`; under `ps_AF.utf8` the radix
   is the two bytes `d9 ab` (U+066B), so it is a byte *string*, not a `char`.
   The Rust had `.` hard-coded. Fixed by reading `localeconv()->decimal_point`
   on every call and emitting those bytes verbatim (the output line is now built
   as a `Vec<u8>`, since the radix bytes need not be ASCII).
2. **FP rounding direction.** glibc's `%.4f` honours `fegetround()`, applied to
   the *signed* value: `0.00012345` prints `0.0002` under `FE_UPWARD` but
   `-0.00012345` prints `-0.0001`; `0.09375` prints `0.0937` under
   `FE_DOWNWARD`. Rust's `{:.4}` always rounds half-to-even and ignores the
   mode. Fixed by replacing `format!("{:.4}", f)` with exact integer arithmetic:
   the value is decomposed to `M * 2^E`, scaled by `10^4` with a small
   `Vec<u64>` bignum, and the remainder is compared against a half via bit
   inspection, then rounded per `fegetround()` (`FE_TONEAREST` and any
   unrecognised mode → half-to-even, matching glibc's `default:` case).
   `%a` needs no such treatment: it is exact at full precision, and the C's
   output was confirmed identical across all four modes.

Rows 34–35 are verified individually, as a 24-combination cross product, and —
for the locale axis — mechanically across **every locale installed on the
system** (`cfg_34d_every_installed_locale`, 869 candidates, 866 loadable),
rather than a hand-picked list.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the full set of feature
combinations is `{default}` = `{}`. Rows 1–35 are therefore already "every
combination". `scripts/check_features.sh` re-derives this from `cargo metadata`
rather than trusting the assertion, and checks both `--all-targets` and
`--no-default-features --all-targets`.

## Binary executable

Neither side builds one, so there is no binary stdout comparison to make:
`c_src/CMakeLists.txt` has `add_library(driver SHARED ...)` and no
`add_executable`; `Cargo.toml` has `crate-type = ["cdylib"]`, no `[[bin]]`, no
`src/main.rs`, and no `src/bin/`. `scripts/verify.sh` asserts this mechanically
and fails if either ever grows an executable. All output the library produces is
compared byte-for-byte through the `.so` exports instead, which is where it
originates.

## How rows are executed

| test binary | rows | how |
|-------------|------|-----|
| `tests/smoke.rs` | — | harness plumbing (both `.so`s load and export `driver`) |
| `tests/differential.rs` | 1–32 | one `#[test]` per row |
| `tests/ambient.rs` | 33–35 | plus `34b`/`34c` (comma and multi-byte radix pinned against literal glibc text), `34d` (all installed locales), `35b` (each rounding mode pinned against literal glibc text), and the 34×35 cross product |
| `tests/errors.rs` | `ERRORS.md` 1–33 | plus two `#[ignore]`d soak tests |

Both `.so`s are loaded with `libloading` and called only through their exported
`driver` symbol; the Rust implementation is never called directly, so the
`#[no_mangle] extern "C"` wrapper and the C calling convention are part of what
is under test. Output is captured by pointing fd 1 at an anonymous `memfd`;
both libraries write through the *same* process-wide glibc `stdout` `FILE`.

## Totals actually compared

| run | values | result |
|-----|--------|--------|
| rows 1–32 (`differential.rs`) | ≈ 830 000 | all match |
| rows 33–35 (`ambient.rs`), incl. 24 locale×mode combos and 866 locales | ≈ 250 000 | all match |
| `ERRORS.md` rows (`errors.rs`) | ≈ 210 000 + process-outcome checks | all match |
| `soak_20m_random_values` (`--ignored`, release) | 20 000 000 uniform random `u64` | all match, 72 s |
| `soak_exhaustive_mantissa_slices` (`--ignored`, release) | 16 × 2²² = 67 108 864 **exhaustive** over contiguous mantissa slices at exponent fields 0, 1, 0x3fd, 0x3fe, 0x3ff, 0x400, 0x7fe, 0x7ff × both signs | all match, 151 s |

Every run was performed against both the debug and the release `cdylib`
(the release profile is `panic = "abort"`, so it is also the configuration in
which a stray panic would be fatal rather than caught).

