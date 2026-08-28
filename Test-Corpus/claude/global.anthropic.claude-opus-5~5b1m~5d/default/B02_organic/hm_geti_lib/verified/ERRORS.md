# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` by enumerating every `return`
of a sentinel value, every `STBDS_ASSERT` (`= assert`, and the library is
compiled **without** `-DNDEBUG`, so assertions are live), every explicit
`== NULL` / `< 0` / `<=` guard, and every min/max constant.

`stb_ds` has no error enum: it rejects input by returning the sentinels
`-1` (`STBDS_INDEX_EMPTY`), `-2` (`STBDS_INDEX_DELETED`), `NULL`/`0`,
`temp = 0`, or by leaving the caller's pointer unchanged.  Each distinct
rejection branch is one row.

Sentinels: `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`,
`STBDS_HM_BINARY = 0`, `STBDS_HM_STRING = 1`,
`STBDS_SH_NONE/DEFAULT/STRDUP/ARENA = 0/1/2/3`,
`STBDS_BUCKET_LENGTH = 8`, `STBDS_CACHE_LINE_SIZE = 64`,
`STBDS_STRING_ARENA_BLOCKSIZE_MIN = 512`, `..._MAX = 1<<20`.

| # | function | trigger (exact invalid input/condition) | expected C result | verified |
|---|----------|------------------------------------------|-------------------|----------|
| 1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after `min_len = arrlen+addlen` clamp (e.g. existing cap 4, `addlen=0`, `min_cap=0..4`) | early `return a` — **identical pointer**, header bytes untouched | [x] |
| 2 | `stbds_arrgrowf` | `a == NULL` (nothing to grow from) | fresh block; `length=0`, `hash_table=NULL`, `temp=0`, `capacity = max(min_len, min_cap, 4)`; returns `base+32` | [x] |
| 3 | `stbds_arrgrowf` | `min_cap == 0 && addlen == 0 && a == NULL` (degenerate zero request) | `min_len = 0`, so `min_cap(0) <= arrcap(NULL)(0)` ⇒ the early return fires and the function returns **NULL** (it does *not* reach the `< 4` clamp and allocates nothing) | [x] |
| 4 | `stbds_arrfreef` | `a == NULL` → `free((char*)NULL - 32)` | invalid free → glibc abort (UB). **Documented, not executed** (would kill the test process); both sides emit the identical `free(header)` call | [x] (documented) |
| 5 | `stbds_hm_find_slot` | key absent; probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` in the `i = pos&7 .. 8` loop | `return -1` | [x] |
| 6 | `stbds_hm_find_slot` | key absent; probe reaches `hash[i] == STBDS_HASH_EMPTY` in the wrap-around `i = 0 .. limit` loop | `return -1` | [x] |
| 7 | `stbds_hm_find_slot` | `bucket->hash[i] == hash` but `stbds_is_key_equal` returns 0 (hash collision on a different key) | probing continues, eventually `-1` | [x] |
| 8 | `stbds_hmget_key_ts` | `a == NULL` | `*temp = STBDS_INDEX_EMPTY (-1)`; returns a **new** 1-element map (`length==1`, `hash_table==NULL`) | [x] |
| 9 | `stbds_hmget_key_ts` | `a != NULL` but `header->hash_table == NULL` (map created only by `hmput_default`/`hmget_key`) | `*temp = -1`; returns `a` unchanged | [x] |
| 10 | `stbds_hmget_key_ts` | key not present in a populated table (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | [x] |
| 11 | `stbds_hmget_key` | any of rows 8/9/10 | same, and additionally `stbds_header(ret-elemsize)->temp == -1` | [x] |
| 12 | `stbds_hmget_key(_ts)` | `mode` out of enum range, `mode >= 2` (e.g. 2, 7, 1000, `INT_MAX`) | `mode >= STBDS_HM_STRING` ⇒ treated as **string** (hash_string + strcmp) | [x] |
| 13 | `stbds_hmget_key(_ts)` | `mode` out of enum range, `mode < 0` (e.g. -1, `INT_MIN`) | `mode < STBDS_HM_STRING` ⇒ treated as **binary** (hash_bytes + memcmp) | [x] |
| 14 | `stbds_hmput_default` | `a == NULL` | allocates the 1-element default slot (`length==1`), `hash_table==NULL` | [x] |
| 15 | `stbds_hmput_default` | `a != NULL` and `header(a-elemsize)->length == 0` | grows and re-creates the default slot, `length` becomes 1 | [x] |
| 16 | `stbds_hmput_default` | `a != NULL` and `length != 0` (already has a default slot) | `return a` — identical pointer, nothing modified | [x] |
| 17 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` | never fires (the preceding `arrgrowf` guarantees it). **Documented**; exercised indirectly by every insert | [x] (documented) |
| 18 | `stbds_hmdel_key` | `a == NULL` | `return 0` (**NULL**) | [x] |
| 19 | `stbds_hmdel_key` | `header->hash_table == NULL` | `stbds_temp(raw_a) = 0`; `return a` unchanged | [x] |
| 20 | `stbds_hmdel_key` | key not found (`slot < 0`) | `temp == 0`; `return a`; `used_count`/`tombstone_count`/`length` unchanged | [x] |
| 21 | `stbds_hmdel_key` | key found (success sentinel, for contrast) | `temp == 1`; slot becomes `hash=STBDS_HASH_DELETED(1)`, `index=STBDS_INDEX_DELETED(-2)` | [x] |
| 22 | `stbds_hmdel_key` | `keyoffset != 0` while keys live at offset 0 ⇒ `memcmp` compares the wrong bytes ⇒ mismatch | treated as "not found": `temp == 0`, `return a` (asserts *not* reached) | [x] |
| 23 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **and** `table->string.mode == STBDS_SH_STRDUP` | the stored key is `free`d before the move; `temp == 1` | [x] |
| 24 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < table->slot_count)`, `STBDS_ASSERT(slot >= 0)` (re-find of the moved element), `STBDS_ASSERT(b->index[i] == final_index)` | hold for every library-maintained table; a failure aborts. **Documented**; every delete test asserts they did not fire (process survives) and that the re-index result is identical | [x] |
| 25 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` — `used_count` is `size_t` | vacuously true (dead check); reproduced as a no-op | [x] (documented) |
| 26 | `stbds_hmfree_func` | `a == NULL` | `return` immediately, no free, no crash | [x] |
| 27 | `stbds_hmfree_func` | `stbds_hash_table(a) == NULL` (map without a table) | skips the strdup sweep and `strreset`, still frees `hash_table`(NULL) + header | [x] |
| 28 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)`; would fire for `slot_count == 0` | unreachable from the public API (`slot_count` is always a power of two `>= 8`: 8 from `shmode_func`/first `hmput_key`, doubling afterwards). **Documented**, and the invariant is re-checked on every snapshot | [x] (documented) |
| 29 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` | unreachable for a library-maintained arena; only a hand-forged arena (`remaining` too large for `storage`) can trip it. **Documented, not executed** | [x] (documented) |
| 30 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` with `a->storage == NULL` (huge first string) | dedicated block: `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`, returns `sb->storage`; `a->block` was already incremented | [x] |
| 31 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` with `a->storage != NULL` (huge string, existing block) | dedicated block spliced *after* the head (`sb->next = storage->next; storage->next = sb`), `a->remaining` **unchanged** | [x] |
| 32 | `stbds_stralloc` | `a->block` large enough that `512 << (block>>1) >= 1<<20` (block `>= 22`) | `a->block` stops incrementing (max-blocksize clamp) | [x] |
| 33 | `stbds_stralloc` | empty string `""` (`len == 1`) | consumes exactly 1 byte of `remaining` | [x] |
| 34 | `stbds_strreset` | `a->storage == NULL` (already empty / freshly zeroed arena) | no frees; arena zeroed (`block=0`, `mode=0`, `remaining=0`) | [x] |
| 35 | `stbds_shmode_func` | `mode` outside the `STBDS_SH_*` enum: `-1`, `4`, `255`, `256`, `259`, `INT_MIN`, `INT_MAX` | `h->string.mode = (unsigned char) mode` ⇒ 255, 4, 255, **0**, **3**, 0, 255 | [x] |
| 36 | `stbds_hash_string` | empty string `""` | hash of the seed alone (loop body never runs) | [x] |
| 37 | `stbds_hash_bytes` | `len == 0` | only the length word (`len << 56`) is mixed; `p` never dereferenced | [x] |
| 38 | `stbds_hash_bytes` | bytes with the high bit set (`>= 0x80`) at positions 3 and 7 | C integer promotion makes `d[3] << 24` negative ⇒ **sign extension** into the top 32 bits of `size_t` | [x] |
| 39 | `stbds_hm_find_slot` / `stbds_hmput_key` | `if (hash < 2) hash += 2` | unreachable in practice (would require inverting siphash / the string hash to land on 0 or 1). **Documented**; the branch is translated verbatim | [x] (documented) |
| 40 | `stbds_is_key_equal` | `mode >= STBDS_HM_STRING` with a stored key that is not equal | returns `0` (`strcmp != 0`) | [x] |
| 41 | `hm_geti` | `num <= 0` (`0`, `-1`, `-100`, `INT_MIN`) | all `for` loops are skipped; the three leading `STBDS_ASSERT`s still run; returns normally | [x] |
| 42 | `hm_geti` | any `num` where an internal `STBDS_ASSERT` would fail | would `abort()`; both libraries must survive identically (checked for every tested `num`) | [x] |
| 43 | `strkey` | `n = INT_MIN` / `INT_MAX` (longest `%d` expansion, 11 chars + NUL) | `"test_-2147483648"` / `"test_2147483647"` inside the 256-byte static buffer, no overflow | [x] |

