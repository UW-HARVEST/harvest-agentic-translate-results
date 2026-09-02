# CONFIGS.md — configuration surface table (Phase B gate)

Derived mechanically from the branch conditions in `c_src/src/lib.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| **A. `mode` (hash-map key mode)** | `mode >= STBDS_HM_STRING` (string) vs `mode < 1` (binary) — dispatch in `stbds_is_key_equal`, `stbds_hm_find_slot`, `stbds_hmput_key`; plus the exact `mode == STBDS_HM_STRING` checks inside `stbds_hmdel_key` | L559, L688, L713, L737, L764, L836, L843 |
| **B. `string.mode` (key-storage mode)** | `STBDS_SH_NONE(0)` / `STBDS_SH_DEFAULT(1)` / `STBDS_SH_STRDUP(2)` / `STBDS_SH_ARENA(3)` — the `switch (table->string.mode)` that decides how the key is stored, and the strdup-free loops in `hmfree_func`/`hmdel_key` | L786–L790, L575, L836 |
| **C. table creation path** | `stbds_hmput_key` with `a == NULL` (auto-bootstrap, `string.mode` inferred from `mode`) vs `stbds_shmode_func` (explicit `string.mode`) vs `stbds_hmput_default` first | L683, L695–L703, L795 |
| **D. table lifecycle event** | fresh (`table == NULL`) / grow (`used_count >= used_count_threshold` → `slot_count*2`) / shrink (`used_count < used_count_shrink_threshold && slot_count > 8` → `slot_count>>1`) / rebuild (`tombstone_count > tombstone_count_threshold` → same `slot_count`) | L695, L855–L862 |
| **E. probe path inside a bucket** | first inner loop (`i` from `pos & 7` to 7) vs second wrap-around loop (`i` from 0 to `pos & 7`) vs bucket overflow (`pos += step; step += 8`) | L601–L624, L710–L775 |
| **F. slot state encountered** | `hash == target` + key equal / `hash == STBDS_HASH_EMPTY (0)` / `hash == STBDS_HASH_DELETED (1)` with `index == STBDS_INDEX_DELETED` (tombstone reuse) / hash collision with unequal key | L604–L620, L714–L772 |
| **G. `elemsize` / `keysize`** | `elemsize == 0`; `keysize < / == / > elemsize`; `keysize` 0,1,2,…,8 (siphash tail `switch`), 8 (one full block), 9–64 (multi-block + tail) | siphash L522–L544, `memcmp` L563 |
| **H. siphash input length mod 8** | `len % 8 ∈ {0,1,2,3,4,5,6,7}` — 8 distinct `case`s with fall-through; plus `len == 0` and `len` spanning several blocks | L532–L543 |
| **I. high-bit bytes in the hashed key** | `d[3] >= 0x80` and `d[7] >= 0x80` take the C `int`-promotion / sign-extension path | L523–L524, L536 |
| **J. `stbds_arrgrowf` growth branch** | `min_cap <= arrcap` (no-op) / `min_cap < 2*arrcap` (double) / `min_cap >= 2*arrcap && min_cap < 4` (bump to 4) / `min_cap >= 4` (exact) ; and `a == NULL` vs `a != NULL` | L275–L309 |
| **K. arena block path in `stbds_stralloc`** | `len <= remaining` (carve from current block) / `len > remaining && len <= blocksize` (new 512<<n block) / `len > blocksize` (dedicated oversize block) × `storage == NULL` vs `storage != NULL` ; plus `block` saturation at `1<<20` | L880–L917 |
| **L. `sh_geti` driver `num`** | `num <= 0` (all loops skipped) / `1` / `2` / small odd/even / large enough to force table grow **and** shrink **and** rebuild; the `j` loop runs the whole thing once with `SH_STRDUP` and once with `SH_ARENA` | L945–L985 |
| **M. global hash seed** | `stbds_hash_seed` starts at `0x31415926` and is advanced by every `stbds_make_hash_index(_, NULL)`; `stbds_rand_seed` overrides it | L344, L406–L413 |

## Configuration rows

Each row is exercised with many randomized inputs (fixed seed
`0x5eed_1234_abcd_0001`, see `tests/common/mod.rs`) unless the row is inherently
a single shape.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `stbds_hash_bytes` | `len = 0`, random seeds | [x] |
| 2 | `stbds_hash_bytes` | `len = 1..7` (every siphash tail `case`), random bytes, random seeds | [x] |
| 3 | `stbds_hash_bytes` | `len = 8` (exactly one block, empty tail) | [x] |
| 4 | `stbds_hash_bytes` | `len = 9..64`, i.e. multi-block + every `len % 8` tail | [x] |
| 5 | `stbds_hash_bytes` | `len = 8..64` with **all bytes `>= 0x80`** (C `int` sign-extension path, axis I) | [x] |
| 6 | `stbds_hash_bytes` | `len` large (512..4096), random bytes, seed `0`, seed `usize::MAX` | [x] |
| 7 | `stbds_hash_string` | empty string, 1-byte, and random ASCII strings 1..64 bytes, random seeds | [x] |
| 8 | `stbds_hash_string` | strings containing bytes `>= 0x80` (`(unsigned char) *str` path) | [x] |
| 9 | `stbds_rand_seed` + `stbds_shmode_func` | seed set to `0`, `1`, `0x31415926`, `usize::MAX`, random; observe the seed stored in the new table and the advance of the global | [x] |
| 10 | `stbds_arrgrowf` | `a = NULL`, `elemsize ∈ {1,2,4,8,16,64}`, `addlen ∈ {0,1,2,3,4,5,100}`, `min_cap ∈ {0,1,3,4,5,100}` (full cross-product, axis J) | [x] |
| 11 | `stbds_arrgrowf` | `a != NULL` (chained growth), repeated grows, each of the four growth branches | [x] |
| 12 | `stbds_arrgrowf` + `stbds_arrfreef` | grow then free a non-NULL array (no leak, header intact before free) | [x] |
| 13 | `stbds_hmput_key` BINARY | `a = NULL` bootstrap, `elemsize = keysize = 8`, insert 1 key | [x] |
| 14 | `stbds_hmput_key` BINARY | `elemsize=8, keysize=8`, N random `u64` keys, N ∈ {1,2,7,8,9,32,200} — forces grow at `used_count_threshold` | [x] |
| 15 | `stbds_hmput_key` BINARY | `elemsize=16, keysize=4` (key smaller than element; value bytes present) | [x] |
| 16 | `stbds_hmput_key` BINARY | `elemsize=16, keysize=16` (whole element is the key) | [x] |
| 17 | `stbds_hmput_key` BINARY | `elemsize=4, keysize=1`, keys drawn from `0..=255` → many duplicate re-puts (axis F: existing-key branch) | [x] |
| 18 | `stbds_hmput_key` BINARY | `keysize = 0` (degenerate: all keys equal) | [x] |
| 19 | `stbds_hmget_key` BINARY | `a = NULL` | [x] |
| 20 | `stbds_hmget_key` BINARY | populated table, lookup of present keys (all of them) | [x] |
| 21 | `stbds_hmget_key` BINARY | populated table, lookup of absent keys (miss → `-1`) | [x] |
| 22 | `stbds_hmget_key_ts` BINARY | same three shapes as rows 19–21, checking the `*temp` out-param instead of `hdr->temp` | [x] |
| 23 | `stbds_hmdel_key` BINARY | delete present key, table stays above shrink threshold | [x] |
| 24 | `stbds_hmdel_key` BINARY | delete absent key | [x] |
| 25 | `stbds_hmdel_key` BINARY | delete **all** keys in insertion order → forces shrink (`slot_count>>1`) repeatedly | [x] |
| 26 | `stbds_hmdel_key` BINARY | delete in **reverse** order (tail element == deleted element, `old_index == final_index`) | [x] |
| 27 | `stbds_hmdel_key` BINARY | delete in **random** order interleaved with re-inserts → tombstone reuse + rebuild (`tombstone_count > threshold`) | [x] |
| 28 | `stbds_hmput_default` BINARY | before any put / after puts / twice in a row | [x] |
| 29 | `stbds_hmfree_func` BINARY | free a populated binary table (`string.mode == STBDS_SH_NONE`) | [x] |
| 30 | `stbds_hmput_key` STRING, auto-bootstrap (`a = NULL`, `mode = 1`) | `string.mode` inferred as `STBDS_SH_DEFAULT`: keys stored **by pointer**; N random strings | [x] |
| 31 | `stbds_shmode_func(STBDS_SH_DEFAULT)` + put/get/del | explicit DEFAULT mode, N random strings, N ∈ {1,2,7,8,9,32,200} | [x] |
| 32 | `stbds_shmode_func(STBDS_SH_STRDUP)` + put/get/del | keys `strdup`ed; delete frees the dup (axis B); N random strings | [x] |
| 33 | `stbds_shmode_func(STBDS_SH_ARENA)` + put/get/del | keys arena-allocated; `strreset` on free; N random strings | [x] |
| 34 | `stbds_shmode_func(STBDS_SH_NONE)` + put | `string.mode == 0` → `default:` branch of the `switch` → `memcpy(key, keysize)` **even in string mode** | [x] |
| 35 | STRING mode, all four `string.mode`s | duplicate keys (same content, *different* pointers) re-put → existing-key branch, `temp_key` behaviour | [x] |
| 35b | STRING mode, all four `string.mode`s | randomized 500-op streams × 8 trials × 4 modes with 1-4 byte keys (heavy bucket collisions), comparing `table->temp_key` after **every** put. `temp_key` is seeded with a shared sentinel whenever the table object is (re)created, because `stbds_make_hash_index` never initialises it, and reseeded after every `del`, because STRDUP mode frees the key the field points at. This is the only row that reaches the wrap-around inner loop's *missing* `temp_key` write. | [x] |
| 36 | STRING mode, all four `string.mode`s | keys with a **common prefix** (`test_1`, `test_10`, …) → `strcmp` collisions | [x] |
| 37 | STRING mode | empty-string key `""` | [x] |
| 38 | STRING mode | long keys (600 and 2000 bytes) → arena oversize path when `string.mode == ARENA` | [x] |
| 39 | `stbds_hmdel_key` STRING | delete all keys → shrink, in DEFAULT / STRDUP / ARENA | [x] |
| 40 | `stbds_hmfree_func` STRING | free populated tables in DEFAULT / STRDUP / ARENA (strdup loop + `strreset`) | [x] |
| 41 | full pipeline BINARY | randomized op stream (put / get / get_ts / del / put_default) over 400 ops, `elemsize=16, keysize=8`, comparing the whole serialized table after **every** op | [x] |
| 42 | full pipeline STRING/DEFAULT | randomized op stream, 400 ops, keys from a fixed pool | [x] |
| 43 | full pipeline STRING/STRDUP | randomized op stream, 400 ops | [x] |
| 44 | full pipeline STRING/ARENA | randomized op stream, 400 ops | [x] |
| 45 | `stbds_stralloc` | fresh arena (`storage == NULL`), short random strings (`len <= 512`) | [x] |
| 46 | `stbds_stralloc` | many short strings until the current block is exhausted → new-block path, `block` increments 0,1,2,… | [x] |
| 47 | `stbds_stralloc` | oversize string (`len > blocksize`) into a **fresh** arena (`storage == NULL` branch) | [x] |
| 48 | `stbds_stralloc` | oversize string into a **non-empty** arena (`storage != NULL` splice-after-head branch) | [x] |
| 49 | `stbds_stralloc` | enough allocations to saturate `block` at blocksize `1<<20` | [x] |
| 50 | `stbds_stralloc` | empty string `""` repeatedly | [x] |
| 51 | `stbds_strreset` | empty arena / arena with 1 block / arena with many blocks / called twice | [x] |
| 52 | `strkey` | `n ∈ {0,1,9,10,99,100,12345, -1, INT_MIN, INT_MAX}` and random `i32` | [x] |
| 53 | `sh_geti` | `num ∈ {0, -1, INT_MIN}` (all loops skipped) — stdout must be identical (empty) | [x] |
| 54 | `sh_geti` | `num ∈ {1,2,3,4,5,6,7,8}` (small; below/at first grow) — stdout compared byte-for-byte | [x] |
| 55 | `sh_geti` | `num ∈ {9,12,16,17,32,33,64,100}` (forces grow, shrink and rebuild) | [x] |
| 56 | `sh_geti` | `num = 500` (deep growth; both `SH_STRDUP` and `SH_ARENA` halves of the `j` loop) | [x] |
| 57 | `sh_geti` | called repeatedly in one process (global `stbds_hash_seed` has advanced) — sequence of `num = 4, 4, 4` must match between libs | [x] |
| 58 | `stbds_shmode_func` | `elemsize ∈ {1,4,8,16,24,64}` × `mode ∈ {0,1,2,3}` — the new table's fields and the seed advance | [x] |
| 59 | mixed | `stbds_rand_seed(k)` then a full put/get/del pipeline, for `k ∈ {0, 1, usize::MAX, random}` — the seed feeds every hash | [x] |
| 60 | mixed | `stbds_hmput_default` **before** `stbds_hmput_key` on a NULL table (order axis C) in binary and string mode | [x] |
| 61 | `stbds_hmdel_key` | non-zero `keyoffset` (4, 8, 12) — a public parameter the `hmdel`/`shdel` macros always pass as 0 | [x] |

## How the rows are checked

Every row is driven through the `.so` exports only (`libloading` + `dlsym`); no
Rust function is ever called directly. After each operation the *entire*
observable state of both libraries is serialised and compared byte-for-byte:

* array header — `length`, `capacity`, `temp`;
* every live element (keys rendered as strings when `string.mode ∈ {1,2,3}`,
  raw bytes otherwise — matching the C `switch`);
* `stbds_hash_index` — `slot_count`, `used_count`, all four thresholds,
  `tombstone_count`, `seed`, `slot_count_log2`, and the embedded arena's
  `remaining` / `block` / `mode`;
* **every slot of every bucket** (`hash[j]` and `index[j]`), so probe order,
  tombstone placement and rehash results are all compared, not just lookups.

Raw pointers are deliberately excluded (the two libraries allocate
independently). Bytes the C never initialises are excluded too: the harness
writes the value half of each element after every put, exactly as the
`stbds_hmput`/`shput` macros do, so no `realloc` garbage enters a comparison.

Test files: `tests/hash_arr.rs` (rows 1-12, 52, 58), `tests/hashmap.rs`
(13-44, 59-61), `tests/arena.rs` (45-51), `tests/sh_geti.rs` (53-57),
`tests/common/mod.rs` (harness). Fixed RNG seed `0x5eed_1234_abcd_0001`.

`sh_geti` has no return value, so rows 53-57 compare its **stdout**: fd 1 is
redirected to a temporary file around each call and the bytes are compared
exactly. `cfg54b` additionally asserts the output is non-empty and well-formed,
so those rows cannot pass by comparing two empty strings.
