# CONFIGS.md — Phase B configuration-surface table

## How the axes were derived

There are no build-time or runtime *option flags* in this library — `grep -E
'#if|#ifdef|switch|enum'` over `c_src/` returns 0 matches, `Cargo.toml` has no
`[features]` table, and no function takes a mode/flag argument. The
configuration surface is therefore entirely **caller-supplied state + input
shape**. Enumerated mechanically from the source:

**Axis 1 — entry point.** All three exported symbols, including the two
lowest-level ones that `lib.h` does not even declare:
`tflac_pack_u64le` (leaf) → `tflac_md5_addsample` (mid) → `update_md5` (top).
`update_md5` is the only convenience/one-shot wrapper; it is *not* tested alone.

**Axis 2 — `tflac_md5.pos` on entry.** The single `if` in the library is
`if (m->pos >= 64)`, and `pos` also feeds `pos2 = pos % 64` which selects the
write offset. Distinguished values: `0`; `1..55` (write stays inside the first
64 bytes); `56..63` (write spills into the `+8` tail region `buffer[64..72]`);
`≥ 64` (unreduced, out of ring range); near `UINT32_MAX` (add wraps).

**Axis 3 — whether the carry-down branch is taken, and with what `bytes`.**
`pos + bits/8 >= 64` decides. When taken, the loop length is the *reduced* `pos`,
which selects between: length 0 (no copy), length 1..7 (in-bounds source), and
length 8..63 (source index `64+bytes` runs past the 72-byte `buffer`).

**Axis 4 — `bits`.** Not validated, and used two different ways (`total += bits`
vs. `bytes = bits/8`). Distinguished shapes: `0`; multiples of 8 (`8`, `64`,
`512`); non-multiples of 8 (truncating division); `≥ 512` so `bytes ≥ 64`;
`UINT32_MAX`.

**Axis 5 — `tflac_md5.total`.** Only ever accumulated; the only distinct shapes
are "small" and "close enough to `UINT64_MAX` to wrap".

**Axis 6 — `cur_blocksize` × `channels` for `update_md5`.** The product feeds
`b`, from which `5 * 8 = 40` is unconditionally subtracted. Distinguished:
product `> 40`, `== 40`, `< 40` (underflow), and product that overflows `u32`.

**Axis 7 — `samples` value shape.** The `(tflac_uint)x & 0xFF` chain keeps only
the low byte of each of 8 elements, after **sign-extending** to 64 bits.
Distinguished: all zero; all `-1`/`INT32_MIN` (sign extension); values whose low
byte is zero but upper bytes are not (masking); fully random.

**Axis 8 — `samples` index footprint.** `samples += 8 * sizeof(tflac_s32)` is
+32 *elements* per iteration, so the fixed 5-iteration loop reads the disjoint
element ranges `[0,8) [32,40) [64,72) [96,104) [128,136)`. Distinguished:
a buffer large enough (≥ 136) with distinct values in every gap, so any wrong
stride shows up; and a buffer whose contents differ *only* in the skipped gaps,
which must **not** change the result.

**Axis 9 — destination pointer shape for `tflac_pack_u64le`.** aligned vs.
unaligned; inside `buffer` vs. a standalone allocation.

Every row is run against **many randomized inputs with a fixed seed**
(deterministic xorshift64\* PRNG in the test, seed printed on failure), not one
hand-picked value. Every row asserts byte-for-byte equality of the *entire*
96-byte `tflac` / 88-byte `tflac_md5` state **and** the surrounding backing
bytes **and** the returned `tflac_u32`.