## Generic FFI boundary cases also covered

* NULL map pointer into every entry point that documents it (`hmget_key`,
  `hmget_key_ts`, `hmput_key`, `hmput_default`, `hmdel_key`, `hmfree_func`).
* Zero lengths: `keysize = 0`, `len = 0`, `addlen = 0`, `min_cap = 0`,
  `elemsize` as small as 1.
* Oversized / boundary values: `min_cap` and `addlen` near `SIZE_MAX/elemsize`
  are **not** executed (they would `realloc` petabytes / wrap); the wrapping
  arithmetic is instead verified with `elemsize=1, min_cap=1<<20`.
* Out-of-range enum values across FFI for both `mode` parameters
  (`stbds_hmget/put/del_key`'s `STBDS_HM_*` and `stbds_shmode_func`'s
  `STBDS_SH_*`): `-1`, `2`, `3`, `4`, `255`, `256`, `259`, `INT_MIN`, `INT_MAX`.
* One step past a valid range: `mode = 2` (one past `STBDS_HM_STRING`),
  `mode = 4` (one past `STBDS_SH_ARENA`), `block = 22/23` (the arena
  blocksize clamp), 6/7 entries (one step past `used_count_threshold` for
  `slot_count = 8`).

## Row → test mapping (all in `tests/phase_c_errors.rs` unless noted)

