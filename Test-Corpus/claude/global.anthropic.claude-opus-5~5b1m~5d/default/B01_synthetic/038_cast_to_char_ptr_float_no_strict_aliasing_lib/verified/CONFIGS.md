# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

## Mechanical derivation of the axes

### Full set of public entry points

From `c_src/include/driver.h` (the only public header) — everything it declares:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(float x)` | this is simultaneously the highest AND the lowest-level public entry point — there is no convenience wrapper and no underlying public primitive |

Internal (`static`, not exported, not reachable through any other public symbol):

| function | signature | reachable from public API? |
|----------|-----------|----------------------------|
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | only via `driver`, always with `len == sizeof(float) == 4` and `p` = address of the 4-byte copy of `x` |

So the "call hierarchy" is exactly `driver` → `print_hex` → `printf`, and the
lowest-level entry point available to an external consumer *is* `driver`. Tests
therefore exercise `driver` directly through the `.so` export (there is nothing
lower to reach), and additionally reconstruct `print_hex`'s behaviour indirectly.

### Axis 1 — runtime options / modes / flags

| option / mode / flag | exists? | evidence |
|----------------------|---------|----------|
| any parameter other than `x` | no | `driver.h:27` declares a single `float` parameter |
| global/`static` mutable state, init function, context struct | no | `driver.c` has no file-scope variables; nothing to configure or tear down |
| `#ifdef`-selected behaviour | no | the only preprocessor conditional is the `DRIVER_H_` include guard |
| build-time knobs | one, non-behavioural | `CMakeLists.txt:28` `-fno-strict-aliasing` (affects codegen legality of the `char raw[]` type-pun only, not observable output) |
| Rust cargo features | none | `translation/Cargo.toml` has no `[features]` section and no optional deps |

**There are zero runtime options.** The configuration surface is consequently
driven entirely by the *shape and value* of the single `float` argument, plus the
*call pattern* (how the shared `stdout` stream is used across calls).

### Axis 2 — input shapes the code distinguishes

`driver` performs no value inspection, but the *IEEE-754 class* of the argument
partitions the 32-bit input space into the regions where a translation can
plausibly diverge (canonicalisation, flush-to-zero, x87 vs SSE argument passing,
`%02x` sign-extension of bytes ≥ 0x80). Derived from the byte-level operations
the C actually performs — `memcpy` of 4 bytes, then `p[i]` promoted to `int` for
`%02x`:

| shape axis | distinct values the code's byte-level behaviour ranges over |
|------------|-----------------------------------------------------------|
| float class | +0, −0, subnormal, normal, ±inf, qNaN, sNaN |
| sign bit | 0, 1 |
| exponent field | 0x00 (zero/subnormal), 0x01…0xFE (normal), 0xFF (inf/NaN) |
| mantissa | 0, 1 (min), 0x7fffff (max), arbitrary payload |
| per-byte value | each of the 4 bytes < 0x80 vs ≥ 0x80 (exercises `%02x` on a *signed* `char` copy — `raw` is `char`, cast to `unsigned char*`) |
| digit shape | bytes needing the `0` pad (`< 0x10`) vs not; hex digits `a`–`f` (lowercase) vs `0`–`9` |
| byte order | native only (LE on x86-64); `memcpy`/`to_ne_bytes` must agree |

### Axis 3 — call pattern (shared `stdout` state)

| axis | values |
|------|--------|
| number of calls per capture | 0, 1, 2, many |
| interleaving | C-only run, Rust-only run, and C-then-Rust in the *same* process/stream |
| stream destination | pipe/file-backed fd 1 (fully buffered) — the case where a mismatched buffer would reorder output |

## Configuration-surface table

