# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from the C source: every `STBDS_ASSERT` (== `assert`,
compiled **without** `NDEBUG` by `c_src/CMakeLists.txt`, so it aborts), every
`return` that reports "no/absent/failed", every explicit null check, every
sentinel constant, and every min/max constant. One row per distinct rejection
site.

Sentinels used by the C code:

* `STBDS_INDEX_EMPTY  = -1`, `STBDS_INDEX_DELETED = -2`
* `STBDS_HASH_EMPTY   =  0`, `STBDS_HASH_DELETED  =  1`
* `stbds_hm_find_slot` returns `-1` for "not found"
* `stbds_hmdel_key` returns `NULL` for a `NULL` map, and signals
  found/not-found through `stbds_header(raw_a)->temp` (`1` / `0`)
* min/max constants: `STBDS_BUCKET_LENGTH = 8`,
  `STBDS_STRING_ARENA_BLOCKSIZE_MIN = 512u`,
  `STBDS_STRING_ARENA_BLOCKSIZE_MAX = 1u<<20`

Abort rows are compared by forking a child and comparing the child's
termination status/signal for C vs Rust (`SIGABRT` for `assert`, `SIGSEGV` for
a null dereference). "same result" therefore means the same observable
termination, not merely "both failed".

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 1 | `stbds_arrgrowf` (`lib.c:287`) | `min_cap <= stbds_arrcap(a)` and `arrlen(a)+addlen <= min_cap`, e.g. `(a=cap4, e, 0, 2)` | returns `a` unchanged, no realloc, header untouched | [x] |
| 2 | `stbds_arrgrowf` (`lib.c:287`) | `a == NULL, addlen == 0, min_cap == 0` → `min_cap(0) <= arrcap(NULL)(0)` | returns `NULL` (no allocation at all) | [x] |
| 3 | `stbds_arrgrowf` (`lib.c:298`) | `elemsize*min_cap + 32` overflows / exhausts memory → `realloc` returns `NULL` | `b = NULL+32`, then `stbds_header(b)->capacity = …` writes near address 0 → `SIGSEGV` | [x] |
| 4 | `stbds_arrfreef` (`lib.c:319`) | `a == NULL` — unconditional `free(stbds_header(NULL))` = `free((char*)0 - 32)` | glibc rejects the wild pointer: abort (`SIGABRT`) / `SIGSEGV` | [x] |
| 5 | `stbds_hash_string` (`lib.c:471`) | `str == NULL` | `SIGSEGV` on `*str` | [x] |
| 6 | `stbds_hash_string` (`lib.c:472`) | `str == ""` (zero-length) — loop body never runs | well-defined: finalizer applied to `hash == seed` | [x] |
| 7 | `stbds_hash_bytes` (`lib.c:497`) | `len == 0` (with any `p`, incl. `NULL`) — no byte is read | well-defined constant-per-seed value, no fault | [x] |
| 8 | `stbds_hash_bytes` (`lib.c:497`) | `p == NULL, len > 0` | `SIGSEGV` on `d[0]` | [x] |
| 9 | `stbds_make_hash_index` (`lib.c:401`) | `slot_count ∈ {0,1,2}` → `used_count_threshold + tombstone_count_threshold < slot_count` is false | `assert` fails → `SIGABRT`. **Unreachable from the public API** (`slot_count` is always `8` or `slot_count*2` or `slot_count>>1` with `slot_count > 8`); documented, verified by construction not by call | [x] |
| 10 | `stbds_hmfree_func` (`lib.c:573`) | `a == NULL` | returns immediately, nothing freed | [x] |
| 11 | `stbds_hm_find_slot` (`lib.c:610`) | key absent, `STBDS_HASH_EMPTY` found in the `i = pos&7 .. 7` scan | returns `-1` | [x] |
| 12 | `stbds_hm_find_slot` (`lib.c:621`) | key absent, `STBDS_HASH_EMPTY` found in the wrapped `i = 0 .. pos&7` scan | returns `-1` | [x] |
| 13 | `stbds_hmget_key_ts` (`lib.c:634-639`) | `a == NULL` (lookup on an unborn map) | allocates a 1-element array, `length = 1`, element 0 zeroed, `*temp = -1`, returns non-`NULL` handle | [x] |
| 14 | `stbds_hmget_key_ts` (`lib.c:644-645`) | handle whose `hash_table == NULL` (e.g. built by `stbds_hmput_default` or by row 13) | `*temp = -1`, returns `a` unchanged | [x] |
| 15 | `stbds_hmget_key_ts` (`lib.c:648-649`) | populated table, key not present (`slot < 0`) | `*temp = -1` (`STBDS_INDEX_EMPTY`), returns `a` | [x] |
| 16 | `stbds_hmget_key_ts` (`lib.c:638/649/653`) | `temp == NULL` | `SIGSEGV` on `*temp = …` | [x] |
| 17 | `stbds_hmget_key` (`lib.c:661-662`) | any of rows 13-15 | same as row, plus `stbds_header(p-elemsize)->temp = temp` (so `-1` is observable in the header) | [x] |
| 18 | `stbds_hmput_key` (`lib.c:778`) | `assert((size_t)i+1 <= stbds_arrcap(a))` after the growth branch | cannot fail: the preceding `if` grows when needed. Dead check — asserted to never abort across the whole Phase-B corpus | [x] |
| 19 | `stbds_hmput_key` (`lib.c:721/731`) | `mode` out of range: `2, 3, 127, INT_MAX` (`mode >= STBDS_HM_STRING`) | treated as *string*: hashes `key` as a C string, `string.mode` initialised to `STBDS_SH_DEFAULT` on a fresh table, key stored as a `char*` | [x] |
| 20 | `stbds_hmput_key` (`lib.c:721/731`) | `mode` out of range negative: `-1, INT_MIN` | treated as *binary*: `stbds_hash_bytes`, `memcmp`, `string.mode = 0`, `memcpy` of `keysize` bytes | [x] |
| 21 | `stbds_hmput_key` (`lib.c:721`) | `key == NULL`, binary mode, `keysize > 0` | `SIGSEGV` inside `stbds_hash_bytes` | [x] |
| 22 | `stbds_hmput_key` (`lib.c:721`) | `key == NULL`, binary mode, `keysize == 0` | no byte read; all keys hash identically and `memcmp(...,0)==0` so the first entry always matches → single-entry map | [x] |
| 23 | `stbds_hmput_key` (`lib.c:721`) | `key == NULL`, string mode | `SIGSEGV` inside `stbds_hash_string` | [x] |
| 24 | `stbds_shmode_func` (`lib.c:804`) | `mode` out of range: `4, 255, 256, -1, INT_MIN, INT_MAX` — stored via `(unsigned char) mode` | truncates to `mode & 0xff`; later `switch (table->string.mode)` falls to `default:` (raw `memcpy` of `keysize` bytes) for any value ∉ {1,2,3} | [x] |
| 25 | `stbds_hmdel_key` (`lib.c:809-810`) | `a == NULL` | returns `0` (`NULL`) — the only pointer-sentinel error return in the library | [x] |
| 26 | `stbds_hmdel_key` (`lib.c:816-817`) | handle with `hash_table == NULL` | `stbds_temp(raw_a) = 0`, returns `a`, length unchanged | [x] |
| 27 | `stbds_hmdel_key` (`lib.c:821-822`) | populated table, key absent (`slot < 0`) | `stbds_temp(raw_a) = 0`, returns `a`, length unchanged, `used_count`/`tombstone_count` unchanged | [x] |
| 28 | `stbds_hmdel_key` (`lib.c:828`) | `assert(slot < (ptrdiff_t)table->slot_count)` | cannot fail (`stbds_hm_find_slot` masks `pos` with `slot_count-1`). Dead check — asserted not to abort across the corpus | [x] |
| 29 | `stbds_hmdel_key` (`lib.c:832`) | `assert(table->used_count >= 0)` with `used_count` a `size_t` | tautology, never fires (even after the `--table->used_count` wrap) | [x] |
| 30 | `stbds_hmdel_key` (`lib.c:846`) | `assert(slot >= 0)` — the re-find of the *moved* last element fails. Triggered by deleting with a `keyoffset` / `mode` that differs from the one used at insert (e.g. insert with `mode=0,keysize=4`, delete with `keyoffset=4`) while `old_index != final_index` | `SIGABRT` | [x] |
| 31 | `stbds_hmdel_key` (`lib.c:849`) | `assert(b->index[i] == final_index)` — re-find lands on a slot whose index is not the moved element (mismatched `keysize` between insert and delete, duplicate raw key bytes) | `SIGABRT` | [x] |
| 32 | `stbds_hmdel_key` (`lib.c:857`) | shrink boundary: `used_count < used_count_shrink_threshold && slot_count > 8` | rebuilds the index at `slot_count>>1`; `slot_count == 8` never shrinks (`used_count_shrink_threshold` forced to `0`) | [x] |
| 33 | `stbds_hmdel_key` (`lib.c:860`) | tombstone boundary: `tombstone_count > (slot_count>>3)+(slot_count>>4)` | rebuilds the index at the same `slot_count`, clearing tombstones | [x] |
| 34 | `stbds_stralloc` (`lib.c:913`) | `assert(len <= a->remaining)` | unreachable as written (both branches above guarantee it). Dead check — asserted not to abort | [x] |
| 35 | `stbds_stralloc` (`lib.c:915`) | arena with `remaining >= len` but `storage == NULL` (e.g. `{storage:NULL, remaining:1000}`) — skips the alloc branch, then dereferences `a->storage` | `SIGSEGV` | [x] |
| 36 | `stbds_stralloc` (`lib.c:886`) | `a == NULL` | `SIGSEGV` on `a->remaining` | [x] |
| 37 | `stbds_stralloc` (`lib.c:885`) | `str == NULL` | `SIGSEGV` in `strlen` | [x] |
| 38 | `stbds_stralloc` (`lib.c:889-891`) | `block` saturation: `blocksize = 512u << (block>>1)`, `++a->block` only while `blocksize < 1<<20` → `block` stops growing at `22` | `blocksize` caps at `1<<20`; `block` never exceeds `22` | [x] |
| 39 | `stbds_stralloc` (`lib.c:894-905`) | `len > blocksize` (oversize string, e.g. 4096 bytes into a fresh arena) | dedicated block spliced *after* the head (or installed as head with `remaining = 0`); returns `sb->storage`; the `++a->block` side effect has already happened | [x] |
| 40 | `stbds_strreset` (`lib.c:925`) | `a == NULL` | `SIGSEGV` on `a->storage` | [x] |
| 41 | `stbds_strreset` (`lib.c:925-932`) | already-zeroed arena (`storage == NULL`) | loop body never runs, arena memset to 0 — idempotent, no fault | [x] |
| 42 | `hm_geti` (`lib.c:952`) | `assert(hmgeti(intmap, 1) == -1)` on the `NULL` map | must hold; abort otherwise | [x] |
| 43 | `hm_geti` (`lib.c:954-955`) | `assert(hmgeti == -1)` / `assert(hmget == -2)` after `hmdefault(-2)` (table still `NULL`) | must hold | [x] |
| 44 | `hm_geti` (`lib.c:959-962`) | the 4 asserts over present/absent keys, incl. the `_ts` variants | must hold for every `num` | [x] |
| 45 | `hm_geti` (`lib.c:967-968`) | the 2 asserts after re-`hmput` with new values | must hold | [x] |
| 46 | `hm_geti` (`lib.c:972-973`) | the 2 asserts after deleting every 4th key | must hold | [x] |
| 47 | `hm_geti` (`lib.c:977`) | `assert(hmget == -2)` after deleting every key | must hold | [x] |
| 48 | `hm_geti` | `num <= 0` (`0`, `-1`, `INT_MIN`) — every loop body is skipped | runs only rows 42-43, then `hmfree` twice; returns normally | [x] |
| 49 | generic FFI | `elemsize == 0` passed to `stbds_arrgrowf` (all `addlen`/`min_cap` classes) and to `stbds_hmput_key`/`hmget_key` together with `keysize == 0` (so the `memcpy` writes nothing out of bounds) | degenerate but defined: header-only allocation, every element aliases offset 0, map saturates at one entry | [x] |
| 50 | generic FFI | `keysize == 0` in binary mode for `hmput_key` / `hmget_key` / `hmdel_key` | `stbds_hash_bytes(key,0,seed)` is key-independent and `memcmp(...,0) == 0` always → every key collides into entry 1 | [x] |
| 51 | generic FFI | `keysize` larger than the logical key but still in bounds (`keysize == elemsize == 16` while only the leading 4 bytes are the "real" key) | hashes/compares the payload bytes too; both libraries must agree bit-for-bit on the resulting slot layout and on which puts count as updates | [x] |
| 52 | generic FFI | `seed == 0` and `seed == SIZE_MAX` for `stbds_rand_seed` / `stbds_hash_bytes` / `stbds_hash_string` | no special-casing; `hash < 2 → hash += 2` is the only clamp, applied in `hm_find_slot`/`hmput_key`, not in the hash functions. **Quirk:** `stbds_siphash_bytes` XORs `seed` into `v0..v3` twice (`v0 = X ^ seed`, then `v0 ^= C ^ seed`), so it cancels and `stbds_hash_bytes` is entirely **seed-independent**; `stbds_hash_string` does honour the seed. Both behaviours are pinned by `harness_hash_bytes_is_seed_independent_in_both` | [x] |
| 53 | `stbds_hmdel_key` (`lib.c:841-846`) | `mode > STBDS_HM_STRING` (e.g. `2`, the `STBDS_HM_PTR_TO_STRING` value used by the unused `pshdel` macros) on a string-keyed table, deleting an entry with `old_index != final_index`. `mode == STBDS_HM_STRING` is false, so the re-find takes the `else` branch and passes `(char*)a + elemsize*old_index + keyoffset` — the element's *address* — while `stbds_hm_find_slot` hashes it as a C string because `mode >= STBDS_HM_STRING`. The lookup therefore always misses | `assert(slot >= 0)` fails → `SIGABRT` | [x] |

