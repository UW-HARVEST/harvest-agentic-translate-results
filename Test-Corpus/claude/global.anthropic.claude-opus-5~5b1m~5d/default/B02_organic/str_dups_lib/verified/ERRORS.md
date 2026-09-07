# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

The library has no error enum and no `RETURN_ERROR` macro. Its rejection
surface consists of:

* early `return`s on NULL / empty input,
* the `-1` (`STBDS_INDEX_EMPTY`) and `-2` (`STBDS_INDEX_DELETED`) sentinels,
* `NULL` returns,
* `STBDS_ASSERT` (= `assert`, **enabled**: CMake sets no `NDEBUG`),
* implicit truncation / saturation of out-of-range scalar arguments
  (`int mode` → `unsigned char`, `mode >= STBDS_HM_STRING` comparisons,
  `unsigned char block` shift saturation).

Every row below was derived by grepping the C source for `return`, `assert`,
`STBDS_ASSERT`, `== NULL`, `== 0`, `< 0`, `>=`, and the `MIN`/`MAX` constants.

| # | function | trigger (exact invalid input / condition) | expected C result |
|---|----------|-------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` (line 286) | `min_cap <= stbds_arrcap(a)` (incl. `a=NULL, elemsize=0, min_cap=0`) | returns `a` **unchanged** (same pointer, header untouched) |
| 2 | `stbds_arrgrowf` (line 283) | `addlen` s.t. `arrlen+addlen > min_cap` | `min_cap` silently raised to `min_len`; no error |
| 3 | `stbds_arrgrowf` (line 289/291) | `min_cap` between `arrcap` and `2*arrcap`, and `min_cap < 4` | capacity forced to `max(2*arrcap, 4)`; observable via header `capacity` |
| 4 | `stbds_arrgrowf` | `addlen` huge so `elemsize*min_cap + 32` overflows / `realloc` returns NULL | C dereferences NULL → SIGSEGV. **UB — not differentially testable**, documented only |
| 5 | `stbds_arrfreef(NULL)` (line 314) | `a == NULL` → `free((char*)NULL - 32)` | invalid free / abort. **UB — not testable**, documented only |
| 6 | `stbds_hmfree_func` (line 573) | `a == NULL` | returns immediately, no crash, no free |
| 7 | `stbds_hmfree_func` (line 574) | `stbds_hash_table(a) == NULL` (array built by `stbds_arrgrowf`, never `hmput`) | skips strdup-free + `strreset`, still frees header and (NULL) table |
| 8 | `stbds_hm_find_slot` (line 610/620) | key absent, probe hits `STBDS_HASH_EMPTY` slot | returns `-1` |
| 9 | `stbds_hmget_key_ts` (line 634) | `a == NULL` | allocates 1-elem array, zeroes it, sets `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` |
| 10 | `stbds_hmget_key_ts` (line 644) | `a != NULL` but `hash_table == NULL` | `*temp = -1`, returns `a` unchanged |
| 11 | `stbds_hmget_key_ts` (line 648) | key not present | `*temp = STBDS_INDEX_EMPTY (-1)` |
| 12 | `stbds_hmget_key_ts` (line 652) | slot found but bucket index is `STBDS_INDEX_DELETED` — unreachable, `find_slot` never returns tombstone slots | n/a (documented) |
| 13 | `stbds_hmget_key` (line 663) | any of rows 9–11 | additionally writes `temp` into the array header (`header->temp == -1`) |
| 14 | `stbds_hmput_default` (line 669) | `a == NULL` | grows a fresh 1-element array, `length = 1`, zeroed |
| 15 | `stbds_hmput_default` (line 669) | `a != NULL` but `header(a-elemsize)->length == 0` | grows/zeroes again, `length` becomes 1 |
| 16 | `stbds_hmput_key` (line 686) | `a == NULL` | allocates, `length = 1`, then proceeds |
| 17 | `stbds_hmput_key` (line 698) | `table == NULL` | allocates hash index with `slot_count = STBDS_BUCKET_LENGTH (8)` |
| 18 | `stbds_hmput_key` (line 698) | `used_count >= used_count_threshold` (6 of 8 slots) | rehash into `slot_count*2`; observable via header/`slot_count` |
| 19 | `stbds_hmput_key` (line 707) | `mode < STBDS_HM_STRING` on a fresh table | `string.mode = 0` (`STBDS_SH_NONE`) → key copied with `memcpy` |
| 20 | `stbds_hmput_key` (line 707/713/732) | `mode` out of range **negative** (`mode = -1`, `INT_MIN`) | `mode >= 1` false → treated exactly as `STBDS_HM_BINARY` |
| 21 | `stbds_hmput_key` (line 707/713/732) | `mode` out of range **positive** (`mode = 2, 7, 12345, INT_MAX`) | `mode >= 1` true → treated exactly as `STBDS_HM_STRING` (string hash + `strcmp`) |
| 22 | `stbds_hmput_key` (line 766) | probe found a tombstone before the empty slot | reuses tombstone, `--tombstone_count` |
| 23 | `stbds_hmput_key` (line 778) | `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` | assert holds for all reachable inputs (grow happens on line 774); documented |
| 24 | `stbds_hmput_key` (line 789 `default:`) | `table->string.mode` not one of `STRDUP/ARENA/DEFAULT` (i.e. `0` or any value `>= 4` injected via `stbds_shmode_func`) | `memcpy(elem, key, keysize)` — raw binary key copy even in string mode |
| 25 | `stbds_shmode_func` (line 803) | `mode` out of range (`mode = 4 … 255`, `256`, `-1`, `INT_MAX`) | `(unsigned char) mode` truncation: `256→0`, `-1→255`, `INT_MAX→255`; stored verbatim in `string.mode`, later hitting the `default:` arm of row 24 |
| 26 | `stbds_hmdel_key` (line 809) | `a == NULL` | returns `0` (**NULL**) |
| 27 | `stbds_hmdel_key` (line 816) | `hash_table == NULL` | sets `header->temp = 0`, returns `a` unchanged |
| 28 | `stbds_hmdel_key` (line 821) | key absent (`find_slot` → `-1`) | `header->temp = 0`, returns `a`, `length`/`used_count` unchanged |
| 29 | `stbds_hmdel_key` (line 828) | `STBDS_ASSERT(slot < table->slot_count)` | always holds (`find_slot` masks by `slot_count-1`); documented |
| 30 | `stbds_hmdel_key` (line 832) | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t`, so this can never fire even after underflow; documented |
| 31 | `stbds_hmdel_key` (line 836) | `mode == STBDS_HM_STRING` **exactly 1** and `string.mode == STRDUP` | frees the key. For `mode = 2, 7, INT_MAX` (also "string" for hashing) the key is **leaked**, not freed — asymmetric check that must be replicated |
| 32 | `stbds_hmdel_key` (line 846) | `STBDS_ASSERT(slot >= 0)` after moving the final element | fires (abort) only if the moved element is absent from the table — unreachable in normal use; documented |
| 33 | `stbds_hmdel_key` (line 849) | `STBDS_ASSERT(b->index[i] == final_index)` | documented, unreachable |
| 34 | `stbds_hmdel_key` (line 854) | `used_count < used_count_shrink_threshold && slot_count > 8` | table shrinks to `slot_count>>1` |
| 35 | `stbds_hmdel_key` (line 858) | `tombstone_count > tombstone_count_threshold` | table rebuilt at same `slot_count`, tombstones cleared |
| 36 | `stbds_hmdel_key` | delete on a map of length 1 (`final_index == old_index == 0`) | no `memmove`, `length` → 0 (i.e. `hmlen` → −1 … guard) |
| 37 | `stbds_is_key_equal` (line 560) | `mode >= STBDS_HM_STRING` with a NULL stored key pointer | `strcmp(key, NULL)` → SIGSEGV. **UB — not testable**, documented |
| 38 | `stbds_make_hash_index` (line 401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | holds for every reachable `slot_count` (powers of two ≥ 8); documented |
| 39 | `stbds_stralloc` (line 885) | `len > a->remaining` on a **fresh** arena (`remaining = 0`) | allocates a `512`-byte block, `++a->block` |
| 40 | `stbds_stralloc` (line 890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` stops incrementing (saturates at 22) |
| 41 | `stbds_stralloc` (line 893) | `len > blocksize` (oversized string, e.g. 2000 bytes on a fresh arena) | dedicated block spliced in **after** `a->storage`; `remaining` **not** consumed; when `a->storage == NULL` also sets `remaining = 0` |
| 42 | `stbds_stralloc` (line 913) | `STBDS_ASSERT(len <= a->remaining)` | holds after the grow path; documented |
| 43 | `stbds_stralloc` | empty string `""` (`len == 1`) | allocates a block on a fresh arena, returns pointer to `'\0'` |
| 44 | `stbds_stralloc(a, NULL)` | NULL string | `strlen(NULL)` → SIGSEGV. **UB — not testable**, documented |
| 45 | `stbds_strreset` (line 924) | `a->storage == NULL` (fresh or already reset arena) | loop body skipped, arena zeroed; idempotent |
| 46 | `stbds_hash_string` (line 480) | empty string `""` | loop skipped, avalanche applied to `seed` alone |
| 47 | `stbds_hash_string(NULL, seed)` | NULL pointer | SIGSEGV. **UB — not testable**, documented |
| 48 | `stbds_hash_bytes` (line 522/532) | `len == 0` (with `p == NULL` too) | no loop iteration, `switch` case 0 → hash of `len<<56` only; **no dereference**, so this IS testable |
| 49 | `stbds_hash_bytes` (line 532) | `len - i == 1 … 7` (non-multiple-of-8 length) | fall-through `switch` assembles a partial word |
| 50 | `stbds_hm_find_slot` (line 596) / `stbds_hmput_key` (line 719) | computed `hash < 2` (collides with `STBDS_HASH_EMPTY`/`DELETED`) | `hash += 2` fixup |
| 51 | `str_dups` (line 952) | `num <= 0` (`0`, `-1`, `INT_MIN`) | `stralloc` loop skipped entirely; still prints one line |
| 52 | `str_dups` (line 960–962) | three `STBDS_ASSERT`s on the strdup'd entry | must all hold; a divergence in `hmput_key`/`shmode_func` would abort the C build |
| 53 | `strkey` (line 941) | `n` negative / `INT_MIN` | `sprintf` into a 256-byte static buffer, no overflow for any `int` |

---

## Row 32b (discovered while writing the Phase C tests)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 32b | `stbds_hmdel_key` (lines 842–846) | `mode >= 2` (out-of-range "string" mode) **and** `old_index != final_index`, i.e. the swap-with-last branch | `mode == STBDS_HM_STRING` is *false*, so the `else` branch hands `stbds_hm_find_slot` the **address of the element** instead of the stored `char *`; `find_slot` then string-hashes the pointer bytes, returns `-1`, and `STBDS_ASSERT(slot >= 0)` **aborts the process (SIGABRT)** |

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-DNDEBUG`, so every
`STBDS_ASSERT` is live — confirmed by `nm -D --undefined-only` on the C `.so`
showing `U __assert_fail@GLIBC_2.2.5`. The Rust translation therefore now
replicates all of them via an `STBDS_ASSERT!` macro that calls libc `abort()`,
so the two libraries die on the same signal for the same inputs.

## Test coverage map

| ERRORS.md rows | test |
|---|---|
| 1 | `phase_c::err01_arrgrowf_no_grow_returns_same_pointer` |
| 2 | `phase_c::err02_arrgrowf_addlen_raises_min_cap` |
| 3 | `phase_c::err03_arrgrowf_min4_and_doubling_clamps` |
| 4, 5, 37, 44, 47 | **not differentially testable** — the C dereferences NULL / frees an invalid pointer (documented UB; both libraries would fault, with no defined result to compare) |
| 6 | `phase_c::err06_hmfree_null` |
| 7 | `phase_c::err07_hmfree_no_hash_table` |
| 8 | `phase_c::err08_find_slot_returns_minus_one_for_absent` |
| 9 | `phase_c::err09_get_ts_null_map_bootstraps` (× all 14 `mode` values) |
| 10 | `phase_c::err10_get_ts_table_null` (× all 14 `mode` values) |
| 11 | `phase_c::err08_…`, `phase_b::row32/row33` |
| 12 | unreachable by construction — documented |
| 13 | `phase_c::err13_get_key_writes_temp_on_miss` |
| 14, 15 | `phase_c::err14_15_hmput_default_null_and_empty` |
| 16, 17 | `phase_c::err16_17_put_null_map_fresh_table` |
| 18 | `phase_c::err18_put_crosses_used_count_threshold` |
| 19, 20, 21 | `phase_c::err19_21_put_out_of_range_modes`, `phase_c::gen_all_modes_through_full_lifecycle` |
| 22 | `phase_c::err22_tombstone_reuse_decrements_count` |
| 23 | `phase_c_abort` scenario `zero_slot_count` covers the sibling assert; the `arrcap` assert holds for every reachable input (verified by the whole suite running with the assert live in both libraries) |
| 24 | `phase_c::err24_default_switch_arm_memcpy` (`string.mode` ∈ {0,4,5,7,100,255,…}) |
| 25 | `phase_c::err25_shmode_func_out_of_range_truncation` (× all 14 `mode` values, incl. `256 → 0`, `-1 → 255`, `INT_MAX → 255`) |
| 26 | `phase_c::err26_del_null_map_returns_null` (× `elemsize`, `mode`, `keyoffset`) |
| 27 | `phase_c::err27_del_table_null_sets_temp_zero` |
| 28 | `phase_c::err28_del_absent_key_is_noop` |
| 29, 30, 33 | asserts that cannot fail for any reachable input; live in both libraries and never fired across the whole suite |
| 31 | `phase_c::err31_del_string_mode_variants` (`mode` ∈ {1,2,3,7,255,12345,INT_MAX} × `string.mode` ∈ {STRDUP, ARENA, DEFAULT}) |
| 32 | `phase_c_abort::abort_parity_between_c_and_rust`, scenario `del_swap_bad_mode` (both abort with SIGABRT) |
| 32b | same as row 32 |
| 34, 35 | `phase_c::err34_35_del_shrink_and_rebuild_thresholds`, `phase_b::row41` |
| 36 | `phase_c::err36_del_last_remaining_entry` |
| 38 | `phase_c_abort::abort_parity_between_c_and_rust`, scenario `zero_slot_count` (both abort with SIGABRT) |
| 39 | `phase_c::err39_stralloc_fresh_arena` |
| 40 | `phase_c::err40_stralloc_block_saturates`, `phase_b::row52` |
| 41 | `phase_c::err41_stralloc_oversized_paths`, `phase_b::row49/row50` |
| 42 | assert live in both libraries; never fires on the grow path (whole suite) |
| 43 | `phase_c::err43_stralloc_empty_string` |
| 45 | `phase_c::err45_strreset_idempotent` |
| 46 | `phase_c::err46_hash_string_empty_and_one_byte` |
| 48 | `phase_c::err48_hash_bytes_zero_len` (incl. `p == NULL`) |
| 49 | `phase_c::err49_hash_bytes_every_remainder` (`len` 0..=64) |
| 50 | `phase_c::err50_hash_fixup_never_stores_0_or_1` (2000 inserts) |
| 51, 52 | `stdout_diff::str_dups_stdout_is_byte_identical` (`num` = 0, −1, −2, −100, −12345, `INT_MIN`, `INT_MIN+1`) |
| 53 | `phase_c::err53_strkey_extremes` |

Additional generic FFI-boundary sweeps (not tied to a single row):
`gen_zero_and_large_elemsize`, `gen_keysize_boundaries` (`keysize = 0` and
`keysize == elemsize`), `gen_keyoffset_out_of_range_but_in_element`,
`gen_all_modes_through_full_lifecycle`, `gen_seed_extremes_full_lifecycle`.

**Status: every testable row has a passing differential test.**
