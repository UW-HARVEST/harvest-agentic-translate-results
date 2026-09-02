# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from the branch points in `c_src/src/lib.c`. The library
has no `#ifdef` feature switches left after amalgamation (`STBDS_HAS_TYPEOF`,
`STBDS_SIPHASH_2_4`, `STBDS_UNIT_TESTS` are all resolved at author time), so the
configuration axes are all **runtime**.

## Axes the C actually branches on

**A1 — `mode` argument** (`hmput_key`, `hmget_key`, `hmget_key_ts`, `hmdel_key`)
The C tests it two different ways:
* `mode >= STBDS_HM_STRING` (i.e. `>= 1`) → string hashing (`stbds_hash_string`)
  + `strcmp` key comparison. Used in `stbds_is_key_equal`, `hm_find_slot`,
  `hmput_key`.
* `mode == STBDS_HM_STRING` (exact) → gates the strdup-free and the string
  re-find inside `hmdel_key`.
Distinct classes: `mode < 1` (binary; includes negatives), `mode == 1`
(string), `mode > 1` (string hashing **but** binary re-find in `hmdel_key`).

**A2 — `table->string.mode`** (the `switch` at line 785, plus `hmfree_func`)
`SH_NONE = 0` (memcpy key bytes), `SH_DEFAULT = 1` (store caller pointer),
`SH_STRDUP = 2` (`malloc`+copy, freed on delete/free), `SH_ARENA = 3`
(`stbds_stralloc`). Set implicitly by `hmput_key` on a fresh table
(`(mode>=1) ? SH_DEFAULT : 0`) or explicitly by `stbds_shmode_func`.

**A3 — `elemsize`** Element stride. Affects `elemsize*i` addressing everywhere
and the `arr <-> hash` pointer bias. Shapes: 8 (pointer-sized), 16 (the `helxo`
element), 24, 12 (not a multiple of 8), 4, 1.

**A4 — `keysize`** `memcmp`/`memcpy` width in binary mode. Shapes: 0, 1, 2, 4,
8, 16, 24. (Ignored in string mode.)

**A5 — element count / table growth** `used_count_threshold = n - n/4`, so with
`slot_count = 8` the 7th distinct insert triggers the doubling to 16. Shapes:
0, 1, 5 (below threshold), 6 (at threshold), 7 (first grow), 100, 1000
(multiple grows).

**A6 — deletion pattern** Drives `final_index` swap-back, tombstone
accumulation (`tombstone_count_threshold = n/8 + n/16`) and shrink
(`used_count < n/4 && slot_count > 8`). Shapes: delete-absent, delete-last
(no swap), delete-first (swap), delete-middle, delete-all, delete-then-reinsert
(tombstone reuse), interleaved churn.

**A7 — `seed`** `stbds_rand_seed` sets a file-static that is captured into
`table->seed` at table-creation time and then advanced by the LCG
`seed = seed*a + b`. Shapes: untouched default `0x31415926`, `0`, `1`,
`SIZE_MAX`, an arbitrary 64-bit value.

**A8 — `stbds_hash_bytes` length** `len/8` full blocks plus a `len%8`
fall-through tail. Shapes: 0, 1..7 (each tail case), 8, 9..15, 16, 32, 33, 64,
and every length 0..64 in the sweep. Value shape: bytes `< 0x80` vs `>= 0x80`
at offsets 3 and 7 (the `int` sign-extension path).

**A9 — `stbds_hash_string` content** empty, 1 byte, ≤ 8 bytes, > 8 bytes, bytes
`>= 0x80`, embedded high-bit runs.

**A10 — `stbds_arrgrowf` shape** `a == NULL` vs existing; `addlen` 0 / 1 / n;
`min_cap` below capacity (early return), between capacity and `2*capacity`
(doubling), above `2*capacity` (exact), `< 4` (bumped to 4).

**A11 — arena state for `stbds_stralloc`** fresh (`storage = NULL`), partially
filled (`remaining > 0`), exhausted (`remaining < len`), `block` counter 0 / mid
/ saturated, string length ≤ blocksize vs > blocksize.

