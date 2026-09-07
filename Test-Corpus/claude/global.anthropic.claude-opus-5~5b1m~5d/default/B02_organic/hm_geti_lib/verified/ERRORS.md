# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return -1` / `return 0` /
`return NULL` / sentinel write / early `return` / `assert` / explicit bound
constant is listed. `STBDS_ASSERT` == `assert` from `<assert.h>`; the CMake build
uses no `NDEBUG`, so assertions are LIVE and a failure aborts the process
(`SIGABRT`).

Sentinels: `STBDS_INDEX_EMPTY == -1`, `STBDS_INDEX_DELETED == -2`,
`STBDS_HASH_EMPTY == 0`, `STBDS_HASH_DELETED == 1`.

| #  | function | trigger (exact invalid input/condition) | expected C result | status |
|----|----------|------------------------------------------|-------------------|--------|
| 1  | `stbds_arrgrowf` (lib.c:286) | `min_cap <= stbds_arrcap(a)` — request no bigger than current capacity (incl. `addlen==0, min_cap==0` on an existing array) | early `return a;` — pointer returned unchanged, no realloc, header untouched | [x] |
| 2  | `stbds_arrgrowf` (lib.c:280) | `a == NULL` (no array yet) | `stbds_arrlen(NULL)==0`, fresh alloc; `length=0, hash_table=NULL, temp=0`, `capacity=max(min_len,min_cap,4)` | [x] |
| 3  | `stbds_arrgrowf` (lib.c:291) | `min_cap >= 2*arrcap(a)` and `min_cap < 4` (i.e. `a==NULL`/cap 0 and tiny request) | `min_cap` clamped **up** to 4 | [x] |
| 4  | `stbds_hmfree_func` (lib.c:573) | `a == NULL` | early `return;` — no free, no crash | [x] |
| 5  | `stbds_hmfree_func` (lib.c:574) | `stbds_hash_table(a) == NULL` (array made by `stbds_arrgrowf`, never `hmput`) | skips strdup-key loop and `strreset`; still frees `hash_table` (NULL) and header | [x] |
| 6  | `stbds_hmget_key_ts` (lib.c:634-639) | `a == NULL` | allocates a 1-elem map, zeroes elem 0, writes `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash-side ptr | [x] |
| 7  | `stbds_hmget_key_ts` (lib.c:644) | `a != NULL` but `header(raw_a)->hash_table == 0` (only the default slot exists, e.g. after `hmput_default` only) | `*temp = -1`, returns `a` unchanged | [x] |
| 8  | `stbds_hmget_key_ts` (lib.c:648) | key absent from a populated table (`stbds_hm_find_slot` returns `< 0`) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | [x] |
| 9  | `stbds_hm_find_slot` (lib.c:609) | probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` in the **first** (`i = pos&MASK .. 7`) scan | `return -1` (key not found) | [x] |
| 10 | `stbds_hm_find_slot` (lib.c:620) | probe reaches `bucket->hash[i] == STBDS_HASH_EMPTY` in the **second** (`i = 0 .. limit`) wrap scan | `return -1` (key not found) | [x] |
| 11 | `stbds_hmget_key` (lib.c:663) | any of rows 6/7/8 | same as `_ts`, but the sentinel is also stored into `header(arr)->temp` and is observable via `stbds_temp` | [x] |
| 12 | `stbds_hmdel_key` (lib.c:809) | `a == NULL` | `return 0;` (**NULL pointer**) — note this differs from `hmget_key_ts`, which allocates | [x] |
| 13 | `stbds_hmdel_key` (lib.c:816) | `header(raw_a)->hash_table == 0` | `stbds_temp(raw_a) = 0` then `return a` (unchanged); `hmdel` macro therefore yields 0 | [x] |
| 14 | `stbds_hmdel_key` (lib.c:821) | key not present (`slot < 0`) | `stbds_temp(raw_a) = 0`, `return a`; length/used_count/tombstone_count unchanged | [x] |
| 15 | `stbds_hmdel_key` (lib.c:836) | `mode == STBDS_HM_STRING` **exactly 1** (not `>=1`) *and* `string.mode == STBDS_SH_STRDUP` | frees the old key; with `mode == 2` the free is **skipped** (leak) even though hashing used the string path | [x] |
| 16 | `stbds_hmdel_key` (lib.c:828) | `assert(slot < (ptrdiff_t) table->slot_count)` | unreachable via the public API (`find_slot` masks by `slot_count-1`) — reaching it needs a corrupted table. Covered indirectly: `err40_41_42` compares the whole bucket array after every op, so a differing `slot` would show up | [x] |
| 17 | `stbds_hmdel_key` (lib.c:846) | `assert(slot >= 0)` after re-finding the moved final element. REACHABLE from the FFI boundary: `mode >= 2` deleting a non-final element makes the re-find hash the raw element bytes as a C string, which does not match | **`SIGABRT`** — verified equal in a child process by `phase_c_abort.rs::err17_abort_parity_hmdel_mode2_nonfinal` (both signal 6) | [x] |
| 18 | `stbds_hmdel_key` (lib.c:849) | `assert(b->index[i] == final_index)` | holds for every well-formed table; asserted implicitly by the per-operation bucket-array comparison in rows 19-23 / 40-42 (a mismatch would abort one side only) | [x] |
| 19 | `stbds_hmdel_key` (lib.c:832) | `assert(table->used_count >= 0)` | `used_count` is `size_t`, so this is **always true** in C; the Rust must likewise never abort here (a signed reading would abort on underflow) | [x] |
| 20 | `stbds_hmput_key` (lib.c:778) | `assert((size_t) i+1 <= stbds_arrcap(a))` | always satisfied because the preceding `if` grows the array; must not abort | [x] |
| 21 | `stbds_make_hash_index` (lib.c:401) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` — would fail only for `slot_count == 0` (`0 + 0 < 0`) | `SIGABRT`, but `slot_count` is only ever `8` (from `shmode_func` / a fresh `hmput_key`) or `slot_count*2` / `slot_count>>1` with `slot_count > 8`, so 0 is unreachable through any exported symbol. `err40_41_42` walks slot_count 8→1024→8 and neither library aborts | [x] |
| 22 | `stbds_stralloc` (lib.c:913) | `assert(len <= a->remaining)` | always satisfied by the block-allocation branch above; must not abort for any string length | [x] |
| 23 | `stbds_stralloc` (lib.c:885,893) | `len > a->remaining` **and** `len > blocksize` (oversized string, e.g. > 512 on a fresh arena) | takes the "huge block" path: allocates exactly `sizeof(block)-8+len`, splices it *behind* the head block (or becomes the head with `remaining = 0`), returns `sb->storage` | [x] |
| 24 | `stbds_stralloc` (lib.c:890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` stops incrementing (saturates); blocksize stays at the max step | [x] |
| 25 | `stbds_stralloc` | `str` is `""` (len 1) on a fresh arena (`remaining == 0`) | `1 > 0` so a 512-byte block is allocated; returns pointer to the last byte | [x] |
| 26 | `stbds_strreset` (lib.c:924) | `a->storage == NULL` (empty/already-reset arena) | loop body never runs; arena is `memset` to 0 (incl. `block`/`mode`) — idempotent | [x] |
| 27 | `stbds_arrfreef` (lib.c:314) | `a == NULL` | `free((char*)NULL - 32)` — UB; both libraries die with the same signal. Verified in a child process by `phase_c_abort.rs::err27_abort_parity_arrfreef_null` (both `SIGSEGV`, signal 11) | [x] |
| 28 | out-of-range `mode` enum, all of `stbds_hmget_key{,_ts}` / `stbds_hmput_key` / `stbds_hmdel_key` | `mode` = 2, 7, 1000 (no valid variant; C enums accept any `int`) | every `mode >= STBDS_HM_STRING` test takes the **string** path (`strcmp`/`stbds_hash_string`), so key is treated as `char*`; only the `mode == STBDS_HM_STRING` equality test in `hmdel_key` line 836/842 behaves differently | [x] |
| 29 | out-of-range `mode` enum, `stbds_hmget_key{,_ts}` / `stbds_hmput_key` / `stbds_hmdel_key` | `mode` = -1, -1000 (negative) | `mode >= STBDS_HM_STRING` is false → **binary** path (`memcmp`/`stbds_hash_bytes`); `nt->string.mode` set to `0` | [x] |
| 30 | out-of-range `mode` enum, `stbds_shmode_func` | `mode` = 0/1/2/3 valid, plus 4, 99, 256, -1 | `h->string.mode = (unsigned char) mode` truncates: 256→0, -1→255; a later `hmput_key` `switch` hits `default:` → `memcpy(key, keysize)` instead of a pointer store | [x] |
| 31 | `stbds_hash_string` (lib.c:480) | `str` = `""` (empty string) | loop skipped; the fixed avalanche is applied to `seed` alone | [x] |
| 32 | `stbds_hash_bytes` (lib.c:522,532) | `len == 0` | main loop skipped, `switch(0)` → `break`; hashes only `data = 0 << 56` | [x] |
| 33 | `stbds_hash_bytes` | `len` = 1..7 (partial tail, `switch` fall-through) and `len` = 8,9,15,16 (block boundaries) | each fall-through case contributes its byte; `d[3]<<24` is an `int` shift that **sign-extends** into the top 32 bits of `size_t` | [x] |
| 34 | `stbds_hmput_default` (lib.c:669) | `a == NULL` | allocates the default slot, `length = 1`, returns hash-side pointer (so `t[-1]` is the default element) | [x] |
| 35 | `stbds_hmput_default` (lib.c:669) | `a != NULL` and `header(raw_a)->length == 0` | re-grows and bumps length to 1 — the "resurrect a zero-length map" branch | [x] |
| 36 | `stbds_hmput_default` (lib.c:675) | `a != NULL` and `length != 0` | `return a` unchanged (idempotent — no double-allocation of the default slot) | [x] |
| 37 | `hm_geti` (lib.c:945) | `num <= 0` (0, -1, INT_MIN) | all `for` loops are skipped; only the 4 leading assertions run; returns normally | [x] |
| 38 | `hm_geti` (lib.c:952-977) | any `num` for which a lookup result mismatches | `SIGABRT` from `assert`. Must never trigger for either library | [x] |
| 39 | `strkey` (lib.c:939) | `n` = `INT_MIN`, `-1`, `0`, large | `sprintf(buffer,"test_%d",n)` into a 256-byte static; result is `"test_<n>"` NUL-terminated | [x] |
| 40 | `stbds_hmput_key` (lib.c:698) | `table == NULL` **or** `used_count >= used_count_threshold` (75 % load) | (re)allocates the hash index: `slot_count = 8` if new else `slot_count*2`; when new, `nt->string.mode = (mode>=1 ? SH_DEFAULT : 0)` | [x] |
| 41 | `stbds_hmdel_key` (lib.c:854) | `used_count < used_count_shrink_threshold && slot_count > 8` | index is rebuilt at `slot_count>>1` (shrink) | [x] |
| 42 | `stbds_hmdel_key` (lib.c:858) | `tombstone_count > tombstone_count_threshold` (`(sc>>3)+(sc>>4)`) | index is rebuilt at the same `slot_count` (tombstone purge) | [x] |
| 43 | `stbds_hm_find_slot` / `stbds_hmput_key` (lib.c:596,719) | computed `hash < 2` (collides with the EMPTY/DELETED tags) | `hash += 2` before probing — must be applied identically in both | [x] |

## Result

All 43 rows have a passing differential test.

| test file | rows covered | tests |
|-----------|--------------|-------|
| `tests/phase_c.rs` | 1-15, 19, 20, 22-26, 28-43 (+ generic NULL / zero-length / oversized-length boundaries) | 31 |
| `tests/phase_c_abort.rs` | 17, 27 (child-process termination-status parity) | 3 |

Rows 16, 18 and 21 are asserts that no input reachable through the 16 exported
symbols can trigger; they are covered indirectly because every Phase B/C row
compares the complete bucket array (`hash[]` and `index[]`) plus
`slot_count` / `used_count` / `tombstone_count` after every single operation, so
any divergence in the quantities those asserts guard would fail a comparison (or
abort exactly one of the two libraries).

Note on abort parity: C's `assert` prints a diagnostic to `stderr` before
`abort()` while the Rust translation calls `abort()` silently. The *termination
status* — the thing a caller can observe — is identical (`SIGABRT`); only the
stderr text differs.
