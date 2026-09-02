# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the branches in `c_src/src/lib.c` and the public
surface in `c_src/include/lib.h`. This is the mirror of `ERRORS.md`: only
**valid** inputs, enumerated over the axes the C actually distinguishes.

## Public entry points (both are tested directly — no wrapper-only coverage)

| entry point | declared in header? | exported? |
|-------------|---------------------|-----------|
| `flac_validate(tflac *t)` | yes | yes |
| `tflac_size_memory(tflac_u32 blocksize)` | **no** (undeclared but non-`static`) | yes — the lowest-level entry point, tested directly through `nm`-visible symbol |

There is no convenience/one-shot wrapper in this library; `flac_validate` is
driven by mutating the caller-owned `struct tflac` in place, so every "option"
is a struct field. The struct is the configuration surface.

## Axes the C branches on

**`flac_validate` — option fields (all caller-set state):**

| axis | states the C distinguishes | source |
|------|----------------------------|--------|
| `channel_mode` | `0` (INDEPENDENT) vs **any nonzero**; `1`/`2`/`3` valid, `4` = `TFLAC_CHANNEL_MODE_COUNT`, `5..=255` out-of-range | line 32 |
| `channel_mode` reset predicate | `channels == 2 && bitdepth != 32` (mode kept) vs otherwise (mode forced to 0) | line 33 |
| `max_rice_value` | `0` → auto-fill; `1..=30` → kept verbatim | line 37, 43 |
| auto-fill split | `bitdepth <= 16` → 14; `bitdepth > 16` → 30 | line 38 |
| `min_partition_order` / `max_partition_order` | `min == max` (loop cannot advance) vs `min < max` (loop may advance); `max == 0`; `max == 15` (shift amount reaches 16) | lines 46–53 |

**`flac_validate` — input-shape axes:**

| axis | shapes the C distinguishes | source |
|------|----------------------------|--------|
| `blocksize` | boundary `16`, boundary `65535`; power-of-two vs not; **2-adic valuation** `v2(blocksize)` decides how far the partition-order loop runs | lines 16–18, 52 |
| `samplerate` | boundary `1`, boundary `655350`; otherwise opaque (no other branch) | lines 20–22 |
| `channels` | `1`, `2` (only value that preserves a stereo `channel_mode`), `3..=8` | lines 24–26, 33 |
| `bitdepth` | `1`, `16` / `17` (the `<= 16` auto-fill split), `32` (only value that kills a stereo `channel_mode`), `31` | lines 28–30, 33, 38 |
| padding bytes 21..23 | never written by either side — compared byte-for-byte to prove neither writes them | struct layout |

**`tflac_size_memory` — input-shape axes:**

| axis | shapes | source |
|------|--------|--------|
| `blocksize mod 4` | decides whether `15 + 4*blocksize` has its low nibble masked off by `& 0xFFFFFFF0` | line 12 |
| magnitude | no wrap; `blocksize * 4U` wraps `u32` (`blocksize > 0x3FFFFFFF`); `5U * masked` wraps `u32` (`masked > 0x33333333`); both wrap | line 12 |

## Configuration rows

Every row is exercised with **many randomized inputs** (fixed-seed xorshift64\*
PRNG, no external dep) over the free axes, plus the named boundary values, and
compared **byte-for-byte over all 28 struct bytes + the `int` return value**
between the C `.so` and the Rust `.so`.

### `tflac_size_memory`

| # | entry point | configuration (options set + input shape) | [x] |
|---|-------------|-------------------------------------------|-----|
| S1 | `tflac_size_memory` | `blocksize == 0` | [x] |
| S2 | `tflac_size_memory` | `blocksize ∈ 1..=15` (exhaustive; sub-mask-granularity) | [x] |
| S3 | `tflac_size_memory` | `blocksize ≡ 0 (mod 4)`, no wrap — low nibble of `15+4b` is `0xF`, fully masked | [x] |
| S4 | `tflac_size_memory` | `blocksize ≡ 1, 2, 3 (mod 4)`, no wrap (all three residues) | [x] |
| S5 | `tflac_size_memory` | `blocksize ∈ 16..=65535` (the FLAC-legal range), randomized | [x] |
| S6 | `tflac_size_memory` | `5U * masked` wraps but `4U*b` does not: `masked > 0x33333333` ⟹ `b ≳ 0x0CCCCCCC`, randomized in `0x0CCCCCCD..=0x3FFFFFFF` | [x] |
| S7 | `tflac_size_memory` | `blocksize * 4U` wraps `u32`: `b > 0x3FFFFFFF`, randomized (both multiplies wrap) | [x] |
| S8 | `tflac_size_memory` | exhaustive boundary sweep: `0x3FFFFFFE..=0x40000002`, `0x0CCCCCCB..=0x0CCCCCCF`, `0xFFFFFFFD..=0xFFFFFFFF`, `0x7FFFFFFF`, `0x80000000` | [x] |
| S9 | `tflac_size_memory` | unconstrained random `u32` (full domain) | [x] |

