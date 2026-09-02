# CONFIGS.md — configuration surface table (valid inputs)

## Mechanical derivation of the axes

The library exposes **three** entry points (all three are exported by the
`.so`, so all three are public even though only one is declared in the header):

| level | symbol | declared in | role |
|-------|--------|-------------|------|
| low   | `tflac_pack_u64le(tflac_u8 *d, tflac_u64 n)` | `src/lib.c:5` | 8-byte little-endian store |
| mid   | `tflac_md5_addsample(tflac_md5 *m, tflac_u32 bits, tflac_uint val)` | `src/lib.c:17` | buffer/position state machine |
| high  | `update_md5(tflac *t, const tflac_s32 *samples)` | `include/lib.h:21` | 5-iteration driver over the mid level |

`update_md5` is the convenience/one-shot wrapper; the tests below drive
`tflac_pack_u64le` and `tflac_md5_addsample` **directly** as well, and also
interleave all three against one shared struct (row 25).

There are **no runtime flags, modes, or `#ifdef`s** (`grep -nE '#if|switch'`
over the C source returns nothing) and **no features** in `Cargo.toml`. The
configuration axes are therefore entirely *state* and *input shape*, read off
the operations the C actually performs:

* **A1 — `m->pos` on entry** (`lib.c:22,23,24,25`): the sole branch is
  `if (m->pos >= 64)`, and `pos` is used both as `pos % 64` (write offset) and
  raw (`pos += bytes`). Distinguished values: `0`; `1..55` (branch not taken);
  `56` (`pos+8 == 64` exactly, branch taken but copy loop empty); `57..63`
  (branch + non-empty copy loop); `63` (max write offset `pos%64==63`);
  `>= 64` (out of the array's own range → OOB source reads); `0xFFFFFFFF`
  (`pos+8` wraps below 64 → branch *not* taken).
* **A2 — `bits`** (`lib.c:20,21`): `total += bits` uses the full value while
  `bytes = bits/8` truncates. Distinguished: `0`; non-multiples of 8; each
  multiple `8,16,…,64`; `64` (the only value `update_md5` ever passes);
  `0xFFFFFFFF`.
* **A3 — `m->total` on entry** (`lib.c:20`): `0`; mid-range; near `u64::MAX`
  (wraps).
* **A4 — `m->buffer` contents on entry** (`lib.c:23,27,28`): the carry-down copy
  makes prior buffer content observable. Shapes: all-zero; ramp `0..71`;
  random; and the 40 bytes *past* the array that the OOB read touches.
* **A5 — `val` / `n` value shape** (`lib.c:6..13`, `lib.c:38..45`): `0`;
  `u64::MAX`; each single bit `1<<k`, k=0..63; random.
* **A6 — destination byte offset for `tflac_pack_u64le`**: `0` (8-aligned),
  `1..7` (misaligned — matters because the Rust must not assume alignment),
  `len-8` (writes the final byte of the region).
* **A7 — `cur_blocksize` × `channels`** (`lib.c:33`, `b = cbs * ch`, u32 mul):
  `0` in either factor; product `< 40`; product `== 40` exactly; product
  `> 40`; product overflowing u32.
* **A8 — `samples` value shape** (`lib.c:38..45`): the `(tflac_uint)` cast
  **sign-extends** an `i32` before `& 0xFF`, so only the low byte survives but
  the sign matters for how the value is produced. Shapes: all `0`; all `-1`;
  `INT32_MIN`; `INT32_MAX`; values whose low byte is `0x00`/`0x80`/`0xFF`;
  full-range random.
* **A9 — `samples` array length / stride** (`lib.c:47`): the pointer advances
  `8*sizeof(tflac_s32) == 32` elements per iteration, 5 iterations, reading 8
  each time ⇒ indices `0..7, 32..39, 64..71, 96..103, 128..135`. Shapes:
  exactly 136 elements (minimum non-OOB); larger; the interior gap elements
  (24 of every 32) must be *ignored*.
* **A10 — call multiplicity**: one call vs. many calls accumulating `pos` and
  `total` in the same struct (the state machine's behaviour is history
  dependent).

Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift64\* PRNG (seed `0x2545F4914F6CDD1D`) — not a single hand-picked value.
Every comparison checks the return value **and** the full byte image of a padded
512-byte allocation holding the struct, so stray or out-of-bounds *writes* are
caught too.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `tflac_pack_u64le` | A6 offset 0 (aligned) × A5 random `n` (512 iters) | [x] |
| 2  | `tflac_pack_u64le` | A6 offsets 1..7 (every misalignment) × A5 random `n` | [x] |
| 3  | `tflac_pack_u64le` | A6 offset 0 × A5 boundary values: `0`, `u64::MAX`, `1<<k` for all k=0..63, byte-lane masks | [x] |
| 4  | `tflac_pack_u64le` | A6 offset `len-8` (store ends exactly at region end) × A5 random | [x] |
| 5  | `tflac_md5_addsample` | A1 `pos=0` × A2 `bits=64` × A3 `total=0` × A4 zeroed × A5 random — branch not taken | [x] |
| 6  | `tflac_md5_addsample` | A1 `pos` swept over every value `1..55` × A2 `bits=64` × A4 ramp × A5 random — branch not taken, all write offsets | [x] |
| 7  | `tflac_md5_addsample` | A1 `pos=56` × A2 `bits=64` — `pos+bytes == 64` exactly: branch taken, copy loop empty | [x] |
| 8  | `tflac_md5_addsample` | A1 `pos` swept `57..63` × A2 `bits=64` × A4 random — branch taken with a non-empty, fully in-bounds copy loop | [x] |
| 9  | `tflac_md5_addsample` | A1 `pos` swept `0..63` × A2 every multiple of 8 in `0..64` (9 values) × A4 random × A5 random — full pruned cross-product of the two state axes | [x] |
| 10 | `tflac_md5_addsample` | A1 `pos` random `0..63` × A2 `bits` random non-multiples of 8 — truncating `bytes` vs. exact `total` | [x] |
| 11 | `tflac_md5_addsample` | A3 `total` near `u64::MAX` (u64 wrap) × A2 random `bits` | [x] |
| 12 | `tflac_md5_addsample` | A1 `pos >= 64` (`64,65,72,100,1000,0xFFFF`, random large) × A4 randomized *padding past the array* — the OOB source region | [x] |
| 13 | `tflac_md5_addsample` | A10 many calls (64 per trial) on one struct, A2 `bits` random per call, A5 `val` random — history-dependent state machine | [x] |
| 14 | `tflac_md5_addsample` | A1 `pos=63` (max write offset: store covers `buffer[63..70]`) × A5 boundary `val`s | [x] |
| 15 | `update_md5` | A7 random `cur_blocksize`/`channels` × A1 `pos=0` × A3 `total=0` × A8 full-range random samples × A9 length 136 | [x] |
| 16 | `update_md5` | A1 `pos` swept over **every** value `0..63` × A8 random samples — all 64 entry positions of the composed 5-iteration pipeline | [x] |
| 17 | `update_md5` | A1 `pos >= 64` (`64,65,100,1000,0xFFFFFFFF`, random large) × A8 random samples × randomized padding | [x] |
| 18 | `update_md5` | A8 degenerate sample shapes: all `0`; all `-1`; `INT32_MIN`; `INT32_MAX`; low byte `0x00`/`0x80`/`0xFF`; alternating sign | [x] |
| 19 | `update_md5` | A7 `channels == 0` and `cur_blocksize == 0` (product 0 → return wraps) | [x] |
| 20 | `update_md5` | A7 product `< 40`, `== 40`, `== 41` (the underflow boundary swept) | [x] |
| 21 | `update_md5` | A7 product overflowing u32 (`0x10000*0x10000`, `0xFFFFFFFF*3`, random large pairs) | [x] |
| 22 | `update_md5` | A3 `total` near `u64::MAX` (five `+= 64` overflow it) × A1 random `pos` | [x] |
| 23 | `update_md5` | A9 stride check: samples buffer 4096 elems with *distinct* values everywhere, so any wrong stride (8 instead of 32) diverges; also length exactly 136 | [x] |
| 24 | `update_md5` | A10 repeated: 10 consecutive `update_md5` calls on one struct, fresh random samples each — accumulates `pos`/`total` across calls | [x] |
| 25 | all three, composed | interleaved pipeline on one shared struct: `update_md5` → direct `tflac_md5_addsample` (random `bits`) → direct `tflac_pack_u64le` into the same buffer → `update_md5` again, 200 randomized steps | [x] |

## Status

All 25 rows are covered by `translation/tests/differential.rs` (module
`configs`), each named `row<NN>_…`. A row is checked off only after it passes
across its full randomized sweep, comparing return values and the entire padded
memory image byte-for-byte.

Verified across both profiles and both feature configurations by `run_all.sh`
(`debug`/`release` × `default`/`--no-default-features`): 52 tests, 0 failures
each time, symbol diff empty each time. `mutation_check.sh` confirms the suite
detects all 11 injected translation bugs, so these check marks are not vacuous.
