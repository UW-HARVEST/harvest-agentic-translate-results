# CONFIGS.md — configuration surface table (Phase A, gates Phase B)

Derived mechanically from the branches in `c_src/src/lib.c` and the public
types in `c_src/include/lib.h`.

## Public entry points

`nm -D` shows exactly one exported function, so the "full set of public entry
points" is:

| entry point | linkage | notes |
|---|---|---|
| `dequantize_granule(float*, bs_t*, L12_scale_info*, int)` | exported | the only public symbol; there is no convenience wrapper and no lower-level exported function |
| `get_bits(bs_t*, int)` | `static` | **lowest-level** routine. Not exported, so it is exercised *only* indirectly. Its whole input space is reached by controlling `bs->buf/pos/limit` and the `n` implied by `bitalloc[i]` — that is what the `pos` / `limit` / `ba` axes below are for. |

## Axes the C actually branches on

| axis | source of the branch | values exercised |
|---|---|---|
| **A1 `group_size`** | `k < group_size` loop guard; `dst = grbuf + group_size * j`; `return group_size * 4` | `< 0`, `0`, `1`, `2`, `3`, `12`, `18`, `32`, `64`, `576` |
| **A2 `sci->total_bands`** | `i < 2 * sci->total_bands`; also decides whether `bitalloc[i]` stays inside its 64-byte array | `0`, `1`, `2`, `8`, `32` (max `i`=63, in bounds), `33`/`64` (`i`≥64 → reads `scfcod`), `65` (→ struct padding), `128`/`255` (→ **past end of struct**, max `i`=509) |
| **A3 `bitalloc[i]` value class** | `if (ba != 0)` / `if (ba < 17)` / `2 << (ba - 17)` | `0` (skip), `1`, `16` (linear ends), `1..16`, `17`, `18`, `19`…`25`, `30`, `46`, `47` (signed overflow), `48` (`mod==1`), `49` (shift wraps, period 32), `255`, `17..48`, `49..255`, full random `0..255`, sparse (≈50 % zeros), boundary mix |
| **A4 `bs->pos` (start)** | `s = pos & 7` (first-byte mask), `p = buf + (pos >> 3)`, `pos += n` | `0`, `1`…`7` (all 8 alignments), `8`, random `0..64`, `-1`, `-1000`, `500000` |
| **A5 `bs->limit`** | `(bs->pos += n) > bs->limit` | `0`, negative, `pos+7`, `pos+8` (exact boundary), tiny (mid-granule exhaustion), huge (no exhaustion) |
| **A6 `bs->buf` contents** | `*p++ & (255 >> s)`, `next << shl`, `next >> -shl` | uniform random, all `0x00`, all `0xFF` |
| **A7 `n` implied by grouped `mod`** | `get_bits(bs, mod + 2 - (mod >> 3))` | `n ∈ {3,5,7,10,17,31,59,115,227,451,…,7340035,1879048195}` — selected via A3; `n+s ≥ 40` is what makes `next << shl` an over-wide shift |
| **A8 surrounding struct bytes** | the out-of-bounds `bitalloc[i]` read for `i ≥ 64` | `scf`, `stereo_bands`, `scfcod` and 380 bytes past the struct are all randomized identically for C and Rust |

`j < 4` is a constant loop bound (no axis). `choff` is not an input: it starts at
`576` and alternates `576 / -558`; because `2 * total_bands` is always even it is
always back to `576` at the top of each `j` iteration.

## Configuration table

