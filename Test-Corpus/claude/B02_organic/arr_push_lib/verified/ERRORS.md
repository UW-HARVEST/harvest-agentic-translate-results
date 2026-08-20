# ERRORS.md — error / rejection surface table (Phase C)

Mechanically derived from `c_src/src/lib.c`. Every distinct rejection, early
return, sentinel, `STBDS_ASSERT` and boundary check gets one row.

`STBDS_ASSERT` is `assert()` and the CMake build defines **no** `NDEBUG`
(`CMAKE_BUILD_TYPE` is empty, `C_FLAGS = -fPIC`), so **asserts are live** and
fire as `__assert_fail` -> `SIGABRT`. The Rust port calls libc `abort()`
-> `SIGABRT`.

Rows whose expected result is a fatal signal are therefore compared by the
*wait status* of a `fork()`ed child (`common::fork_run` / `common::assert_same_fate`,
driven from `tests/errors_fatal.rs`); the `assert()` message text legitimately
differs (it names the C file and line) so only the signal is compared.

Those fatal rows are compared against the **release** Rust `.so`. The C is built
with neither `-DNDEBUG` nor `-O`, i.e. uninstrumented, and the Rust release
profile matches; the Rust *dev* profile enables `debug_assertions`, which insert
MIR null-pointer-dereference checks that convert a C `SIGSEGV` into a
non-unwinding panic (`SIGABRT`). That difference is itself asserted by
`errors_fatal.rs::dev_build_only_traps_the_same_ub`.

All non-fatal rows are compared against the **dev** Rust `.so`, whose
arithmetic-overflow checks are additionally active — so no row silently relies on
release-mode wrapping.

