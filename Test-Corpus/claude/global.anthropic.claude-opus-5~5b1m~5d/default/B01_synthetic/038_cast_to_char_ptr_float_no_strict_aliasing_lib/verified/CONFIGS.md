# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the C source. The axes below are exactly the ones the
C code branches on — nothing else exists in `c_src/`.

## Enumerating the axes from the source

**Public entry points** (`c_src/include/driver.h`, the only public header):

* `void driver(float x)` — the *only* public entry point. It is simultaneously
  the highest- and lowest-level exported function; there is no convenience
  wrapper layer to skip past.
* `static void print_hex(unsigned char *p, int len)` — internal, `static`, not
  exported (see `SYMBOLS.md`). Its only call site is `driver`, always with
  `len == sizeof(float) == 4` and `p` pointing at a 4-byte local buffer. It is
  therefore exercised transitively via `driver`, with `len` fixed.

**Runtime options / modes / flags:** *none.* Greps over `c_src/` for `#ifdef`,
`#if`, `switch`, setter functions, or global configuration state find nothing:
there is no state, no mode, no flag, no global variable. `CMakeLists.txt` sets
only `-fno-strict-aliasing` (a codegen flag, not a behavioral option) and
defines a single build configuration.

**Control flow the C actually branches on:**

* `for (int i = 0; i < len; i++)` in `print_hex` — the only branch. `len` is
  always 4, so the loop always runs exactly 4 iterations.
* `printf("%02x", p[i])` — `p[i]` is an `unsigned char` promoted to `int`, so the
  branch-relevant input *shapes* are the 256 possible values of each byte
  (notably ≥ 0x80, which a signed-char mistranslation would print as
  `ffffff80`, and < 0x10, which requires the `%02x` zero-padding).
* `memcpy(raw, &x, sizeof(x))` — reinterprets the float's object
  representation, so **byte order** (native little-endian on the test host) and
  the exact bit pattern of the argument are behavioral axes.

**Distinct input shapes the code special-cases:** the argument is a single
scalar `float` passed by value, so the shape axis is the IEEE-754 binary32
*class* of the value plus its exact bit pattern: `+0`, `-0`, positive/negative
subnormal, positive/negative normal, `FLT_MIN`, `FLT_MAX`, `±inf`, quiet NaN,
signalling NaN, NaN with non-canonical payload. There are no lengths, counts,
arrays, element types, or formats — the API accepts no such parameters, so
"empty / one / many" collapses onto "one call / many calls".

## Configuration-surface table

One row per combination the C treats differently (cross-product of
{entry point} × {value class} × {byte values} × {call multiplicity}, pruned to
what the code distinguishes). Every row is verified with many randomized inputs
(fixed seed `0x9E3779B97F4A7C15`) against both `.so`s, byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | positive normal floats, randomized over the whole normal exponent range (10 000 samples) | [x] |
| 2 | `driver` | negative normal floats, randomized (10 000 samples) — exercises the sign bit landing in byte 3 ≥ 0x80 | [x] |
| 3 | `driver` | small-magnitude "everyday" values: randomized floats in `[-1.0, 1.0]` (10 000 samples) | [x] |
| 4 | `driver` | large-magnitude values: randomized floats scaled toward `FLT_MAX` (10 000 samples) | [x] |
| 5 | `driver` | positive subnormals: randomized mantissas with zero exponent (5 000 samples) | [x] |
| 6 | `driver` | negative subnormals: randomized mantissas with zero exponent, sign set (5 000 samples) | [x] |
| 7 | `driver` | signed zeros: `+0.0` and `-0.0` | [x] |
| 8 | `driver` | integral-valued floats from randomized `i32`s cast to `float` (10 000 samples) | [x] |
| 9 | `driver` | `±inf` | [x] |
| 10 | `driver` | quiet NaNs with randomized payloads, both signs (5 000 samples) — payload must survive the FFI boundary unquieted/unmodified | [x] |
| 11 | `driver` | signalling NaNs with randomized payloads, both signs (5 000 samples) | [x] |
| 12 | `driver` | IEEE boundary constants: `FLT_MIN`, `FLT_MAX`, `FLT_EPSILON`, `FLT_TRUE_MIN`, largest subnormal, smallest normal, `1.0`, `-1.0`, `2.0`, `0.5` | [x] |
| 13 | `driver` (→ `print_hex` byte loop) | every byte value 0x00–0xff placed in byte position 0, 1, 2 and 3 (1 024 patterns) — exercises `%02x` zero-padding and `unsigned char` promotion in all 4 loop iterations | [x] |
| 14 | `driver` | unconstrained bit patterns: 200 000 random `u32`s reinterpreted as `float`, covering every value class simultaneously | [x] |
| 15 | `driver` | call multiplicity: one single call (fresh stdout capture), verifying exactly 9 bytes are emitted (8 hex + `\n`) | [x] |
| 16 | `driver` | call multiplicity: 1 000 calls to C then 1 000 identical calls to Rust in a single capture — verifies no hidden state and identical stdout buffering/flush behavior | [x] |
| 17 | `driver` | call multiplicity: C and Rust calls **interleaved** within one capture, asserting the output alternates in identical pairs | [x] |
| 18 | `driver` | output-shape invariant: for randomized inputs, the emitted text is exactly `[0-9a-f]{8}\n` (lowercase hex, zero-padded, native little-endian byte order) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only configuration is the default (empty) feature set. Verified by:

```
$ grep -n '^\[features\]' translation/Cargo.toml   # no match
```

The test suite is nevertheless run under `--no-default-features` as well as the
default build, and both pass (see `run_all.sh`).

## Binary executable

`CMakeLists.txt` builds only `add_library(driver SHARED src/driver.c)` — there is
no `add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]`. **No driver binary exists**, so the "compare C and
Rust binary stdout" gate is not applicable; the equivalent coverage is provided
by rows 15–18, which compare the libraries' actual stdout bytes.
