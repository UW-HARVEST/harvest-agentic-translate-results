# CONFIGS.md — Phase B: the CONFIGURATION-SURFACE TABLE (valid inputs)

The mirror of `ERRORS.md`: every *valid* configuration the C code actually
distinguishes. Rows are derived mechanically from the source, not guessed.

## Axis derivation (what the C actually branches on)

Public header `c_src/include/driver.h` declares exactly one entry point:

```c
void driver(float x);
```

`nm -D` on the C `.so` confirms `driver` is the **only** exported symbol, so the
"full set of public entry points, including the lowest-level ones" is
`{ driver }`. The lowest-level routine, `static void print_hex(unsigned char *p,
int len)`, is not exported; it is reachable only via `driver`, always with
`len == sizeof(float) == 4`. Rows B1–B4 therefore drive `print_hex` *through*
`driver` at every byte value and position, which is the only way a real consumer
can reach it and is exhaustive for its actual input domain.

Axes the code branches on:

| axis | source of the branch | states |
|------|----------------------|--------|
| A1. runtime options / modes / flags | grep for `if` / `switch` / `#ifdef` / parameters other than `x`: **none exist**. The library is stateless and has no configuration API. | 1 (none) |
| A2. loop trip count `len` | `for (i = 0; i < len; i++)` with `len = sizeof(float)` | 1 (always 4) |
| A3. `%02x` conversion of one byte | `printf("%02x", p[i])` — `unsigned char` promoted to `int` | 256 byte values × 4 positions |
| A4. the 32-bit object representation of `x` | `(unsigned char *)&x` — every IEEE-754 binary32 class is a distinct input *shape*: `+0`, `-0`, subnormal, normal, `inf`, quiet NaN, signalling NaN | 7 classes |
| A5. byte order | `&x` is read low address → high address, i.e. native (little-endian on x86-64) | 1 (native) |
| A6. output-stream state of `stdout` | `printf` target: buffering mode (`_IOFBF` / `_IOLBF` / `_IONBF`), and whether one call's 9 bytes straddle the buffer boundary | 3 modes × {straddle, no straddle} |
| A7. call count / sequencing | `driver` is called repeatedly by a consumer; output must concatenate in call order | empty(0) / one(1) / many(N) |

Rows below are the cross-product of A3–A7, pruned to the combinations the code
actually treats differently. Every row is exercised with **many randomised
inputs** (fixed seed `0x5EED_1234_ABCD_0001`, SplitMix64 PRNG, so runs are
reproducible), not one hand-picked value.

