# CONFIGS.md — configuration surface (valid inputs)

Axes the C code actually branches on (derived from `c_src/src/lib.c`):

* **A. entry point** — the 16 exported functions, from lowest level
  (`stbds_hash_bytes`, `stbds_hash_string`, `stbds_arrgrowf`) up through the
  hash-map primitives (`stbds_hmput_key`, `stbds_hmget_key[_ts]`,
  `stbds_hmdel_key`, `stbds_hmput_default`, `stbds_shmode_func`) to the
  one-shot driver `str_dups`.
* **B. `int mode`** — `STBDS_HM_BINARY (0)` vs `STBDS_HM_STRING (1)`; the code
  tests `mode >= STBDS_HM_STRING` (hash + compare) but `mode == STBDS_HM_STRING`
  (strdup free in `hmdel_key`).
* **C. `string.mode`** (set by `stbds_shmode_func` / implicitly by
  `hmput_key`) — `STBDS_SH_NONE (0)`, `STBDS_SH_DEFAULT (1)`,
  `STBDS_SH_STRDUP (2)`, `STBDS_SH_ARENA (3)`; selects the `switch` at line 785
  and the free loop in `hmfree_func`.
* **D. `elemsize` / `keysize`** — 8, 16, 24, 32, 40 … (key width 1/2/4/8/16
  bytes, padding, key at offset 0).
* **E. `keyoffset`** — `hmdel_key` takes it explicitly (0 and non-zero).
* **F. element count** — 0, 1, 2, 5 (`used_count_threshold` for 8 slots = 6),
  6 (first grow), 7…12 (second grow), 100, 1000 (repeated grows).
* **G. table lifecycle** — fresh / after grow / after delete-shrink / after
  tombstone rebuild / after `hmput_default`.
* **H. seed** — default `0x31415926`, plus `stbds_rand_seed(0)`, `1`,
  `usize::MAX`, `1<<63`; and the *evolution* of the global seed across
  successive `make_hash_index` calls.
* **I. byte-string shape** — length 0..24 (multiple-of-8 vs remainder 1..7),
  bytes `>= 0x80` (sign-extension paths), all-zero, all-`0xff`.
* **J. arena shape** — string length vs `blocksize` (`512 << (block>>1)`),
  fresh vs populated arena, `block` saturation at `1<<20`, reuse after
  `strreset`.
* **K. `str_dups(num)`** — `num` = 0, 1, 2, 3, 7, 8, 64, 100, 512, 2000.

