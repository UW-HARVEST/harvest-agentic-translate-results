# ERRORS.md — differential verification log

Scope: `c_src/src/main.c` (the ground truth) vs `translation/src/main.rs`.

The C program is four lines of behaviour:

```c
float x = 0.f;
scanf("%f", &x);
driver(x);          /* prints the 4 raw bytes of x as %02x, then "\n" */
return 0;
```

It never writes to stderr and always returns 0, so the *only* channel that can
disagree is the 8 hex digits on stdout — i.e. the exact IEEE-754 binary32 bit
pattern that `scanf("%f", ...)` leaves in `x`. All verification effort therefore
went into the `%f` conversion.

## Mismatches found

**None.** Every input class listed below produced byte-identical stdout,
byte-identical stderr (empty) and exit status 0 from both binaries. No change to
`translation/src/main.rs` was required, and nothing in `c_src/` was modified.

This is a genuine "no defects found" result, not an untested one; the evidence is
recorded below so the next reader can re-check it.

## What was compared, and how

Both programs were built and driven as subprocesses over the same stdin, with
stdout, stderr and exit status all asserted (`translation/tests/differential.rs`).

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`

Roughly 1.1 million distinct inputs were compared in total, counting the
in-suite exhaustive/randomised sweeps and additional ad-hoc sweeps run during
investigation (exhaustive over all 4-character strings from a 24-character
grammar-relevant alphabet, ~330k inputs, and randomised binary files).

## Input classes enumerated from the C source

| Class | Examples | Result |
| --- | --- | --- |
| EOF before any conversion (input failure) → `x` stays `0.f` | `""`, `" "`, `"\n"`, whitespace-only, closed stdin, `/dev/null` | identical |
| Matching failure → `x` stays `0.f` | `abc`, `-`, `+`, `.`, `-.`, `e5`, `..`, `0b101`, `\x80\x81` | identical |
| Single item, happy path | `0`, `1`, `-1`, `+1`, `1.5`, `42` | identical |
| Leading whitespace skipped across newlines (`scanf`, not `fgets`) | `"\n\n\n3.5"`, `"  \r\n 42"`, `"\v\f1"`, 100 000 spaces then `2.5` | identical |
| Only the first item read; trailing input ignored | `1 2`, `1\n2`, `3.5abc` | identical |
| Signed zero (sign bit must survive) | `-0`, `-0.0`, `-0e0`, `-0x0p0`, `-1e-400` | identical |
| Decimal with optional integer / fractional part | `.5`, `5.`, `1.`, `1.2.3`, `5..` | identical |
| Exponent present but with no digits (marker not part of the number) | `1e`, `1e+`, `1e-`, `1ex`, `.e5` | identical |
| Hex float | `0x1p3`, `0X1P3`, `0x1.8p1`, `0x.8p1`, `0xABCDEF` | identical |
| Hex with no significand / no exponent digits | `0x`, `0x.`, `0xp1`, `0x1p`, `0x1p+`, `0xz`, `0x1z` | identical |
| `inf` / `infinity`, all case patterns | `inf`, `INF`, `InFiNiTy`, `-infinity`, `inf.5` | identical |
| Partial `inf…` prefixes (glibc treats an incomplete `infinity` suffix as a matching failure, giving `0.f`, **not** `inf`) | `i`, `in`, `infi`, `infin`, `infini`, `infinit`, `INFI` | identical |
| `nan`, with/without n-char-sequence (glibc's scanf discards the payload and yields the default quiet NaN `0x7fc00000`) | `nan`, `-nan`, `nan()`, `nan(abc_123)`, `nan(`, `nan(a b)` | identical |
| Overflow → `±inf` | `1e39`, `1e400`, `3.4028236e38`, `0x1p128`, `1e999999999999999999999` | identical |
| Largest finite | `3.4028235e38`, `0x1.fffffep127` | identical |
| Subnormals and the smallest-subnormal boundary | `1e-40`, `1e-45`, `1.4e-45`, `0x1p-149`, `7.0064923e-46` | identical |
| Underflow → `±0` | `1e-46`, `1e-400`, `0x1p-150`, `0x1p-151` | identical |
| Round-to-nearest-even ties | `16777217`, `1.000000059604644775390625`, `0x1.0000001p0`, plus computed exact midpoints for every binary32 exponent | identical |
| Every binary32 exponent round-tripped from an exact decimal expansion | `{:.80e}` of `(be<<23)\|frac` for all 255 exponents | identical |
| Inputs far larger than any fixed buffer | 100 000 zeros/nines/newlines, `0x` + 100 000 `f` + `p-400000`, `1e` + 100 000 `0` + `5` | identical |
| Significands far wider than 24 bits (sticky-bit handling) | 200- and 5000-hex-digit significands, 2000-digit decimals | identical |
| Non-text input | embedded NULs, all 256 byte values, 4 KiB of NULs, 2 KB of `/dev/urandom` | identical |

## Behaviours that had to be replicated exactly (and were)

These are the places where a naive translation would diverge. They are recorded
because each one is a mismatch that *would* have existed had the translation used
`str::parse` on a line of input:

1. **`x` is pre-initialised to `0.f`.** A matching or input failure is
   indistinguishable from successfully parsing zero, so both must print
   `00000000` — but they must do so for inputs like `abc` where no conversion
   happens at all.
2. **`scanf` is not line-oriented.** Leading whitespace, including any number of
   newlines, is skipped; reading must not stop at the first `\n`, and must not
   require a trailing newline.
3. **A partial `infinity` is a failure, not `inf`.** glibc cannot un-read a
   partially matched word, so `infi` yields `0.f`. Returning `inf` here would be
   the single most likely bug.
4. **`nan(payload)` does not set the NaN payload** under glibc's `scanf` (unlike
   `strtof`); the result is always `0x7fc00000`, with the sign bit applied
   separately for `-nan`.
5. **C99 hex-float syntax is accepted** by `%f`. Rust's `str::parse::<f32>()`
   rejects `0x1p3`, so this needs its own parser, including the
   `0x`-with-no-digits and `p`-with-no-digits failure paths.
6. **Correct rounding at every magnitude**, including gradual underflow into the
   subnormal range and ties-to-even, for significands of unbounded width.
7. **Byte-exact output framing**: `%02x` per byte in native (little-endian)
   memory order, one trailing `\n`, nothing on stderr, exit status 0.

## Completion gate

- [x] both programs build with no errors
- [x] every enumerated input produces identical stdout, stderr and exit status
- [x] `cargo test` passes in `translation/` (18 tests, debug and release)
- [x] no test is disabled, skipped or `#[ignore]`d
- [x] nothing in `c_src/` has been modified
