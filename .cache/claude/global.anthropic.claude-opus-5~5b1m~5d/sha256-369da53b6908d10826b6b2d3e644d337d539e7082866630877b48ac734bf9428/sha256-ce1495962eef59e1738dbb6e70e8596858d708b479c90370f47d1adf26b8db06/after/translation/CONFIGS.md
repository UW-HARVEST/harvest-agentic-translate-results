# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Public entry points (complete set)

`nm -D` gives exactly one: **`bitwriter_add`**. There are no convenience wrappers
and no higher-level API, so the "lowest-level entry point" and the "full API" are
the same function. Driving it "the way a real consumer does" therefore means
driving the **struct state** across sequences of calls, because
`tflac_bitwriter` is the accumulator that carries state between calls
(`val`, `bits`, `tot` are all read-modify-write).

## Axes the C actually branches / varies on

There are only two syntactic branch sites, but both are value-dependent, and the
body is dominated by UB-shift and wraparound behaviour that is value-dependent too:

* **A1 — `while ((bw->bits + bits >= 64) && i < 100)`** (line 11). 32-bit unsigned
  sum, compared against 64. Outcomes: (a) not entered, (b) entered and drains
  naturally, (c) entered and terminates on the `i < 100` cap, (d) not entered
  because the 32-bit sum *wrapped* below 64.
* **A2 — `b = b > bits ? bits : b`** (line 13, `cmovbe`, unsigned). Outcomes:
  `b` clamped to `bits`, or `b` kept.
* **A3 — `bits` (argument, `tflac_u32`)**: shape classes `0`, `1..63`, `64`,
  `65..127`, `>=128`, `0x80000000`, `u32::MAX`. Selects the line-8 shift count
  `64 - bits` (mod 64).
* **A4 — `bw->bits` (state, `tflac_u32`)**: `0`, `1..62`, `63` (the no-progress
  value), `64`, `65..127`, huge, `u32::MAX`. Selects `b = 63 - bw->bits` (mod 2^32)
  and the `val >> bw->bits` shift counts (mod 64).
* **A5 — `val` (argument, `tflac_u64`)**: `0`, `u64::MAX`, single-bit, low-bits-only,
  high-bits-only, random. Interacts with `|=`.
* **A6 — `bw->val` (state, `tflac_u64`)**: `0`, `u64::MAX`, random. `|=` accumulates
  and `&= 0xFFFFFFFFFFFFFFFE` (line 16) clears bit 0 *inside* the loop only, so bit 0
  can only survive via the post-loop `|=` on line 21.
* **A7 — `bw->tot` (state, `tflac_u32`)**: `0`, mid, `0xFFFFFFFF` (wrap on line 9).
* **A8 — untouched fields `pos`, `len`, `buffer`**: never read/written by the C.
  Must be preserved bit-exactly; also fixes the struct ABI (size 32, align 8,
  offsets 0/8/12/16/20/24).
* **A9 — call multiplicity**: 1 call vs. a long sequence of calls against one
  accumulator (the realistic bit-packing pipeline; exposes composed-state drift
  that per-call tests miss).

There are **no** runtime options, modes, flags, `switch`es, `#ifdef`s, byte-order
choices or element-type choices anywhere in the C source — greps for
`if (`/`switch`/`#ifdef`/`#if ` return no matches, and the header exposes no
setters. So the configuration surface is exactly the cross-product of the data
shapes above, pruned to combinations the code distinguishes.

Every row is driven with **many randomized inputs (fixed seed, SplitMix64)**, not a
single hand-picked value, and asserts the return value **plus all six struct fields**
byte-for-byte between the C `.so` and the Rust `.so`.

