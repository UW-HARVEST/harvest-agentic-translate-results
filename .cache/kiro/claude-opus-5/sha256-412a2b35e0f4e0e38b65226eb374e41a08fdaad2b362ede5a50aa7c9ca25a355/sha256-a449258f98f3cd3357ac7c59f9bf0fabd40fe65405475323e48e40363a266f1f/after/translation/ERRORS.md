# Verification record

Differential verification of `translation/` against `c_src/` (the ground truth).

Environment: glibc 2.34, gcc 11.5.0, rustc 1.97.1, x86-64 Linux. `strtod` and
`pow` behaviour — particularly the `errno` side effects — is libc-specific, so
these findings are tied to that glibc version.

## How it was checked

Three layers, because the top-level program hides most of the detail: every
`double` reaches stdout through `%.2f`, which collapses whole classes of
distinct values (all NaNs, everything below `0.005`) onto the same text.

1. **End-to-end** — both binaries run as subprocesses with identical `argv`;
   stdout, stderr and exit status compared byte for byte. 34,696 cases.
2. **`strtod` in isolation** — a C probe and a Rust probe print
   `(value bits, endptr offset, errno == ERANGE)` for the same byte string.
   487,820 inputs (476,740 non-empty), covering a random lexical grammar fuzz,
   well-formed decimals across the full exponent range, the exact decimal
   expansions of the low subnormals and of the values bracketing `DBL_MIN` and
   `DBL_MAX`, exact midpoints between consecutive subnormals, hex forms across
   every binade, all `inf`/`nan` spellings, and raw random bytes.
3. **`pow` + `%.2f` in isolation** — same idea, printing
   `(result bits, errno, %.2f of result, base and exponent)`. 718,699 operand
   pairs.

`argv[0]` is forced to a fixed string in every comparison (`arg0()` in the Rust
tests, `executable=` in the scratch harness). Without that the `Usage:` message
echoes each binary's own path and can never match — an artefact of the test
setup, not a translation defect.

## Mismatches found

### 1. `nan(<payload>)` mantissa payload was discarded

**Symptom.** 36 of the 476,740 isolated `strtod` inputs differed, all one root
cause. `endptr` and `errno` were already correct; only the returned bits were
wrong.

| input | C | Rust (before) |
|---|---|---|
| `nan(1)` | `7ff8000000000001` | `7ff8000000000000` |
| `-nan(1)` | `fff8000000000001` | `fff8000000000000` |
| `nan(0xdeadbeef)` | `7ff80000deadbeef` | `7ff8000000000000` |
| `nan(0xfffffffffffff)` | `7fffffffffffffff` | `7ff8000000000000` |

**Cause.** `strtod::parse_inf_nan` consumed the parenthesised n-char-sequence to
place `endptr` correctly but never decoded it, always returning the default quiet
NaN. glibc treats the sequence as a bitmask: it runs it through
`strtoull(seq, &end, 0)` and honours the result *only* if that consumes the
sequence in full, then ORs the low 51 bits into the mantissa, leaving the quiet
bit set. Base 0 is what makes the corner cases odd, and all of them were wrong
before the fix:

- `nan(010)` is 8 — a leading `0` selects octal.
- `nan(08)` is 0 — `8` is not an octal digit, so the sequence is not fully
  consumed and the payload is dropped.
- `nan(0x)`, `nan(0xg)`, `nan(1e5)`, `nan(1_2)`, `nan(abc)` are all 0, for the
  same "not fully consumed" reason.
- `nan(0x8000000000000)` is 0 — bit 51 is the quiet bit and falls outside the
  51-bit payload mask.
- `nan(99999999999999999999999)` saturates to `ULLONG_MAX`, so the payload is
  the full 51-bit mask. glibc does **not** leak the `ERANGE` that `strtoull`
  raises here; that was checked explicitly, because a leak would have turned
  this input into a `Range error while converting base ...` in the driver.

