# CONFIGS.md — Configuration surface (VALID inputs) of `c_src/src/lib.c`

## Mechanical derivation of the axes

The C library has **no runtime option struct, no setters, no flags, no enums and
no `#ifdef`s** (see `ERRORS.md` for the grep evidence). Its "configuration" is
therefore entirely carried by (a) which of the three exported entry points is
called, (b) the scalar parameters, and (c) the *incoming state* of the
`tflac` / `tflac_md5` records. The axes below are exactly the things the C code
branches on or index-computes from:

### Axis 1 — entry point (all three exported symbols, incl. the two low-level ones)

| entry point | in public header? | why it must be driven directly |
|---|---|---|
| `tflac_pack_u64le(u8 *d, u64 n)` | no (but external linkage ⇒ in the `.so` ABI) | lowest level; the only byte-order-sensitive code |
| `tflac_md5_addsample(tflac_md5 *m, u32 bits, u64 val)` | no (external linkage) | mid level; owns *all* the branching (`if (m->pos >= 64)`, `while (bytes--)`) and is reachable with `bits != 64`, which `update_md5` can never produce |
| `update_md5(tflac *t, const s32 *samples)` | **yes** | top level; composes the pipeline, fixes `bits = 64`, iterates 5×, applies the `+32`-element stride |

### Axis 2 — `bits` (the only mode-like parameter; `lib.c:20,23` derive `bytes = bits/8`)

`0` · `8` (1 byte/step) · `24` · `64` (the value `update_md5` hard-codes) ·
non-multiples of 8 (`1`, `7`, `63`, `65`) · `512` (exactly the 64-byte block) ·
`576` (the whole 72-byte array) · huge (`0xFFFF_FFFF`).

### Axis 3 — incoming `m->pos` (drives `pos2 = pos % 64`, the `pos >= 64` branch, and the copy-loop length)

`0` · `1..7` (mid-word) · `8,16,24,32,40,48,56` (word-aligned) ·
`57..63` (⇒ `pack_u64le` spills into the `buffer[64..72]` tail) · `63` (max legal) ·
`64` · `> 64` · `0xFFFF_FFFF`.

### Axis 4 — the `if (m->pos >= 64)` branch (`lib.c:24`) and copy-loop length (`lib.c:27`)

not-taken (no copy) · taken with reduced `pos == 0` ⇒ **0** iterations ·
taken with reduced `pos` in `1..=8` ⇒ copy source `buffer[64..72]` fully **in** the array ·
taken with reduced `pos` in `9..=63` ⇒ copy source runs **past** `buffer[72]`.

### Axis 5 — incoming `m->total` (`lib.c:19`)

`0` · mid · `u64::MAX - k` (wraps).

### Axis 6 — incoming `buffer[72]` contents

all-`0x00` · all-`0xFF` · seeded pseudo-random · byte-index pattern (`buffer[i] = i`),
so a wrong copy offset is visible.

### Axis 7 — `val` / `n` payload shape (`tflac_pack_u64le`, `lib.c:5-14`)

`0` · `u64::MAX` · `0x0123_4567_89AB_CDEF` (all 8 bytes distinct ⇒ detects any
byte-order or shift-amount error) · one-hot per byte lane · seeded random.

### Axis 8 — destination pointer shape for `tflac_pack_u64le`

8-byte aligned · each of the 7 misalignments · positioned so `d[7]` is the last
writable byte.

### Axis 9 — `cur_blocksize` × `channels` shape (`lib.c:34`, `b = cur_blocksize * channels`)

`(0,0)`, `(0,n)`, `(n,0)` ⇒ `b = 0` (empty) · `(1,1)` ⇒ `b = 1` · products
`b = 1..39` (return underflows) · `b = 40` (returns exactly `0`) · `b = 41..`
(normal) · realistic `(4096, 2)`, `(1152, 8)` · products that overflow `u32`.

