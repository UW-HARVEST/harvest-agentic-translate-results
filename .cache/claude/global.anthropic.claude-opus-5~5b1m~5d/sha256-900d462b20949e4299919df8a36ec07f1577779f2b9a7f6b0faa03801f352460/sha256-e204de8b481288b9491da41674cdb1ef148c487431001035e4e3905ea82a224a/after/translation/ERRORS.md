# Differential verification log

Ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Candidate: `translation/src/main.rs`, built to `translation/target/{debug,release}/driver`.

Both programs are compared by *running* them: same stdin, then stdout, stderr
and exit status are compared byte for byte (`translation/tests/differential.rs`).

## What the C program actually does

```c
int main() {
    double f = 0.0f;
    scanf("%lf", &f);          // return value ignored
    driver(f);                 // always called
    return 0;                  // always 0
}
void driver(double f) {
    raw_double_t u = {.f = f};
    printf("%llx %a %.4f\n", u.x, f, f);
}
```

Input classes it branches on (all covered by the test suite):

| # | Input class | C behaviour |
|---|---|---|
| 1 | empty stdin / EOF | `scanf` returns EOF, `f` keeps `0.0`, prints `0 0x0p+0 0.0000`, exit 0 |
| 2 | white space only (` \t\n\r\v\f`) | same as (1); `scanf` skips white space across newlines |
| 3 | matching failure (`abc`, `+`, `-`, `.`, `e5`, ...) | nothing converted, `f` stays `0.0`, **exit still 0** |
| 4 | plain decimal (`1`, `-3.14`, `.5`, `5.`) | converted, printed |
| 5 | decimal exponent (`1e10`, `1E-5`) | converted |
| 6 | exponent marker with no digits (`1.5e`, `0e+`) | marker is pushed back, mantissa alone converts |
| 7 | `inf` / `infinity`, any case, any sign | ±infinity; `%a` and `%.4f` both print `inf` |
| 8 | partial infinity (`in`, `infi`, `infinit`) | matching failure → `0.0` |
| 9 | `nan`, `-nan`, `nan(...)` | quiet NaN, sign preserved in the raw bits and in `%a`/`%.4f` |
| 10 | hex float (`0x1p3`, `0x.8p1`, `0xabcdef`) | converted |
| 11 | `0x` with no digit/radix point | matching failure → `0.0` |
| 12 | `0x.` with no hex digits | subject sequence is just `0` → `0.0` |
| 13 | `p` exponent with no digits (`0x1p+`) | pushed back, significand alone converts |
| 14 | overflow (`1e400`, `0x1p1024`) | ±inf |
| 15 | underflow (`1e-400`, `0x1p-1075`) | ±0 (sign preserved: `-0x0p+0`, `-0.0000`) |
| 16 | subnormals (`5e-324`, `1e-320`) | `%a` prints `0x0.<13 hex digits>p-1022`, trailing zeros trimmed |
| 17 | trailing input after the item (`12 34`, `1\n2`) | only the first item is read, the rest is never consumed |
| 18 | non-UTF-8 / NUL bytes on stdin | treated as ordinary non-matching bytes |

Neither program ever writes to stderr and both always exit 0. That is exactly
why the tests assert on stderr and status too: a translation that, say,
panicked on non-UTF-8 stdin would still match on stdout for the happy path.

## Mismatches found

**None.** Across ~22,000 differential probes (the 19 tests in
`tests/differential.rs` plus ad-hoc sweeps used during investigation) stdout,
stderr and exit status were identical for every input. The probe sets were:

* all 18 input classes above, enumerated by hand from the C source;
* all 104 single-bit subnormal bit patterns, both signs;
* significands with trailing zero nibbles at every one of 53 positions, for six
  different exponents (smallest, largest and mid-range);
* ~1,200 random f64 bit patterns fed back in as exact `%a`-style hex literals;
* ~1,200 random decimal literals with exponents in `[-350, 350)`;
* ~1,200 random hex literals with up to 60 significand digits and 40 fraction
  digits (over-long significands, so the sticky/dropped-bits path runs);