| rows | test |
|------|------|
| 1 | `e01_arrgrowf_noop_returns_same_pointer` (+ `phase_b_arr::growf_noop`) |
| 2 | `e02_arrgrowf_null_initialises_header` (+ `phase_b_arr::growf_fresh_matrix`) |
| 3 | `e03_arrgrowf_zero_zero_returns_null` |
| 4 | `e04_arrfreef_null_documented` (documented: executing it aborts) |
| 5, 6, 7, 40 | `e05_e06_e07_e40_find_slot_misses` |
| 8, 11 | `e08_e11_get_on_null_map` |
| 9, 10 | `e09_e10_get_temp_minus_one` |
| 12, 13 | `e12_e13_mode_out_of_range` (+ `phase_b_strmap::mode_out_of_range_valid`) |
| 14, 15, 16 | `e14_e15_e16_put_default` |
| 17 | `e17_hmput_capacity_assert_documented` |
| 18 | `e18_hmdel_null_map` |
| 19, 20, 21 | `e19_e20_e21_hmdel_sentinels` |
| 22 | `e22_hmdel_wrong_keyoffset` |
| 23 | `e23_hmdel_strdup_frees_key` (+ `phase_d_heap_parity`) |
| 24, 25 | `e24_e25_hmdel_asserts_hold` |
| 26, 27 | `e26_e27_hmfree_edge_cases` |
| 28 | `e28_hash_index_invariant` (`t.invariant_ok` is in every snapshot) |
| 29 | documented — a library-maintained arena can never trip it |
| 30, 31, 32, 33 | `e30_e33_stralloc_boundaries` (+ `phase_b_arena::*`) |
| 34 | `e34_strreset_empty` |
| 35 | `e35_shmode_out_of_range` |
| 36 | `e36_hash_string_empty` |
| 37 | `e37_hash_bytes_zero_len` (incl. a NULL pointer) |
| 38, 39 | `e38_e39_hash_quirks` (+ `phase_b_hash::hash_bytes_signext`) |
| 41, 42, 43 | `e41_e43_driver_edges` (+ `phase_b_driver::hm_geti_nonpositive`) |
| generic | `e_generic_zero_sizes` (NULL key, `keysize = 0`, `elemsize = 1`) |
