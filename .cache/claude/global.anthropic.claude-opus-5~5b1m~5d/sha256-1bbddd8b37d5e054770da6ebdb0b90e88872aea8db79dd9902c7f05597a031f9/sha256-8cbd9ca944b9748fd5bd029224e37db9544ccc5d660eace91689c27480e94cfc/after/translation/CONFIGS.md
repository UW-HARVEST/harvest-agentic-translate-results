# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Axes mechanically derived from the C source

`c_src/include/lib.h` + `c_src/src/lib.c` in full: one type and one function.

### Axis 1 — runtime options / modes / flags

```
$ grep -nE '#if|#ifdef|#ifndef|#define|if|switch|enum|flag|mode|option' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:1: #include "lib.h"
c_src/include/lib.h:1: #include <stdint.h>
```

**None.** There are no `#ifdef`s, no `#define`d build knobs, no setter
functions, no context/handle carrying state, no flags argument. The library is
stateless and has exactly one behaviour mode. So this axis has a single value:
`(no options)`.

### Axis 2 — public entry points (full set, lowest level included)

`nm -D --defined-only` on the C `.so` yields exactly one entry point, which is
also the lowest-level one — there is no convenience wrapper / one-shot layer to
mistake for the real API:

| entry point | signature |
|---|---|
| `md5_digest` | `void md5_digest(const tflac_md5 *m, tflac_u8 out[16])` |

There is deliberately **no** `md5_init` / `md5_update` / `md5_finalize` in this
tree — only the digest-serialization step — so "drive the library as a real
consumer" here means: build a `tflac_md5` state, call `md5_digest`, inspect all
16 output bytes.

### Axis 3 — input shapes the code distinguishes

The 16 assignments read the four `tflac_u32` fields at fixed offsets and shift
by 0/8/16/24. The shapes that can therefore change behaviour are:

* **field value class**: zero, all-ones, single-bit-set (each of the 32 bit
  positions), byte-boundary values (`0x000000FF`, `0x0000FF00`, `0x00FF0000`,
  `0xFF000000`), sign-bit `0x80000000`, `0x7FFFFFFF`, and uniformly random.
* **which fields are non-zero**: none / only `a` / only `b` / only `c` /
  only `d` / all four (isolates a wrong field offset or a swapped field).
* **byte order**: the code hard-codes little-endian serialization via shifts,
  so the *value → byte* mapping is the shape under test (a Rust `to_le_bytes`
  vs `to_be_bytes` mistake shows up here).
* **`out` buffer**: pre-fill pattern (0x00 / 0xFF / 0xAA / random) so that any
  byte the C leaves untouched is detectable; exact-16-byte vs larger arena with
  guard bytes (detects over-write past index 15 and before index 0).
* **`out` alignment**: offsets 0..8 inside an over-aligned arena.
* **`m` alignment**: `tflac_md5` placed at offsets 0..8 inside a byte arena
  (the C does 4-byte loads; x86-64 tolerates misalignment).
* **aliasing**: `out` disjoint from `m`, `out == (u8*)m`, and `out` at every
  overlap offset — the C re-loads each field from memory *after* every byte
  store, so overlap is a distinct, observable code path.
* **repetition / statefulness**: one call, and many calls in a row on the same
  buffers (proves the Rust keeps no hidden state).

There is no count/length/element-type/format axis, because the API has no
length parameter, no element-type selector and a fixed 16-byte output.

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is driven through **both** `.so` files via `libloading` and compared
byte-for-byte. Rows marked "randomized" use a fixed-seed xorshift64* PRNG
(seed `0x243F6A8885A308D3`) with the iteration count shown.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `md5_digest` | ABI probe: `sizeof`/`alignof`/field offsets of `tflac_md5` vs the C compiler's own values | [x] |
| 2 | `md5_digest` | all fields `0x00000000`; `out` pre-filled `0xFF` (detects "no write at all") | [x] |
| 3 | `md5_digest` | all fields `0xFFFFFFFF`; `out` pre-filled `0x00` | [x] |
| 4 | `md5_digest` | field-isolation: exactly one of `a`/`b`/`c`/`d` = `0xDEADBEEF`, rest `0` (4 sub-cases; detects swapped/wrong offsets) | [x] |
| 5 | `md5_digest` | byte-isolation: each field set in turn to `0x000000FF`, `0x0000FF00`, `0x00FF0000`, `0xFF000000` (16 sub-cases; detects wrong shift amount / endianness) | [x] |
| 6 | `md5_digest` | single-bit sweep: for each of the 128 bits of the struct, only that bit set (128 sub-cases; exact bit→(byte,bit) map) | [x] |
| 7 | `md5_digest` | boundary values `0x7FFFFFFF`, `0x80000000`, `0x00000001`, `0xFFFFFFFE` in all four fields (cross-product sample) | [x] |
| 8 | `md5_digest` | randomized fields, `out` = exact 16-byte buffer, disjoint from `m`, aligned — 20 000 iterations | [x] |
| 9 | `md5_digest` | randomized fields, `out` pre-filled with random bytes (so unwritten bytes are visible) — 20 000 iterations | [x] |
| 10 | `md5_digest` | randomized fields, `out` inside a 64-byte arena with 0x5A guard bytes before/after the 16-byte window — asserts guards unchanged in both — 10 000 iterations | [x] |
| 11 | `md5_digest` | randomized fields, **`out` misaligned** at every offset 0..=8 in an arena (9 × 2 000 iterations) | [x] |
| 12 | `md5_digest` | randomized fields, **`m` misaligned** at every offset 0..=8 in a byte arena (9 × 2 000 iterations) | [x] |
| 13 | `md5_digest` | randomized fields, **both `m` and `out` misaligned** independently (cross-product of offsets 0..=4, 5 × 5 × 500 iterations) | [x] |
| 14 | `md5_digest` | aliasing: `out == (tflac_u8 *)m` exactly (in-place serialization) — randomized, 5 000 iterations | [x] |
| 15 | `md5_digest` | aliasing: `out` overlapping `m` at every offset `-16..=16` inside one arena, randomized — 33 × 500 iterations (exercises the per-store re-load cascade) | [x] |
| 16 | `md5_digest` | repeated invocation: 1 000 back-to-back calls on the same `m`/`out` pair with fresh random values, comparing after every call (no hidden state, idempotent) | [x] |
| 17 | `md5_digest` | idempotence on a fixed input: the same call made 8 times must yield identical bytes each time in both libraries | [x] |
| 18 | `md5_digest` | multi-threaded: 4 threads × 2 000 randomized calls each through both `.so`s (stateless / reentrant, no shared mutable state) | [x] |
| 19 | `md5_digest` | struct read from an over-large allocation whose trailing bytes are random — asserts the C/Rust read only 16 bytes (extra bytes must not influence output) | [x] |
| 20 | `md5_digest` | feature configuration sweep: rows 1–19 re-run under the default build **and** `--no-default-features` (the crate declares no `[features]`, so these are the only two configurations) | [x] |
