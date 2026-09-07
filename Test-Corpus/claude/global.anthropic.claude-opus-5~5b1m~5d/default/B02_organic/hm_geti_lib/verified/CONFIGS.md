# CONFIGS.md — Phase B configuration surface

Axes derived mechanically from the branches in `c_src/src/lib.c`:

* **entry point** — all 16 exported symbols; the low-level `stbds_*` ones are
  driven directly, not only through `hm_geti`.
* **`mode`** (`int`, passed to `hmget_key`, `hmget_key_ts`, `hmput_key`,
  `hmdel_key`) — branch is `mode >= STBDS_HM_STRING(1)`; `hmdel_key` also has a
  distinct `mode == STBDS_HM_STRING` **equality** test. Values that matter:
  `<0`, `0` (binary), `1` (string), `>=2`.
* **`string.mode`** (`unsigned char` in the hash index) — `SH_NONE(0)`,
  `SH_DEFAULT(1)`, `SH_STRDUP(2)`, `SH_ARENA(3)`, plus out-of-range. Set either
  implicitly by the first `hmput_key` (`mode>=1 ? SH_DEFAULT : 0`) or explicitly
  by `stbds_shmode_func`.
* **`elemsize` / `keysize`** — arbitrary; `keyoffset` is always 0 from the
  macros but `hmdel_key` takes it as a parameter.
* **table shape** — `slot_count` 8 → 16 → 32 … (grows when
  `used_count >= slot_count - (slot_count>>2)`); shrinks when
  `used_count < slot_count>>2 && slot_count > 8`; rebuilds when
  `tombstone_count > (slot_count>>3)+(slot_count>>4)`.
* **array shape** — `NULL` / capacity-0 / 1 / many; `capacity` doubling in
  `stbds_arrgrowf` with the `<4` clamp.
* **seed state** — `stbds_hash_seed` starts at `0x31415926` and is advanced by
  `seed = seed*a + b` on **every** `make_hash_index(.., NULL)`; `stbds_rand_seed`
  overrides it. All hash values therefore depend on the call history, so each
  row replays an identical call sequence against both libraries.
* **arena shape** — `remaining` 0 vs >0, `block` 0..11+ (blocksize
  `512 << (block>>1)` saturating at `1<<20`), `len <= blocksize` vs `len >`.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `stbds_hash_bytes` | `len` = 0, 1..8, 9, 15, 16, 17, 23, 24, 31, 32, 33, 64, 127 × random bytes × seeds {0, 1, 0x31415926, `usize::MAX`, random} | [x] |