Sentinels used below: `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`, `STBDS_HM_BINARY = 0`,
`STBDS_HM_STRING = 1`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1 | `stbds_arrgrowf` (`lib.c:286`) | `min_cap <= stbds_arrcap(a)` — request that already fits (incl. `a=NULL, elemsize=X, addlen=0, min_cap=0`) | returns `a` **unchanged** (same pointer, incl. `NULL`); no allocation, no header write | `errors.rs::err01_arrgrowf_no_grow_returns_same_ptr` | [x] |
| 2 | `stbds_arrgrowf` (`lib.c:280`) | `a=NULL` + huge `addlen`/`min_cap` such that `elemsize*min_cap + 32` wraps/OOMs → `realloc` returns `NULL` | `b = NULL + 32` then `stbds_header(b)->length = 0` → write to address `0x0` → `SIGSEGV` | `errors_fatal.rs::fatal_error_paths` (row 2) | [x] |
| 3 | `stbds_arrgrowf` (`lib.c:289-292`) | `min_cap` in `(arrcap, 4)` with `arrcap == 0` → neither `min_cap < 2*cap` nor... : boundary of the growth policy (`min_cap=1,2,3` → 4; `min_cap=5` with cap 4 → 8) | returns array with `capacity` = `max(min_len, min_cap, 2*cap, 4)` per the exact C ladder | `errors.rs::err03_arrgrowf_growth_ladder` | [x] |
| 4  | `stbds_arrfreef` (`lib.c:312-315`) | `a = NULL` | `free((char*)NULL - 32)` = `free(0xffff…ffe0)`. **Observed: `SIGSEGV`** — glibc reads the chunk header below that address and faults before it can raise its "free(): invalid pointer" abort. Compared as "same fatal signal" | `errors_fatal.rs` row 4 | [x] |
| 5 | `stbds_make_hash_index` (`lib.c:401`) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count <= 2` (`slot_count=2` → `2+0 < 2` false) | `assert` → `SIGABRT`. Unreachable from the public API (callers only pass `8`, `slot_count*2`, `slot_count>>1` with `slot_count>8`) — documented, reachability proven by construction below | `errors.rs::err05_make_hash_index_assert_unreachable` | [x] |
| 6 | `stbds_hmfree_func` (`lib.c:573`) | `a = NULL` | returns immediately, no free, no crash | `errors.rs::err06_hmfree_null_is_noop` | [x] |
| 7 | `stbds_hmfree_func` (`lib.c:574`) | `a != NULL` but `stbds_header(a)->hash_table == NULL` (array built by `arrgrowf` only, or by `hmput_default`) | skips the strdup-loop and `strreset`, still `free(hash_table)` (= `free(NULL)`, a no-op) and `free(header)` | `errors.rs::err07_hmfree_no_hash_table` | [x] |
| 8 | `stbds_hm_find_slot` (`lib.c:609-611`) | probe walks into a slot with `bucket->hash[i] == STBDS_HASH_EMPTY` before finding the key (key absent, forward half of the bucket) | returns `-1` | `errors.rs::err08_09_find_slot_miss_both_halves` | [x] |
| 9 | `stbds_hm_find_slot` (`lib.c:620-622`) | same, but the empty slot is found in the *wrap-around* half (`i < limit`, i.e. `pos & 7 != 0`) | returns `-1` | `errors.rs::err08_09_find_slot_miss_both_halves` | [x] |
| 10 | `stbds_hmget_key_ts` (`lib.c:634-639`) | `a = NULL` | allocates a 1-element array (`arrgrowf(0,elemsize,0,1)`), `length = 1`, element zeroed, `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` | `errors.rs::err10_hmget_ts_null_table` | [x] |
| 11 | `stbds_hmget_key_ts` (`lib.c:644-645`) | `a != NULL` but `hash_table == NULL` (e.g. from `hmput_default`) | `*temp = -1`, returns `a` unchanged, key never hashed | `errors.rs::err11_hmget_ts_no_hash_table` | [x] |
| 12 | `stbds_hmget_key_ts` (`lib.c:648-649`) | key not present in a populated table | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | `errors.rs::err12_hmget_ts_key_absent` | [x] |
| 13 | `stbds_hmget_key` (`lib.c:659-665`) | `a = NULL` | delegates to `_ts`; then writes `stbds_temp(p-elemsize) = -1` into the freshly allocated header. Returns `arr+elemsize` with `header->temp == -1` | `errors.rs::err13_hmget_key_null` | [x] |
| 14 | `stbds_hmget_key` | key absent (populated table) | returns `a`, `header->temp == -1` | `errors.rs::err14_hmget_key_absent_temp` | [x] |
| 15 | `stbds_hmput_default` (`lib.c:669`) | `a = NULL` | allocates, `length = 1`, element zeroed, returns `arr+elemsize` | `errors.rs::err15_16_17_hmput_default` | [x] |
| 16 | `stbds_hmput_default` (`lib.c:669`) | `a != NULL` but `stbds_header(a-elemsize)->length == 0` (array grown but empty) | re-grows in place, `length += 1` (→1), zeroes elem 0, returns `arr+elemsize` | `errors.rs::err15_16_17_hmput_default` | [x] |
| 17 | `stbds_hmput_default` (`lib.c:675`) | `a != NULL`, `length != 0` | returns `a` **unchanged** (identical pointer), nothing written | `errors.rs::err15_16_17_hmput_default` | [x] |
| 18 | `stbds_hmput_key` (`lib.c:686-691`) | `a = NULL` | bootstraps a fresh 1-element array before inserting | `errors.rs::err18_19_20_hmput_key_bootstrap_and_grow` | [x] |
| 19 | `stbds_hmput_key` (`lib.c:698`) | `table == NULL` | fresh index with `slot_count = STBDS_BUCKET_LENGTH (8)`; `nt->string.mode = (mode >= 1 ? STBDS_SH_DEFAULT : 0)` | `errors.rs::err18_19_20_hmput_key_bootstrap_and_grow` | [x] |
| 20 | `stbds_hmput_key` (`lib.c:698`) | `table->used_count >= table->used_count_threshold` (`6` for 8 slots) → rehash/double | `slot_count *= 2`, old index freed, all live entries reinserted | `errors.rs::err18_19_20_hmput_key_bootstrap_and_grow` | [x] |
| 21 | `stbds_hmput_key` (`lib.c:778`) | `(size_t)i+1 > stbds_arrcap(a)` still true after `arrgrowf` (only possible if `arrgrowf` failed to grow) | `assert` → `SIGABRT`. Unreachable via the public API; documented | `errors.rs::err21_27_28_hmdel_and_hmput_invariant_asserts` | [x] |
| 22 | `stbds_hmput_key` (`lib.c:729-735`) | key **already present** (duplicate put) | no new slot, `header->temp = existing index`, `used_count` unchanged, `length` unchanged; returns same logical pointer | `errors.rs::err22_hmput_key_duplicate` | [x] |
| 23 | `stbds_hmput_key` (`lib.c:739-741`, `766-769`) | a tombstoned slot (`hash==1`, `index==-2`) is passed before the empty slot | insert reuses the tombstone: `pos = tombstone`, `--tombstone_count` | `errors.rs::err23_hmput_key_reuses_tombstone` | [x] |
| 24 | `stbds_hmdel_key` (`lib.c:809-810`) | `a = NULL` | returns `0` (`NULL`) | `errors.rs::err24_hmdel_null_returns_null` | [x] |
| 25 | `stbds_hmdel_key` (`lib.c:815-817`) | `hash_table == NULL` | `stbds_temp(a-elemsize) = 0`, returns `a` | `errors.rs::err25_hmdel_no_hash_table` | [x] |
| 26 | `stbds_hmdel_key` (`lib.c:821-822`) | key absent | `header->temp` stays `0` (the "nothing deleted" sentinel the `hmdel` macro yields), returns `a`, `length`/`used_count` unchanged | `errors.rs::err26_hmdel_key_absent` | [x] |
| 27 | `stbds_hmdel_key` (`lib.c:828`) | `slot >= (ptrdiff_t)table->slot_count` | `assert` → `SIGABRT`. Unreachable: `stbds_hm_find_slot` masks `pos` with `slot_count-1`; documented | `errors.rs::err21_27_28_hmdel_and_hmput_invariant_asserts` | [x] |
| 28 | `stbds_hmdel_key` (`lib.c:832`) | `table->used_count >= 0` — `used_count` is `size_t`, so this is a tautology and can never fire | never aborts (also verified for the `used_count == 0` underflow case, which wraps instead of asserting) | `errors.rs::err21_27_28_hmdel_and_hmput_invariant_asserts` | [x] |
| 29 | `stbds_hmdel_key` (`lib.c:846`) | after the swap-with-last `memmove`, re-lookup of the moved key returns `slot < 0`. **Reachable** with any `mode >= 2` deleting a NON-last element: the re-lookup branch tests `mode == STBDS_HM_STRING` (`lib.c:842`) and so passes `&elem.key` (the *address* of the key pointer), while `stbds_hm_find_slot` tests `mode >= STBDS_HM_STRING` and hashes those pointer bytes *as a string* — a different hash, so the slot is never found | `assert` → `SIGABRT` | `errors_fatal.rs::fatal_error_paths` (rows 29/30) | [x] |
| 30 | `stbds_hmdel_key` (`lib.c:849`) | `b->index[i] != final_index` after a re-lookup that *did* succeed. Same `mode >= 2` class; the `slot >= 0` assert (row 29) fires first on every input we can construct, so row 30 is dominated by row 29 and never observed independently — proven by enumerating the branch | `assert` → `SIGABRT` (dominated by row 29) | `errors_fatal.rs::fatal_error_paths` (rows 29/30) | [x] |
| 31 | `stbds_hmdel_key` (`lib.c:836`) | `mode == STBDS_HM_STRING` **exactly** (not `>= 1`) and `string.mode == STBDS_SH_STRDUP` | frees the strdup'd key. With `mode = 2` (`STBDS_HM_PTR_TO_STRING`) the string compare still happens (`>= 1`) but the key is **not** freed — a leak the C has and the Rust must reproduce | `errors.rs::err31_hmdel_mode_exactly_1_frees`, `map_string.rs::cfg52_*` | [x] |
| 31b | `stbds_hmdel_key` (`lib.c:836`) | the *leak* half of row 31: with `mode >= 2` the `stbds_strdup`'d key is never freed. A leak has no return value, so it is detected through the allocator — glibc's tcache is LIFO per size class, so a freed 101-byte chunk is handed straight back to the next `stbds_strdup` of that length, and a leaked one is not | `mode == 1`: chunk recycled. `mode >= 2`: **not** recycled | `errors.rs::err31b_hmdel_strdup_free_only_for_mode_1` | [x] |
| 32 | `stbds_hmdel_key` (`lib.c:854`) | `used_count < used_count_shrink_threshold && slot_count > 8` after a delete | index shrinks to `slot_count >> 1` | `errors.rs::err32_33_hmdel_shrink_and_rebuild` | [x] |
| 33 | `stbds_hmdel_key` (`lib.c:858`) | `tombstone_count > tombstone_count_threshold` (`(sc>>3)+(sc>>4)`) and shrink did not trigger | index rebuilt at the same `slot_count` | `errors.rs::err32_33_hmdel_shrink_and_rebuild` | [x] |
| 34a | `stbds_stralloc` (`lib.c:913`) | `STBDS_ASSERT(len <= a->remaining)`. **Unreachable**: if `len <= remaining` on entry the `if` is skipped and the assert is trivially true; if `len > remaining` then either `len > blocksize` (the oversize branch `return`s before the assert) or `len <= blocksize` and `remaining` is set to `blocksize >= len`. Proven by exhaustive enumeration of the three paths | never fires | `errors2.rs::err34a_stralloc_remaining_assert_unreachable` | [x] |
| 34b | `stbds_stralloc` (`lib.c:914`) | the *reachable* failure at the same spot: an arena with `remaining > 0` but `storage == NULL` takes the fast path and dereferences `a->storage->storage` | `SIGSEGV` (NULL + 8 deref) | `errors_fatal.rs::fatal_error_paths` (row 34b) | [x] |
| 35 | `stbds_stralloc` (`lib.c:885`) | `len <= a->remaining` (fast path) | no allocation: `p = storage->storage + remaining - len`, `remaining -= len` | `errors2.rs::err35_stralloc_fast_path` | [x] |
| 36 | `stbds_stralloc` (`lib.c:893-904`) | `len > blocksize` (oversize string) **and** `a->storage == NULL` | dedicated block, `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`; returns `sb->storage` | `errors2.rs::err36_stralloc_oversize_no_storage` | [x] |
| 37 | `stbds_stralloc` (`lib.c:896-898`) | `len > blocksize` **and** `a->storage != NULL` | block spliced in *after* the head (`sb->next = storage->next; storage->next = sb`), `a->remaining` left untouched | `errors2.rs::err37_stralloc_oversize_with_storage` | [x] |
| 38a | `stbds_stralloc` (`lib.c:890-891`) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`, i.e. `a->block >= 22` | `a->block` is **not** incremented (saturates at 23). Tested for `block` 0..=30 and 110..=127 (the latter: `512 << 55` wraps to 0, which is well defined for unsigned, so the oversize path is taken) | `errors2.rs::err38_stralloc_block_saturates` | [x] |
| 38b | `stbds_stralloc` (`lib.c:894`/`906`) | `a->block` in `44..=109` → `blocksize` in `2^40..2^63` → the `STBDS_REALLOC` fails → `sb->next = a->storage` writes through `NULL` | `SIGSEGV` | `errors_fatal.rs::fatal_error_paths` (row 38-fatal) | [x] |
| 39 | `stbds_stralloc` | `str = ""` (empty string, `len == 1`) | works: 1 byte consumed | `errors2.rs::err39_stralloc_empty_string` | [x] |
| 40 | `stbds_strreset` (`lib.c:920-930`) | `a->storage == NULL` (already-empty / zeroed arena) | loop body never runs, arena memset to 0 | `errors2.rs::err40_strreset_empty` | [x] |
| 41 | `stbds_hash_bytes` (`lib.c:531-541`) | `len = 0` (`p` may even be a dangling/non-null pointer — nothing is read) | `data = 0 << 56 = 0`, `switch(0)` → `break`; a well-defined hash value | `errors2.rs::err41_hash_bytes_len0` | [x] |
| 42 | `stbds_hash_bytes` (`lib.c:522-530`) | `len` not a multiple of 8 → the fall-through `switch (len - i)` cases `7..1`, each of which OR-s a *sign-extended* `int` for cases `4`,`3`,`2` (`d[3]<<24` overflows `int`) | value-dependent hash; must match bit-for-bit incl. the sign-extension quirk | `errors2.rs::err42_hash_bytes_tail_signext`, `hash.rs::cfg_exhaustive_*` | [x] |
| 43 | `stbds_hash_string` (`lib.c:477-491`) | `str = ""` (immediately `\0`) | `hash = seed` then the avalanche; deterministic value | `errors2.rs::err43_hash_string_empty` | [x] |
| 44 | `stbds_hash_string` (`lib.c:481`) | bytes `>= 0x80` — read via `(unsigned char) *str++`, i.e. **not** sign-extended even though `char` is signed on x86-64 | must match | `errors2.rs::err44_hash_string_high_bytes` | [x] |
| 45 | `stbds_hm_find_slot` / `stbds_hmput_key` (`lib.c:596`, `719`) | the computed hash is `0` or `1` (would collide with `STBDS_HASH_EMPTY`/`STBDS_HASH_DELETED`) | `hash += 2` | `errors2.rs::err45_hash_lt2_bumped` | [x] |
| 46 | out-of-range `mode` enum across FFI | `mode < 0` (e.g. `-1`, `INT_MIN`) — C `int` accepts any value; `mode >= STBDS_HM_STRING` is false | treated as **binary** mode everywhere (`memcmp` key compare, `hash_bytes`, `nt->string.mode = 0`) | `errors2.rs::err46_mode_negative_is_binary` | [x] |
| 47 | out-of-range `mode` enum across FFI | `mode > 1` (e.g. `2 = PTR_TO_STRING`, `1000`, `INT_MAX`) | treated as **string** mode by every `>= STBDS_HM_STRING` test, but *not* by the `== STBDS_HM_STRING` test in `hmdel_key` (row 31) | `errors2.rs::err47_mode_gt1_is_string` | [x] |
| 48 | out-of-range `mode` enum for `stbds_shmode_func` (`lib.c:803`) | `mode` outside `{0,1,2,3}` — stored as `(unsigned char) mode`, so `256` → `0`, `-1` → `255` | `h->string.mode` is the truncated byte; the later `switch (table->string.mode)` in `hmput_key` hits `default:` → `memcpy(key, keysize)` | `errors2.rs::err48_49_shmode_func_out_of_range` | [x] |
| 49 | `stbds_hmput_key` `switch (table->string.mode)` `default:` (`lib.c:789`) | `string.mode` not in `{STRDUP, ARENA, DEFAULT}` (e.g. `SH_NONE`, or a truncated garbage byte) while `mode >= STBDS_HM_STRING` | `memcpy(a + elemsize*i, key, keysize)` — copies `keysize` bytes of the *pointer*, not a strdup | `errors2.rs::err48_49_shmode_func_out_of_range` | [x] |
| 50 | `keysize = 0` | `stbds_hash_bytes(key, 0, seed)` + `memcmp(...,0) == 0` (always true) | every binary-mode key compares equal ⇒ all keys collapse onto one entry | `errors2.rs::err50_keysize_zero` | [x] |
| 51 | `keyoffset != 0` for `stbds_hmdel_key` | valid non-zero `keyoffset` matching a struct whose key is not first — but `hmput_key`/`hmget_key` hard-code `keyoffset = 0` | asymmetry is part of C behaviour: delete looks at `a + elemsize*i + keyoffset` | `errors2.rs::err51_hmdel_keyoffset_nonzero` | [x] |
| 52 | `arr_push` (`lib.c:950-955`) | `num <= 0` (`0`, `-1`, `INT_MIN`) | the `STBDS_ASSERT(arrlen(NULL)==0)` passes, the `for` body never runs, nothing allocated, returns | `errors2.rs::err52_53_arr_push_boundaries` | [x] |
| 53 | `arr_push` | `num` in `1..=50` → exactly one outer iteration with `i == 0`, inner loop empty, `arrfree(NULL)` | no allocation at all (the `(a) ? free : (void)0` guard) | `errors2.rs::err52_53_arr_push_boundaries` | [x] |
| 54 | `strkey` (`lib.c:939-943`) | `n = INT_MIN` → `"test_-2147483648"` (16 chars + NUL, fits the 256-byte static) | same bytes in the static buffer, same returned pointer target contents | `errors2.rs::err54_strkey_int_min` | [x] |
| 55 | `stbds_hmget_key_ts` | `temp` output pointer aliasing / value on the `a == NULL` path is written **before** the return | `*temp` is `-1`; verified the callee writes through the caller's pointer | `errors.rs::err55_hmget_ts_temp_written_through_caller_pointer` | [x] |
| 56 | `stbds_hmfree_func` (`lib.c:575-579`) | `string.mode == STBDS_SH_STRDUP` → frees `*(char**)(a + elemsize*i)` for `i` in `1..length`. If `length <= 1` the loop is empty | double-free safety: index 0 (the default slot) is never freed | `errors_fatal.rs::fatal_error_paths` (row 56) | [x] |
| 57 | `stbds_hmdel_key` | delete the **last** element (`old_index == final_index`) | the `memmove`/re-lookup/`assert` block is skipped entirely | `errors.rs::err57_hmdel_last_element` | [x] |
| 58 | `stbds_hmdel_key` | delete from a table with exactly one live entry, then delete again (key now absent) | second call: `temp = 0`, `length` unchanged | `errors.rs::err58_hmdel_twice` | [x] |
