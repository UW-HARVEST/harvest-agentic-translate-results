# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/lib.c` by grepping **every** `return`
that yields a sentinel, **every** `STBDS_ASSERT`, **every** explicit range /
null / bound check, every `switch` default and every min/max constant.
The library has no error-code enum and no `RETURN_ERROR` macro: it rejects
input exclusively via (a) sentinel returns (`NULL`, `-1`, `temp == 0/-1`,
"pointer returned unchanged"), (b) `assert()` → `SIGABRT`, or (c) silently
taking a different branch.

Legend for "expected C result":

* `temp` = `stbds_header(raw_a)->temp`, the out-of-band result slot the stb_ds
  macros read (`hmgeti` / `hmdel` / `shgeti` all return it).
* `SIGABRT+msg` = `__assert_fail` prints
  `<file>:<line>: <func>: Assertion \`<expr>' failed.` then `abort()`s.

Tests live in `tests/errors.rs` (differential, C `.so` vs Rust `.so`, both via
`libloading`).  `[x]` = a differential test exists **and passes**.

## `stbds_arrgrowf`

| # | function | trigger (exact invalid input / condition) | expected C result | test | ok |
|---|----------|-------------------------------------------|-------------------|------|----|
| 1 | `stbds_arrgrowf` | `a == NULL` (no array yet) | fresh `realloc`, `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap,4)` | `err_arrgrowf_null_a` | [x] |
| 2 | `stbds_arrgrowf` | `min_cap <= arrcap(a)` (nothing to do) — L286 | returns **the same pointer**, no realloc, header untouched | `err_arrgrowf_noop_returns_same_ptr` | [x] |
| 3 | `stbds_arrgrowf` | `min_cap == 0 && addlen == 0 && a == NULL` | `min_len == min_cap == 0` and `arrcap(NULL) == 0`, so `min_cap <= arrcap(a)` hits first (L286) ⇒ returns **`NULL` verbatim, no allocation** (so `arrfreef` on the result is UB) | `err_arrgrowf_min_cap_zero` | [x] |
| 4 | `stbds_arrgrowf` | `min_cap` in `1..=3` with `a == NULL` | clamped to 4 (`min_cap < 4` branch) | `err_arrgrowf_min_cap_below_4` | [x] |
| 5 | `stbds_arrgrowf` | `min_cap` between `cap+1` and `2*cap-1` | clamped up to `2*cap` (doubling branch) | `err_arrgrowf_doubling` | [x] |
| 6 | `stbds_arrgrowf` | `addlen` huge so `arrlen+addlen` wraps `size_t` | wrap-around reproduced bit-for-bit (`min_len` wraps, may re-enter the no-op path) | `err_arrgrowf_addlen_wrap` | [x] |
| 7 | `stbds_arrgrowf` | `elemsize == 0` | `realloc(NULL, 0*cap+32)`, capacity still set | `err_arrgrowf_elemsize_zero` | [x] |
| 8 | `stbds_arrgrowf` | `realloc` fails (`elemsize*min_cap` ≈ 2^54) | `b = NULL+32`, then writes → **SIGSEGV** (no NULL check in C) | `err_arrgrowf_oom_segv` (child) | [x] |
| 9 | `stbds_arrfreef` | `a == NULL` | `free((stbds_array_header*)NULL - 1)` = `free((void*)-32)` → glibc **SIGSEGV/SIGABRT** ("free(): invalid pointer") | `err_arrfreef_null_crashes` (child) | [x] |

## `stbds_hash_string` / `stbds_hash_bytes`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 10 | `stbds_hash_string` | `str` points at `""` (loop body never runs) | avalanche of `seed` alone; deterministic value | `err_hash_string_empty` | [x] |
| 11 | `stbds_hash_string` | `str == NULL` | dereferences `*str` → **SIGSEGV** | `err_hash_string_null_segv` (child) | [x] |
| 12 | `stbds_hash_bytes` | `len == 0` (and `p == NULL`) | never touches `p`; returns finalisation of `seed` only | `err_hash_bytes_zero_len_null_ptr` | [x] |
| 13 | `stbds_hash_bytes` | `len - i == 1..7` tail (`switch` fall-through, L532-541) | each of the 7 fall-through cases, incl. the **sign-extending** `d[3]<<24` in `case 4` | `err_hash_bytes_tail_all_lengths` | [x] |
| 14 | `stbds_hash_bytes` | `len` so large it is used as a shift (`len << 56`) — L531 | only the low 8 bits of `len` survive the shift; identical wrap | `err_hash_bytes_len_shift_wrap` | [x] |
| 15 | `stbds_hash_bytes` | `p == NULL, len != 0` | **SIGSEGV** | `err_hash_bytes_null_ptr_nonzero_len` (child) | [x] |