| 2  | `stbds_hash_bytes` | all-`0x00`, all-`0xFF`, and high-bit-set tail bytes (exercises the `d[3]<<24` sign-extension) at every `len % 8` | [x] |
| 3  | `stbds_hash_string` | lengths 0..64, random ASCII, random bytes incl. `>= 0x80` (signed-`char` promotion), × seeds {0, 1, default, `usize::MAX`, random} | [x] |
| 4  | `stbds_rand_seed` + `stbds_hash_bytes`/`stbds_hash_string` | seed set to 0 / 1 / `usize::MAX` / random, then hash — confirms the global is the *only* seed input to explicit hashing | [x] |
| 5  | `stbds_rand_seed` + `stbds_hmput_key` | seed reset, then a fixed insert sequence — confirms the seed advance `seed*a+b` in `make_hash_index` is identical (else hashes and hence probe order diverge) | [x] |
| 6  | `stbds_arrgrowf` | `a = NULL`, `(addlen, min_cap)` ∈ {(0,0),(0,1),(1,0),(0,4),(5,0),(0,100),(100,7)} × elemsize {1,2,4,8,16,24,64} — checks the `<4` clamp and the zeroed header | [x] |
| 7  | `stbds_arrgrowf` | existing array, repeated growth `addlen` 1..N — checks the `min_cap < 2*cap` doubling and the `min_cap <= cap` early return (same pointer, untouched header) | [x] |
| 8  | `stbds_arrgrowf` + `stbds_arrfreef` | grow, write element bytes, grow again, verify the payload survives the realloc, then free | [x] |
| 9  | `stbds_hmput_default` | `a = NULL` (fresh) → default slot at `t[-1]`; then again on the same map (idempotent branch) | [x] |
| 10 | `stbds_hmput_default` | on a map whose `length` was forced to 0 (the "resurrect" branch) | [x] |
| 11 | `stbds_hmput_default` + `stbds_hmget_key` | default set, then miss lookup — value read from `t[-1]`, `temp == -1` | [x] |
| 12 | `stbds_hmput_key` / `stbds_hmget_key` | **binary** mode 0, elemsize 8 / keysize 4 (`{int key; int value;}`), N ∈ {0,1,2,5,6,7,8,9,13,50,200,1000} random distinct keys, then look up every inserted key + misses | [x] |
| 13 | `stbds_hmput_key` / `stbds_hmget_key` | **binary** mode 0, elemsize 16 / keysize 8 (`{size_t key; size_t value;}`), same N sweep | [x] |
| 14 | `stbds_hmput_key` / `stbds_hmget_key` | **binary** mode 0, elemsize 24 / keysize 8 and elemsize 12 / keysize 4 (odd/unaligned strides) | [x] |
| 15 | `stbds_hmput_key` | **binary** duplicate-key re-put (update path: `is_key_equal` hit in the first probe scan) — `temp` must be the existing index, `length` unchanged | [x] |
| 16 | `stbds_hmput_key` | **binary**, keys crafted so probing wraps into the *second* (`i = 0..limit`) scan of the bucket, and so `pos += step` chains across buckets | [x] |
| 17 | `stbds_hmput_key` | **binary**, insertion count driven exactly across each growth threshold 8→16→32→64→128 (`used_count >= slot_count-(slot_count>>2)`) with `length`/`capacity`/`temp` compared at every step | [x] |
| 18 | `stbds_hmget_key_ts` | **binary**, `temp` out-param on hit / miss / `a == NULL` / `hash_table == NULL`; header `temp` must be left ALONE by `_ts` (unlike `hmget_key`) | [x] |
| 19 | `stbds_hmdel_key` | **binary**, delete each key in insertion order / reverse order / random order; `temp` = 1 on success, 0 on miss; `length` decrements; moved-last-element re-index verified by re-looking-up every survivor | [x] |
| 20 | `stbds_hmdel_key` | **binary**, delete-until-shrink (`used_count < slot_count>>2 && slot_count > 8`) from a 200-key map | [x] |
| 21 | `stbds_hmdel_key` | **binary**, delete pattern producing `tombstone_count > (slot_count>>3)+(slot_count>>4)` (rebuild-at-same-size branch) | [x] |
| 22 | `stbds_hmdel_key` | **binary**, delete then re-insert the same keys (tombstone reuse in `hmput_key`: `tombstone >= 0` → `--tombstone_count`) | [x] |
| 23 | `stbds_hmdel_key` | **binary**, non-zero `keyoffset` (e.g. elemsize 16, key at offset 8) — only reachable via the raw entry point | [x] |
| 24 | `stbds_shmode_func` | `mode` = `SH_NONE(0)` → `hmput_key(mode=1)`: `switch` hits `default:` → `memcpy` of `keysize` bytes over the `char*` field | [x] |
| 25 | `stbds_shmode_func` | `mode` = `SH_DEFAULT(1)` → `hmput_key(mode=1)` stores the caller's `char*` verbatim; `temp_key` updated | [x] |
| 26 | `stbds_shmode_func` | `mode` = `SH_STRDUP(2)` → keys `strdup`'d; `hmdel_key(mode==1)` frees them; `hmfree_func` frees the rest | [x] |
| 27 | `stbds_shmode_func` | `mode` = `SH_ARENA(3)` → keys go through `stbds_stralloc`; `hmfree_func` runs `strreset` | [x] |
| 28 | `stbds_shmode_func` | `mode` out of range: 4, 99, 255, 256 (truncates to 0), −1 (truncates to 255) — all fall into the `default: memcpy` arm | [x] |
| 29 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmdel_key` | **string** mode 1 with no `shmode_func`: first put sets `string.mode = SH_DEFAULT`; keys are borrowed pointers; N ∈ {1,2,7,8,50,300} random strings, incl. `""` and 200-char strings | [x] |
| 30 | same | **string** mode 1, duplicate-key re-put — checks `temp_key` is written in the first probe scan but NOT in the wrap scan | [x] |
| 31 | same | **string** mode 1 + `SH_STRDUP`: put, get, del, re-put, `hmfree_func` | [x] |
| 32 | same | **string** mode 1 + `SH_ARENA`: enough keys/lengths to force multiple arena blocks and the oversized-string path | [x] |
| 33 | `hmget_key`/`hmput_key`/`hmdel_key` | `mode = 2` (out-of-range enum, `>= STBDS_HM_STRING`): hashing/compare take the STRING path everywhere, but `hmdel_key`'s `mode == STBDS_HM_STRING` strdup-free and re-find branches take the BINARY arm | [x] |
| 34 | `hmget_key`/`hmput_key`/`hmdel_key` | `mode = -1` / `-1000` (negative, out-of-range enum): BINARY path everywhere; `string.mode` initialised to 0 | [x] |
| 35 | `stbds_hmfree_func` | on a binary map, a `SH_DEFAULT` string map, a `SH_STRDUP` map, a `SH_ARENA` map, and on a plain `arrgrowf` array with `hash_table == NULL` | [x] |
| 36 | `stbds_stralloc` | fresh arena; strings of length 1 (`""`), 10, 511, 512, 513, 1000, 4096, 100000 — covers `len <= blocksize` and `len > blocksize` | [x] |
| 37 | `stbds_stralloc` | repeated allocation until several blocks are consumed, checking `block` progression `512<<(block>>1)` and the `<1<<20` saturation; returned strings all readable | [x] |
| 38 | `stbds_stralloc` + `stbds_strreset` | alloc many, reset, alloc again (arena fully zeroed and reusable); reset on an untouched arena | [x] |
| 39 | `strkey` | `n` = 0, 1, −1, 12345, `INT_MAX`, `INT_MIN` — returned C string bytes compared | [x] |
| 40 | `hm_geti` | `num` = 0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 32, 33, 63, 64, 100, 257, 1000, 5000, and negatives — the full driver, run in both libs; must not abort | [x] |
| 41 | `hm_geti` | repeated calls in one process (seed global keeps advancing) — 20 consecutive calls with the same `num`, and after `stbds_rand_seed(k)` | [x] |
| 42 | mixed pipeline | `shmode_func(SH_STRDUP)` → 100 puts → 40 dels → 60 puts → gets of all → `hmfree_func`, with `rand_seed` fixed; full state snapshot compared after every operation | [x] |
| 43 | mixed pipeline | binary map: `hmput_default` → puts → dels → `hmput_default` again → gets → grow/shrink cycles, snapshot compared after every operation | [x] |

## Result

All 43 rows pass across randomized inputs (fixed PRNG seeds — splitmix64, seeded
per row, so every run is reproducible).

| test file | rows | tests |
|-----------|------|-------|
| `tests/phase_b.rs`  | 1-23  | 23 |
| `tests/phase_b2.rs` | 24-43 (+ an extra forward-delete variant of 25/27) | 21 |

Notes on what is compared, per operation, for every row:

* the returned index / `temp` sentinel;
* the full array header (`length`, `capacity`, `temp`, `hash_table != NULL`);
* the full hash index (`slot_count`, `used_count`, all three thresholds,
  `slot_count_log2`, `tombstone_count`, `seed`, `string.mode`, `string.block`,
  `string.remaining`);
* **every** bucket slot: the complete `hash[8]` and `index[8]` arrays of every
  bucket (so probe order, tombstones and rehash placement are all compared);
* the raw element bytes `t[-1 .. length-1]` (masked over the key field only when
  it holds an implementation-chosen pointer, in which case the pointed-to C
  string is compared instead), plus the hash index's `temp_key` string.

**Binary executable:** `c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)` — the project builds no driver executable, so
there is no stdout to compare. The nearest equivalent, the `hm_geti` driver
entry point, is run in both libraries (rows 40/41) and compared through its only
observable effect: the advance of the internal `stbds_hash_seed` global.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so there is exactly one
configuration. `verify.sh` extracts the feature list from `Cargo.toml`
mechanically and loops over every combination; with none declared it runs the
single default configuration. The suite additionally passes under the `dev`
profile (`cargo test --offline`), which turns on `overflow-checks` — confirming
the translation reproduces C's wrapping arithmetic explicitly rather than
relying on release-mode wrapping.
