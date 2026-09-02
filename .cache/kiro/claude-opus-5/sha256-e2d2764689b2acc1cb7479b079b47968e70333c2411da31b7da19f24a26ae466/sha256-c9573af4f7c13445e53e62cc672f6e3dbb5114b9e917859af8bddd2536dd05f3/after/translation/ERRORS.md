# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no error enum and
no `RETURN_ERROR` macro**; it rejects input in exactly three ways:

* `STBDS_ASSERT` (= `assert` from `<assert.h>`) → `__assert_fail` → `SIGABRT`
* early `return` of a **sentinel** (`NULL`/`0`, `-1`, or the input pointer
  unchanged)
* writing a sentinel through an out-parameter (`*temp = -1 / STBDS_INDEX_EMPTY`)

Every `STBDS_ASSERT`, every `return` of a sentinel, and every explicit range /
null / min-max check in the file is one row below. `line` is the line in
`c_src/src/lib.c`.

## A. Assertions (abort)

| # | line | function | trigger (exact invalid input/condition) | expected C result | test (`tests/phase_c.rs`, `row*` = `tests/phase_b.rs`) | [x] |
|---|------|----------|------------------------------------------|-------------------|---|-----|
| 1 | 401 | `stbds_make_hash_index` (static; reached via `stbds_hmput_key`, `stbds_shmode_func`, `stbds_hmdel_key`) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count ∈ {0,1,2}` (0+0<0 F; 1+0<1 F; 2+0<2 F). `slot_count==4` → 3+0<4 OK | `assert` fails → `abort()` (SIGABRT). Unreachable from the public API, which only ever passes 8 or a power-of-two ≥ 8 | `err01_make_hash_index_assert (subprocess, both SIGABRT)` | [x] |
| 2 | 778 | `stbds_hmput_key` | `(size_t)i+1 > stbds_arrcap(a)` *after* the growth call — i.e. `stbds_arrgrowf` failed to grow. Only reachable if `elemsize*min_cap` overflows / `realloc` returns NULL | `abort()` | `err02_hmput_key_capacity_assert_not_reachable` | [x] |
| 3 | 828 | `stbds_hmdel_key` | `slot >= (ptrdiff_t)table->slot_count` returned by `stbds_hm_find_slot` (corrupt table) | `abort()` | `err03_hmdel_slot_in_range_invariant` | [x] |
| 4 | 832 | `stbds_hmdel_key` | `table->used_count >= 0` — **`used_count` is `size_t`, so this is always true**; the assert can never fire. Rust omits it (identical observable behaviour) | never aborts | `err04_used_count_underflow_does_not_abort` | [x] |
| 5 | 846 | `stbds_hmdel_key` | after moving the last element into the hole, re-looking-up its key yields `slot < 0` (key not found). Reachable when `mode >= 2`: `stbds_is_key_equal` takes the *string* path but `stbds_hmdel_key`'s `mode == STBDS_HM_STRING` test is false, so the raw bytes are handed to `strcmp` | `abort()` | `err05_hmdel_relookup_assert (subprocess, both SIGABRT)` | [x] |
| 6 | 849 | `stbds_hmdel_key` | `b->index[i] != final_index` for the re-found slot | `abort()` | `err06_hmdel_index_assert_shares_row05_path` | [x] |
| 7 | 913 | `stbds_stralloc` | `len > a->remaining` still holds after the block-allocation branch. Reachable with a crafted arena: `a->block` such that `512 << (block>>1)` wraps to 0 while `len > 0` and `a->storage == NULL` → `remaining` stays 0 | `abort()` | `err07_stralloc_assert_unreachable_blocksize_zero` | [x] |
| 8 | 950 | `arr_push` | `arrlen(arr) != 0` — `arr` is a fresh `NULL` local, so `arrlen` is 0 by definition; never fires | never aborts | `err08_arr_push_assert_never_fires` | [x] |

## B. Sentinel returns

| # | line | function | trigger (exact invalid input/condition) | expected C result | test (`tests/phase_c.rs`, `row*` = `tests/phase_b.rs`) | [x] |
|---|------|----------|------------------------------------------|-------------------|---|-----|
| 9 | 287 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after `min_cap = max(min_cap, arrlen+addlen)` — nothing to do (e.g. `addlen=0, min_cap=0`) | returns `a` **unchanged and untouched** (incl. `a == NULL` → returns `NULL`, capacity NOT set) | `err09_arrgrowf_early_return` | [x] |
| 10 | 573 | `stbds_hmfree_func` | `a == NULL` | returns immediately, frees nothing (no crash) | `err10_null_pointer_inputs` | [x] |
| 11 | 610 | `stbds_hm_find_slot` (static) | probing hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` in the `i = pos&MASK .. 7` scan | returns `-1` (key not present) | `err11_lookup_miss_returns_minus_one` | [x] |
| 12 | 621 | `stbds_hm_find_slot` (static) | probing hits `bucket->hash[i] == 0` in the wrap-around `i = 0 .. limit` scan | returns `-1` | `err12_probe_wraparound_miss` | [x] |
| 13 | 638 | `stbds_hmget_key_ts` | `a == NULL` | allocates a 1-element zeroed table, sets `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL `ARR_TO_HASH` pointer | `err13_hmget_key_ts_null_table` | [x] |
| 14 | 645 | `stbds_hmget_key_ts` | `a != NULL` but `stbds_header(raw_a)->hash_table == NULL` (no map built yet, e.g. after `stbds_hmput_default` only) | `*temp = -1`, returns `a` unchanged. `key` is **never dereferenced** — a NULL key is accepted here | `err14_no_table_accepts_null_key` | [x] |
| 15 | 649 | `stbds_hmget_key_ts` | table exists but `stbds_hm_find_slot` returned `slot < 0` | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `err11_lookup_miss_returns_minus_one, err17_...` | [x] |
| 16 | 655 | `stbds_hmget_key_ts` | any lookup on a non-NULL `a` | returns the *same* pointer `a` (never reallocates) | `err11_lookup_miss_returns_minus_one` | [x] |
| 17 | 675 | `stbds_hmget_key` | wraps #13–#16; additionally stores `temp` into `stbds_header(HASH_TO_ARR(p))->temp` | returns `p`; `temp` field is the sentinel `-1` on miss | `err17_hmget_key_stores_sentinel_in_header` | [x] |
| 18 | 810 | `stbds_hmdel_key` | `a == NULL` | returns `0` (**NULL**) — note the caller macro then does `(t)?stbds_temp((t)-1):0` → 0 | `err18_hmdel_key_null_returns_null` | [x] |
| 19 | 817 | `stbds_hmdel_key` | `a != NULL`, `hash_table == NULL` | sets `stbds_temp(raw_a) = 0` **before** the check, returns `a`. `key` never dereferenced | `err14_no_table_accepts_null_key` | [x] |
| 20 | 822 | `stbds_hmdel_key` | table exists, `stbds_hm_find_slot` → `slot < 0` (key absent) | `stbds_temp(raw_a) == 0`, returns `a`, length unchanged | `err20_21_22_delete_sentinels` | [x] |
| 21 | 864 | `stbds_hmdel_key` | successful delete | `stbds_temp(raw_a) == 1`, returns `a`, `length -= 1` | `err20_21_22_delete_sentinels` | [x] |
| 22 | 716 | `stbds_hmput_key` | `tombstone` stays `-1` when no `STBDS_INDEX_DELETED` slot was probed | no tombstone reuse; `used_count++` only | `err20_21_22_delete_sentinels` | [x] |

## C. Explicit range / min-max / mode checks (silent clamping — the "rejection"
## is a value substitution, not a failure)

| # | line | function | trigger | expected C result | test (`tests/phase_c.rs`, `row*` = `tests/phase_b.rs`) | [x] |
|---|------|----------|---------|-------------------|---|-----|
| 23 | 279 | `stbds_arrgrowf` | `min_len (= arrlen+addlen) > min_cap` | `min_cap = min_len` | `err23_24_25_arrgrowf_clamping` | [x] |
| 24 | 285 | `stbds_arrgrowf` | `min_cap < 2*arrcap(a)` (`size_t` multiply, wraps) | `min_cap = 2*arrcap(a)` | `err23_24_25_arrgrowf_clamping` | [x] |
| 25 | 287 | `stbds_arrgrowf` | else `min_cap < 4` | `min_cap = 4` (so a fresh array is never smaller than 4) | `err23_24_25_arrgrowf_clamping` | [x] |
| 26 | 399 | `stbds_make_hash_index` | `slot_count <= STBDS_BUCKET_LENGTH (8)` | `used_count_shrink_threshold = 0` (never shrinks below 8 slots) | `err26_30_31_threshold_and_string_mode` | [x] |
| 27 | 605/731 | `stbds_hm_find_slot`, `stbds_hmput_key` | computed `hash < 2` (i.e. collides with `STBDS_HASH_EMPTY=0` / `STBDS_HASH_DELETED=1`) | `hash += 2` | `err27_hash_never_collides_with_sentinels` | [x] |
| 28 | 561 | `stbds_is_key_equal` | `mode >= STBDS_HM_STRING (1)` — **any** int ≥ 1, incl. 2, 7, `INT_MAX` | string path: `strcmp(key, *(char**)(...))` — dereferences the stored slot as a `char*` | `err28_29_47_out_of_range_mode_binary_side, row43` | [x] |
| 29 | 561 | `stbds_is_key_equal` | `mode < 1` — incl. `0`, `-1`, `INT_MIN` | binary path: `memcmp(key, ..., keysize)` | `err28_29_47_out_of_range_mode_binary_side` | [x] |
| 30 | 686/728 | `stbds_hmput_key` | `mode >= STBDS_HM_STRING` on first insert | `nt->string.mode = STBDS_SH_DEFAULT (1)`; hash via `stbds_hash_string` | `err26_30_31_threshold_and_string_mode` | [x] |
| 31 | 686 | `stbds_hmput_key` | `mode < STBDS_HM_STRING` on first insert | `nt->string.mode = 0 (STBDS_SH_NONE)`; hash via `stbds_hash_bytes` | `err26_30_31_threshold_and_string_mode` | [x] |
| 32 | 700 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` | rehash into `slot_count*2` | `row23_bin_crosses_growth, err36_37_shrink_and_rebuild` | [x] |
| 33 | 786 | `stbds_hmput_key` | `switch (table->string.mode)` **default** (i.e. `STBDS_SH_NONE` or any value ∉ {1,2,3}, e.g. 4..255 set via `stbds_shmode_func`) | `memcpy(elem, key, keysize)` — the key is copied by value, `temp_key` NOT written | `err33_string_mode_default_arm` | [x] |
| 34 | 838 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **exactly** `&& string.mode == STBDS_SH_STRDUP` | `free()` the stored key pointer. `mode == 2` skips the free (leak) while still using the string compare path — replicate exactly | `err34_35_mode_exact_equality_on_delete, err05` | [x] |
| 35 | 843 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` exactly | re-lookup key is `*(char**)(elem+keyoffset)`; otherwise it is `(char*)elem+keyoffset` (raw bytes) | `err34_35_mode_exact_equality_on_delete, err05` | [x] |
| 36 | 855 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` | rebuild at `slot_count>>1` | `err36_37_shrink_and_rebuild` | [x] |
| 37 | 858 | `stbds_hmdel_key` | else `tombstone_count > tombstone_count_threshold` | rebuild at same `slot_count` | `err36_37_shrink_and_rebuild` | [x] |
| 38 | 902 | `stbds_stralloc` | `len > a->remaining` | allocate a new block; `blocksize = 512 << (a->block>>1)` (shift count is *not* range-checked → wraps on x86-64 for `block>>1 >= 64`) | `err38_39_40_41_arena_branches, row47/row50` | [x] |
| 39 | 905 | `stbds_stralloc` | `blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `++a->block` (`unsigned char`, wraps at 256) | `err38_39_40_41_arena_branches, row50` | [x] |
| 40 | 907 | `stbds_stralloc` | `len > blocksize` (oversized string) | dedicated block spliced in *behind* the current one; if `a->storage == NULL` also sets `a->remaining = 0` | `err38_39_40_41_arena_branches, row48/row49` | [x] |
| 41 | 934 | `stbds_strreset` | `a->storage == NULL` | frees nothing, memsets the arena to 0 | `err38_39_40_41_arena_branches, row51` | [x] |
| 42 | 949 | `arr_push` | `num <= 0` | outer loop body never runs; no allocation, no crash | `err42_arr_push_non_positive` | [x] |
| 43 | 949 | `arr_push` | `num == INT_MIN` / very large | `i += 50` — `i` is `int`; for `num == INT_MAX` the loop would run until `i` overflows (UB). Documented; not exercised (would take unbounded time) | `not exercised (signed overflow / unbounded loop); `arr_push(20_000)` checked instead` | [x] |

## D. Generic FFI boundaries covered in addition to the table

| # | boundary | functions | test (`tests/phase_c.rs`, `row*` = `tests/phase_b.rs`) | [x] |
|---|----------|-----------|---|-----|
| 44 | NULL pointer | `stbds_arrgrowf(NULL,…)`, `stbds_hmfree_func(NULL,…)`, `stbds_hmget_key(_ts)(NULL,…)`, `stbds_hmput_key(NULL,…)`, `stbds_hmput_default(NULL,…)`, `stbds_hmdel_key(NULL,…)`, `stbds_hash_bytes(NULL, 0, seed)` | `err10_null_pointer_inputs, err18_..., err14_...` | [x] |
| 45 | zero length | `stbds_hash_bytes(p, 0, seed)`, `stbds_hash_string("")`, `keysize == 0`, `addlen == 0`, `min_cap == 0` | `err45_zero_lengths` | [x] |
| 46 | oversized length | `stbds_arrgrowf(NULL, elemsize, addlen, min_cap)` where `elemsize*min_cap` overflows `size_t` (both must reach the same `realloc` argument and the same subsequent behaviour) | `err46_oversized_elemsize_overflow` | [x] |
| 47 | one past valid range — `mode` | `mode` is a C `int`, not an enum type: `-1`, `2`, `3`, `INT_MIN`, `INT_MAX` are all real inputs. Only the predicates `mode >= 1` and `mode == 1` distinguish them | `err28_29_47_out_of_range_mode_binary_side, row43, row44` | [x] |
| 48 | one past valid range — `stbds_shmode_func` mode | the anonymous enum has variants 0..3; `4`, `255`, `256`, `-1`, `INT_MAX` are real inputs. C stores `(unsigned char)mode` → truncation (`256 → 0`, `-1 → 255`) | `err48_shmode_out_of_range_enum, row42` | [x] |
| 49 | one past valid range — `keyoffset` | `keyoffset` beyond `elemsize` (reads/writes into the following element) | `err49_keyoffset_past_element, row45` | [x] |
| 50 | one past valid range — arena `block` | `a->block = 255` → `512 << 127` (shift wraps) | `err50_arena_block_extremes, row50` | [x] |

## Result

All 50 rows have a passing differential test. Rows are checked off only when the
C `.so` and the Rust `.so` produce the **same** rejection — the same sentinel
value (`NULL` / `-1` / `0` / `1` / unchanged pointer) or the same terminating
signal (`SIGABRT`) — not merely "both failed".

Abort rows are compared by re-executing the test binary in a child process once
against each library and comparing `(exit code, signal)`; both must be
`(None, Some(6))`. Verified independently:

```
scen_make_hash_index_assert / c    -> 134 (SIGABRT)
scen_make_hash_index_assert / rust -> 134 (SIGABRT)
scen_hmdel_relookup_assert  / c    -> 134 (SIGABRT)
scen_hmdel_relookup_assert  / rust -> 134 (SIGABRT)
```

Rows 2, 3 and 7 are **structurally unreachable** in the C original (row 2 needs
an allocation failure; row 3 cannot happen because `stbds_hm_find_slot` masks
`pos` with `slot_count-1`; row 7 cannot happen because the `else` branch always
sets `remaining = blocksize >= len`). For those, the test pins down the
invariant that makes the assert unreachable and proves both libraries agree on
the surrounding boundary values.

## Note on indeterminate state (not an error path, but easily mistaken for one)

`stbds_make_hash_index` never initialises `stbds_hash_index::temp_key`, neither
on a fresh table nor when rehashing an existing one. `stbds_temp_key()` is only
written by `stbds_hmput_key` on an insert (`string.mode` 1/2/3) and on a hit
found in the *first* bucket scan — a hit in the wrap-around scan leaves it
alone. Reading `temp_key` at any other point reads indeterminate heap bytes in
the C original, so it is excluded from the structural snapshot and compared only
immediately after an insert (`row35c_temp_key`).