## Configuration table

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|-------------------------------------------|-----|
| C1  | `bitwriter_add` | **A1(a) fast path, loop never entered**: `bw->bits + bits < 64`; `bw->bits` random `0..62`, `bits` random `0..(63-bw->bits)`, `val`/`bw->val` random | [x] |
| C2  | `bitwriter_add` | **A1(b) loop entered, drains naturally**: `bw->bits` random `0..62`, `bits` random such that sum `>= 64` and `bw->bits != 63` | [x] |
| C3  | `bitwriter_add` | **A1(c) loop hits the `i < 100` cap** via `bw->bits == 63`, `bits` random `1..=u32::MAX` (`b == 0`, no progress) | [x] |
| C4  | `bitwriter_add` | **A1(c) cap via `bits == 0` and `bw->bits >= 64`** (`b` clamped to 0, no progress) | [x] |
| C5  | `bitwriter_add` | **A1(d) 32-bit sum wraps below 64** so the loop is skipped: `bw->bits` near `u32::MAX`, `bits` chosen so `bw->bits + bits` wraps to `< 64` | [x] |
| C6  | `bitwriter_add` | **A2 `b` clamped to `bits`** (`bits < 63 - bw->bits`) — reached with `bw->bits >= 64` where `63 - bw->bits` wraps huge, so the ternary always takes `bits` | [x] |
| C7  | `bitwriter_add` | **A2 `b` kept (not clamped)**: `bits >= 63 - bw->bits`, `bw->bits` random `0..62` | [x] |
| C8  | `bitwriter_add` | **A3 `bits == 0`** (line-8 shift count 64, UB→mod 64) × `bw->bits` random `0..63` | [x] |
| C9  | `bitwriter_add` | **A3 `bits == 64`** (shift count 0) × `bw->bits` ∈ {0, 1, 32, 62, 63, 64} | [x] |
| C10 | `bitwriter_add` | **A3 `bits` in `1..=63`** (the fully valid/meaningful range) × `bw->bits` random `0..=63`, exhaustive over `bits` | [x] |
| C11 | `bitwriter_add` | **A3 `bits` in `65..=127`** (`64 - bits` wraps) × `bw->bits` random | [x] |
| C12 | `bitwriter_add` | **A3 `bits` >= 128 / `0x80000000` / `u32::MAX`** (oversized) × `bw->bits` random | [x] |
| C13 | `bitwriter_add` | **A4 `bw->bits == 0`** (empty accumulator) × `bits` swept `0..=64` | [x] |
| C14 | `bitwriter_add` | **A4 `bw->bits` in `1..=62`** (partially filled) × `bits` random — exhaustive over `bw->bits` | [x] |
| C15 | `bitwriter_add` | **A4 `bw->bits == 63`** (the stall value) × `bits` ∈ {0, 1, random} — `bits == 0` skips the loop, `bits >= 1` stalls it | [x] |
| C16 | `bitwriter_add` | **A4 `bw->bits == 64` / `65..127` / huge / `u32::MAX`** (invalid state, out-of-range `>>` counts) × `bits` random | [x] |
| C17 | `bitwriter_add` | **A5/A6 value shapes**: `val` and `bw->val` each ∈ {0, `u64::MAX`, single-bit at every position 0..63, low-32-only, high-32-only} × random `bits`/`bw->bits` | [x] |
| C18 | `bitwriter_add` | **A6 bit-0 / `mask` interaction**: `bw->val` with bit 0 set, plus `val` shapes that land a 1 in bit 0, across loop-entered and loop-skipped configs (checks `&= ~1` applies inside the loop only) | [x] |
| C19 | `bitwriter_add` | **A7 `bw->tot` wraparound**: `bw->tot` ∈ {0, 0x7FFFFFFF, 0xFFFFFFFF, random} × `bits` random | [x] |
| C20 | `bitwriter_add` | **A8 untouched fields / struct ABI**: `pos`, `len`, `buffer` seeded with random junk (incl. non-null bogus pointer) — must be bit-identical after the call | [x] |
| C21 | `bitwriter_add` | **A9 realistic sequential pipeline**: fresh zeroed writer, then 2000 randomized `bitwriter_add` calls with `bits` in `1..=32`, comparing full struct after **every** call | [x] |
| C22 | `bitwriter_add` | **A9 sequential pipeline, unconstrained**: fresh zeroed writer, 2000 calls with fully random `bits` (`u32`) and `val` (`u64`) — drives the accumulator into invalid states and keeps going | [x] |
| C23 | `bitwriter_add` | **full-random fuzz over all six fields + both args** simultaneously (100 000 cases, fixed seed) — the unpruned cross-product | [x] |
| C24 | `bitwriter_add` | **exhaustive small grid**: `bw->bits` ∈ `0..=70` × `bits` ∈ `0..=70` (5041 combos) × fixed non-trivial `val`/`bw->val`/`tot` | [x] |
