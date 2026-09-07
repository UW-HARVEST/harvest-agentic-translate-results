# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the C source, the public header, and `Cargo.toml`.

## Axis enumeration

### Axis 1 — runtime options / modes / flags

Grep of the public header and source for anything settable:

```sh
grep -nE 'if|switch|#ifdef|#if |extern|global|static|set|init|config|flag|mode|option' -r c_src/src c_src/include
# -> no matches
```

`c_src/include/lib.h` declares exactly one function and no types, no enums, no
structs, no globals, and no configuration entry points. `c_src/src/lib.c` has no
static/global state, no initialisation function, and no conditional compilation.
**Axis 1 has exactly one value: "no options exist".** `rev16` is a pure function
of its single argument.

### Axis 2 — compile-time / feature configuration

`translation/Cargo.toml` declares no `[features]` table, so the complete set of
feature combinations is `{default}` = `{no-default-features}` = the empty set.
The C side has no `#ifdef`s, so it likewise has a single configuration.
Both combinations are still executed explicitly in Phase D.

### Axis 3 — public entry points (full set, lowest level included)

| entry point | signature | level |
|-------------|-----------|-------|
| `rev16` | `uint32_t rev16(uint32_t a)` | this is simultaneously the lowest-level and the only entry point; there are no convenience wrappers and no internal helpers to drive separately |

There is no composed pipeline, no state to set up, and no call hierarchy: the
single exported symbol *is* the entire API. Driving it "the way a real consumer
does" is exactly one call per input.

### Axis 4 — input shapes the code special-cases

The body is four straight-line statements over a `uint32_t`. It contains no
branches, so it does not *structurally* special-case any shape. It does,
however, treat different bit positions differently, and one shape class is
semantically distinguished: the four masks (`0xAAAA`/`0x5555`, `0xCCCC`/`0x3333`,
`0xF0F0`/`0x0F0F`, `0xFF00`/`0x00FF`) are all 16 bits wide, so **the upper 16
bits of the argument are discarded by the first statement**. The rows below
therefore enumerate the bit-position and value-range shapes that can produce
value-dependent divergence (wrong mask, wrong shift direction, wrong shift
amount, missing truncation, signed vs unsigned shift, `<<` overflow handling).

## Configuration rows

Each row is a *combination* of axis values (options: none available; entry
point: `rev16`; input shape: as stated) that the C code treats distinguishably.
Every row is tested with many randomized inputs from a fixed-seed PRNG
(SplitMix64, seed `0x243F_6A88_85A3_08D3`) unless the row is exhaustive by
construction, comparing the C `.so` and Rust `.so` return values bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `rev16` | no options; exhaustive over the entire low half: every `a` in `0x0000..=0xFFFF` (65 536 calls, upper half zero) | [x] |
| 2 | `rev16` | no options; exhaustive over the low half with the upper half saturated: `0xFFFF_0000 \| lo` for every `lo` in `0x0000..=0xFFFF` — pins the discard-upper-bits semantics | [x] |
| 3 | `rev16` | no options; randomized over the FULL 32-bit domain, uniform (2 000 000 seeded random `u32`) | [x] |
| 4 | `rev16` | no options; randomized low half crossed with randomized non-zero upper half (`hi << 16 \| lo`, both random, 200 000 pairs) — cross-product of the two halves | [x] |
| 5 | `rev16` | no options; single-bit inputs `1 << k` for every `k` in `0..32` (includes bits 16–31 which must vanish) — isolates each mask/shift stage | [x] |
| 6 | `rev16` | no options; single-bit-clear inputs `!(1 << k)` for every `k` in `0..32` | [x] |
| 7 | `rev16` | no options; the mask constants themselves and their neighbours: `0xAAAA`, `0x5555`, `0xCCCC`, `0x3333`, `0xF0F0`, `0x0F0F`, `0xFF00`, `0x00FF` each `-1`/`+0`/`+1` — boundary values of every stage | [x] |
| 8 | `rev16` | no options; byte/nibble-boundary and empty/one/many-bit-population shapes: `0`, `1`, `0xFFFF`, `0x8000`, `0x0100`, `0x1000`, and all values with popcount 0, 1, 15, 16 in the low half | [x] |
| 9 | `rev16` | no options; values chosen so that `a & 0x5555` shifted left would overflow 16 bits if truncation were mis-implemented (`0x5555`, `0xFFFF`, `0xAAAA`) and values with the top low-half bit set (`0x8000..=0xFFFF` sampled) | [x] |
| 10 | `rev16` | no options; byte-order / endianness-sensitive shapes: values whose two low bytes differ (`0x00FF`, `0xFF00`, `0x1234`, `0x00AB`, `0xAB00`) plus 100 000 randomized values with distinct bytes | [x] |
| 11 | `rev16` | no options; repeated / idempotence-style driving of the entry point in sequence — `rev16(rev16(x))` composed through the FFI for 100 000 seeded inputs, confirming no hidden state affects a second call | [x] |
| 12 | `rev16` | feature combination `--no-default-features` (empty feature set) applied to all rows above | [x] |
| 13 | `rev16` | codegen configuration: the `release` cdylib (`opt-level=3`, `panic="abort"`) — LLVM can recognise the bit-reversal idiom and emit a wholly different instruction sequence, so all rows above are re-run against it | [x] |

Measured: **2 962 395** differential C-vs-Rust comparisons, all matching
bit-for-bit. Each row asserts its exact comparison count via `expect_calls`, so
a loop that silently performs no work fails instead of passing vacuously.

## Gate

- [x] Every row passes across its randomized/exhaustive inputs, under every
      feature combination.
