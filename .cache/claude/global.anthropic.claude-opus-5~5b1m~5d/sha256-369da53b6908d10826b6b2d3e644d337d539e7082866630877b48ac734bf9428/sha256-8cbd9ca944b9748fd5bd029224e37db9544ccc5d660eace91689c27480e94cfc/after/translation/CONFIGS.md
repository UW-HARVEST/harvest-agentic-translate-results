# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches in `c_src/src/lib.c`. There is exactly
one public entry point (`bitwriter_add`), no runtime options, no flags, no
`#ifdef`, and no modes — so the configuration surface is the cross-product of
the **input-state axes the code actually branches on**.

## Axes the C branches on

* **A. `bits` (argument)** — feeds `64 - bits` (shift, may underflow), the loop
  guard `bw->bits + bits >= 64`, the clamp `b > bits ? bits : b`, `bits -= b`,
  `bw->tot += bits`, and the trailing `bw->bits += bits`.
  Distinguished shapes: `0`, `1`, mid (`2..62`), `63`, `64`, `65`, large
  (`100`, `0xFFFF`), `0xFFFF_FFFF` (max), and values chosen so `bw->bits+bits`
  wraps to `< 64`.
* **B. `bw->bits` (incoming state)** — feeds `64 - bw->bits - 1` (may
  underflow), `val >> bw->bits` (shift count masked), the loop guard.
  Distinguished shapes: `0`, mid (`1..62`), `63` (makes `b == 0` → spin until
  `i == 100`), `64` (underflow to `0xFFFF_FFFF` → clamp), `> 64`, `0xFFFF_FFFF`.
* **C. `val` (argument)** — pure data through `<<` / `>>` / `|`. Distinguished
  shapes: `0`, `1`, all-ones, high-bit-only, low-bit-only, alternating,
  random 64-bit.
* **D. `bw->val` (incoming state)** — OR-accumulated and masked with
  `0xFFFF_FFFF_FFFF_FFFE`, so bit 0 is observable state. Shapes: `0`,
  all-ones, odd (bit 0 set → must be cleared by the mask on any loop
  iteration, preserved otherwise), random.
* **E. `bw->tot` (incoming state)** — wrapping `u32` accumulator. Shapes: `0`,
  mid, near-`u32::MAX` (overflow).
* **F. untouched fields `pos`, `len`, `buffer`** — the function never reads or
  writes them; must come back bit-identical (including a non-null garbage
  pointer and a null pointer).
* **G. loop-trip count** — `0` iterations, exactly `1`, several, and exactly
  the `i < 100` cap.
* **H. call sequencing** — a single call vs. a long chain of calls threading
  the same `bw` (the real consumer pattern; catches state-dependent bugs a
  one-shot test cannot).

## Configuration table

Every row is exercised against **both** `.so`s with many randomized inputs
(fixed seed, deterministic xorshift64\*), asserting equality of the return
value **and** all 6 struct fields (compared as raw bytes over the full 32-byte
struct, so padding is covered too).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `bitwriter_add` | zeroed `bw`; `bits = 0`; randomized `val` (shift-by-64 path, loop skipped) | [x] |
| 2 | `bitwriter_add` | zeroed `bw`; `bits = 1`; randomized `val` (minimal normal path) | [x] |
| 3 | `bitwriter_add` | zeroed `bw`; every `bits` in `1..=63`; randomized `val` (full mid sweep) | [x] |
| 4 | `bitwriter_add` | zeroed `bw`; `bits = 63`; randomized `val` | [x] |
| 5 | `bitwriter_add` | zeroed `bw`; `bits = 64`; randomized `val` (loop entered, `64 - bits == 0`) | [x] |
| 6 | `bitwriter_add` | zeroed `bw`; `bits = 65` and `bits = 100`; randomized `val` (`64 - bits` underflows) | [x] |
| 7 | `bitwriter_add` | zeroed `bw`; `bits = 0xFFFF` / `0xFFFF_FFFF` (extreme, long loop → hits `i` cap) | [x] |
| 8 | `bitwriter_add` | `bw->bits` swept over `0..=64`; `bits` swept over `0..=64`; randomized `val`, `bw->val` (full 65×65 cross-product of axes A×B) | [x] |
| 9 | `bitwriter_add` | `bw->bits = 63` (forces `b == 0`, spin to the `i < 100` cap); `bits` in `1..=64`; randomized `val` | [x] |
| 10 | `bitwriter_add` | `bw->bits = 64` exactly (`64-64-1` underflow → clamp `b = bits`); randomized `bits`, `val` | [x] |
| 11 | `bitwriter_add` | `bw->bits > 64` (`65`, `128`, `1000`, `0xFFFF_FFFF`); randomized `bits`, `val` | [x] |
| 12 | `bitwriter_add` | `bw->bits + bits` wraps `u32` to `< 64` (e.g. `0xFFFF_FFFF` + `1`, `0xFFFF_FFC0` + `0x41`) | [x] |
| 13 | `bitwriter_add` | `bw->val` pre-seeded: `0`, `u64::MAX`, odd values (bit-0 mask observability), randomized; combined with loop-taken and loop-skipped `bits` | [x] |
| 14 | `bitwriter_add` | `val` boundary patterns (`0`, `1`, `u64::MAX`, `1<<63`, `0xAAAA…`, `0x5555…`) × `bits` in `{0,1,32,63,64,65}` | [x] |
| 15 | `bitwriter_add` | `bw->tot` near `u32::MAX` (`0xFFFF_FFFF`, `0xFFFF_FFF0`) with `bits` large → wrapping overflow | [x] |
| 16 | `bitwriter_add` | `pos`, `len` set to random non-zero values and `buffer` to a real heap pointer → must be returned untouched | [x] |
| 17 | `bitwriter_add` | `buffer = NULL`, `len = 0`, `pos > len` (inconsistent state) → still untouched, still returns `0` | [x] |
| 18 | `bitwriter_add` | **sequenced/pipeline**: 500 chained calls on one `bw` with randomized `bits` in `0..=64` (realistic bit-packing consumer), comparing state after **every** call | [x] |
| 19 | `bitwriter_add` | **sequenced/pipeline**: 500 chained calls with randomized `bits` in `0..=0xFFFF_FFFF` (adversarial, drives `bw->bits` past 64 and wraps `tot`) | [x] |
| 20 | `bitwriter_add` | fully randomized fuzz: all 3 args + all 5 struct fields random, 200 000 cases | [x] |
| 21 | ABI | `struct tflac_bitwriter` size/align/offsets identical between C and Rust (checked via a C-computed probe + Rust `repr(C)` writes) | [x] |

## Non-axes (explicitly checked to be absent)

No runtime option/flag/mode setter, no byte-order handling, no element-type or
format selection, no `#ifdef` in `c_src/`, and no second entry point. No Cargo
features are declared in `translation/Cargo.toml`, so the only feature
combination is the default (see Phase D).