## `stbds_hmget_key_ts` / `stbds_hmget_key`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 16 | `stbds_hmget_key_ts` | `a == NULL` — L634 | allocates the sentinel element, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash pointer (key never inspected, may be `NULL`) | `err_hmget_ts_null_a` | [x] |
| 17 | `stbds_hmget_key_ts` | non-NULL `a` whose `hash_table == 0` (e.g. from `hmput_default`) — L644 | `*temp = -1`, `a` returned unchanged, no hashing | `err_hmget_ts_no_table` | [x] |
| 18 | `stbds_hmget_key_ts` | key absent from a populated table (`find_slot` → -1) — L648 | `*temp = -1` | `err_hmget_ts_missing_key` | [x] |
| 19 | `stbds_hmget_key` | `a == NULL` | as #16 **plus** `header->temp = -1` written into the freshly created array | `err_hmget_null_a_sets_temp` | [x] |
| 20 | `stbds_hmget_key` | key absent | `header->temp = -1` (this is what `hmgetp_null` tests against) | `err_hmget_missing_key_sets_temp` | [x] |
| 21 | `stbds_hmget_key*` | `mode` out of enum range: `-1`, `2`, `7`, `i32::MIN`, `i32::MAX` | `mode >= 1` ⇒ **string** path (`strcmp`/`hash_string`), `mode < 1` ⇒ **binary** path (`memcmp`/`hash_bytes`); no validation at all | `err_mode_out_of_range_get` | [x] |
| 22 | `stbds_hmget_key_ts` | `temp == NULL` | writes through NULL → **SIGSEGV** | `err_hmget_ts_null_temp_segv` (child) | [x] |

## `stbds_hmput_default`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 23 | `stbds_hmput_default` | `a == NULL` — L669 | allocates array, `length = 1`, element 0 zeroed, returns `arr+elemsize` | `err_hmput_default_null_a` | [x] |
| 24 | `stbds_hmput_default` | `a != NULL` but `header->length == 0` (hand-made) | same allocation path, `realloc`s the existing block, `length` becomes 1 | `err_hmput_default_zero_length` | [x] |
| 25 | `stbds_hmput_default` | `a != NULL, length > 0` | returns `a` **unchanged** (idempotent) | `err_hmput_default_idempotent` | [x] |