### `flac_validate` — valid (accepted, returns 0) configurations

| # | entry point | configuration (options set + input shape) | [x] |
|---|-------------|-------------------------------------------|-----|
| V1 | `flac_validate` | `channel_mode=0`, `max_rice_value=0`, `min=max=0`; blocksize/samplerate/channels/bitdepth randomized in range | [x] |
| V2 | `flac_validate` | `channel_mode=0`, `max_rice_value=0`, `bitdepth ≤ 16` (auto-fill ⇒ 14), orders randomized | [x] |
| V3 | `flac_validate` | `channel_mode=0`, `max_rice_value=0`, `bitdepth ∈ 17..=32` (auto-fill ⇒ 30), orders randomized | [x] |
| V4 | `flac_validate` | `max_rice_value ∈ 1..=30` (explicit, no auto-fill), everything else randomized | [x] |
| V5 | `flac_validate` | `max_rice_value == 30` exactly (upper boundary, accepted) | [x] |
| V6 | `flac_validate` | `max_rice_value == 1` exactly (lower nonzero boundary) | [x] |
| V7 | `flac_validate` | `channel_mode ∈ 1..=3` **with** `channels == 2 && bitdepth != 32` ⇒ mode **kept** | [x] |
| V8 | `flac_validate` | `channel_mode ∈ 1..=3` **with** `channels == 2 && bitdepth == 32` ⇒ mode **reset to 0** | [x] |
| V9 | `flac_validate` | `channel_mode ∈ 1..=3` **with** `channels != 2` (1, 3..=8) ⇒ mode **reset to 0** | [x] |
| V10 | `flac_validate` | `channel_mode == 4` (`TFLAC_CHANNEL_MODE_COUNT`, no real variant) × the kept/reset predicate | [x] |
| V11 | `flac_validate` | `channel_mode ∈ 5..=255` (out-of-range enum across FFI) × the kept/reset predicate | [x] |
| V12 | `flac_validate` | `min_partition_order == max_partition_order` (loop cannot advance), orders randomized `0..=15` | [x] |
| V13 | `flac_validate` | `min_partition_order < max_partition_order`, `blocksize` **odd** (loop cannot advance past `min`) | [x] |
| V14 | `flac_validate` | `min=0`, `max=15`, `blocksize = 32768` (`v2 = 15`, maximal loop run; shift reaches `1 << 16`) | [x] |
| V15 | `flac_validate` | `min=0`, `max=15`, `blocksize` a random power of two in `16..=32768` (loop stops at `v2`) | [x] |
| V16 | `flac_validate` | `min=0`, `max=15`, `blocksize = 2^k * odd` with randomized `k ∈ 0..=15` (loop stops at `min(v2, max)`) | [x] |
| V17 | `flac_validate` | `min` randomized `0..=15`, `max` randomized `min..=15`, `blocksize` randomized — full order cross-product | [x] |
| V18 | `flac_validate` | `max_partition_order == 15` (upper boundary, accepted) with `min` randomized | [x] |
| V19 | `flac_validate` | `blocksize == 16` (lower boundary) × all order combos | [x] |
| V20 | `flac_validate` | `blocksize == 65535` (upper boundary, odd ⇒ loop never advances) × all order combos | [x] |
| V21 | `flac_validate` | `samplerate == 1` and `samplerate == 655350` (both boundaries) | [x] |
| V22 | `flac_validate` | `channels` swept exhaustively `1..=8` × `bitdepth` swept exhaustively `1..=32` (256 combos) with orders/mode randomized | [x] |
| V23 | `flac_validate` | `bitdepth == 16` / `17` (auto-fill split boundary) × `max_rice_value == 0` | [x] |
| V24 | `flac_validate` | pre-dirtied output fields: `partition_order` and `cur_blocksize` pre-set to garbage, proving both sides overwrite them identically | [x] |
| V25 | `flac_validate` | **repeated invocation** — call `flac_validate` twice on the same struct (idempotence / second-pass state, since the first call rewrites `channel_mode` and `max_rice_value`) | [x] |
| V26 | `flac_validate` | fully unconstrained random 28-byte struct (all fields random `u32`/`u8`, mostly rejected) — the catch-all cross-product row | [x] |
| V27 | `tflac_size_memory` + `flac_validate` | composed pipeline: validate a randomized struct, then feed the resulting `cur_blocksize` into `tflac_size_memory`, comparing the C-pair result to the Rust-pair result | [x] |