* 2,000 random short strings over the alphabet `0-9 + - . eE pP xX infaNt() ws`,
  which is dominated by matching-failure and backtracking paths;
* full walks of the decimal exponent range `-330..-290` and `290..320` and the
  binary exponent range `-1090..-1020` and `1020..1030`;
* 5,000-digit literals, 400-digit fractions, 40-digit exponents, 10,000 leading
  spaces, and raw bytes `\x00`, `\x80`, `\xff`, `\xc3\x28`.

## Hazards that were specifically checked (and hold)

These are the places where a plausible translation *would* have diverged; each
is exercised by a named test so a regression would be caught.

1. **`scanf` failure must not change the exit status.** C ignores `scanf`'s
   return value and still returns 0. A translation that reported a parse error
   and exited non-zero would pass a stdout-only test on good input and fail
   here. `matching_failure_leaves_value_zero`, `empty_and_whitespace_only_input`.
2. **`scanf` skips newlines; it is not line-based.** `"\n\n\n1.25"` must
   convert `1.25`. An `fgets`/`read_line`-based translation returns `0.0`.
   `scanf_reads_across_newlines_and_stops_early`.
3. **`f` keeps its initialiser on failure.** `double f = 0.0f;` — note the `f`
   suffix, which is still exactly `0.0` as a double. Same test as (1).
4. **Only one item is read; the rest of stdin is abandoned.** The Rust reader
   must not slurp all of stdin and reject trailing junk.
   `scanf_reads_across_newlines_and_stops_early`.
5. **Push-back / backtracking.** `1.5e`, `0e+`, `0x1p-` must convert the
   mantissa and un-read the dangling exponent; `infi` must *fail* rather than
   yield infinity. `decimal_exponents_and_backtracking`, `infinity_forms`,
   `hex_float_forms`.
6. **`0x` vs `0x.`** — the first is a matching failure (`0.0` via the
   initialiser), the second converts to `0.0` via the subject sequence `0`.
   Both print the same bytes here, but the paths differ. `hex_float_forms`.
7. **Signed zero.** `-0` must print `8000000000000000 -0x0p+0 -0.0000`; losing
   the sign bit is invisible in `%.4f`-only checks but not in `%llx`.
   `single_plain_values`, `overflow_underflow_and_subnormals`.
8. **NaN sign and payload.** `%llx` exposes the raw bits, so the quiet-NaN
   pattern must be `7ff8000000000000` / `fff8000000000000`, and `%a`/`%.4f`
   must print `nan`/`-nan` (Rust's own `{}`/`{:.4}` never prints `-nan`, which
   is why `main.rs` special-cases it). `nan_forms`.
9. **`%a` formatting details:** lowercase, `0x1.<hex>p+<dec>` with a mandatory
   exponent sign, no `.` when the significand is zero, leading digit `0` and
   exponent `p-1022` for subnormals, and trailing zero nibbles trimmed.
   `sweep_all_single_bit_subnormals`, `sweep_trailing_zero_significands`,
   `sweep_random_hex_bit_patterns`.
10. **`%.4f` rounding, including exact ties.** Dyadic rationals such as
    `0.03125` are exactly halfway at the 4th decimal; glibc rounds to even
    (`0.0312`) under the default rounding mode, and Rust's `{:.4}` agrees.
    Large values must print all integer digits in full (the max double expands
    to 309 digits). `fixed_four_rounding_and_ties`,
    `overflow_underflow_and_subnormals`.
11. **Correct rounding of the string→double conversion**, including the
    subnormal and overflow boundaries, over-long significands (sticky bits) and
    absurd exponents that must saturate to ±inf / ±0 rather than wrap.
    `sweep_boundaries`, `sweep_random_long_hex`, `sweep_random_decimals`.
12. **Non-UTF-8 stdin.** The reader is byte-oriented; a `String`-based reader
    would error or panic on `\x80` / `\xff`. `non_utf8_input`.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # 19 differential tests
```

`tests/differential.rs` builds `c_src` itself with CMake if `c_src/build/driver`
is missing, so `cargo test` alone is sufficient. Nothing in `c_src/` is
modified by the tests other than the generated `build/` directory.
