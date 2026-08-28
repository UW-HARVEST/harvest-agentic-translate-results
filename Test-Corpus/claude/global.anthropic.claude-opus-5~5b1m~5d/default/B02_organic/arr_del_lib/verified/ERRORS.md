# ERRORS.md — Error / rejection surface table (Phase A, gated in Phase C)

Derived mechanically from `c_src/src/lib.c`.  Every `return -1`, `return 0`,
`return a` early-out, every `STBDS_ASSERT`, every explicit `== NULL` /
`< 0` / range check, and every min/max constant is listed.

The C library is built **without** `NDEBUG` (`C_FLAGS = -fPIC` only), so
`assert()` is live in the C `.so`.

`E` = exercised by an error-path differential test in
`tests/errors.rs` (or `tests/crash_parity.rs` for the rows whose C behaviour is
a fault/abort).

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after `min_len` clamp (i.e. no growth needed) | `return a;` — pointer returned unchanged, header untouched, **no** realloc | [x] |
| 2 | `stbds_arrgrowf` | `a == NULL` (null array) | not an error: allocates, `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap,4)` | [x] |
| 3 | `stbds_arrgrowf` | `addlen` so large that `stbds_arrlen(a)+addlen` wraps `size_t` | wraps; `min_len` becomes small, so the `min_cap <= arrcap` early-out can trigger and `a` is returned unchanged | [x] |
| 4 | `stbds_arrgrowf` | `elemsize*min_cap + sizeof(header)` un-allocatable (`realloc` returns `NULL`) | `b = NULL + 32`; store to `stbds_header(b)->length` faults → **SIGSEGV** | [x] |
| 5 | `stbds_arrfreef` | `a == NULL` | `free((char*)NULL - 32)` → glibc "free(): invalid pointer" → **SIGABRT** | [x] |
| 6 | `stbds_hmfree_func` | `a == NULL` | `return;` — no-op, nothing freed | [x] |
| 7 | `stbds_hmfree_func` | `a != NULL`, `stbds_header(a)->hash_table == NULL` | skips the strdup/arena cleanup, `free(NULL)` then frees the header | [x] |
| 8 | `stbds_hm_find_slot` | key absent; probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` in the **forward** inner loop | `return -1` | [x] |
| 9 | `stbds_hm_find_slot` | key absent; probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` in the **wrap-around** (`i < limit`) inner loop | `return -1` | [x] |
| 10 | `stbds_hmget_key_ts` | `a == NULL` | `*temp = STBDS_INDEX_EMPTY (-1)`; returns a **new** 1-element zeroed array (hash pointer), i.e. it allocates rather than failing | [x] |
| 11 | `stbds_hmget_key_ts` | `a != NULL` but `stbds_header(a-elemsize)->hash_table == NULL` | `*temp = -1`; returns `a` unchanged; **no** allocation | [x] |
| 12 | `stbds_hmget_key_ts` | key absent (`stbds_hm_find_slot` returned `< 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`; returns `a` | [x] |
| 13 | `stbds_hmget_key` | `a == NULL` | `stbds_temp(...) = -1` written into the freshly allocated header; returns hash pointer | [x] |
| 14 | `stbds_hmget_key` | key absent | `stbds_header(a-elemsize)->temp = -1` | [x] |
| 15 | `stbds_hmget_key_ts` | `temp == NULL` (null out-param) | `*temp = ...` faults → **SIGSEGV** | [x] |
| 16 | `stbds_hmdel_key` | `a == NULL` | `return 0;` → returns **NULL** | [x] |
| 17 | `stbds_hmdel_key` | `hash_table == NULL` | `stbds_temp(raw_a) = 0`; `return a;` — length unchanged | [x] |
| 18 | `stbds_hmdel_key` | key absent (`slot < 0`) | `stbds_temp(raw_a) = 0`; `return a;` — length unchanged, `used_count` unchanged | [x] |
| 19 | `stbds_hmdel_key` | key present | `stbds_temp(raw_a) = 1`; slot → `HASH_DELETED`/`INDEX_DELETED`; `length -= 1` | [x] |
| 20 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | unreachable: `stbds_hm_find_slot` masks `pos` with `slot_count-1` | n/a |
| 21 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t`, always true — dead assert | n/a |
| 22 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` after relocating the final element | **SIGABRT**. Reachable two ways: (a) the relocated element's key no longer hashes to a live slot (map corrupted by the caller), (b) `mode >= 2` — then the re-find takes the `else` branch and passes the *address* of the element while `stbds_hm_find_slot` hashes it as a string. Both are covered in `tests/crash_parity.rs` | [x] |
| 23 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` | unreachable for a map built only through the public API | n/a |
| 24 | `stbds_hmdel_key` | deleting the **only** real element so `old_index == final_index` | no `memmove`, no re-find; `length -= 1` | [x] |
| 25 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **and** `table->string.mode != STBDS_SH_STRDUP` | stored key pointer is **not** freed | [x] |
| 26 | `stbds_hmdel_key` | `mode != STBDS_HM_STRING` (e.g. `mode == 2`) on a strdup table | key **not** freed even though the table is `STBDS_SH_STRDUP` (`==` not `>=`) | [x] |
| 27 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | for `slot_count == 8`: `6 + 1 < 8` holds. Unreachable failure: the only call sites pass `8`, `slot_count*2` or `slot_count>>1` (guarded by `slot_count > 8`) | n/a |
| 28 | `stbds_hash_string` | `str == NULL` | `while (*str)` derefs NULL → **SIGSEGV** | [x] |
| 29 | `stbds_hash_string` | `str == ""` (empty, min length) | loop body never runs; well-defined result = avalanche of `seed` | [x] |
| 30 | `stbds_hash_bytes` | `len == 0` (and `p == NULL`) | no byte is read; well-defined result | [x] |
| 31 | `stbds_hash_bytes` | `p == NULL`, `len > 0` | derefs NULL → **SIGSEGV** | [x] |
| 32 | `stbds_is_key_equal` | `mode >= STBDS_HM_STRING` and the *stored* key pointer is `NULL` (slot 0 / default element) | `strcmp(key, NULL)` → **SIGSEGV** | [x] |
| 33 | `stbds_stralloc` | `a->remaining >= len` but `a->storage == NULL` | `a->storage->storage` derefs NULL → **SIGSEGV** | [x] |
| 34 | `stbds_stralloc` | `STBDS_ASSERT(len <= a->remaining)` | unreachable: either the early `return sb->storage` fires or `remaining` was just set to `blocksize >= len` | n/a |
| 35 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` (huge string, `> 512<<(block>>1)`) | dedicated over-sized block spliced in; `remaining` **not** changed when `a->storage != NULL`; returns `sb->storage` | [x] |
| 36 | `stbds_stralloc` | `blocksize` already at `STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` is **not** incremented (saturates at 22) | [x] |
| 37 | `stbds_stralloc` | `str == NULL` | `strlen(NULL)` → **SIGSEGV** | [x] |
| 38 | `stbds_strreset` | `a->storage == NULL` (fresh/empty arena) | loop does not run; arena memset to 0 | [x] |
| 39 | `stbds_strreset` | `a == NULL` | `a->storage` derefs NULL → **SIGSEGV** | [x] |
| 40 | `stbds_shmode_func` | `mode` outside the `STBDS_SH_*` enum (`4..=255`, `256`, `-1`, `INT_MIN`) | `(unsigned char) mode` is stored verbatim in `string.mode`; later `switch` falls to `default:` (raw `memcpy` of the key) | [x] |
| 41 | `stbds_hmput_key` | `mode` outside `{0,1}` but `>= 1` (`2`, `7`, `INT_MAX`) | treated as **string** mode (`mode >= STBDS_HM_STRING`) | [x] |
| 42 | `stbds_hmput_key` | `mode < 0` (`-1`, `INT_MIN`) | treated as **binary** mode | [x] |
| 43 | `stbds_hmget_key` / `stbds_hmdel_key` | `mode` out of range as in 41/42 | same `>= 1` / `< 1` split inside `stbds_hm_find_slot` / `stbds_is_key_equal` | [x] |
| 44 | `stbds_hmput_key` | `keysize == 0` | `memcmp(...,0) == 0` always true → every key collides with the first key that lands in a slot with the same hash | [x] |
| 45 | `stbds_hmput_key` | `keysize > elemsize` (oversized key) | `memcpy` writes past the element — heap corruption; **not** differentially testable, documented only | n/a |
| 46 | `stbds_hmput_default` | `a == NULL` | allocates a 1-element zeroed array, `length = 1` | [x] |
| 47 | `stbds_hmput_default` | `a != NULL` and `stbds_header(a-elemsize)->length == 0` | re-grows and sets `length = 1` (idempotent-ish path) | [x] |
| 48 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` unchanged | [x] |
| 49 | `stbds_hmput_key` | duplicate key found in the **forward** inner probe loop, `mode >= 1` | `stbds_temp` set **and** `stbds_temp_key` (`table->temp_key`) updated | [x] |
| 50 | `stbds_hmput_key` | duplicate key found in the **wrap-around** inner probe loop, `mode >= 1` | `stbds_temp` set, `stbds_temp_key` **NOT** updated (upstream quirk — must be reproduced) | [x] |
| 51 | `stbds_hmput_key` | insert lands on a tombstone (`INDEX_DELETED`) recorded before an empty slot | `pos = tombstone`, `--tombstone_count`, `++used_count` | [x] |
| 52 | `stbds_hmfree_func` | `elemsize` larger than the real element size on a `STRDUP` table | frees pointers read at the wrong stride → **SIGABRT/SIGSEGV**; not differentially testable | n/a |
| 53 | `arr_del` | any `int` (incl. `INT_MIN`, `INT_MAX`) | `void`; must not fault. `arrdel(arr,3)` gives a 0-length `memmove` | [x] |
| 54 | `strkey` | `n == INT_MIN` (`-2147483648`) | `"test_-2147483648"` (16 chars, fits the 256-byte buffer) | [x] |

Rows marked `n/a` are unreachable through the public API, or are UB that
corrupts the heap in a way that cannot be compared meaningfully; the reason is
given inline.  Every other row has a test.

## Where each row is tested

| rows | file |
|------|------|
| 1, 3, 6, 7, 8, 9, 10–14, 16–19, 24, 25, 26, 29, 30, 40–43, 46–51, 53, 54, 45b | `tests/errors.rs` |
| 4, 5, 15, 22, 28, 31, 32, 33, 37, 39, 41(+22) | `tests/crash_parity.rs` (subprocess, compares exit code / signal) |
| 2, 35, 36, 38, 44, 52-adjacent | `tests/arrays.rs`, `tests/arena_misc.rs`, `tests/maps_binary.rs`, `tests/maps_string.rs` |

`tests/crash_parity.rs` asserts, for every case, both that **the C really did
fail** (so the case cannot silently rot into a no-op) and that C and Rust
terminated with the *same* exit code / signal — `11` = SIGSEGV, `6` = SIGABRT.

## Note on `assert()`

The C `.so` is compiled with `C_FLAGS = -fPIC` and no `-DNDEBUG`, so all seven
`STBDS_ASSERT`s are **live** in the reference library.  The Rust translation
therefore carries the same `assert!`s (with `panic = "abort"` in
`[profile.release]`, a failing one aborts with SIGABRT exactly like C's).  The
one C assert that is a tautology — `STBDS_ASSERT(table->used_count >= 0)` on a
`size_t` — is left as a comment.