## Table

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|-------------------------------------------|-----|
| 1 | `tflac_pack_u64le` | standalone aligned 8-byte destination; `n` = randomized full-range `u64` | [x] |
| 2 | `tflac_pack_u64le` | standalone aligned destination; `n` ∈ boundary set {0, 1, 0xFF, 0x100, 0x00FF00FF00FF00FF, `u64::MAX`, single-bit-set for all 64 bits} | [x] |
| 3 | `tflac_pack_u64le` | **unaligned** destination (offsets 1..7 into a scratch region); randomized `n` | [x] |
| 4 | `tflac_pack_u64le` | destination inside `tflac_md5.buffer` at every offset 0..64 (incl. 56..63 spilling into the `+8` tail, and 64 = the last fully-legal slot); randomized `n` | [x] |
| 5 | `tflac_md5_addsample` | `pos = 0`, `bits = 64` ⇒ carry-down **not** taken; randomized `val`, randomized initial buffer | [x] |
| 6 | `tflac_md5_addsample` | `pos` ∈ 1..55, `bits = 64` ⇒ write inside first 64 bytes, carry-down not taken; randomized `val` | [x] |
| 7 | `tflac_md5_addsample` | `pos` ∈ 56..63, `bits = 64` ⇒ write spills into `buffer[64..72]` **and** carry-down taken with `bytes` = 0..7 (in-bounds source); randomized `val` | [x] |
| 8 | `tflac_md5_addsample` | `pos` ∈ 0..63, `bits = 512` ⇒ `bytes = 64`, `pos+64 ≥ 64` always, reduced `pos` = original ⇒ carry-down with `bytes` up to 63 (**out-of-bounds source**, up to `buffer[126]`); randomized `val` + randomized backing bytes | [x] |
| 9 | `tflac_md5_addsample` | `bits = 0` (zero length) with `pos` ∈ {0, 32, 63} (branch not taken) and `pos` ∈ {64, 100} (branch taken with `bytes` = reduced `pos`); randomized `val` | [x] |
| 10 | `tflac_md5_addsample` | `bits` non-multiple of 8 (randomized from {1..7, 9, 15, 63, 65, 4095}) × `pos` ∈ {0, 31, 63, 64, 65535} ⇒ truncating `bits/8` and `total`/`pos` divergence | [x] |
| 11 | `tflac_md5_addsample` | `pos ≥ 64` unreduced (randomized from {64, 65, 100, 127, 128, 1000, 0xFFFF, 0x7FFFFFFF, `u32::MAX`}) × `bits` ∈ {0, 8, 64} | [x] |
| 12 | `tflac_md5_addsample` | `pos + bits/8` overflows `u32`: `pos` near `u32::MAX` × `bits` = `u32::MAX` (`bytes = 0x1FFFFFFF`) and `bits = 64` | [x] |
| 13 | `tflac_md5_addsample` | `total` near `u64::MAX` (randomized within 0x100 of it) so `total += bits` wraps; `bits` randomized | [x] |
| 14 | `tflac_md5_addsample` | **repeated** calls (16 in a row) with randomized `bits`/`val` per call, threading the same context ⇒ the composed ring-buffer pipeline, which per-call tests cannot reach | [x] |
| 15 | `update_md5` | product `> 40` (randomized `cur_blocksize`/`channels` with product in 41..100000), `md5_ctx` zeroed, 160 randomized `samples` | [x] |
| 16 | `update_md5` | product `== 40` exactly (all factor pairs: 1×40, 2×20, 4×10, 5×8, 8×5, 10×4, 20×2, 40×1) ⇒ return value exactly 0 | [x] |
| 17 | `update_md5` | product `< 40` ⇒ `b` underflows (randomized products 0..39 incl. `0×0`, `0×n`, `n×0`) | [x] |
| 18 | `update_md5` | product overflows `u32` (randomized large factors, plus 0x10000×0x10000, `u32::MAX`×3, `u32::MAX`×`u32::MAX`) | [x] |
| 19 | `update_md5` | `samples` all zero / all `-1` / all `INT32_MIN` / all `INT32_MAX` ⇒ sign-extension-then-mask behaviour | [x] |
| 20 | `update_md5` | `samples` whose low bytes are 0 but upper bytes randomized (`x & !0xFF`) ⇒ result must be independent of the masked-off bits | [x] |
| 21 | `update_md5` | `samples` identical in the 5 *read* windows but randomized in the *skipped* gaps ⇒ result must be unchanged (pins the +32-element stride) | [x] |
| 22 | `update_md5` | `md5_ctx.pos` seeded ∈ 0..63 randomized ⇒ carry-down fires *inside* the loop for `pos ≥ 56`, OOB source reads included | [x] |
| 23 | `update_md5` | `md5_ctx.pos` seeded ≥ 64 (randomized from {64, 100, 1000, 0xFFFFFFF8, `u32::MAX`}) and `total` seeded near `u64::MAX` | [x] |
| 24 | `update_md5` | **repeated** calls (8 in a row) on the same `tflac`, advancing `samples` by 136 each time, randomized ⇒ end-to-end composed pipeline | [x] |
| 25 | all three, interleaved | randomized script of 64 operations chosen uniformly from {`pack_u64le` into the buffer, `addsample`, `update_md5`} against one shared `tflac` in one shared backing region ⇒ full cross-entry-point state interaction | [x] |

## Test-suite non-vacuity (mutation control)

A differential suite that passes proves nothing unless it can fail. Nine
mutations were injected into `src/lib.rs`, rebuilt, and run under **both**
profiles; the source was restored byte-identical afterwards
(`diff` clean against a pre-mutation snapshot).

| # | mutation | rows that caught it |
|---|----------|---------------------|
| M1 | carry-down source index `64 + bytes` → `63 + bytes` | 12 of 25 |
| M2 | `samples` stride 32 elements → 8 (the "obvious fix" to the C quirk) | 11 of 25 |
| M3 | `pack_u64le` byte 0 takes `n >> 56` | 25 of 25 |
| M4 | loop `while i <= 4` → `while i < 4` (5 iterations → 4) | 11 of 25 |
| M5 | `total += bits` → `total += bits / 8` | 20 of 25 |
| M6 | `STEP` 8 → 4 | 11 of 25 |
| M7 | `MD5_BUFFER_OFFSET` 16 → 8 | 21 of 25 |
| M8 | `pos2 = pos % 64` → `pos % 72` | 9 of 25 |
| M9 | sign extension suppressed on `samples[0]` (`as u32` inserted) | **0 — correctly undetected** |

M9 is a semantically *equivalent* edit, not a missed bug: `(x as u32) as u64 &
0xFF` and `(x as i64) as u64 & 0xFF` agree on the low 8 bits, and the mask
discards everything else. The C disassembly does the same thing (`cltq` followed
by `and $0xff`). So its being undetected is the correct outcome, and rows 20/21
independently pin that masking invariant on the C side.

M2 and M4 are the two mutations that "fix" the C's quirks. Both are caught,
which is what makes this suite meaningful for a faithful-translation contract.