## `stbds_hmput_key`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 26 | `stbds_hmput_key` | `a == NULL` — L686 | bootstraps array (`length=1`, elem0 zeroed) then continues | `err_hmput_null_a` | [x] |
| 27 | `stbds_hmput_key` | `hash_table == NULL` — L698 | builds an 8-slot index; `nt->string.mode = (mode>=1 ? SH_DEFAULT : 0)` (L707) | `err_hmput_builds_index` | [x] |
| 28 | `stbds_hmput_key` | `used_count >= used_count_threshold` (6 of 8) — L698 | doubles `slot_count`, rehashes, frees the old index | `err_hmput_grow_threshold` | [x] |
| 29 | `stbds_hmput_key` | key already present | `temp = existing index`, **no** new element, `length` unchanged; string modes also refresh `temp_key` (L733) | `err_hmput_duplicate_key` | [x] |
| 30 | `stbds_hmput_key` | duplicate found in the **wrap-around** scan (L746-751) | `temp` set but `temp_key` **not** refreshed (C quirk: the second loop omits it) | `err_hmput_duplicate_wraparound_no_tempkey` | [x] |
| 31 | `stbds_hmput_key` | `hash_string`/`hash_bytes` returns 0 or 1 — L719 | `hash += 2` (0/1 are reserved for EMPTY/DELETED) | `err_hmput_reserved_hash_bumped` | [x] |
| 32 | `stbds_hmput_key` | a tombstone was passed before the empty slot | reuses tombstone slot, `--tombstone_count` (L766) | `err_hmput_reuses_tombstone` | [x] |
| 33 | `stbds_hmput_key` | assert L778 `(size_t) i+1 <= stbds_arrcap(a)` | `SIGABRT+msg`; **unreachable** through the public API (`arrgrowf` on L775 always yields `cap >= i+1`) → see note (C) | `err_assert_778_unreachable` (documents) | [x] |
| 34 | `stbds_hmput_key` | `table->string.mode == SH_STRDUP (2)` | key `strdup`ed, `temp_key` = the copy | `err_hmput_string_mode_strdup` | [x] |
| 35 | `stbds_hmput_key` | `table->string.mode == SH_ARENA (3)` | key copied into the arena, `temp_key` = arena pointer | `err_hmput_string_mode_arena` | [x] |
| 36 | `stbds_hmput_key` | `table->string.mode == SH_DEFAULT (1)` | key pointer **stored verbatim** (no copy) | `err_hmput_string_mode_default` | [x] |
| 37 | `stbds_hmput_key` | `table->string.mode` out of enum range (0, 4..255 via `shmode_func` truncation) — `switch` **default** L789 | falls into `memcpy(elem, key, keysize)`, i.e. the first `keysize` bytes of the key **text** are copied into the element (not a `char *`) | `err_hmput_string_mode_out_of_range` | [x] |
| 37b | `stbds_hmget_key` / `_ts` | a **string lookup** (`mode >= 1`) on a table whose `string.mode` is in that `memcpy` default branch | `stbds_is_key_equal` reads the element's first 8 bytes as a `char *` (L561) — for the key `"AAAAAAAA"` that is `0x4141414141414141` ⇒ **SIGSEGV**.  Deterministic: the address is derived from the key bytes, which are identical in both libraries | `err_memcpy_mode_lookup_segv` (child) | [x] |
| 37c | `stbds_hmdel_key` | same condition as 37b (`hm_find_slot` is shared) | **SIGSEGV** at the same place | `err_memcpy_mode_del_segv` (child) | [x] |
| 38 | `stbds_hmput_key` | `keysize == 0` (binary mode) | `hash_bytes(key,0,seed)`; `memcmp(...,0)==0` ⇒ every key compares equal ⇒ only 1 element ever inserted | `err_hmput_keysize_zero` | [x] |
| 39 | `stbds_hmput_key` | `elemsize == 0` with `keysize == 0` | degenerate but well-defined: `ARR_TO_HASH == HASH_TO_ARR == identity`, all writes 0 bytes | `err_hmput_elemsize_zero` | [x] |
| 40 | `stbds_hmput_key` | `key == NULL`, string mode | `strlen`/`hash_string` deref NULL → **SIGSEGV** | `err_hmput_null_key_string_segv` (child) | [x] |
| 41 | `stbds_hmput_key` | `mode` out of range (`-1`, `2`, `999`, `i32::MIN/MAX`) | `mode >= 1` ⇒ string hashing; `mode < 1` ⇒ binary; `string.mode` seeded as `SH_DEFAULT` for **any** `mode>=1` | `err_mode_out_of_range_put` | [x] |

## `stbds_shmode_func`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 42 | `stbds_shmode_func` | `mode` outside `{0,1,2,3}` (`-1`, `4`, `256`, `257`, `i32::MIN/MAX`) | `(unsigned char) mode` **truncates** (`256`→0, `-1`→255); no validation | `err_shmode_out_of_range` | [x] |
| 43 | `stbds_shmode_func` | `elemsize == 0` | `arrgrowf(0,0,0,1)` ⇒ 32-byte allocation only; `length = 1`; index built | `err_shmode_elemsize_zero` | [x] |