### Axis 10 — `samples` data shape (`lib.c:38-45`, `((u64)samples[k]) & 0xFF`)

all `0` · all `-1` · `i32::MIN` / `i32::MAX` · negatives (⇒ sign-extension then
`& 0xFF`) · values whose low byte is `0x00`/`0x7F`/`0x80`/`0xFF` · seeded random
full-range `i32` · **a distinct value in every one of the 136+ read slots plus
the 24-element gaps that the `+32` stride skips**, so a wrong stride is visible.

### Axis 11 — call count / streaming

1 call · 2 calls · many (16, 64) chained calls that carry `pos`/`total`/`buffer`
forward, since all state lives in the caller's record.

### Axis 12 — build configuration

`Cargo.toml` has no `[features]`; the only combination is "no features"
(≡ default). Each row is additionally run against **both** the `debug` and the
`release` (`panic = "abort"`, overflow-checks off) Rust `.so`. See `run_all.sh`.

---

## CONFIGURATION-SURFACE TABLE

Every row is a differential test: the *same* configuration is applied to a
byte-identical 4096-byte arena for C and for Rust, both `.so`s are called through
`libloading`, and the **return value plus the entire arena** are compared
byte-for-byte. Rows marked "randomized" run ≥ 200 seeded pseudo-random
instances (fixed seed ⇒ reproducible).