Comparison method: every row drives BOTH `.so`s through identical operation
sequences via `libloading`, then compares the *entire* observable state —
`stbds_array_header{length,capacity,temp}`, `stbds_hash_index{slot_count,
used_count,used_count_threshold,used_count_shrink_threshold,tombstone_count,
tombstone_count_threshold,seed,slot_count_log2,string{remaining,block,mode}}`,
every `stbds_hash_bucket.hash[8]`/`.index[8]`, the element payload bytes, and
the C-string contents behind key pointers. Raw pointer values are excluded
(they legitimately differ); `temp_key` is compared by string content.
Randomized rows use a fixed-seed xorshift PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, `p = NULL`, seeds {0, 1, 0x31415926, 1<<63, MAX} | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (every `switch` fall-through arm), random bytes incl. `>= 0x80`, 200 random cases/len | [x] |
| 3 | `stbds_hash_bytes` | `len = 8` exactly (one full loop iteration, `rem = 0`) | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..23` (loop + remainder), random bytes, 200 cases/len | [x] |
| 5 | `stbds_hash_bytes` | `len = 24, 64, 256, 1000` (many loop iterations), random bytes | [x] |
| 6 | `stbds_hash_bytes` | all-zero and all-`0xff` buffers, `len = 1..17` | [x] |
| 7 | `stbds_hash_bytes` | random seeds (500 random `usize` seeds) × random 16-byte payload | [x] |
| 8 | `stbds_hash_string` | `""` (empty), seeds {0, 1, default, 1<<63, MAX} | [x] |
| 9 | `stbds_hash_string` | ASCII strings length 1..40, random, 200 cases | [x] |
| 10 | `stbds_hash_string` | strings containing bytes `0x80..0xff` (high-bit chars), length 1..32 | [x] |
| 11 | `stbds_hash_string` | long strings (256, 1024 bytes) | [x] |
| 12 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to {0, 1, MAX, 1<<63, 0x31415926}; observe `table->seed` and the LCG evolution over 8 successive table creations | [x] |
| 13 | `stbds_arrgrowf` | `a = NULL`, `elemsize` ∈ {1,4,8,16,24}, `addlen` ∈ {0,1,3,7}, `min_cap` ∈ {0,1,4,5,100} — check `length/capacity/temp/hash_table` | [x] |
| 14 | `stbds_arrgrowf` | existing array: repeated growth (doubling), `min_cap <= arrcap` no-op, `min_cap` in `(arrcap, 2*arrcap)`, `min_cap < 4` | [x] |
| 15 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free a non-NULL array (no leak/crash, header intact before free) | [x] |
| 16 | `stbds_hmput_key` | `mode = BINARY`, `elemsize = 8`, `keysize = 4`, 1 insert (fresh table, `string.mode = NONE`) | [x] |
| 17 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 4`, `elemsize = 8`, 5 inserts (below grow threshold) | [x] |
| 18 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 4`, 6 inserts (exactly triggers first grow to 16 slots) | [x] |
| 19 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 4`, 100 random `u32` keys (multiple grows, duplicate keys revisited) | [x] |
| 20 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 8`, `elemsize = 16`, 1000 random `u64` keys | [x] |
| 21 | `stbds_hmput_key` | `mode = BINARY`, `keysize = 1` / `2` / `16`, `elemsize` = 8 / 8 / 32, 200 random keys each (narrow + wide keys, padding bytes) | [x] |
| 22 | `stbds_hmput_key` | `mode = BINARY`, re-put of an existing key (update path, line 730 → `temp` = existing index, no `used_count` change) | [x] |
| 23 | `stbds_hmput_key` | `mode = STRING` on a NULL map (implicit `string.mode = STBDS_SH_DEFAULT`), 1 insert — key pointer stored verbatim | [x] |
| 24 | `stbds_hmput_key` | `mode = STRING`, implicit `SH_DEFAULT`, 200 random distinct strings (grows) | [x] |
| 25 | `stbds_shmode_func(SH_STRDUP)` + `stbds_hmput_key(STRING)` | 1 insert; key must be a *copy* (content equal, pointer different) | [x] |
| 26 | `stbds_shmode_func(SH_STRDUP)` + `stbds_hmput_key(STRING)` | 200 random strings, lengths 0..40, duplicates re-put | [x] |
| 27 | `stbds_shmode_func(SH_ARENA)` + `stbds_hmput_key(STRING)` | 200 random strings incl. > 512 bytes (forces arena block growth); compare `string.{remaining,block,mode}` | [x] |
| 28 | `stbds_shmode_func(SH_NONE)` + `stbds_hmput_key(STRING)` | `string.mode = 0` → `default:` arm `memcpy`s the *pointer bytes*; hashing still by string | [x] |
| 29 | `stbds_shmode_func(SH_DEFAULT)` + `stbds_hmput_key(STRING)` | explicit `SH_DEFAULT` table | [x] |
| 30 | `stbds_hmget_key_ts` | `a = NULL` (bootstrap path), `elemsize` ∈ {8,16,32} | [x] |
| 31 | `stbds_hmget_key_ts` | array with `hash_table == NULL` (built by `arrgrowf`) → `*temp = -1` | [x] |
| 32 | `stbds_hmget_key_ts` | `mode = BINARY`, 100-key map: lookup of every present key + 100 absent keys | [x] |
| 33 | `stbds_hmget_key_ts` | `mode = STRING`, `SH_STRDUP` map, 100 present + 100 absent string keys | [x] |
| 34 | `stbds_hmget_key` | same as rows 32/33 but through the non-`_ts` wrapper (also writes `header->temp`) | [x] |
| 35 | `stbds_hmput_default` | `a = NULL`; then again on the returned map (`length != 0`, no-op); then on a `length == 0` array | [x] |
| 36 | `stbds_hmput_default` + `stbds_hmput_key` | default entry present at index −1, then inserts (`elemsize` 8 and 16) | [x] |
| 37 | `stbds_hmdel_key` | `mode = BINARY`, map of 1 element, delete it (`old_index == final_index`) | [x] |
| 38 | `stbds_hmdel_key` | `mode = BINARY`, map of 10, delete first (swap-with-last + slot re-point) | [x] |
| 39 | `stbds_hmdel_key` | `mode = BINARY`, map of 10, delete last (no memmove) | [x] |
| 40 | `stbds_hmdel_key` | `mode = BINARY`, 200 inserts then random interleaved delete/insert/get (drives shrink AND tombstone rebuild) | [x] |
| 41 | `stbds_hmdel_key` | `mode = BINARY`, 64 inserts then delete 60 → repeated shrink (`slot_count` 128→…→8) | [x] |
| 42 | `stbds_hmdel_key` | `mode = BINARY`, delete-then-reinsert to force tombstone reuse (`tombstone_count` −1) | [x] |
| 43 | `stbds_hmdel_key` | `mode = STRING`, `SH_STRDUP`, 100 strings then delete all (strdup keys freed on the strdup path) | [x] |
| 44 | `stbds_hmdel_key` | `mode = STRING`, `SH_ARENA`, 100 strings, delete half | [x] |
| 45 | `stbds_hmdel_key` | `mode = STRING`, `SH_DEFAULT`, delete half | [x] |
| 46 | `stbds_hmdel_key` | `keyoffset != 0`: `elemsize = 16`, key at offset 8, `mode = BINARY`, insert-by-hand then delete | [x] |
| 47 | `stbds_hmfree_func` | strdup map (frees each key), arena map, default map, binary map, and `hash_table == NULL` array | [x] |
| 48 | `stbds_stralloc` | fresh arena, one short string (`len <= 512`) — check `remaining`, `block`, returned content | [x] |
| 49 | `stbds_stralloc` | fresh arena, one oversized string (`len = 2000 > 512`) — dedicated-block path with `storage == NULL` | [x] |
| 50 | `stbds_stralloc` | populated arena then oversized string (splice-after-head path) | [x] |
| 51 | `stbds_stralloc` | 500 random strings length 0..100 on one arena (many block rolls, `block` increments) | [x] |
| 52 | `stbds_stralloc` | strings sized to walk `block` up to saturation: lengths 1<<9 … 1<<21, verify `block` stops at 22 | [x] |
| 53 | `stbds_stralloc` + `stbds_strreset` | alloc, reset, alloc again (arena reuse); reset on fresh arena (idempotent) | [x] |
| 54 | `strkey` | `n` = 0, 1, 9, 10, 99, 100, 12345, `INT_MAX`, −1, `INT_MIN` | [x] |
| 55 | `str_dups` | `num` = 0, 1, 2, 3, 7, 8, 64, 100, 512, 2000, −1, `INT_MIN`; **stdout captured and compared byte-for-byte** | [x] |
| 56 | mixed pipeline | `rand_seed` → `shmode_func(STRDUP)` → 300 random put/get/del ops → `hmfree_func`, full state compared after every op | [x] |
| 57 | mixed pipeline | `rand_seed` → binary map `elemsize=24, keysize=8` → 300 random put/get/del ops → `hmfree_func` | [x] |

## Status

All 57 rows verified: `tests/phase_b.rs` has one `#[test]` per row
(`row01_…` … `row57_…`), plus `tests/stdout_diff.rs` for row 55.
`cargo test --release` (single- and multi-threaded) passes 56 + 1 tests.
Only one cargo configuration exists (`Cargo.toml` declares no `[features]`),
so the default build is the complete feature matrix; `./run_all.sh` enumerates
it mechanically and would fan out automatically if features were added.
