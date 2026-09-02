# CONFIGS.md — configuration-surface table

Derived mechanically from `c_src/`. This is the mirror of `ERRORS.md`: it
enumerates the axes of **valid** input the C code actually branches on.

## Axis enumeration (from the source, not from assumptions)

### Runtime options / modes / flags

```
$ grep -nE 'if|switch|#ifdef|#if |#ifndef|extern|static [^v]|global' c_src/src/driver.c
(no matches other than the `static void print_hex` definition)
```

* Public headers declare exactly one function and **zero** options, flags,
  modes, context/handle structs, setters or globals.
* `CMakeLists.txt` sets no `-D` defines; `translation/Cargo.toml` declares
  **no `[features]`** table, so the only Cargo feature combination is the
  default (empty) one. `--no-default-features` is therefore also the empty set
  and is exercised for completeness.
* Conclusion: **the option axis is a single point.**

### Public entry points (full set, including the lowest level)

| entry point | linkage | signature | caller-reachable? |
|-------------|---------|-----------|-------------------|
| `driver` | external (`T` in `nm -D`) | `void driver(float)` | yes — tested via `.so` export |
| `print_hex` | `static` (internal) | `void print_hex(unsigned char *, int)` | no — not in either `.so`'s export table |

`driver` *is* the lowest-level public entry point; there is no convenience
wrapper layered over anything else. `print_hex` is the lower internal layer and
is exercised transitively (and only transitively, as in C) through `driver`.

### Input shapes the code distinguishes

`driver`'s single parameter is a `float`. The code path is straight-line, so the
"shapes" are the classes of the IEEE-754 binary32 encoding plus the classes of
the individual bytes that `%02x` formats:

* IEEE-754 class: `+0`, `-0`, positive/negative subnormal, positive/negative
  normal, `+inf`, `-inf`, quiet NaN, signalling NaN.
* Exponent field: `0x00` (zero/subnormal), `0x01` (smallest normal), mid
  exponents, `0xFE` (largest finite), `0xFF` (inf/NaN).
* Mantissa: all-zero, all-ones, low bit only, high bit only (the qNaN/sNaN
  discriminator), random.
* Sign bit: 0 and 1.
* Per-byte value class driving `%02x`: `0x00`, `0x01..0x0f` (needs zero
  padding), `0x10..0x7f`, `0x80..0xff` (must not sign-extend through the
  `unsigned char` -> `int` variadic promotion).
* Byte position within the 4-byte object representation: 0, 1, 2, 3 (native
  little-endian order on x86-64 must be preserved by `memcpy` /
  `f32::to_ne_bytes`).
* Call multiplicity / stream state: one call, and many calls in sequence into
  the same `stdout` stream (buffering / newline emission must not drift).

## Configuration rows

Cross-product of {single option point} x {entry point `driver`} x {input
shapes}, pruned to the combinations the C actually treats differently.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | no options (none exist); `x = +0.0` and `x = -0.0` (sign bit toggled, all other bits zero) | [x] |
| 2 | `driver` | no options; positive subnormals — `f32::from_bits(1)`, largest subnormal `0x007fffff`, random subnormal mantissas | [x] |
| 3 | `driver` | no options; negative subnormals — sign bit set over row 2's set | [x] |
| 4 | `driver` | no options; smallest normal `f32::MIN_POSITIVE` (`0x00800000`) and its negation | [x] |
| 5 | `driver` | no options; ordinary normal magnitudes, both signs — `1.0`, `-1.0`, `2.0`, `0.5`, `3.14159`, `-2.71828`, integral and fractional values | [x] |
| 6 | `driver` | no options; largest finite `f32::MAX` (`0x7f7fffff`) / `f32::MIN` (`0xff7fffff`) | [x] |
| 7 | `driver` | no options; `+INFINITY` (`0x7f800000`) and `-INFINITY` (`0xff800000`) | [x] |
| 8 | `driver` | no options; quiet NaN — mantissa high bit set, every other payload class (`0x7fc00000`, `0x7fffffff`, random payloads, both signs) | [x] |
| 9 | `driver` | no options; **signalling** NaN — mantissa high bit clear, payload non-zero (`0x7f800001`, `0x7fbfffff`, random, both signs); must pass through with no canonicalisation | [x] |
| 10 | `driver` | no options; exponent-field sweep — all 256 exponent values with fixed sign/mantissa, and boundary exponents `0x00`/`0x01`/`0xFE`/`0xFF` | [x] |
| 11 | `driver` | no options; byte-value coverage — every value `0x00..0xff` placed in byte position 0, 1, 2 and 3 (4 x 256 = 1024 bit patterns) to cover `%02x` zero-padding and non-sign-extension in every position | [x] |
| 12 | `driver` | no options; uniform-random 32-bit patterns reinterpreted as `float` (seeded LCG, 20 000 samples) — value-independent full-domain sweep | [x] |
| 13 | `driver` | no options; exhaustive sweep of a contiguous bit-pattern window (`0x00000000..0x0000ffff`, 65 536 patterns) — dense low-order coverage | [x] |
| 14 | `driver` | no options; exhaustive sweep of high-order bit patterns (`bits << 16` for all 65 536 high halves) — dense exponent/sign coverage | [x] |
| 15 | `driver` (composed pipeline) | no options; **many sequential calls** into one `stdout` stream (500 randomized calls, one capture) — verifies output framing/newline emission does not drift across calls and the streams stay in lockstep | [x] |
| 16 | `driver` | `--no-default-features` build (identical to default: crate declares no features) — all rows above re-run | [x] |
| 17 | `driver` | argument register class: `float` in `%xmm0` vs an integer parameter in `%edi`, pinned against literal expected bytes | [x] |

All rows verified by `./verify.sh`, which rebuilds the C reference, enumerates
the feature combinations from `Cargo.toml`, checks symbol parity, and runs the
25-case differential suite for each combination.