## `stbds_hmdel_key`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 44 | `stbds_hmdel_key` | `a == NULL` — L809 | returns **`NULL` (0)**; the `hmdel` macro then yields `0` | `err_hmdel_null_a` | [x] |
| 45 | `stbds_hmdel_key` | `hash_table == 0` — L816 | `temp = 0` (⇒ "nothing deleted"), returns `a` unchanged | `err_hmdel_no_table` | [x] |
| 46 | `stbds_hmdel_key` | key absent — L821 | `temp = 0`, returns `a`, `length`/`used_count` unchanged | `err_hmdel_missing_key` | [x] |
| 47 | `stbds_hmdel_key` | key present | `temp = 1`, `--used_count`, `++tombstone_count`, slot → `HASH_DELETED/INDEX_DELETED`, `--length` | `err_hmdel_present_key` | [x] |
| 48 | `stbds_hmdel_key` | assert L828 `slot < (ptrdiff_t) table->slot_count` | `SIGABRT+msg`; unreachable — `find_slot` masks `pos` with `slot_count-1` → note (C) | `err_assert_828_unreachable` (documents) | [x] |
| 49 | `stbds_hmdel_key` | `old_index == final_index` (deleting the **last** element) — L839 | skips the swap-with-last + re-find entirely | `err_hmdel_last_element_no_swap` | [x] |
| 50 | `stbds_hmdel_key` | `mode == 2` (>STRING) on a `SH_STRDUP` table — L836 exact `==` | the `strdup`ed key is **NOT** freed (leak), and L842's re-find takes the **binary** branch even though `find_slot` hashed it as a string ⇒ assert 846/849 may fire | `err_hmdel_mode_two_strdup_quirk` | [x] |
| 51 | `stbds_hmdel_key` | assert L846 `slot >= 0` | `SIGABRT+msg` — reachable via #50 (mode=2, string table, >1 element) | `err_assert_846_via_mode2` (child) | [x] |
| 52 | `stbds_hmdel_key` | assert L849 `b->index[i] == final_index` | `SIGABRT+msg` — reachable with a corrupted bucket index | `err_assert_849_corrupt_index` (child) | [x] |
| 53 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` — L854 | rebuilds the index at **half** size, frees the old one | `err_hmdel_shrink` | [x] |
| 54 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (else-branch, L858) | rebuilds at the **same** size (tombstones purged, `tombstone_count` reset to 0) | `err_hmdel_rebuild_tombstones` | [x] |
| 55 | `stbds_hmdel_key` | `keyoffset != 0` | key read at `elem + elemsize*i + keyoffset` for both the compare and the re-find.  Note `hmput_key` hardcodes `keyoffset = 0`, so a *wrong* offset simply never matches ⇒ `temp = 0` | `err_hmdel_keyoffset_nonzero` | [x] |
| 56 | `stbds_hmdel_key` | `mode` out of range (`-1`, `2`, `i32::MIN/MAX`) | `find_slot` uses `mode>=1`, but the strdup-free and the re-find use `mode == 1` exactly | `err_mode_out_of_range_del` | [x] |
| 57 | `stbds_hmdel_key` | `key == NULL`, string mode | `hash_string(NULL)` → **SIGSEGV** | `err_hmdel_null_key_segv` (child) | [x] |

## `stbds_hmfree_func`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 58 | `stbds_hmfree_func` | `a == NULL` — L573 | **no-op**, returns immediately (the only explicit NULL guard in the file) | `err_hmfree_null_a` | [x] |
| 59 | `stbds_hmfree_func` | `hash_table == NULL` — L574 | skips the key/arena cleanup, still frees `hash_table` (NULL) and the header | `err_hmfree_no_table` | [x] |
| 60 | `stbds_hmfree_func` | `string.mode == SH_STRDUP` — L575 | frees `*(char**)(a + elemsize*i)` for `i` in `1..length` (element 0 is the sentinel) | `err_hmfree_strdup_frees_keys` | [x] |
| 61 | `stbds_hmfree_func` | `string.mode != SH_STRDUP` (0/1/3/255) | does **not** free the keys; `strreset` still runs on the arena | `err_hmfree_non_strdup` | [x] |

## `stbds_stralloc` / `stbds_strreset`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 62 | `stbds_stralloc` | `len <= a->remaining` (fits) | no allocation, `remaining -= len`, pointer inside the current block | `err_stralloc_fits` | [x] |
| 63 | `stbds_stralloc` | `len > a->remaining`, `len <= blocksize` — L905 | new `512<<(block>>1)`-byte block pushed at the **head**, `remaining = blocksize` | `err_stralloc_new_block` | [x] |
| 64 | `stbds_stralloc` | `len > blocksize`, `a->storage == NULL` — L899 | dedicated block becomes the head, `next = NULL`, **`remaining` forced to 0** | `err_stralloc_oversize_empty_arena` | [x] |
| 65 | `stbds_stralloc` | `len > blocksize`, `a->storage != NULL` — L896 | dedicated block spliced in as `storage->next`; **`remaining` left untouched** | `err_stralloc_oversize_nonempty_arena` | [x] |
| 66 | `stbds_stralloc` | `a->block` at/over the max (`512<<(block>>1) >= 1<<20`) — L890 | `a->block` stops incrementing (saturates at 22, blocksize 1 MiB) | `err_stralloc_block_saturates` | [x] |
| 67 | `stbds_stralloc` | `a->block` large enough that `block>>1 >= 64` (110, 118, 120, 127, 128, 130, 140, 148, 255) | `512 << n` — C UB; x86-64 `shl` masks the count to 6 bits and Rust's `wrapping_shl` masks identically.  For `block == 255` the product is `512<<63 == 0`, so the dedicated-block path runs and `++a->block` wraps `255 → 0` | `err_stralloc_block_shift_wrap` | [x] |
| 67b | `stbds_stralloc` | `a->block == 108` ⇒ `blocksize == 512<<54 == 2^63`, `len <= blocksize` | `realloc(NULL, 8 + 2^63)` fails, then `sb->next = a->storage` writes through NULL ⇒ **SIGSEGV** (no allocation-failure check anywhere in the file) | `err_stralloc_block_realloc_fail_segv` (child) | [x] |
| 68 | `stbds_stralloc` | assert L913 `len <= a->remaining` | `SIGABRT+msg`; unreachable from a consistent arena (both branches guarantee it) → note (C) | `err_assert_913_unreachable` (documents) | [x] |
| 69 | `stbds_stralloc` | `a->storage == NULL` **and** `a->remaining >= len` (inconsistent arena) | skips the whole `if`, then `a->storage->storage` derefs NULL → **SIGSEGV** | `err_stralloc_null_storage_segv` (child) | [x] |
| 70 | `stbds_stralloc` | `str == NULL` | `strlen(NULL)` → **SIGSEGV** | `err_stralloc_null_str_segv` (child) | [x] |
| 71 | `stbds_stralloc` | `a == NULL` | `a->remaining` derefs NULL → **SIGSEGV** | `err_stralloc_null_arena_segv` (child) | [x] |
| 72 | `stbds_stralloc` | `str == ""` (`len == 1`) | still consumes 1 byte for the NUL | `err_stralloc_empty_string` | [x] |
| 73 | `stbds_strreset` | empty arena (`storage == NULL`) — L924 | loop body never runs; struct zeroed anyway | `err_strreset_empty` | [x] |
| 74 | `stbds_strreset` | `a == NULL` | `a->storage` derefs NULL → **SIGSEGV** | `err_strreset_null_segv` (child) | [x] |
| 75 | `stbds_strreset` | arena with N blocks | frees every block along `next`, then zeroes all 24 bytes (`storage`,`remaining`,`block`,`mode`) | `err_strreset_multi_block` | [x] |

## `stbds_make_hash_index` (static, reached through the public API)

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 76 | `stbds_make_hash_index` | `slot_count <= STBDS_BUCKET_LENGTH (8)` — L399 | `used_count_shrink_threshold` forced to **0** (so an 8-slot table never shrinks) | `err_make_index_no_shrink_at_8` | [x] |
| 77 | `stbds_make_hash_index` | assert L401 `uct + tct < slot_count` | `SIGABRT+msg`; fires for `slot_count ∈ {0,1,2,3}` and for non-power-of-two counts. Reachable by corrupting `table->slot_count` to 1 and forcing a grow ⇒ `make_hash_index(2)` | `err_assert_401_slot_count_2` (child) | [x] |
| 78 | `stbds_make_hash_index` | `ot != NULL` | inherits `string` **and** `seed` from the old table; the global `stbds_hash_seed` is **not** advanced | `err_make_index_inherits_seed` | [x] |
| 79 | `stbds_make_hash_index` | `ot == NULL` | `string` zeroed, `seed = stbds_hash_seed`, then `stbds_hash_seed = seed*A + B` (LCG advance) | `err_make_index_advances_global_seed` | [x] |

## `stbds_hm_find_slot` (static)

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 80 | `stbds_hm_find_slot` | first EMPTY (`hash == 0`) hit in the forward scan — L609 | `return -1` ⇒ "absent" | covered by #18/#46 | [x] |
| 81 | `stbds_hm_find_slot` | first EMPTY hit in the wrap-around scan — L620 | `return -1` | `err_find_slot_wraparound_miss` | [x] |
| 82 | `stbds_hm_find_slot` | **no** EMPTY slot anywhere (all in use / tombstones) | **infinite loop** — there is no termination check.  Not reachable through the public API because `used_count + tombstone_count < slot_count` is enforced by the thresholds (assert L401) | note (D) | [x] |
| 83 | `stbds_hm_find_slot` | `hash < 2` — L596 | `hash += 2`; must match `hmput_key`'s identical bump or lookups would miss | `err_hmput_reserved_hash_bumped` | [x] |

## `strkey` / `str_dups`

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 84 | `strkey` | `n == INT_MIN`, `INT_MAX`, `-1`, `0` | `sprintf(buffer,"test_%d",n)`; longest output `test_-2147483648` = 17 bytes ≤ 256 ⇒ never overflows; returns the **same static** buffer every call | `err_strkey_extremes` | [x] |
| 85 | `strkey` | called twice | second call **overwrites** the first result (shared static buffer) | `err_strkey_shared_buffer` | [x] |
| 86 | `str_dups` | `num <= 0` (`0`, `-1`, `INT_MIN`) — L952 | arena loop skipped; the `sh_new_strdup`/`shputs` block still runs; prints `a <num>` | `err_str_dups_non_positive` | [x] |
| 87 | `str_dups` | asserts L960/961/962 | always hold (`strdup`ed key differs from `s.key`, first byte `'a'`, value preserved) ⇒ never abort → note (C) | `err_str_dups_asserts_hold` | [x] |

## Generic C-API boundaries (required even though the C has no explicit check)

| # | function | trigger | expected C result | test | ok |
|---|----------|---------|-------------------|------|----|
| 88 | `hash_bytes`, `hmfree_func`, `hmdel_key`, `arrgrowf`, `rand_seed` | **NULL pointers** in every argument position the C tolerates | `hash_bytes(NULL,0,s)` → seed avalanche; `hmfree_func(NULL,_)` → no-op; `hmdel_key(NULL,…)` → `NULL`; `arrgrowf(NULL,_,0,0)` → `NULL`; `rand_seed(0)` / `rand_seed(SIZE_MAX)` accepted | `err_generic_null_pointers` | [x] |
| 89 | `hash_bytes`, `hmput_default`, `hmfree_func` | **zero and oversized lengths**: `len ∈ {0,1,7,8,9,4095,4096}`, `elemsize ∈ {0,1,8,16}` | identical hashes; `hmfree_func` with `elemsize == 0` still releases the header (the key loop is skipped because `string.mode != SH_STRDUP`) | `err_generic_zero_and_oversized_lengths` | [x] |
| 90 | `hmget_key_ts`, `shmode_func` | **one step past each documented enum range**: `mode ∈ {-1,0,1,2}` (valid `{0,1}`) and `shmode ∈ {-1,0,1,2,3,4}` (valid `{0,1,2,3}`) | no validation: `mode` is only ever compared with `>=`/`==`, `shmode` is truncated to `unsigned char`; both libraries take the identical branch | `err_generic_one_past_valid_enum` | [x] |
| 91 | all | **ABI layout across the FFI boundary**: header field offsets (`length` -32, `capacity` -24, `hash_table` -16, `temp` -8) and `stbds_hash_index` offsets (`string` +72, `string.mode` +89, `storage` +96) derived from live `.so` behaviour | byte-identical layout, verified for `elemsize ∈ {8,16,24}` and `shmode ∈ {0..3}` | `abi_layout_matches_c` | [x] |

## Notes

**(A) Out-of-range enum values.** `mode` is a plain `int` in every signature and
is compared with `>=` (`stbds_is_key_equal`, `hm_find_slot`, `hmput_key`) but
with `==` in `hmdel_key` (L836, L842).  `stbds_shmode_func` truncates `mode` to
`unsigned char`.  Nothing is validated, so **every** `int` is a legal input; the
tests above pass `-1`, `2`, `4`, `7`, `255`, `256`, `257`, `999`, `i32::MIN` and
`i32::MAX` through all four entry points.

**(B) Sentinels.** `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`.  `hmdel_key` uses
`temp = 0/1` as its *boolean* result, which collides with `temp` being an
*index* after `hmget_key` — reproduced verbatim.

**(C) Unreachable asserts.** Rows 33 (L778), 48 (L828), 68 (L913) and 87
(L960-962) are tautologies given the code that precedes them; their tests assert
that neither library aborts on the nearest reachable boundary, and that the
assertion *strings* are byte-identical in both binaries (checked by
`check_symbols.sh` / `SYMBOLS.md`).  Rows 51, 52 and 77 *are* reachable (via a
`mode == 2` delete on a string table, and via a hand-corrupted `slot_count`) and
are exercised in child processes comparing exit signal + message.

**(D) Non-terminating input.** Row 82 is a genuine C hang, not an error return.
It is unreachable through the public API; no test drives it (it would hang the
suite for both libraries alike).

**(E) Crash-equivalence testing.** Rows marked *(child)* re-execute the test
binary for one `#[ignore]`d `child_*` case, once per library
(`DIFFTEST_CHILD_LIB=c` / `=rust`), and compare the terminating **signal**, the
**exit code** and the normalised `assert()` **diagnostic**
(`tests/harness/mod.rs::assert_same_crash`).  "Both crashed somehow" is never
accepted.  Measured outcomes:

