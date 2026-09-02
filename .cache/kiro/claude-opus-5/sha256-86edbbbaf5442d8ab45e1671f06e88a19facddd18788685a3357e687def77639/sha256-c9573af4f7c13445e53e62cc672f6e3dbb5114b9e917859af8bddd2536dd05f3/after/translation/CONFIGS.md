# CONFIGS.md — configuration surface (valid inputs)

Axes derived from the branches `c_src/src/lib.c` actually takes.

**Runtime options / modes the public API can set**

| axis | values the C code distinguishes | where it branches |
|------|--------------------------------|-------------------|
| `mode` (compare/hash selector) | `< 1` → binary (`memcmp` + `hash_bytes`); `>= 1` → string (`strcmp` + `hash_string`). `stbds_hmdel_key` additionally tests `mode == 1` **exactly** for the STRDUP free. | `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_hmput_key`, `stbds_hmdel_key` |
| `table->string.mode` (key-storage mode) | `STBDS_SH_NONE 0` → `memcpy` key bytes; `STBDS_SH_DEFAULT 1` → store caller pointer; `STBDS_SH_STRDUP 2` → `stbds_strdup`; `STBDS_SH_ARENA 3` → `stbds_stralloc`; anything else → `default:` = `memcpy` | `switch (table->string.mode)` in `stbds_hmput_key`; set by `stbds_shmode_func` or implicitly by the first `stbds_hmput_key` |
| global hash seed | `stbds_hash_seed` starts at `0x31415926`, is advanced by an LCG on every *fresh* `stbds_make_hash_index` (`ot == NULL`), and is settable by `stbds_rand_seed` | `stbds_make_hash_index`, `stbds_rand_seed` |
| arena `block` counter | `blocksize = 512 << (block >> 1)`, `++block` only while `blocksize < 1<<20` | `stbds_stralloc` |

**Input shapes the C code special-cases**

`elemsize` (any), `keysize` (any), key length vs `sizeof(size_t)` in siphash
(`< 8`, `== 8`, `> 8`, tail `1..7`), element count vs `STBDS_BUCKET_LENGTH 8`
and vs `used_count_threshold = slot_count - slot_count/4` (grow),
vs `used_count_shrink_threshold = slot_count/4` (shrink, disabled while
`slot_count <= 8`), vs `tombstone_count_threshold = slot_count/8 + slot_count/16`
(rebuild), `hash < 2` fix-up, `old_index == final_index` vs not on delete,
empty / one / many, `NULL` array, array with no hash index.

**Every public entry point** (16 of them, incl. the lowest-level ones):
`stbds_arrgrowf`, `stbds_arrfreef`, `stbds_rand_seed`, `stbds_hash_bytes`,
`stbds_hash_string`, `stbds_hmget_key_ts`, `stbds_hmget_key`,
`stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmdel_key`,
`stbds_shmode_func`, `stbds_stralloc`, `stbds_strreset`, `stbds_hmfree_func`,
`strkey`, `intput`.