| # | entry point(s) | configuration (options set + input shape) | randomized | test fn | [x] |
|---|----------------|-------------------------------------------|-----------|---------|-----|
| C1 | `tflac_pack_u64le` | aligned `d`; `n` = 0, `u64::MAX`, `0x0123456789ABCDEF`, per-byte one-hot (8 values), per-byte `0xFF`-hot | fixed + 512 random | `cfg_c1_pack_values` | [x] |
| C2 | `tflac_pack_u64le` | `d` at every misalignment 0..7 within the arena × random `n` | 8×256 random | `cfg_c2_pack_misalignment` | [x] |
| C3 | `tflac_pack_u64le` | `d` at every offset 0..=(arena_len-8), i.e. incl. writing the final 8 bytes; arena pre-filled with the byte-index pattern to catch over/under-writes | sweep + random `n` | `cfg_c3_pack_offset_sweep` | [x] |
| C4 | `tflac_pack_u64le` | 4 consecutive/overlapping writes at offsets `p, p+1, p+7, p+8` (later writes partially clobber earlier ones) | 256 random | `cfg_c4_pack_overlapping_writes` | [x] |
| C5 | `tflac_md5_addsample` | `bits = 0` (⇒ `bytes = 0`, branch not taken) × `pos` ∈ {0,1,7,8,56,57,63} × random `val`, `total`, buffer | 7×128 random | `cfg_c5_addsample_bits0` | [x] |
| C6 | `tflac_md5_addsample` | `bits = 8` (1 byte/step) × `pos` ∈ {0..63} full sweep × random `val` — exercises branch-not-taken for `pos<56` and branch-taken w/ small copy for `pos>=56` | 64×64 random | `cfg_c6_addsample_bits8_pos_sweep` | [x] |
| C7 | `tflac_md5_addsample` | `bits = 64` (the `update_md5` value) × `pos` ∈ {0..63} full sweep — covers *not-taken* (`pos=0`? no: `0+8<64`) … *taken w/ 0 iterations* (`pos=56`) … *taken w/ 1..8 in-bounds copy* (`pos=57..63`) | 64×64 random | `cfg_c7_addsample_bits64_pos_sweep` | [x] |
| C8 | `tflac_md5_addsample` | branch **taken, reduced `pos == 0`** ⇒ `while(bytes--)` runs 0 times: `(pos,bits)` ∈ {(56,64),(0,512),(32,256),(63,8)} | 4×128 random | `cfg_c8_addsample_zero_copy_iterations` | [x] |
| C9 | `tflac_md5_addsample` | branch **taken, reduced `pos` ∈ 1..=8** ⇒ copy source entirely inside `buffer[64..72]` (no OOB read): swept over all 8 reduced values | 8×128 random | `cfg_c9_addsample_copy_in_bounds` | [x] |
| C10 | `tflac_md5_addsample` | branch **taken, reduced `pos` ∈ 9..=63** ⇒ copy source runs past `buffer[72]` into the record's tail / neighbouring arena bytes; swept over all 55 reduced values, arena seeded so the OOB source bytes are defined | 55×64 random | `cfg_c10_addsample_copy_out_of_bounds` | [x] |
| C11 | `tflac_md5_addsample` | `pos` ∈ {57..63} ⇒ `pack_u64le` **spills into `buffer[64..72]`** *before* the copy loop reads that same region (write/read interaction) × `bits` ∈ {8,64,512} | 7×3×64 random | `cfg_c11_addsample_write_read_interaction` | [x] |
| C12 | `tflac_md5_addsample` | `bits` non-multiple of 8 ∈ {1,2,7,9,63,65,511,513} (truncating `bits/8` **and** un-truncated `total +=`) × `pos` ∈ {0,7,56,63} | 8×4×64 random | `cfg_c12_addsample_bits_not_multiple_of_8` | [x] |
| C13 | `tflac_md5_addsample` | `bits` = whole-block/whole-array sizes {512, 576, 4096} and `pos` ∈ {0,8,63} | 3×3×64 random | `cfg_c13_addsample_large_bits` | [x] |
| C14 | `tflac_md5_addsample` | incoming `total` ∈ {0, 1, 0x7FFF…, `u64::MAX`, `u64::MAX-63`, `u64::MAX-64`} × `bits` ∈ {0,64,0xFFFFFFFF} | 6×3×64 random | `cfg_c14_addsample_total_shapes` | [x] |
| C15 | `tflac_md5_addsample` | buffer pre-fill ∈ {all-0x00, all-0xFF, byte-index pattern, random} × `(pos,bits)` = (40, 64) and (63, 64) | 4×2×64 random | `cfg_c15_addsample_buffer_prefills` | [x] |
| C16 | `tflac_md5_addsample` | `val` payload shapes: 0, `u64::MAX`, per-lane one-hot, `0x0123456789ABCDEF`, random × `pos` ∈ {0,57,63} | 12×3 fixed + random | `cfg_c16_addsample_val_shapes` | [x] |
| C17 | `tflac_md5_addsample` | **streaming**: 64 chained calls carrying `pos`/`total`/`buffer` forward, `bits = 64` each (the `update_md5` cadence), starting from `pos = 0` | 64 random seeds | `cfg_c17_addsample_stream_bits64` | [x] |
| C18 | `tflac_md5_addsample` | **streaming with random `bits` per call**: 64 chained calls, each `bits` drawn from the full `u32` range (mixes every branch, wraps `pos` and `total`) | 64 random seeds × 64 calls | `cfg_c18_addsample_stream_random_bits` | [x] |
| C19 | `update_md5` | `b = 0` via `(cur_blocksize, channels)` ∈ {(0,0),(0,7),(7,0)}; `pos = 0`, `total = 0`, random samples | 3×128 random | `cfg_c19_update_b_zero` | [x] |
| C20 | `update_md5` | `b` swept over `1..=80` (spans the `b < 40` underflow, `b == 40` ⇒ returns 0, and `b > 40`), via factor pairs incl. prime `b` (`b×1`) | 80×32 random | `cfg_c20_update_b_sweep` | [x] |
| C21 | `update_md5` | realistic FLAC shapes: `(cur_blocksize, channels)` ∈ {(1,1),(1,2),(576,1),(1152,2),(4096,2),(4608,8),(16,8)} × `pos = 0`, `total = 0` | 7×64 random | `cfg_c21_update_realistic_shapes` | [x] |
| C22 | `update_md5` | `cur_blocksize * channels` **overflows `u32`**: (0x10000,0x10000), (0xFFFFFFFF,3), (0x80000000,2), (0xFFFF,0x10001) | 4×64 random | `cfg_c22_update_multiply_overflow` | [x] |
| C23 | `update_md5` | incoming `md5_ctx.pos` swept `0..=63` (5 iterations × 8 bytes ⇒ each start lands the branch/copy differently) × random samples | 64×32 random | `cfg_c23_update_pos_sweep` | [x] |
| C24 | `update_md5` | incoming `md5_ctx.pos` out of the 0..63 range: {64, 65, 71, 72, 127, 128, 1000, 0xFFFF_FFF8, 0xFFFF_FFFF} | 9×64 random | `cfg_c24_update_pos_out_of_range` | [x] |
| C25 | `update_md5` | incoming `md5_ctx.total` ∈ {0, 1, `u64::MAX`, `u64::MAX-319`, `u64::MAX-320`} (5×64 = 320 bits added) | 5×64 random | `cfg_c25_update_total_shapes` | [x] |
| C26 | `update_md5` | samples data shapes: all-0, all-`-1`, all-`i32::MIN`, all-`i32::MAX`, low-byte ∈ {0x00,0x7F,0x80,0xFF}, alternating sign, ramp `i as i32`, `!(i as i32)` | 12 fixed × `pos` ∈ {0,57} | `cfg_c26_update_sample_shapes` | [x] |
| C27 | `update_md5` | **stride verification**: samples arena filled with a distinct value per index (incl. the 24-element gaps skipped by `samples += 32`), plus a variant where only the *skipped* elements are perturbed (result must be unchanged, identically in C and Rust) | 128 random | `cfg_c27_update_stride` | [x] |
| C28 | `update_md5` | `samples` pointer at every misalignment 0..3 bytes within the arena (C reads `s32` through a misaligned pointer at `-O0`) × random data | 4×64 random | `cfg_c28_update_samples_misaligned` | [x] |
| C29 | `update_md5` | buffer pre-fill ∈ {all-0x00, all-0xFF, byte-index pattern, random} × `pos` ∈ {0, 40, 63} | 4×3×64 random | `cfg_c29_update_buffer_prefills` | [x] |
| C30 | `update_md5` | **streaming**: 32 chained `update_md5` calls on the same `tflac`, advancing the samples window each time; return values of *every* call compared | 32 seeds × 32 calls | `cfg_c30_update_stream` | [x] |
| C31 | `update_md5` **+** `tflac_md5_addsample` **+** `tflac_pack_u64le` | **mixed pipeline**: a randomized program of 64 steps that interleaves all three entry points on one shared arena (`pack_u64le` writing straight into the record, `addsample` with random `bits`, `update_md5` with random shapes) — catches composition bugs invisible to per-function tests | 64 seeds × 64 steps | `cfg_c31_mixed_pipeline_fuzz` | [x] |
| C32 | all three | **full-arena fuzz**: 2000 iterations; every byte of the 96-byte `tflac` record, the sample window and the surrounding arena randomized, entry point and all scalars randomized from full-range values | 2000 random | `cfg_c32_global_fuzz` | [x] |

## Status

**32 / 32 rows verified.** `cargo test --test phase_b_configs` →
`33 passed; 0 failed` (32 row tests + `layout_parity_via_ffi`). Verified under
every feature combination (there is exactly one) and under both the `dev` and
`release` Rust profiles — see `run_all.sh`.

Negative control: with the tests unchanged, 19 deliberate code mutations of
`src/lib.rs` (byte order, shift amounts, `% 64` → `% 63`, `>= 64` → `> 64`,
`bits/8` → `bits/4`, copy-loop off-by-one, `i <= 4` → `i < 4`, the 32-element
stride, `b -= step` → `b += step`, `cur_blocksize * channels` → `+`, …) were
each detected by this suite, so the rows are not vacuous.