## The table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| B1 | `driver` → `print_hex` | **`%02x` domain, exhaustive:** every byte value `0x00..0xFF` placed in byte position 0, then 1, then 2, then 3 (1024 calls). Verifies lowercase, zero-padding, no prefix, no separator, correct address order. | [x] |
| B2 | `driver` → `print_hex` | **Full 32-bit bit-space sweep:** 20 000 uniformly random `u32` patterns via `f32::from_bits` — covers all 7 IEEE classes and all byte combinations jointly. | [x] |
| B3 | `driver` → `print_hex` | **All-bytes-equal patterns:** bits `0xVVVVVVVV` for all 256 `V` (`0x00000000`, `0x01010101`, …, `0xFFFFFFFF`). | [x] |
| B4 | `driver` → `print_hex` | **Byte-boundary patterns:** each of the 4 bytes independently drawn from the boundary set `{0x00,0x01,0x0F,0x10,0x7F,0x80,0x81,0xFE,0xFF}` (9^4 = 6561 calls, exhaustive). Targets `%02x` padding and `unsigned char`-vs-`signed char` promotion at once. | [x] |
| B5 | `driver` | **Shape: `+0.0` and `-0.0`** (bits `0x00000000`, `0x80000000`) — numerically equal, bit-wise different. | [x] |
| B6 | `driver` | **Shape: normal values, positive**, randomised over the whole normal exponent range (`0x00800000..0x7F7FFFFF`), 5 000 samples. | [x] |
| B7 | `driver` | **Shape: normal values, negative** (`0x80800000..0xFF7FFFFF`), 5 000 samples. | [x] |
| B8 | `driver` | **Shape: subnormals**, both signs, randomised mantissa `0x000001..0x7FFFFF`, 5 000 samples. | [x] |
| B9 | `driver` | **Shape: infinities**, `+inf` / `-inf`. | [x] |
| B10 | `driver` | **Shape: quiet NaNs**, both signs, randomised payloads (mantissa MSB set), 5 000 samples. | [x] |
| B11 | `driver` | **Shape: signalling NaNs**, both signs, randomised non-zero payloads (mantissa MSB clear), 5 000 samples — bit preservation through the `xmm0` parameter pass. | [x] |
| B12 | `driver` | **Shape: exact IEEE boundary constants:** `FLT_MIN`, `FLT_MAX`, `FLT_EPSILON`, `FLT_TRUE_MIN`, `1.0`, `-1.0`, `2.0`, `0.5`, `3.0`, `10.0`, `1e-38`, `1e38`, powers of two `2^-149..2^127`. | [x] |
| B13 | `driver` | **Shape: integral-valued and "pretty decimal" floats** (`0`,`1`,…,`1024`, `1e0..1e30`, `0.1`, `0.2`, `1/3`) — the values a real consumer passes. | [x] |
| B14 | `driver` | **Count: exactly one call** on a freshly redirected `stdout` (9 bytes, nothing more, no leading data). | [x] |
| B15 | `driver` | **Count: zero calls** — capture window with no `driver` call must yield exactly 0 bytes from both libraries (guards against constructor/`ctor` side effects in the Rust `cdylib`). | [x] |
| B16 | `driver` | **Count: many calls (10 000) in sequence**, `stdout` fully buffered (`_IOFBF`, 4096) so lines straddle the buffer boundary — verifies concatenation order and that no bytes are lost at the flush point. | [x] |
| B17 | `driver` | **Buffering mode `_IONBF`** (unbuffered): one `write` syscall per `%02x`. Same input set as B1, small sample. | [x] |
| B18 | `driver` | **Buffering mode `_IOLBF`** (line buffered): flush occurs on the `\n` inside `driver`. | [x] |
| B19 | `driver` | **Interleaving with the caller's own writes:** caller writes a marker with `printf` before/after `driver`, sharing the same libc `FILE *stdout` — verifies the Rust `.so` uses the same stream (it imports `printf` from libc) and does not reorder. | [x] |
| B20 | `driver` | **Interleaving C and Rust in one stream:** alternate C `driver` and Rust `driver` calls on the same `stdout`; both halves must produce the identical 9-byte line for the same input, in call order. | [x] |
| B21 | `driver` | **Output-stream target shapes:** `stdout` redirected to (a) a regular file, (b) a pipe, (c) `/dev/null`. The byte stream must be identical in all three. | [x] |
| B22 | `driver` | **ABI/type-shape:** the same 32-bit pattern reached three different ways — `f32::from_bits(bits)`, a `f32` literal, and a value round-tripped through `to_ne_bytes`/`from_ne_bytes` — must yield one identical line, proving no canonicalisation on the argument-passing path. | [x] |
| B23 | `driver` | **Feature configuration:** the crate declares no non-default features, so the single valid combination is the empty feature set; every row above is re-run under `--no-default-features` **and** under the default feature set. | [x] |

## Feature-combination enumeration (Phase A, step 1)

`Cargo.toml` `[features]` contains only `default = []`; `c_src/CMakeLists.txt`
contains no `option()`, no `add_definitions()`, no `target_compile_definitions()`,
and `driver.c`/`driver.h` contain no `#ifdef` other than the header include
guard. The **complete** set of valid build configurations is therefore:

| # | configuration | command |
|---|---------------|---------|
| 1 | empty feature set | `cargo test --offline --no-default-features` |
| 2 | default feature set (identical to #1, verified separately) | `cargo test --offline` |

Both are driven by `./check_features.sh {check,test}` and by `./verify_all.sh`,
which additionally re-runs the whole suite under the `release` profile (where the
optimiser is free to reassociate float moves) — 4 build configurations in total,
each running all 43 cases.

## Randomisation

Every row that says "randomised" draws its inputs from a SplitMix64 PRNG seeded
from the fixed constant `0x5EED_1234_ABCD_0001` XOR'd with the row number, so the
inputs are reproducible run to run yet cover a broad slice of the value space
(~90 000 distinct `driver` invocations per full suite run).

## Negative control

A configuration table is only worth as much as the tests' sensitivity, so
`./mutation_check.sh` deliberately breaks the Rust translation eight different
ways and asserts the suite catches each one:

| mutant | cases that caught it |
|--------|----------------------|
| `printf("%2x")` instead of `"%02x"` (lost zero padding) | 41 |
| `to_be_bytes()` instead of `to_ne_bytes()` (byte order) | 38 |
| quiets signalling NaNs to the canonical NaN | 24 |
| `len = 3` instead of `sizeof(float)` | 41 |
| `printf("%02X")` (uppercase hex) | 39 |
| no trailing newline | 41 |
| signed-char promotion of the byte | 40 |
| `-0.0` folded to `+0.0` | 10 |

`MUTATION CHECK: PASS — every mutant was caught by the suite`.