Every row is run with **many randomized inputs** (`iters` reps, fixed
`SplitMix64` seed derived from the row id, so it is reproducible). "Random"
buffers/bitallocs are redrawn each rep. Outputs compared byte-for-byte:
return value, whole `grbuf` (as raw bytes), `bs->pos`, `bs->limit`, and the
`sci` backing bytes.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dequantize_granule` → `get_bits` | linear: G=1, T=1, ba≡1, pos=0, limit=huge, buf=rand | [x] |
| 2 | `dequantize_granule` → `get_bits` | linear: G=2, T=1, ba≡16, pos=0, limit=huge, buf=rand | [x] |
| 3 | `dequantize_granule` → `get_bits` | linear: G=3, T=2, ba∈1..16, pos=0, limit=huge, buf=rand | [x] |
| 4 | `dequantize_granule` → `get_bits` | linear: G=12, T=8, ba∈1..16, pos=0, limit=huge, buf=rand | [x] |
| 5 | `dequantize_granule` → `get_bits` | linear: G=18, T=32 (max i=63, bitalloc in bounds), ba∈1..16, pos=0, limit=huge | [x] |
| 6 | `dequantize_granule` → `get_bits` | linear: G=32, T=33 (i reaches 64 → reads `scfcod`), ba∈1..16, pos=0, limit=huge | [x] |
| 7 | `dequantize_granule` → `get_bits` | linear: G=64, T=64 (max i=127, all of `scfcod`), ba∈1..16, pos=0, limit=huge | [x] |
| 8 | `dequantize_granule` → `get_bits` | linear: G=12, T=65 (max i=129 → struct trailing padding), ba∈1..16 | [x] |
| 9 | `dequantize_granule` → `get_bits` | linear: G=12, T=128 (max i=255 → past end of struct), ba∈1..16 | [x] |
| 10 | `dequantize_granule` → `get_bits` | linear: G=12, T=255 (max i=509 → 380 B past struct), ba∈1..16 | [x] |
| 11 | `get_bits` alignment | linear: G=12, T=8, ba∈1..16, **pos=1** (s=1), limit=huge | [x] |
| 12 | `get_bits` alignment | linear: G=12, T=8, ba∈1..16, **pos=7** (s=7), limit=huge | [x] |
| 13 | `get_bits` alignment | linear: G=12, T=8, ba∈1..16, **pos=random 0..64** (all 8 alignments), limit=huge | [x] |
| 14 | `get_bits` alignment | linear: G=12, T=8, ba∈1..16, **pos=8** (aligned, non-zero), limit=huge | [x] |
| 15 | `get_bits` negative pos | linear: G=12, T=8, ba∈1..16, **pos=-1** (`pos>>3` = -1, s=7), limit=huge | [x] |
| 16 | grouped path | G=1, T=1, ba≡17 → mod=3, n=5 | [x] |
| 17 | grouped path | G=2, T=1, ba≡18 → mod=5, n=7 | [x] |
| 18 | grouped path | G=3, T=2, ba≡19 → mod=9, n=10 | [x] |
| 19 | grouped path | G=12, T=8, ba≡20 → mod=17, n=17 | [x] |
| 20 | grouped path | G=12, T=8, ba≡21 → mod=33, n=31 | [x] |
| 21 | grouped path, **over-wide shift** | G=12, T=8, ba≡22 → mod=65, n=59 → `shl` reaches 51 ≥ 32 in `next << shl` | [x] |
| 22 | grouped path, over-wide shift | G=12, T=8, ba≡23 → mod=129, n=115 | [x] |
| 23 | grouped path, over-wide shift | G=12, T=8, ba≡24 → mod=257, n=227 | [x] |
| 24 | grouped path, over-wide shift | G=12, T=8, ba≡25 → mod=513, n=451 | [x] |
| 25 | grouped path, wide read | G=12, T=8, ba≡30 → mod=16385, n=14339 (reads 1793 B/call) | [x] |
| 26 | grouped path, huge n | G=12, T=8, ba≡46 → mod=0x40000001, n=0x38000003, **limit=INT_MIN** (see note) | [x] |
| 27 | grouped path, **signed overflow** | G=12, T=8, ba≡47 → `2<<30` overflows int → mod=0x80000001, n=0x70000003, **limit=INT_MIN** | [x] |
| 28 | grouped path, **mod==1** | G=12, T=8, ba≡48 → `2<<31`==0 → mod=1, n=3, all outputs 0.0 | [x] |
| 29 | grouped path, **shift-count wrap** | G=12, T=8, ba≡49 → `(ba-17)&31`==0 → identical to ba=17 (period 32) | [x] |
| 30 | grouped path, max value | G=12, T=8, ba≡255 → k=14, mod=32769, n=28675 | [x] |
| 31 | grouped path | G=12, T=8, ba∈17..48 random (k-capped, see note), pos=0, limit=huge | [x] |
| 32 | grouped path | G=12, T=8, ba∈49..255 random (k-capped), pos=0, limit=huge | [x] |
| 33 | grouped path, unaligned | G=12, T=8, ba∈17..255 random, **pos=3**, limit=huge | [x] |
| 34 | mixed paths | G=12, T=8, ba∈0..255 (skip + linear + grouped interleaved) | [x] |
| 35 | mixed paths | G=12, T=32, ba∈0..255 | [x] |
| 36 | mixed paths, OOB bitalloc | G=12, T=255, ba∈0..255 (reads far past struct) | [x] |
| 37 | mixed paths | G=18, T=64, ba = boundary mix `{0,1,2,15,16,17,18,46,47,48,49,254,255}`, **limit=INT_MIN** | [x] |
| 38 | all bands skipped | G=12, T=8, ba≡0 → `if (ba != 0)` never taken; `dst`/`choff` still advance | [x] |
| 39 | sparse | G=12, T=32, ba = 50 % zeros / 50 % random 1..255 | [x] |
| 40 | **exhaustion** mid-granule | linear: G=12, T=8, ba∈1..16, limit = pos+64 | [x] |
| 41 | exhaustion mid-granule | linear: G=12, T=8, ba∈1..16, limit = pos+1000 | [x] |
| 42 | exhaustion mid-granule | grouped: G=12, T=8, ba∈17..32, limit = pos+500 | [x] |
| 43 | exhaustion mid-granule | mixed: G=12, T=255, ba∈0..255, limit = pos+10000 | [x] |
| 44 | degenerate buf | linear: G=12, T=8, ba∈1..16, buf = all `0x00` | [x] |
| 45 | degenerate buf | linear: G=12, T=8, ba∈1..16, buf = all `0xFF` | [x] |
| 46 | degenerate buf | grouped: G=12, T=8, ba∈17..48, buf = all `0x00` | [x] |
| 47 | degenerate buf | grouped: G=12, T=8, ba∈17..48, buf = all `0xFF` | [x] |
| 48 | **G=0** | linear: G=0, T=8, ba∈1..16 → no writes, **no bits consumed** (k loop guards `get_bits`) | [x] |
| 49 | **G=0**, grouped | G=0, T=8, ba∈17..48 → no writes but **bits still consumed** (`get_bits` precedes k loop) | [x] |
| 50 | **G<0** | G=-1, T=8, ba∈0..255 → negative `dst`, no writes, returns -4 | [x] |
| 51 | **G<0**, large T | G=-7, T=255, ba∈0..255 → returns -28 | [x] |
| 52 | **T=0** | G=12, T=0, ba∈0..255 → i loop never entered, `grbuf`/`bs` untouched, returns 48 | [x] |
| 53 | large stride | linear: G=576, T=2, ba∈1..16 (real MPEG granule size) | [x] |
| 54 | large stride, grouped | G=576, T=2, ba∈17..32 | [x] |
| 55 | large pos | linear: G=12, T=8, ba∈1..16, **pos=500000**, limit=huge | [x] |
| 56 | large negative pos | linear: G=12, T=8, ba∈1..16, **pos=-1000**, limit=huge | [x] |
| 57 | all 8 alignments × both paths | G=12, T=8, ba∈1..255, pos = `rep % 8`, limit=huge | [x] |
| 58 | **limit boundary, inside** | G=1, T=1, ba≡8, pos=0, limit = 8 (`pos+n == limit` → read happens) | [x] |
| 59 | **limit boundary, outside** | G=1, T=1, ba≡8, pos=0, limit = 7 (`pos+n > limit` → early out) | [x] |
| 60 | full ba domain, read-free | G=12, T=8, ba∈0..255 **uncapped**, limit=INT_MIN → all 32 shift residues | [x] |
| 61 | full ba domain, read-free | G=12, T=255, ba∈0..255 uncapped, limit=INT_MIN | [x] |
| 62 | boundary mix, read-free | G=12, T=8, boundary-mix ba, limit=INT_MIN | [x] |
| 63 | k=25, read-free | G=12, T=8, ba≡42 → mod=0x04000001, n=58720259 | [x] |
| 64 | k=26, read-free | G=12, T=8, ba≡43 → n=117440515 | [x] |
| 65 | k=27, read-free | G=12, T=8, ba≡44 → n=234881027 | [x] |
| 66 | k=28, read-free | G=12, T=8, ba≡45 → n=469762051 | [x] |
| 67 | k=25 at high magnitude | G=12, T=8, ba≡234, limit=INT_MIN | [x] |
| 68 | k=30 at high magnitude | G=12, T=8, ba≡239, limit=INT_MIN | [x] |
| 69 | linear with exhausted stream | G=12, T=8, ba∈1..16, limit=INT_MIN → every sample is exactly `-half` | [x] |
| 70 | large stride, read-free | G=576, T=8, ba∈0..255, limit=INT_MIN | [x] |
| 71 | G=0, read-free | G=0, T=8, ba∈0..255, limit=INT_MIN | [x] |
| 72 | G<0, read-free | G=-3, T=8, ba∈0..255, limit=INT_MIN | [x] |
| 73 | **widest feasible read** | G=4, T=1, ba≡39 → k=22, n=7340035; ~917k over-wide-shift loop iterations per call | [x] |
| 74 | every linear ba | G=12, T=2, ba ≡ each of 1..=16, limit=huge (16 sub-rows) | [x] |
| 75 | every readable shift residue | G=6, T=1, ba ≡ 17+k for k=0..=22, limit=huge (23 sub-rows) | [x] |
| 76 | every ba value | G=5, T=2, ba ≡ each of 0..=255, limit=INT_MIN (256 sub-rows) | [x] |
| 77 | every total_bands value | G=4, T = each of 0..=255, ba∈1..16, limit=huge (256 sub-rows) | [x] |
| 78 | group_size sweep × path | G ∈ {-64,-18,-3,-2,-1,0,1,2,3,4,6,12,18,32,64,576} × {linear, grouped, full-domain read-free} (48 sub-rows) | [x] |

Rows 1–78 are implemented in `tests/phase_b_valid_paths.rs`; the row number is
the `Case` id, so a failure message names the row directly.

### Note — why some rows must use `limit = INT_MIN`

`get_bits` does `bs->pos += n` *unconditionally*, before deciding whether to
reject. For shift residues `k >= 25` the grouped `n` exceeds 58 million, and for
`k == 30` it is `1_879_048_195`. After two such calls `bs->pos` overflows `int`;
once it wraps negative the `> bs->limit` guard stops firing and the **C itself**
dereferences `bs->buf + (pos >> 3)` hundreds of megabytes out of bounds and
segfaults. That is a fault in the C for those inputs, not a translation
difference.

Setting `bs->limit = INT_MIN` makes `pos > limit` true for every `pos` except
exactly `INT_MIN`, so `get_bits` always takes its early-out and never touches
the buffer. The full `ba` domain (all 32 shift residues, the `2 << 30` signed
overflow, `mod == 1`, the `code % mod - mod / 2` cast) is therefore compared
read-free in rows 60–72 and 76, while rows that *do* read the buffer have their
reachable `k` capped by `kcap_for()` in `tests/common/mod.rs` so `bs->pos`
provably cannot overflow. `kcap_remap()` preserves the *magnitude* of `ba` while
lowering its residue, so high `ba` values are still exercised on the read path.

The only thing not covered on the read path is `n > 8_388_480` bits
(`k >= 23`), which would need a >1.8 MB bitstream *and* an overflowing
`bs->pos`; row 73 covers the largest feasible case, `k = 22` / `n = 7_340_035`.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the cross-product of
feature combinations is the single default/empty set. `run_all.sh` derives the
list from `Cargo.toml` and runs the suite under both `<default features>` and
`--no-default-features`; both pass.

## Row → test mapping

| rows | test function in `tests/phase_b_valid_paths.rs` |
|---|---|
| 1–10 | `rows_01_10_linear_group_size_and_total_bands` |
| 11–15 | `rows_11_15_bit_alignment_and_negative_pos` |
| 16–30 | `rows_16_30_grouped_path_individual_values` |
| 31–39 | `rows_31_39_ranges_and_mixed_paths` |
| 40–47 | `rows_40_47_exhaustion_and_degenerate_buffers` |
| 48–54 | `rows_48_54_degenerate_sizes` |
| 55–59 | `rows_55_59_pos_extremes_and_limit_boundary` |
| 60–72 | `rows_60_72_full_ba_domain_without_reads` |
| 73 | `row_73_widest_feasible_grouped_read` |
| 74 | `row_74_every_linear_ba_value` |
| 75 | `row_75_every_readable_shift_residue` |
| 76 | `row_76_every_ba_value_0_255_without_reads` |
| 77 | `row_77_every_total_bands_value` |
| 78 | `row_78_group_size_sweep` |

## Known limitations of the fixture

These are properties of the C that the differential harness deliberately does
not put in a position to be observed, because doing so would make the test
itself non-deterministic rather than because the Rust is unverified:

1. **`grbuf` writes aliasing `sci`.** `dst` ranges over
   `[group_size*j, 4*group_size + 5165]`, so with an adjacent allocation the C
   could overwrite `sci->total_bands` mid-loop and change the `i`-loop bound
   (which `-O0` gcc re-reads on every iteration, exactly as the Rust does). The
   fixture gives `grbuf` its own allocation sized by `grbuf_len()` — asserted
   sufficient by `phase_d_symbols::harness_writes_stay_inside_grbuf` — so the
   two never overlap and the outcome is reproducible.
2. **`sci == NULL`.** Faults unconditionally in both; see `ERRORS.md`.
3. **`n > 8_388_480` bits on the reading path** (grouped `k >= 23`). Needs both
   a multi-megabyte bitstream and an overflowing `bs->pos`; row 73 covers the
   largest feasible case.

## Sensitivity evidence

The suite was mutation-tested against `src/lib.rs` to confirm it is not
vacuous. All 15 behaviour-changing mutations were caught, including:
`choff = -choff`, `ba <= 17`, `pos >= limit`, dropping the `255 >> s` first-byte
mask, moving the early-out before `bs->pos += n`, `1 << ba` instead of
`1 << (ba-1)`, masking the `bitalloc` index to `& 63`, dropping the `+ 1` in
`mod`, dropping `- (mod >> 3)` in `n`, `next` instead of `next >> -shl`,
`group_size * 2`, `j < 3`, `total_bands` instead of `2 * total_bands`,
`choff = 18` initially, and `mod / 3` instead of `mod / 2`.

Three mutations were *not* flagged; each was checked by hand and is a
semantically equivalent rewrite, not a coverage gap:

* resetting `choff = 576` at the top of each `j` iteration — `2 * total_bands`
  is always even, so `choff` is already back at `576` there;
* `((code % m) as i32).wrapping_sub((m / 2) as i32)` instead of
  `(code % m).wrapping_sub(m / 2) as i32` — two's-complement wrapping
  subtraction is bit-identical in `u32` and `i32`;
* `if shl < 0 { break }` instead of `if shl <= 0 { break }` — when `shl == 0`
  the extra iteration ORs in `next << 0` and then returns `... | (next >> 8)`,
  and `next` is always a single byte, so `next >> 8 == 0`.