**Fix.** `nan_with_payload`, `nan_payload` and `strtoull_base0` in
`src/strtod.rs`; `parse_inf_nan` now decodes the sequence.

**Observability.** This did **not** change the driver's output. `%.2f` prints
every NaN as `nan`/`-nan` from the sign bit alone, and `pow` propagates NaN, so
stdout, stderr and exit status were already identical. It is recorded because
the module is a stated reimplementation of `strtod` and was not one, and because
the near miss above (a payload that overflows `strtoull`) *would* have been
visible had glibc leaked `errno`.

### 2. `main.rs` was not the translation

**Symptom.** No output mismatch. Confirmed by rebuilding the original `main.rs`
in a scratch crate and running all 34,696 end-to-end cases against it: 0
mismatches.

**Cause.** The delivered `main.rs` declared `strtod`, `pow` and
`__errno_location` as `extern "C"` and called libc directly. It therefore could
not disagree with the C program — but `src/strtod.rs`, `src/cpow.rs` and
`src/cfmt.rs` were unreferenced dead code, compiled by nothing and verified by
nothing. Defect 1 was sitting in that dead code.

**Fix.** `main.rs` now declares `mod cfmt; mod cpow; mod strtod;` and routes
through them. `nm -D` confirms the binary no longer imports `strtod`; the only
remaining libm import is `pow@GLIBC_2.29`, which is the same symbol the C binary
imports and is how `f64::powf` is implemented on this target — so the numeric
result of `pow` is identical by construction, and only the `errno` reporting
around it is reimplemented.

## Behaviours confirmed correct (each one is easy to get wrong)

- **`""` is accepted as `0.0`.** `strtod("")` performs no conversion and leaves
  `endptr == nptr`, so `*endptr` is the terminator and the C code's
  `*endptr != '\0'` test passes. `driver "" ""` prints `Result: 1.00`, exit 0.
  `" "` is *rejected*, because `endptr` is reset to the start and `*endptr` is
  the space.
- **`ERANGE` is checked before the trailing-character test.** `1e400xyz`
  reports `Range error while converting base`, not `Invalid numeric input`.
  Reversing the two checks is caught by the test suite.
- **`strtod` sets `ERANGE` on gradual underflow, `pow` does not.**
  `strtod("1e-320")` fails even though the value is a representable subnormal
  (tininess after rounding *and* inexactness), while `pow(2, -1030)` returns a
  subnormal with `errno == 0`. `pow` only reports `ERANGE` once the result
  reaches zero — the cliff is at `pow(2, -1075)`. The two rules are opposite and
  both are exercised.
- **Underflow needs inexactness.** `strtod("0x1p-1023")` is exact, so no
  `ERANGE`; `strtod("1e-320")` is inexact, so `ERANGE`. The tininess threshold
  `2^-1022 - 2^-1076` hard-coded in `strtod.rs` was recomputed exactly with
  `fractions.Fraction`: all 769 digits and the exponent `-307` match.
- **`%.2f` ties.** Values like `0.125` and `-1024.875` are exact ties at the
  third decimal; glibc rounds half to even and Rust's `{:.2}` agrees. Rust
  differs only on NaN (`NaN` vs `nan`/`-nan`), which `cfmt.rs` special-cases.
  `-0.0` prints `-0.00` in both.
- **Domain check precedes overflow.** `pow(-2, 1e300)` is `EDOM`, not `ERANGE`;
  exponents past `2^53` are all integral, so `pow(-2, 1e300)` is *not* a domain
  error while `pow(-2, 2.5)` is.
- **Non-UTF-8 `argv`.** The diagnostics echo the raw bytes through `%s`, so the
  translation handles arguments as bytes; `String`-based handling would corrupt
  them.
- **`argc != 3` short-circuits everything.** Invalid arguments are never
  inspected when the count is wrong.

## Result

Both programs build without errors. `cargo test` passes in both the `test` and
`release` profiles (21 tests, none `#[ignore]`d, none skipped). Nothing in
`c_src/` was modified — only `c_src/build/`, the CMake output directory, was
added.
