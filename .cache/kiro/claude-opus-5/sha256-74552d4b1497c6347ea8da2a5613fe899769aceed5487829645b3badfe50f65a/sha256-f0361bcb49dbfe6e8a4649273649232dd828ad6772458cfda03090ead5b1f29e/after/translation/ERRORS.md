# ERRORS.md — Phase A error / rejection surface

Mechanically derived from `c_src/src/lib.c`. Every early `return`, sentinel
value, `STBDS_ASSERT`, null check, and range/boundary constant is one row.

Greps used:

```
grep -n 'return' c_src/src/lib.c
grep -n 'STBDS_ASSERT\|assert' c_src/src/lib.c
grep -n 'NULL\|== 0\|< 0\|>= \|<= ' c_src/src/lib.c
grep -n '#define STBDS_.*_\(MIN\|MAX\|EMPTY\|DELETED\|THRESHOLD\)' c_src/src/lib.c
```

Note: the C `.so` is built with **no `-DNDEBUG`** (CMake with empty
`CMAKE_BUILD_TYPE`); `nm -D -u` shows `U __assert_fail`, so `STBDS_ASSERT` is
**live** in the C library. Rows marked *(abort)* mean the C calls
`__assert_fail` and aborts the process.

| # | function | trigger (exact invalid input / condition) | expected C result | status |
|---|----------|-------------------------------------------|-------------------|--------|
| 1 | `stbds_arrgrowf` | `a == NULL`, `min_cap == 0`, `addlen == 0` (min_len 0, min_cap 0 → not `<= arrcap(a)==0`? `0 <= 0` is true) | early `return a` == `NULL` (line 286-287) | [x] |
| 2 | `stbds_arrgrowf` | `a != NULL` and requested `min_cap` (and `arrlen+addlen`) `<= ` existing capacity | early `return a`, pointer **unchanged**, capacity/length untouched | [x] |
| 3 | `stbds_arrgrowf` | `a == NULL`, `min_cap` in `1..=3`, `addlen == 0` | not `<2*0`, so `min_cap < 4` → clamped up to `4`; header length=0, hash_table=0, temp=0 | [x] |
| 4 | `stbds_arrgrowf` | `a != NULL`, `min_cap` between `arrcap+1` and `2*arrcap-1` | `min_cap` bumped to `2*arrcap` (doubling), length/hash_table/temp **preserved** | [x] |
| 5 | `stbds_arrgrowf` | `addlen` huge so `arrlen + addlen` wraps `size_t` (e.g. `addlen = SIZE_MAX`) | wrap-around arithmetic; `min_cap = min_len` then `elemsize*min_cap + 32` wraps; `realloc` of the wrapped size (usually succeeds, tiny) — must match bit-for-bit | [x] |
| 6 | `stbds_arrfreef` | called on the array pointer of a live array | `free(header)`; no return value, no validation whatsoever | [x] |
| 7 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | **no dereference** (loop not entered, `switch(len-i)` hits `case 0: break`); returns the len=0 siphash value | [x] |
| 8 | `stbds_hash_bytes` | `len` not a multiple of 8 → `switch (len - i)` fall-through cases 7,6,5,4,3,2,1 | each remainder length picks a different fall-through chain; `d[3] << 24` is an **`int`** expression → sign-extends when `d[3] >= 0x80` | [x] |
| 9 | `stbds_hash_bytes` | `len == 0` with non-NULL `p` | `data = 0 << 56`; result depends only on seed | [x] |
| 10 | `stbds_hash_string` | empty string `""` | `while(*str)` never runs; hash = seed then avalanche | [x] |
| 11 | `stbds_hash_string` | bytes `>= 0x80` in the string | `(unsigned char) *str++` → **no** sign extension (explicit cast) | [x] |
| 12 | `stbds_hmfree_func` | `a == NULL` | early `return`, no-op (line 573) | [x] |
| 13 | `stbds_hmfree_func` | `stbds_hash_table(a) == NULL` (array with no hash table) | skips strdup-free + `strreset`; still frees `hash_table` (NULL) and header | [x] |
| 14 | `stbds_hmfree_func` | `string.mode != STBDS_SH_STRDUP` | does **not** free the per-key strings | [x] |
| 15 | `stbds_hmget_key_ts` | `a == NULL` | allocates a fresh 1-element array, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash pointer | [x] |
| 16 | `stbds_hmget_key_ts` | `a != NULL` but `stbds_header(raw_a)->hash_table == 0` | `*temp = -1`, returns `a` **unchanged** | [x] |
| 17 | `stbds_hmget_key_ts` | key absent from a populated table (`stbds_hm_find_slot` returns `-1`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | [x] |
| 18 | `stbds_hm_find_slot` (via get/del) | probe reaches a bucket slot with `hash == STBDS_HASH_EMPTY (0)` | returns `-1` (the "not found" sentinel) | [x] |
| 19 | `stbds_hmget_key` | same three cases as #15–#17 | additionally writes `temp` into `stbds_header(raw_a)->temp` | [x] |
| 20 | `stbds_hmput_default` | `a == NULL` | grows a 1-element array, length += 1, zeroed, returns hash pointer | [x] |
| 21 | `stbds_hmput_default` | `a != NULL` but `stbds_header(hash_to_arr(a))->length == 0` | same grow path, taken even though `a` is non-NULL | [x] |
| 22 | `stbds_hmput_default` | `a != NULL`, `length != 0` | returns `a` unchanged (no realloc, no zeroing) | [x] |
| 23 | `stbds_hmdel_key` | `a == NULL` | `return 0` i.e. **NULL** (line 810) | [x] |
| 24 | `stbds_hmdel_key` | `a != NULL`, `hash_table == 0` | `stbds_temp(raw_a) = 0`, `return a` unchanged | [x] |
| 25 | `stbds_hmdel_key` | key absent (`stbds_hm_find_slot` < 0) | `stbds_temp(raw_a)` stays `0`, `return a`; `used_count`/`tombstone_count` untouched | [x] |
| 26 | `stbds_hmdel_key` | key present | `stbds_temp(raw_a) = 1`, slot → `STBDS_HASH_DELETED (1)` / `STBDS_INDEX_DELETED (-2)`, length -= 1 | [x] |
| 27 | `stbds_hmdel_key` | `mode == 2` (`>= STBDS_HM_STRING` for hashing, but `!= STBDS_HM_STRING` for the two `mode == STBDS_HM_STRING` checks) | string hashing/compare is used, but the strdup-free is skipped **and** the re-find uses the raw-bytes key branch → distinct behaviour from `mode==1` | [x] |
| 28 | `stbds_hmdel_key` | deleting the last element (`old_index == final_index`) | skips the memmove + re-find entirely | [x] |
| 29 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` | rebuild at `slot_count>>1`; old table freed | [x] |
| 30 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (`(sc>>3)+(sc>>4)`) | rebuild at same `slot_count`; old table freed | [x] |
| 31 | `stbds_hmdel_key` *(abort)* | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` (line 828) | `__assert_fail` → SIGABRT | [x] check present in Rust; **unreachable** via the public API (`stbds_hm_find_slot` masks `pos` with `slot_count-1` before returning) |
| 32 | `stbds_hmdel_key` *(abort)* | `STBDS_ASSERT(slot >= 0)` after the swap-with-last re-find (line 846). **Reachable:** `mode == 2` (≥STRING for hashing, ≠STRING for the re-find) deleting a non-last element — the re-find hashes the element's `char *` bytes as a string and finds nothing. | `__assert_fail` → SIGABRT | [x] `phase_c_aborts.rs::case_row32` (both abort, signal 6) |
| 33 | `stbds_hmdel_key` *(abort)* | `STBDS_ASSERT(b->index[i] == final_index)` (line 849). **Reachable:** under `STBDS_SH_DEFAULT` the table stores the caller's key pointer, so mutating the last element's key to alias an earlier key makes the post-memmove re-find land on the wrong slot. | `__assert_fail` → SIGABRT | [x] `phase_c_aborts.rs::case_row33` (both abort, signal 6) |
| 34 | `stbds_hmput_key` | `a == NULL` | fresh 1-element array is allocated first, then normal insert | [x] |
| 35 | `stbds_hmput_key` | `table == NULL` | new index at `slot_count = STBDS_BUCKET_LENGTH (8)`; `nt->string.mode = mode>=1 ? STBDS_SH_DEFAULT : 0` | [x] |
| 36 | `stbds_hmput_key` | `used_count >= used_count_threshold` (`slot_count - slot_count/4`) | rehash/grow to `slot_count*2` | [x] |
| 37 | `stbds_hmput_key` | duplicate key found in the **first** inner loop (`i = pos&7 .. 7`) | `temp = index`; **and** for `mode>=1` also sets `stbds_temp_key` | [x] |
| 38 | `stbds_hmput_key` | duplicate key found in the **second** inner loop (`i = 0 .. pos&7`) | `temp = index`; **does NOT** set `stbds_temp_key` (C asymmetry, preserved) | [x] |
| 39 | `stbds_hmput_key` | insert lands on a tombstone (`index == STBDS_INDEX_DELETED`) | `pos = tombstone`, `--tombstone_count` | [x] |
| 40 | `stbds_hmput_key` | `hash < 2` after hashing | `hash += 2` (0 and 1 are the EMPTY/DELETED sentinels) | [x] |
| 41 | `stbds_hmput_key` | `table->string.mode` not in {1,2,3} (e.g. 0, or 255 from `mode=-1`) | `default:` → `memcpy(elem, key, keysize)`, `stbds_temp_key` **not** written | [x] |
| 42 | `stbds_hmput_key` | `mode < 0` (e.g. `-1`, out-of-range enum) | `mode >= STBDS_HM_STRING` false → binary hash + `memcmp` path | [x] |
| 43 | `stbds_hmput_key` | `mode` large out-of-range (e.g. `1000`, `INT_MAX`) | `>= STBDS_HM_STRING` true → string hash + `strcmp` path (any int ≥ 1 is "string") | [x] |
| 44 | `stbds_hmput_key` *(abort)* | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` (line 778) | `__assert_fail` → SIGABRT | [x] check present in Rust; **unreachable** — the preceding `stbds_arrgrowf` guarantees the capacity |
| 45 | `stbds_shmode_func` | `mode` out of the `STBDS_SH_*` enum range, e.g. `-1`, `4`, `259`, `1000` | `(unsigned char) mode` **truncates**: `259 → 3 (SH_ARENA)`, `-1 → 255 (default/memcpy)`, `4 → 4 (default)` | [x] |
| 46 | `stbds_shmode_func` | `mode == STBDS_SH_NONE (0)` | `string.mode = 0` → `default:` memcpy branch in `hmput_key` | [x] |
| 47 | `stbds_stralloc` | `len <= a->remaining` (fast path) | carves from the tail of the current block: `p = storage->storage + remaining - len` | [x] |
| 48 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` (oversized string) | dedicated exact-size block; if `a->storage == NULL` also sets `a->remaining = 0`; returns `sb->storage` | [x] |
| 49 | `stbds_stralloc` | `len > a->remaining`, `len <= blocksize` | new `blocksize` block pushed at head, `a->remaining = blocksize` | [x] |
| 50 | `stbds_stralloc` | `a->block` such that `512 << (block>>1) >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` (i.e. `block >= 22`) | `a->block` is **NOT** incremented (boundary constant) | [x] |
| 51 | `stbds_stralloc` | `a->block` large enough that `block>>1 >= 64` (e.g. `block = 128 → 64`) | shift count `>= 64`: UB in C, x86-64 masks to `& 63` → `blocksize = 512` | [x] |
| 52 | `stbds_stralloc` | `a->block` with `block>>1 == 55` (`block = 110`/`111`) | `512 << 55` overflows to `0` → `0 < MAX` so `++block`, then `len > 0` → oversized-block path | [x] |
| 53 | `stbds_stralloc` | empty string `""` (`len == 1`) | still consumes 1 byte of `remaining` | [x] |
| 54 | `stbds_stralloc` *(abort)* | `STBDS_ASSERT(len <= a->remaining)` (line 913) | `__assert_fail` → SIGABRT | [x] check present in Rust; **unreachable** — the `len > blocksize` arm returns early and the other arm sets `remaining = blocksize >= len` |
| 55 | `stbds_strreset` | `a->storage == NULL` (empty arena) | while loop skipped; `memset(a,0,sizeof *a)` still zeroes `remaining`/`block`/`mode` | [x] |
| 56 | `stbds_strreset` | arena with a chain of blocks | frees the whole `next` chain, then zeroes the arena | [x] |
| 57 | `strkey` | `n < 0` (e.g. `-1`, `INT_MIN`) | `sprintf(buffer,"test_%d",n)` → `"test_-1"` / `"test_-2147483648"`; returns the **same static buffer** each call | [x] |
| 58 | `strkey` | called twice | second call overwrites the first result (shared static `char buffer[256]`) | [x] |
| 59 | `helxo` | `letter == 0` (NUL) | `%c` prints a NUL byte into stdout | [x] |
| 60 | `helxo` | `letter` with the high bit set (e.g. `(char)0x80`, `(char)-1`) | `printf("%c")` converts the (garbage-padded) eightbyte to `unsigned char` | [x] |
| 61 | `stbds_make_hash_index` *(abort)* | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` (line 401) — fails for `slot_count <= 2` | `__assert_fail` → SIGABRT | [x] check present in Rust; **unreachable** — the three call sites only pass `8`, `slot_count*2`, or `slot_count>>1` with `slot_count > 8` |

## Translation gap found and fixed in Phase C

The Rust translation originally omitted **all seven** `STBDS_ASSERT` calls.  Because
`c_src` is compiled without `-DNDEBUG`, the C aborts on rows 31-33/44/54/61 while
the Rust silently continued (and, for row 32, indexed `table->storage` with a
negative slot).  All seven checks were added to `translation/src/lib.rs`
(`stbds_assert!`, which writes a diagnostic to fd 2 and calls `abort()` so the
behaviour is independent of the crate's panic strategy).  Rows 32 and 33 are
reachable from ordinary API inputs and are now covered by differential
subprocess tests that compare wait statuses.

## Row → test map

| rows | test |
|------|------|
| 1-5 | `phase_c_errors.rs::err01_…` … `err05_arrgrowf_size_t_wraparound` |
| 6 | `err06_arrfreef_frees_header` |
| 7-9 | `err07_…`, `err08_hash_bytes_every_switch_fallthrough_arm`, `err09_…` |
| 10-11 | `err10_hash_string_empty`, `err11_hash_string_high_bit_bytes_no_sign_extension` |
| 12-14 | `err12_…`, `err13_hmfree_array_without_hash_table`, `err14_…` |
| 15-17, 19 | `err15_err16_err17_err19_hmget_rejections` |
| 18 | `err18_find_slot_returns_minus_one_on_empty_probe` |
| 20-22 | `err20_err21_err22_hmput_default_branches` |
| 23 | `err23_hmdel_null_returns_null` |
| 24 | `err24_hmdel_table_null_sets_temp_zero` |
| 25-26 | `err25_err26_hmdel_absent_vs_present` |
| 27 | `err27_hmdel_mode_two_skips_strdup_free` |
| 28-30 | `err28_err29_err30_hmdel_last_shrink_rebuild` |
| 31, 44, 54, 61 | unreachable; checks present in `src/lib.rs` (see above) |
| 32-33 | `phase_c_aborts.rs::phase_c_assert_abort_parity` |
| 34-35 | `err34_err35_hmput_key_bootstrap_and_fresh_index` |
| 36 | `err36_hmput_key_growth_threshold` |
| 37-38 | `err37_err38_hmput_key_duplicate_in_both_inner_loops` |
| 39 | `err39_hmput_key_reuses_tombstones` |
| 40 | `err40_hash_below_two_is_bumped` |
| 41 | `err41_string_mode_not_1_2_3_takes_default_memcpy_branch` |
| 42-43 | `err42_err43_out_of_range_mode_enum_values` |
| 45-46 | `err45_err46_shmode_func_mode_truncation` |
| 47-53 | `err47_to_err53_stralloc_boundaries` |
| 55-56 | `err55_err56_strreset_boundaries` |
| 57-58 | `err57_err58_strkey_negative_and_shared_buffer` |
| 59-60 | `err59_err60_helxo_nul_and_high_bit_letters` |
| generic (null ptr / zero size / oversized / out-of-range enum) | `err01`, `err07`, `err12`, `err23`, `errgen_zero_elemsize_and_keysize`, `err42_err43`, `err45_err46` |

### Deliberately NOT tested (identical UB in both, not differentially observable)

* `stbds_stralloc(a, s)` / `stbds_strreset(a)` with `a == NULL` — unconditional
  dereference; both segfault.
* `stbds_hash_bytes(p, len, …)` / `stbds_hash_string(str, …)` with `len` or the
  string extending past the buffer — both read out of bounds.
* `stbds_arrgrowf` combinations whose wrapped byte size is `< sizeof(header)`
  (32) — the C overflows its own allocation. Wraps that land on a large-enough
  size *are* tested (`err05`, `row13b`).
* `stbds_hmput_key` with `string.mode == 0` and `mode >= 1` beyond the first
  insert — `stbds_is_key_equal` then dereferences copied string bytes as a
  `char *`. The single-insert configuration is tested (`row22`, `err41`).
* `a->block` values whose `512 << ((block>>1)&63)` exceeds allocatable memory —
  the C's `realloc` returns NULL and is then dereferenced
  (`common::arena_block_is_testable` filters these).