Cross-product of the axes above, pruned to the combinations the C actually
treats differently. Every row is tested in `translation/tests/valid_paths.rs`
with **many randomised inputs (fixed seed, deterministic SplitMix64)** unless the
row is a singleton bit pattern, comparing the C `.so` and Rust `.so` byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | no options (none exist) + positive zero `0x00000000`; all four bytes 0x00, all-padded digits | [x] |
| C2 | `driver` | negative zero `0x80000000`; sign byte 0x80 ≥ 0x80 → exercises signed-`char` promotion | [x] |
| C3 | `driver` | smallest positive subnormal `0x00000001` … randomised subnormals with exponent field 0x00, mantissa ∈ [1, 0x7fffff], sign 0 | [x] |
| C4 | `driver` | randomised *negative* subnormals (exponent 0x00, sign 1) — top byte ≥ 0x80 | [x] |
| C5 | `driver` | largest subnormal `0x007fffff` and `FLT_MIN` `0x00800000` (the subnormal/normal boundary pair) | [x] |
| C6 | `driver` | randomised positive normals: sign 0, exponent ∈ [0x01,0xFE], random mantissa | [x] |
| C7 | `driver` | randomised negative normals: sign 1, exponent ∈ [0x01,0xFE], random mantissa | [x] |
| C8 | `driver` | small exact integers as floats (`0.0,1.0,2.0,…,±1..±1000` randomised) — the "ordinary consumer" shape | [x] |
| C9 | `driver` | simple fractions/decimals (`0.1, 0.5, 1.5, 3.14159, 1e-10, 1e10`, randomised `f32` from random `f64` division) | [x] |
| C10 | `driver` | `FLT_MAX 0x7f7fffff`, `-FLT_MAX 0xff7fffff`, `FLT_EPSILON 0x34000000`, `FLT_MIN`, `-FLT_MIN` | [x] |
| C11 | `driver` | `+inf 0x7f800000` and `-inf 0xff800000` (exponent 0xFF, mantissa 0) | [x] |
| C12 | `driver` | qNaN / sNaN / signed NaN with randomised 23-bit payloads (exponent 0xFF, mantissa ≠ 0) | [x] |
| C13 | `driver` | inputs chosen so **every** byte value 0x00…0xFF appears in each of the 4 byte positions (drives `%02x` over its whole domain, incl. `a`–`f` lowercase and zero-padding) | [x] |
| C14 | `driver` | fully unconstrained randomised 32-bit patterns reinterpreted as `float` (uniform over the entire input space, 200 000 samples, seeded) | [x] |
| C15 | `driver` | exhaustive sweep of the *structured* space: every exponent 0x00…0xFF × sign × a set of mantissa corner values | [x] |
| C16 | `driver` | call pattern: **zero** calls inside a capture (must produce empty output, no stray newline) | [x] |
| C17 | `driver` | call pattern: single call (baseline; output is exactly 9 bytes = 8 hex digits + `\n`) | [x] |
| C18 | `driver` | call pattern: many sequential calls in one capture — C's N-call output must equal Rust's N-call output *and* the concatenation of individual outputs (shared `stdout` buffering) | [x] |
| C19 | `driver` (both libs) | call pattern: C and Rust `driver` invoked **alternately into the same fd-1 stream** in one process — verifies both write through the same libc `FILE*` and neither buffers independently | [x] |
| C20 | `driver` | fd 1 redirected to a *file* (fully buffered, not line-buffered) with many calls, so a translation using a private buffer flushed at a different time would reorder | [x] |
| C21 | `driver` | structural invariant across all of the above: output is always `^[0-9a-f]{8}\n$` per call, lowercase, no uppercase, no `0x` prefix, exactly 9 bytes | [x] |
| C22 | `driver` | byte-order agreement: output equals `x.to_ne_bytes()` hex — asserted against an independent oracle computed in the test, for randomised inputs | [x] |

## Feature-combination axis

`translation/Cargo.toml` declares no `[features]`, so the complete set of cargo
feature combinations is `{ default }` ≡ `{ --no-default-features }` ≡
`{ --all-features }`. `run_all_features.sh` runs the whole suite under all three
invocations; see `SYMBOLS.md`.

## Phase B gate

All 22 rows checked `[x]` — each passes across its randomised inputs against both
`.so`s. Phase C may proceed.