All rows are exercised with many randomised inputs from a fixed-seed
`SplitMix64`, comparing the C `.so` and the Rust `.so` through `libloading`.
The comparison is a byte-for-byte digest of: returned values, the array header
(`length`, `capacity`, `temp`), every field of `stbds_hash_index` except the two
addresses (`temp_key`, `storage`) and `string.storage`, every bucket's
`hash[8]`/`index[8]`, and the live element bytes (key/value; for string key modes
the pointed-to C string plus the value word).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len == 0`, seeds {default, 0, 1, `SIZE_MAX`, random} | [x] |
| 2 | `stbds_hash_bytes` | `len ∈ 1..7` (tail-only switch, all fall-through cases), random bytes incl. `>= 0x80` in byte 3 | [x] |
| 3 | `stbds_hash_bytes` | `len == 8` (exactly one main-loop word, empty tail) | [x] |
| 4 | `stbds_hash_bytes` | `len ∈ 9..15` (one word + every tail length) | [x] |
| 5 | `stbds_hash_bytes` | `len ∈ {16, 24, 32}` multi-word, no tail | [x] |
| 6 | `stbds_hash_bytes` | `len ∈ 33..256` random (many words + random tail), random seed | [x] |
| 7 | `stbds_hash_string` | empty string, seeds {default, 0, `SIZE_MAX`, random} | [x] |
| 8 | `stbds_hash_string` | random ASCII, length 1..64 | [x] |
| 9 | `stbds_hash_string` | random bytes `0x80..0xFF` (unsigned-char promotion), length 1..64 | [x] |
| 10 | `stbds_hash_string` | long string 256..1024 bytes, mixed ASCII/high-bit | [x] |
| 11 | `stbds_arrgrowf` + `stbds_arrfreef` | `a == NULL`, `addlen ∈ {0,1,2,3,4,5,17,100}` × `min_cap ∈ {0,1,3,4,7,64}` × `elemsize ∈ {1,2,4,8,16,64}` | [x] |
| 12 | `stbds_arrgrowf` | existing array, `min_cap <= capacity` (early-out, pointer identity + header unchanged) | [x] |
| 13 | `stbds_arrgrowf` | existing array, `min_len < 2*capacity` → doubling branch | [x] |
| 14 | `stbds_arrgrowf` | existing array, `min_len >= 2*capacity` → `min_cap = min_len` branch | [x] |
| 15 | `stbds_arrgrowf` | repeated growth chain (10 successive grows) tracking `capacity` sequence | [x] |
| 16 | `stbds_rand_seed` | seed ∈ {0, 1, `0x31415926`, `SIZE_MAX`, random} then N fresh tables → LCG advance sequence of `table->seed` | [x] |
| 17 | `stbds_hmput_default` | `a == NULL`; `a` from `hmput_key` (`length > 0`); raw array with `length == 0` | [x] |
| 18 | `stbds_hmput_key` binary | `mode = 0`, `keysize = 4`, `elemsize = 8`, 1 element | [x] |
| 19 | `stbds_hmput_key` binary | `mode = 0`, insert 2..8 elements (crosses `used_count_threshold = 6` → first table grow to 16 slots) | [x] |
| 20 | `stbds_hmput_key` binary | `mode = 0`, insert 100 and 1000 random distinct keys (repeated grows/rehash, `slot_count` 8→2048) | [x] |
| 21 | `stbds_hmput_key` binary | re-put existing keys (found-in-forward-half and found-after-wrap paths) → `temp` = existing index, no length change | [x] |
| 22 | `stbds_hmput_key` binary | `keysize ∈ {1, 2, 4, 8, 12, 16, 32}` with `elemsize = keysize + 8` (siphash over key bytes of every tail length) | [x] |
| 23 | `stbds_hmput_key` binary | keys chosen so `hash < 2` fix-up and same-bucket collisions occur (dense low-entropy keys, e.g. all-zero / all-`0xFF` key bytes) | [x] |
| 24 | `stbds_hmput_key` binary | `mode` negative (`-1`, `INT_MIN`) — must behave exactly like `mode = 0` | [x] |
| 25 | `stbds_hmput_key` string, implicit `SH_DEFAULT` | `mode = 1` on `a == NULL` → `string.mode = STBDS_SH_DEFAULT`; keys are caller pointers; `temp_key` written | [x] |
| 26 | `stbds_hmput_key` string, `SH_STRDUP` | table from `stbds_shmode_func(elemsize, 2)`, `mode = 1`, random strings | [x] |
| 27 | `stbds_hmput_key` string, `SH_ARENA` | table from `stbds_shmode_func(elemsize, 3)`, `mode = 1`, random strings incl. > 512-byte ones (arena block growth inside the map) | [x] |
| 28 | `stbds_hmput_key` string, `SH_NONE` | table from `stbds_shmode_func(elemsize, 0)` with `mode = 1` → `default:` `memcpy` branch on a "string" table | [x] |
| 29 | `stbds_hmput_key` string | `mode = 2` (`STBDS_HM_PTR_TO_STRING`) and `mode = INT_MAX` — string hash/compare, `SH_DEFAULT` storage | [x] |
| 30 | `stbds_hmput_key` string | duplicate string keys (distinct buffers, equal contents) → dedup + `temp_key` points at the *stored* key | [x] |
| 31 | `stbds_hmget_key_ts` | `a == NULL`; `a` with `hash_table == NULL`; present key; absent key — binary and string modes | [x] |
| 32 | `stbds_hmget_key` | same four shapes; additionally checks `header->temp` is written | [x] |
| 33 | `stbds_shmode_func` | `mode ∈ {0,1,2,3}` × `elemsize ∈ {8,16,24,64}` → fresh table fields + `string.mode` | [x] |
| 34 | `stbds_hmdel_key` binary | delete the last element (`old_index == final_index`, no swap) | [x] |
| 35 | `stbds_hmdel_key` binary | delete a non-last element → swap-in + slot re-find + `index` patch | [x] |
| 36 | `stbds_hmdel_key` binary | delete an absent key (`temp` = 0), delete from a table with no hash index, delete from `NULL` | [x] |
| 37 | `stbds_hmdel_key` binary | delete enough entries to exceed `tombstone_count_threshold` → in-place rebuild at the same `slot_count` | [x] |
| 38 | `stbds_hmdel_key` binary | delete until `used_count < used_count_shrink_threshold` with `slot_count > 8` → shrink to `slot_count/2` | [x] |
| 39 | `stbds_hmdel_key` string | `mode = 1` with `SH_STRDUP` (key freed) and with `SH_ARENA` / `SH_DEFAULT` (key not freed) | [x] |
| 40 | `stbds_hmdel_key` string | `mode = 2` with `SH_STRDUP` — `mode == STBDS_HM_STRING` false, key deliberately leaked | [x] |
| 41 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | randomised 2000-operation mixed put/get/del fuzz over a 64-key space, binary mode, `keysize = 4`, `elemsize = 8` | [x] |
| 42 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | same fuzz, `keysize = 8`, `elemsize = 16` | [x] |
| 43 | `stbds_hmput_key`/`hmget_key`/`hmdel_key` | same fuzz in string mode over `SH_STRDUP`, `SH_ARENA`, `SH_DEFAULT` tables | [x] |
| 44 | `stbds_hmfree_func` | free a `SH_STRDUP` map, a `SH_ARENA` map, a binary map, and a map with `hash_table == NULL` (no crash, no double free under ASan-free run) | [x] |
| 45 | `stbds_stralloc` + `stbds_strreset` | fresh arena, one short string (`< 512`) → first 512-byte block, `remaining` accounting | [x] |
| 46 | `stbds_stralloc` | many short strings until the block is exhausted → next block, `block` counter increments, `blocksize` doubles every 2 blocks | [x] |
| 47 | `stbds_stralloc` | string with `len == remaining` exactly (boundary), then one more byte | [x] |
| 48 | `stbds_stralloc` | over-sized string (`len > blocksize`) with `storage == NULL` (`remaining` → 0) and with `storage != NULL` (spliced as `storage->next`) | [x] |
| 49 | `stbds_stralloc` | `block` pre-set to 21/22/23/… so `blocksize` reaches/saturates at `1<<20`, and `block >> 1 >= 64` (shift-count wrap) | [x] |
| 50 | `stbds_stralloc` | empty string `""`, and 1-byte strings, interleaved with long ones | [x] |
| 51 | `stbds_strreset` | reset a multi-block arena then re-allocate (block counter zeroed) | [x] |
| 52 | `strkey` | `n ∈ {0, 1, -1, 9, 11, 12345, INT_MAX, INT_MIN}` + 200 random `i32` | [x] |
| 53 | `intput` | `num ∉ {9, 11}`: 0, 1, 7, 8, 10, 12, -1, INT_MAX, INT_MIN + random — subprocess exit status and stderr-free normal return | [x] |
| 54 | full pipeline, low-level entry points | replay `intput`'s exact macro expansion (`hmput_key` ×3, `hmget_key` ×3) by hand for random `num`, comparing the whole map digest after every step | [x] |
| 55 | full pipeline + `stbds_rand_seed` | rows 41–43 repeated after `stbds_rand_seed(random)` so a different `table->seed` drives different probe orders | [x] |
| 56 | interleaving of fresh-table creation | alternate `shmode_func` / `hmput_key`-on-NULL / `hmdel_key`-shrink so the global seed LCG advances in a non-trivial order; verify `table->seed` stays in lockstep | [x] |

## Phase B status — every row passes across randomised inputs

All tests live in `tests/phase_b_valid.rs`; each scenario is replayed against the
C `.so` and the Rust `.so` through `libloading` and the digests must be
byte-identical. `cargo test --release --test phase_b_valid` → **52 passed**.

| rows | test |
|------|------|
| 1 | `row01_hash_bytes_len0` |
| 2 | `row02_hash_bytes_tail_only` |
| 3 | `row03_hash_bytes_one_word` |
| 4 | `row04_hash_bytes_word_plus_tail` |
| 5 | `row05_hash_bytes_multiword` |
| 6 | `row06_hash_bytes_large_random` |
| 7 | `row07_hash_string_empty` |
| 8 | `row08_hash_string_ascii` |
| 9 | `row09_hash_string_high_bit` |
| 10 | `row10_hash_string_long` |
| 11 | `row11_arrgrowf_fresh_matrix` |
| 12 | `row12_arrgrowf_early_out` |
| 13, 14 | `row13_row14_arrgrowf_growth_branches` |
| 15 | `row15_arrgrowf_growth_chain` |
| 16 | `row16_rand_seed_lcg` |
| 17 | `row17_hmput_default` |
| 18 | `row18_hmput_one` |
| 19 | `row19_hmput_crosses_grow_threshold` |
| 20 | `row20_hmput_many` |
| 21 | `row21_hmput_existing_keys` |
| 22 | `row22_hmput_keysizes` |
| 23 | `row23_hmput_low_entropy_keys` |
| 24 | `row24_hmput_negative_mode` |
| 25 | `row25_string_implicit_sh_default` |
| 26 | `row26_string_sh_strdup` |
| 27 | `row27_string_sh_arena` |
| 28 | `row28_string_sh_none_memcpy_branch` |
| 29 | `row29_string_mode_out_of_range` |
| 30 | `row30_string_duplicate_keys_temp_key` |
| 31, 32 | `row31_row32_hmget_shapes` |
| 33 | `row33_shmode_func` |
| 34 | `row34_del_last_element` |
| 35 | `row35_del_non_last_element` |
| 36 | `row36_del_absent_and_degenerate` |
| 37, 38 | `row37_row38_del_rebuild_and_shrink` |
| 39, 40 | `row39_row40_del_string_modes` |
| 41 | `row41_fuzz_keysize4` |
| 42 | `row42_fuzz_keysize8` |
| 43 | `row43_fuzz_string_modes` |
| 44 | `row44_hmfree_variants` |
| 45 | `row45_arena_first_block` |
| 46 | `row46_arena_many_short_strings` |
| 47 | `row47_arena_exact_boundary` |
| 48 | `row48_arena_oversized_strings` |
| 49 | `row49_arena_block_counter_extremes` |
| 50 | `row50_arena_empty_and_mixed` |
| 51 | `row51_arena_reset_and_reuse` |
| 52 | `row52_strkey` |
| 53 | `row53_intput_non_aborting` |
| 54 | `row54_intput_expansion_replay` |
| 55 | `row55_fuzz_random_hash_seed` |
| 56 | `row56_interleaved_table_creation` |

### Two configurations that are undefined behaviour in the C original

These are exercised only in the sub-shape where the C code is well defined, and
the restriction is documented at the test site:

* **`mode >= 2` + delete of a non-last element** (rows 39–40, 43). For
  `mode != STBDS_HM_STRING` the C re-find after the swap-in passes
  `(char *) a + elemsize*old_index` — the *element*, not the key — to
  `stbds_hm_find_slot`, which then hashes it *as a NUL-terminated string* and
  `strcmp`s against wild pointers. It segfaults in both implementations.
  Deletes with `mode >= 2` are therefore driven in last-element order only
  (`old_index == final_index`, no re-find), which still covers the
  `mode == STBDS_HM_STRING` inequality that decides whether the strdup'd key is
  freed.
* **`STBDS_SH_NONE` table + `mode >= 1`** (row 28). `hmput_key` memcpy's the raw
  key bytes, but any *lookup* interprets those bytes as a `char *` and
  dereferences them. Only distinct keys are inserted so no hash match occurs.

### Harness notes

* `dlopen` returns the same library instance to every caller, so the two `.so`s'
  `stbds_hash_seed` globals are shared across `#[test]` threads. Every scenario
  runs under `common::SERIAL` and calls `stbds_rand_seed` first; without this,
  parallel tests desynchronise each other's seed LCG.
* Digests are address-free. Heap pointers (`storage`, `string.storage`,
  `hash_table`, stored `char *` keys) are compared by *content* or by
  *position*, never by value. `temp_key` in particular is uninitialised
  `realloc` memory until a string-mode `hmput_key` writes it, and points at
  freed memory after a `STBDS_SH_STRDUP` delete, so it is compared as "index of
  the live element whose key pointer it aliases, or -1" and only immediately
  after a string-mode put.
* `scripts/mutation_check.sh` injects 10 small behavioural mutations into
  `src/lib.rs` and asserts the suite fails for each. Current result: **10 / 10
  caught**.