| child case | C | Rust |
|---|---|---|
| `child_assert_401` | SIGABRT + `lib.c:401: stbds_make_hash_index: Assertion \`t->used_count_threshold + t->tombstone_count_threshold < t->slot_count' failed.` | identical |
| `child_assert_846_mode2` | SIGABRT + `lib.c:846: stbds_hmdel_key: Assertion \`slot >= 0' failed.` | identical |
| `child_assert_849_corrupt` | SIGABRT + `lib.c:849: stbds_hmdel_key: Assertion \`b->index[i] == final_index' failed.` | identical |
| `child_arrgrowf_oom`, `child_arrfreef_null`, `child_hash_string_null`, `child_hash_bytes_null`, `child_hmget_ts_null_temp`, `child_hmput_null_key_string`, `child_hmdel_null_key`, `child_memcpy_mode_lookup`, `child_memcpy_mode_del`, `child_stralloc_null_storage`, `child_stralloc_null_str`, `child_stralloc_null_arena`, `child_strreset_null`, `child_stralloc_block_realloc_fail` | SIGSEGV (139) | SIGSEGV (139) |

**(E2) Build profile.** Exact *signal* parity requires the crate's
`[profile.release]` (`debug-assertions = false`, `panic = "abort"`), which is
what `run_tests.sh` builds and what the tests load by default.  A `dev`-profile
`.so` additionally enables Rust's runtime null-pointer-dereference check, which
turns the C's `SIGSEGV` into a non-unwinding panic (`SIGABRT`); the harness
detects that (`rust_so_is_debug()`), buckets fatal signals together and prints a
note, while still comparing the `assert()` diagnostics exactly.  The suite is run
against **both** profiles: the `dev` `.so` also has `overflow-checks = true`, so
a clean run there proves no arithmetic in the translation overflows where the C
wraps.

**(F) `__FILE__`.** `assert()` embeds the compile-time path.  The C `.so` holds
the absolute cmake path; the Rust translation holds `src/lib.c`.  The comparison
strips everything up to and including the last `/` of the file component, and
then requires `<line>: <func>: Assertion \`<expr>' failed.` to match byte for
byte.