**A12 — entry point** All 16 exported symbols, including the low-level
`stbds_arrgrowf` / `stbds_hash_bytes` / `stbds_hm*_key` primitives, not just the
`helxo` one-shot wrapper.

## Table

Each row is exercised with many randomized inputs (fixed seed, see
`tests/common/mod.rs::Rng`) against **both** `.so`s, comparing return values,
the full `stbds_array_header`, the whole `stbds_hash_index` (slot counts,
thresholds, seed, `used_count`, `tombstone_count`) and every
`bucket.hash[]` / `bucket.index[]` entry byte-for-byte, plus the element payload
bytes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len` swept 0..64, random bytes, seed = default | [x] |
| 2 | `stbds_hash_bytes` | `len` swept 0..64, bytes forced `>= 0x80` (sign-extension path at offsets 3 and 7) | [x] |
| 3 | `stbds_hash_bytes` | `len` swept 0..64, bytes all `0x00`, all `0xFF` | [x] |
| 4 | `stbds_hash_bytes` | `len` ∈ {0,1,…,7} tail-only (no full block), random + high-bit | [x] |
| 5 | `stbds_hash_bytes` | `len` ∈ {8,16,32,64} exact multiples, random | [x] |
| 6 | `stbds_hash_bytes` | `len` large (256, 1024, 4096), random | [x] |
| 7 | `stbds_hash_bytes` | seed ∈ {0, 1, `SIZE_MAX`, `0x31415926`, random}, `len` swept | [x] |
| 8 | `stbds_hash_string` | random NUL-terminated ASCII, length 0..64, seed = default | [x] |
| 9 | `stbds_hash_string` | random bytes `0x80..0xFF`, length 1..64 | [x] |
| 10 | `stbds_hash_string` | seed ∈ {0, 1, `SIZE_MAX`, random} × length 0..32 | [x] |
| 11 | `stbds_rand_seed` + `stbds_hmput_key` | seed set on both libs, then a fresh table: `table->seed` and the LCG advance must match; repeated table creation advances the seed identically | [x] |
| 12 | `stbds_arrgrowf` | `a == NULL`, `elemsize` ∈ {1,4,8,12,16,24}, `addlen` ∈ {0,1,3,17}, `min_cap` ∈ {0,1,3,4,5,100} | [x] |
| 13 | `stbds_arrgrowf` | existing array, `min_cap <= capacity` → early return, pointer & header unchanged | [x] |
| 14 | `stbds_arrgrowf` | existing array, `capacity < min_cap < 2*capacity` → capacity becomes `2*capacity` | [x] |
| 15 | `stbds_arrgrowf` | existing array, `min_cap >= 2*capacity` → capacity becomes `min_cap` | [x] |
| 16 | `stbds_arrgrowf` | repeated growth chain (`addlen = 1` × 200) — capacity doubling sequence must match exactly | [x] |
| 17 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free (non-null) — no crash, allocator agreement | [x] |
| 18 | `stbds_hmput_default` | `a == NULL`, `elemsize` ∈ {1,8,16,24} | [x] |
| 19 | `stbds_hmput_default` | `a` from a previous `hmput_default` (`length == 1`) → unchanged | [x] |
| 20 | `stbds_hmput_default` | `a` whose `length` was forced to 0 → regrow to `length = 1` | [x] |
| 21 | `stbds_hmput_key` | binary mode (`mode = 0`), `elemsize = 8`, `keysize = 8`, 1 insert | [x] |
| 22 | `stbds_hmput_key` | binary mode, `elemsize = 8`, `keysize = 8`, 6 inserts (at threshold, pre-grow) | [x] |
| 23 | `stbds_hmput_key` | binary mode, `elemsize = 8`, `keysize = 8`, 7 inserts (first grow to 16 slots) | [x] |
| 24 | `stbds_hmput_key` | binary mode, `elemsize = 8`, `keysize = 8`, 1000 random inserts (many grows) | [x] |
| 25 | `stbds_hmput_key` | binary mode, `elemsize` ∈ {4,12,16,24} × `keysize` ∈ {1,2,4,8,16}, 200 inserts | [x] |
| 26 | `stbds_hmput_key` | binary mode, `keysize = 0` (every hash-equal key "matches") | [x] |
| 27 | `stbds_hmput_key` | binary mode, duplicate keys re-put (upper-loop hit) → `temp` = existing index, `length` unchanged | [x] |
| 28 | `stbds_hmput_key` | binary mode, keys chosen so the wrap-around sub-loop finds the duplicate (`temp_key` not updated) | [x] |
| 29 | `stbds_hmput_key` | string mode (`mode = 1`) on a fresh table → `string.mode = SH_DEFAULT`, key pointer stored verbatim, `temp_key` set | [x] |
| 30 | `stbds_hmput_key` | string mode, 500 random distinct strings (lengths 0..32) | [x] |
| 31 | `stbds_hmput_key` | string mode, duplicate strings (distinct buffers, equal contents) → existing slot reused | [x] |
| 32 | `stbds_hmput_key` | `mode = 2` / `3` / `99` / `INT_MAX` (out-of-range enum) → string hashing, `string.mode = SH_DEFAULT` | [x] |
| 33 | `stbds_hmput_key` | `mode = -1` / `INT_MIN` (out-of-range enum) → binary hashing, `string.mode = 0` | [x] |
| 34 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_STRDUP` table, 200 random strings → keys `malloc`-duplicated, `temp_key` = the duplicate | [x] |
| 35 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_ARENA` table, 200 random strings → keys arena-allocated; arena `block`/`remaining` progression must match | [x] |
| 36 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_ARENA` table with strings > 512 bytes (oversized-block path inside `stralloc`) | [x] |
| 37 | `stbds_shmode_func` + `stbds_hmput_key` | `SH_NONE` (`0`) table with `mode = 1` → `switch` default arm: key **bytes** memcpy'd | [x] |
| 38 | `stbds_shmode_func` | `mode` ∈ {0,1,2,3,4,255,256,259,-1,`INT_MIN`,`INT_MAX`} → truncated to `unsigned char` | [x] |
| 39 | `stbds_hmget_key` | binary mode, present keys (all of them) after N inserts | [x] |
| 40 | `stbds_hmget_key` | binary mode, absent keys → `temp = -1` | [x] |
| 41 | `stbds_hmget_key` | string mode, present + absent keys | [x] |
| 42 | `stbds_hmget_key` | `SH_STRDUP` / `SH_ARENA` tables, present + absent keys | [x] |
| 43 | `stbds_hmget_key_ts` | same as rows 39–42 but through the `*temp` out-parameter; `header->temp` must stay untouched | [x] |
| 44 | `stbds_hmget_key_ts` | `a == NULL` bootstrap (`*temp = -1`, `length = 1`) | [x] |
| 45 | `stbds_hmdel_key` | binary mode, delete the last element (`old_index == final_index`, no swap) | [x] |
| 46 | `stbds_hmdel_key` | binary mode, delete the first element (swap-back + slot re-find) | [x] |
| 47 | `stbds_hmdel_key` | binary mode, delete a random middle element | [x] |
| 48 | `stbds_hmdel_key` | binary mode, delete every element in insertion order | [x] |
| 49 | `stbds_hmdel_key` | binary mode, delete every element in reverse order | [x] |
| 50 | `stbds_hmdel_key` | binary mode, delete every element in random order (1000 elements → shrink + rebuild both fire) | [x] |
| 51 | `stbds_hmdel_key` | binary mode, delete then re-insert the same keys (tombstone reuse) | [x] |
| 52 | `stbds_hmdel_key` | binary mode, enough deletes to cross `tombstone_count_threshold` → same-size rebuild | [x] |
| 53 | `stbds_hmdel_key` | binary mode, enough deletes to cross `used_count_shrink_threshold` with `slot_count > 8` → halve | [x] |
| 54 | `stbds_hmdel_key` | binary mode with `slot_count == 8` (`shrink_threshold == 0`) → never shrinks | [x] |
| 55 | `stbds_hmdel_key` | string mode (`mode = 1`), `SH_DEFAULT` table, mixed delete order | [x] |
| 56 | `stbds_hmdel_key` | string mode (`mode = 1`), `SH_STRDUP` table → key `free`d on delete | [x] |
| 57 | `stbds_hmdel_key` | string mode (`mode = 1`), `SH_ARENA` table → key **not** freed | [x] |
| 58 | `stbds_hmdel_key` | `mode = 2` (out-of-range) → string hash/compare but **binary** re-find of the moved element | [x] |
| 59 | `stbds_hmdel_key` | non-zero `keyoffset` (`elemsize = 24`, `keyoffset` ∈ {0,8,16}) | [x] |
| 60 | `stbds_hmfree_func` | `SH_NONE` table (binary) | [x] |
| 61 | `stbds_hmfree_func` | `SH_DEFAULT` table (string) | [x] |
| 62 | `stbds_hmfree_func` | `SH_STRDUP` table → all duplicated keys freed | [x] |
| 63 | `stbds_hmfree_func` | `SH_ARENA` table → arena blocks freed via `strreset` | [x] |
| 64 | `stbds_hmfree_func` | array with `hash_table == NULL` (from `arrgrowf` only) | [x] |
| 65 | `stbds_stralloc` | fresh arena (`{0}`), string lengths swept 0..40 | [x] |
| 66 | `stbds_stralloc` | fresh arena, many sequential allocs until the first block is exhausted (`remaining` progression, `block` increments) | [x] |
| 67 | `stbds_stralloc` | strings longer than 512 (first blocksize) → oversized-block path with `storage == NULL` | [x] |
| 68 | `stbds_stralloc` | oversized string **after** a normal block exists → splice-after-head path | [x] |
| 69 | `stbds_stralloc` | `block` pre-set to 1, 2, 10, 24, 40, 200 → `512 << (block>>1)` including saturation and ≥ 64-bit shifts | [x] |
| 70 | `stbds_stralloc` | ~2000 random-length allocations driving `block` from 0 to saturation at `1<<20` | [x] |
| 71 | `stbds_strreset` | arena with 0 / 1 / many blocks → arena fully zeroed | [x] |
| 72 | `strkey` | `n` ∈ {0, 1, -1, 42, `INT_MAX`, `INT_MIN`} + 200 random `i32` | [x] |
| 73 | `helxo` | `letter` swept over all 256 byte values, stdout captured and compared | [x] |
| 74 | `helxo` | `letter` after `stbds_rand_seed(x)` for several `x` (the demo's table seed changes, insertion order must not) | [x] |
| 75 | cross-library | table built by C's `hmput_key`, queried/deleted by Rust's `hmget_key`/`hmdel_key` and vice-versa (proves byte-identical in-memory layout) | [x] |
| 76 | full pipeline | `shmode_func(SH_ARENA)` → 300 `hmput_key` → 150 `hmdel_key` → 300 `hmget_key` → `hmfree_func`, all four `string.mode`s × `mode` ∈ {0,1,2}, headers + buckets compared after **every** step | [x] |

---

## Row → test mapping (Phase B)

All rows are checked off only after the named test passed against both `.so`s,
in both the `release` and `dev` profiles.

| rows | test file / test |
|------|------------------|
| 1–7 | `tests/hashes.rs::row01…row07` |
| 8–10 | `tests/hashes.rs::row08…row10` |
| 11 | `tests/hashes.rs::row11_rand_seed_and_lcg_advance` |
| 12–17 | `tests/arrays.rs::row12…row17` |
| 18–20 | `tests/arrays.rs::row18…row20` |
| 21–28 | `tests/maps_binary.rs::row21…row28` |
| 29–32 | `tests/maps_string.rs::row29…row32` |
| 33 | `tests/maps_binary.rs::row33_negative_mode_is_binary` |
| 34–38 | `tests/maps_string.rs::row34…row38` |
| 39, 40 | `tests/maps_binary.rs::row39_40_binary_get_present_and_absent` |
| 41, 42 | `tests/maps_string.rs::row41_42_string_get_present_and_absent` |
| 43, 44 | `tests/maps_binary.rs::row43_binary_get_ts`, `row44_get_ts_bootstrap_from_null` |
| 45–54 | `tests/maps_binary.rs::row45…row54` |
| 55–58 | `tests/maps_string.rs::row55…row58` |
| 59 | `tests/maps_binary.rs::row59_hmdel_key_with_keyoffset` |
| 60, 64 | `tests/maps_binary.rs::row60_hmfree_binary_table`, `row64_hmfree_array_without_table` |
| 61–63 | `tests/maps_string.rs::row61_62_63_hmfree_string_tables` |
| 65–70 | `tests/arena.rs::row65…row70` |
| 71 | `tests/maps_string.rs::row71_strreset_direct` |
| 72 | `tests/pipeline.rs::row72_strkey` |
| 73, 74 | `tests/helxo.rs::helxo_rows_73_and_74` |
| 75 | `tests/pipeline.rs::row75_cross_library_interop_binary`, `row75b_cross_library_interop_string_modes` |
| 76 | `tests/pipeline.rs::row76_full_pipeline_matrix` |

## What "outputs match byte-for-byte" means here

`stbds_hmput_key` & friends return a heap pointer, and the two libraries get
different `malloc` addresses, so raw pointer values are not comparable. What the
tests compare after **every single call** (`tests/common/mod.rs::Dual::check`) is:

* `stbds_array_header`: `length`, `capacity`, `temp`;
* the whole `stbds_hash_index`: `slot_count`, `slot_count_log2`, `used_count`,
  `used_count_threshold`, `used_count_shrink_threshold`, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, and the arena's `remaining` / `block` /
  `mode` / block-list length;
* **every** `bucket.hash[j]` and `bucket.index[j]` across all `slot_count` slots;
* the element key material — copied bytes for binary / `SH_NONE` tables, the
  (deliberately shared) caller pointer for `SH_DEFAULT`, the pointed-to strings
  for `SH_STRDUP` / `SH_ARENA`;
* a per-insert payload tag stamped into `[key_end, elemsize)`, so `hmdel_key`'s
  element `memmove` is observable;
* `table->temp_key`, once a string-mode put has initialised it.

`stbds_make_hash_index` never initialises `stbds_hash_index::temp_key`, in the C
or in the Rust, and `hmdel_key` swaps in a freshly `realloc`'d index on
shrink/rebuild, so `temp_key` is only compared while it is defined.

`row75`/`row75b` are the strongest layout checks: a table built by one library is
inserted into, queried, and deleted from by the *other* library. That can only
work if the header, hash index, bucket layout, hash functions and probe sequence
are bit-identical.

## Combinations deliberately excluded, and why

Two `(string.mode, mode)` families are excluded from `row76`. In both the C
itself leaves defined behaviour, identically for both libraries, so there is no
comparable observable:

* **`SH_NONE` + `mode >= 1`.** The `switch` default arm `memcpy`s the key
  *bytes* into the element, but `stbds_is_key_equal` then evaluates
  `strcmp(key, *(char **)elem)` — it reads those bytes back as a pointer. The
  first lookup of an already-present key dereferences a fabricated pointer and
  segfaults. The insert-only half of this configuration is row 37 and *is*
  tested.
* **`mode == 2` combined with a delete that relocates the final element.**
  `hmdel_key` gates the string re-find on `mode == STBDS_HM_STRING` exactly, so
  with `mode == 2` it re-finds the moved element in *binary* form while the slot
  was hashed as a string. The re-find fails and
  `STBDS_ASSERT(slot >= 0)` calls `abort()`. `mode == 2` is therefore covered by
  row 32 (inserts + lookups) and row 58 (deletes restricted to the final
  element, where that branch is skipped), plus
  `errors::err46_mode_two_skips_strdup_free`.

`stbds_arrfreef(NULL)`, `stbds_hash_string(NULL, …)` and
`stbds_strreset(NULL)` are missing null checks in the C that terminate the
process; see the "Not-testable rows" section of `ERRORS.md`.
