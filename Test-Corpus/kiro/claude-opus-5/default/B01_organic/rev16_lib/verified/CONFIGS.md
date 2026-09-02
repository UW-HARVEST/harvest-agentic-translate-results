# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Derived mechanically from the C source: the axes below are the ones the C code
actually distinguishes, not the ones that look interesting.

## Mechanical derivation of the axes

**Public entry points** — the full set, from `c_src/include/lib.h`:

| entry point | signature | level |
|-------------|-----------|-------|
| `rev16` | `uint32_t rev16(uint32_t a)` | lowest-level *and* only; there is no convenience wrapper and nothing beneath it |

There is exactly one exported symbol (see `SYMBOLS.md`), so the "call
hierarchy" is a single node and no composed pipeline exists.

**Runtime options / modes / flags:** none. `rev16` takes no flag argument,
reads no global, reads no environment variable, and holds no state. There is no
init/config/teardown call.

**Compile-time configuration:** none. `c_src/CMakeLists.txt` defines no
`-D` options and `lib.c` contains no `#if`/`#ifdef`.

**Branches the code takes:** none — the body is four unconditional assignments
followed by `return`. Consequently the only axis that can change the result is
the *value* of the single argument.

**Input shapes the code effectively special-cases** — all four masks
(`0xAAAA/0x5555`, `0xCCCC/0x3333`, `0xF0F0/0x0F0F`, `0xFF00/0x00FF`) are 16 bits
wide, so the first statement unconditionally discards bits 16..31. That splits
the 32-bit argument into two structurally distinct halves, giving these shape
axes:

* **low half (bits 0..15)** — fully significant; sub-shapes: zero, all-ones,
  single bit set, byte-aligned patterns, nibble-aligned patterns, alternating
  patterns matching the mask literals, bit-reversal palindromes, arbitrary.
* **high half (bits 16..31)** — entirely discarded; sub-shapes: zero, all-ones,
  arbitrary. Must never affect the result.
* **whole-word interpretations** — values that differ if the 32-bit argument
  were ever treated as signed or narrowed (`INT32_MAX`, `0x8000_0000`,
  `UINT32_MAX`).
* **cardinality of the low half** — empty (0 bits set), one (1 bit set),
  many (2..16 bits set).

## Configuration-surface table

Cross-product of {single entry point} × {the shape axes above}, pruned to the
combinations the code actually distinguishes. Every row is exercised in
`tests/differential.rs` against BOTH `.so` objects via `libloading`, with many
randomized inputs per row (seeded, reproducible xorshift64* PRNG, seed
`0x2545F4914F6CDD1D`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `rev16` | high half = 0, low half = 0 (empty; minimum input) | [x] |
| 2 | `rev16` | high half = 0, low half = `0xFFFF` (all-ones, saturated) | [x] |
| 3 | `rev16` | high half = 0, low half = exactly one bit set, all 16 positions (cardinality "one") | [x] |
| 4 | `rev16` | high half = 0, low half = exactly two bits set, all 120 pairs (cardinality "many", minimal) | [x] |
| 5 | `rev16` | high half = 0, low half = uniformly random 16-bit values (cardinality "many", 20 000 samples) | [x] |
| 6 | `rev16` | high half = 0, low half = alternating masks `0xAAAA`, `0x5555`, `0xCCCC`, `0x3333`, `0xF0F0`, `0x0F0F`, `0xFF00`, `0x00FF` (the literals the C branches its masks on) | [x] |
| 7 | `rev16` | high half = 0, low half = byte-shape inputs: `0x00XX`, `0xXX00` for all 256 `XX` (byte-order / swap axis of stage 4) | [x] |
| 8 | `rev16` | high half = 0, low half = nibble-shape inputs: `0x000X`,`0x00X0`,`0x0X00`,`0xX000` for all 16 `X` (nibble axis of stage 3) | [x] |
| 9 | `rev16` | high half = 0, low half = bit-reversal palindromes (`rev16(x) == x`), all of them enumerated | [x] |
| 10 | `rev16` | high half = 0, **exhaustive** sweep of all 65 536 low-half values | [x] |
| 11 | `rev16` | high half = `0xFFFF` (all-ones discarded half) × low half random (20 000 samples) | [x] |
| 12 | `rev16` | high half = uniformly random × low half = 0 (only discarded bits vary; result must stay `0`) | [x] |
| 13 | `rev16` | high half = uniformly random × low half = uniformly random — full 32-bit random sweep (50 000 samples) | [x] |
| 14 | `rev16` | whole-word boundary interpretations: `0x0000_0000`, `0x0000_0001`, `0x0000_FFFF`, `0x0001_0000`, `0x7FFF_FFFF`, `0x8000_0000`, `0x8000_0001`, `0xFFFF_0000`, `0xFFFF_FFFF` | [x] |
| 15 | `rev16` | walking single bit over the **full** 32-bit word, `1u32 << k` for `k = 0..=31` (crosses the low/high-half boundary) | [x] |
| 16 | `rev16` | invariance / statelessness: same input called repeatedly, and inputs interleaved in random order, compared against the single-call result in both objects | [x] |
| 17 | `rev16` | idempotence structure: `rev16(rev16(x))` composed through the `.so` for random `x` (double-application must agree between C and Rust) | [x] |

All 17 rows pass byte-for-byte between the C `.so` and the Rust `.so`.