## Row → test mapping

All tests live in `translation/tests/phase_c_errors.rs` and drive both `.so`
files through `libloading`.

| rows | test |
|------|------|
| 1 | `err_01_arrgrowf_rejects_shrink` |
| 2 | `err_02_arrgrowf_null_null` |
| 3 | `err_03_arrgrowf_alloc_failure` |
| 4 | `err_04_arrfreef_null` |
| 5 | `err_05_hash_string_null` |
| 6 | `err_06_hash_string_empty` |
| 7 | `err_07_hash_bytes_len0` |
| 8 | `err_08_hash_bytes_null_buffer` |
| 9 | `err_09_min_slot_count_is_8` |
| 10 | `err_10_hmfree_null` |
| 11, 12 | `err_11_12_find_slot_misses` |
| 13 | `err_13_get_ts_on_null` |
| 14 | `err_14_get_ts_no_table` |
| 15 | `err_15_get_ts_absent` |
| 16 | `err_16_get_ts_null_temp` |
| 17 | `err_17_get_key_writes_header_temp` |
| 18, 28, 29, 34 | `err_18_28_29_34_dead_asserts_never_fire` |
| 19, 20 | `err_19_20_out_of_range_mode` |
| 21, 23 | `err_21_23_null_key_faults` |
| 22 | `err_22_null_key_zero_keysize` |
| 24 | `err_24_shmode_out_of_range` |
| 25 | `err_25_del_on_null` |
| 26 | `err_26_del_no_table` |
| 27 | `err_27_del_absent` |
| 30, 31 | `err_30_31_del_refind_asserts` |
| 32 | `err_32_shrink_boundary` |
| 33 | `err_33_tombstone_rebuild` |
| 35 | `err_35_stralloc_null_storage_with_remaining` |
| 36 | `err_36_stralloc_null_arena` |
| 37 | `err_37_stralloc_null_str` |
| 38 | `err_38_arena_block_saturation` |
| 39 | `err_39_stralloc_oversize_block` |
| 40 | `err_40_strreset_null` |
| 41 | `err_41_strreset_zeroed` |
| 42-48 | `err_42_48_hm_geti_assertions` (also `phase_b_driver.rs`) |
| 49 | `err_49_zero_elemsize` |
| 50 | `err_50_zero_keysize` |
| 51 | `err_51_keysize_equals_elemsize` |
| 52 | `err_52_boundary_seeds` (quirk pinned by `harness_selftest.rs`) |
| 53 | `err_53_del_mode_gt1_refind_aborts` |
