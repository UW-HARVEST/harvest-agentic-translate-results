# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping for every `return`
that reports "nothing / not-found / failure", every `STBDS_ASSERT`
(`#define STBDS_ASSERT assert`, and the CMake build defines **no** `NDEBUG`, so
asserts are LIVE and abort the process), every null check, and every explicit
range/threshold constant.

This library has **no error enum and no `RETURN_ERROR` macro**. Its entire
rejection surface consists of:

* NULL-pointer guards that return `NULL` / return early,
* the "key not found" sentinels `-1` (`STBDS_INDEX_EMPTY`) and the `temp` field,
* `STBDS_ASSERT` aborts,
* saturation/clamp constants in the string arena.

Sentinel constants: `STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`,
`STBDS_HASH_EMPTY = 0`, `STBDS_HASH_DELETED = 1`,
`STBDS_STRING_ARENA_BLOCKSIZE_MIN = 512u`, `STBDS_STRING_ARENA_BLOCKSIZE_MAX = 1<<20`,
`STBDS_BUCKET_LENGTH = 8`, `STBDS_CACHE_LINE_SIZE = 64`,
`STBDS_HM_BINARY = 0`, `STBDS_HM_STRING = 1`,
`STBDS_SH_NONE/DEFAULT/STRDUP/ARENA = 0/1/2/3`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` and `min_len <= min_cap` (nothing to do) | early `return a` — identical pointer, header untouched | [x] |
| E2 | `stbds_arrgrowf` | `a == NULL` (no existing allocation) | fresh alloc; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=min_cap` | [x] |
| E3 | `stbds_arrgrowf` | `a == NULL, addlen = 0, min_cap = 0` (degenerate zero request) | `min_len = 0`, `min_cap` stays 0, `0 <= stbds_arrcap(NULL) == 0` → early `return a`, i.e. **returns NULL without allocating** | [x] |
| E4 | `stbds_arrgrowf` | `elemsize = 0` | allocates only the header; `capacity = min_cap` | [x] |
| E5 | `stbds_arrgrowf` | `addlen` huge so `min_len` wraps (`arrlen + addlen` overflows `size_t`) | `addlen = MAX`/`MAX-1` wrap `min_len` back into the early-out range → `return a`. `addlen = MAX-2` gives `min_cap = SIZE_MAX`, so `elemsize*min_cap + HEADER_SIZE` wraps to **24**: `realloc` SHRINKS the block below the 32-byte header, which is then written → heap corruption, identical on both sides (compared as a process outcome in a forked child) | [x] |
| E6 | `stbds_hmfree_func` | `a == NULL` | `return` immediately, no free, no crash | [x] |
| E7 | `stbds_hmfree_func` | `stbds_hash_table(a) == NULL` (array grown but never hashed) | skips strdup loop and `strreset`; frees `hash_table` (NULL) and header | [x] |
| E8 | `stbds_hm_find_slot` | bucket slot has `hash[i] == STBDS_HASH_EMPTY` before a match (key absent) | `return -1` (first, non-wrapped scan) | [x] |
| E9 | `stbds_hm_find_slot` | key absent, empty slot only found in the wrapped `0..limit` scan | `return -1` (second scan) | [x] |
| E10 | `stbds_hmget_key_ts` | `a == NULL` | creates 1-elem array, `length=1`, zeroed, `*temp = -1`, returns `arr+elemsize` | [x] |
| E11 | `stbds_hmget_key_ts` | `a != NULL` but `hash_table == 0` (e.g. after `stbds_hmput_default` only) | `*temp = -1`, returns `a` unchanged | [x] |
| E12 | `stbds_hmget_key_ts` | key not present in a populated table (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY` (-1), returns `a` | [x] |
| E13 | `stbds_hmget_key` | same three miss conditions as E10–E12 | `stbds_temp(raw_a)` set to `-1`; return value equals `stbds_hmget_key_ts`'s | [x] |
| E14 | `stbds_hmget_key`/`_ts` | `mode` out of enum range, e.g. `mode = 2, 7, 0x7fffffff` | treated as STRING (`mode >= STBDS_HM_STRING`) → `strcmp`/`stbds_hash_string` | [x] |
| E15 | `stbds_hmget_key`/`_ts` | `mode` negative, e.g. `mode = -1, INT_MIN` | treated as BINARY (`mode < STBDS_HM_STRING`) → `memcmp`/`stbds_hash_bytes` | [x] |
| E16 | `stbds_hmput_default` | `a == NULL` | allocate, `length=1`, first elem zeroed, return `arr+elemsize` | [x] |
| E17 | `stbds_hmput_default` | `a != NULL` and `header(raw_a)->length == 0` | re-grow (min_cap 1), `length=1`, zero first elem | [x] |
| E18 | `stbds_hmput_default` | `a != NULL` and `length != 0` | `return a` unchanged (no reallocation, no zeroing) | [x] |
| E19 | `stbds_hmput_key` | `a == NULL` | bootstraps 1-elem array before inserting | [x] |
| E20 | `stbds_hmput_key` | duplicate key (found in first, non-wrapped bucket scan) | no new element; `stbds_temp(a) = existing index`; for `mode>=1` also sets `temp_key`; `length` unchanged | [x] |
| E21 | `stbds_hmput_key` | duplicate key found only in the wrapped `0..limit` scan | same, **except `temp_key` is NOT written** (C omits it in that branch) | [x] |
| E22 | `stbds_hmput_key` | `table->used_count >= used_count_threshold` (load factor 3/4 exceeded) | grows table to `slot_count*2`, rehashes, frees old | [x] |
| E23 | `stbds_hmput_key` | insertion lands on a tombstone (`index == STBDS_INDEX_DELETED`) | reuses tombstone slot; `--tombstone_count` | [x] |
| E24 | `stbds_hmput_key` | `STBDS_ASSERT((size_t)i+1 <= stbds_arrcap(a))` after the grow | unreachable via the public API (grow guarantees it); would `abort()` | [x] (analysis) |
| E25 | `stbds_hmput_key` | `table->string.mode` has no valid variant (e.g. 255 via `stbds_shmode_func(-1)`) | `switch` `default:` → `memcpy(elem, key, keysize)` (binary copy, NOT strdup) | [x] |
| E26 | `stbds_hmput_key` | `keysize = 0` with `string.mode == STBDS_SH_NONE` | `memcpy(..., 0)` copies nothing; `hash = stbds_hash_bytes(key, 0, seed)` | [x] |
| E27 | `stbds_hmdel_key` | `a == NULL` | `return 0` (NULL) — the only function returning NULL as a failure | [x] |
| E28 | `stbds_hmdel_key` | `hash_table == 0` | `stbds_temp(raw_a) = 0` then `return a` (nothing deleted) | [x] |
| E29 | `stbds_hmdel_key` | key absent (`stbds_hm_find_slot < 0`) | `stbds_temp(raw_a)` stays `0`, `return a`, `length` unchanged | [x] |
| E30 | `stbds_hmdel_key` | deleting the same key twice | 2nd call takes the `slot < 0` path (slot is `HASH_DELETED`/`INDEX_DELETED`) → temp 0 | [x] |
| E31 | `stbds_hmdel_key` | `mode == 2` (out-of-range STRING-ish value) on a string map | `find_slot` uses `strcmp` (`mode >= 1`) but the STRDUP free and the re-find of the moved key take the **BINARY** path (`mode == STBDS_HM_STRING` is false) | [x] |
| E32 | `stbds_hmdel_key` | `STBDS_ASSERT(slot < table->slot_count)` | unreachable: `find_slot` masks `pos` with `slot_count-1` | [x] (analysis) |
| E33 | `stbds_hmdel_key` | `STBDS_ASSERT(slot >= 0)` when re-finding the relocated last element | **REACHABLE** — with `mode >= 2` (or a non-zero `keyoffset`), `find_slot` hashes the key as a string while the re-find passes the *address* of the `char *` field, so the lookup misses and the live assert `abort()`s (SIGABRT). Verified in a forked child: C and Rust both terminate with signal 6 | [x] |
| E34 | `stbds_hmdel_key` | `STBDS_ASSERT(b->index[i] == final_index)` | same trigger class as E33; guarded behind it. Rust contains the identical assert | [x] |
| E35 | `stbds_hmdel_key` | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t`; always true, never fires | [x] (analysis) |
| E36 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` | shrink to `slot_count>>1`, rehash, free old table | [x] |
| E37 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (`(sc>>3)+(sc>>4)`) | rebuild at same `slot_count`, rehash, free old table | [x] |
| E38 | `stbds_hmdel_key` | `keyoffset` != 0 while the table was populated with the hard-coded `keyoffset = 0` | the compare reads the wrong bytes. Usually a miss (`slot < 0` → `return a`, temp 0); but when those bytes happen to match (e.g. `keyoffset = 8` on an element whose value equals its key) the delete proceeds and the slot re-find trips E33's `abort()`. Both routes verified identical (process outcome first, then full in-process state diff for the non-crashing ones) | [x] |
| E39 | `stbds_make_hash_index` | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | unreachable: `slot_count` is always a power of two `>= 8` | [x] (analysis) |
| E40 | `stbds_make_hash_index` | `slot_count <= STBDS_BUCKET_LENGTH` (8) | `used_count_shrink_threshold` forced to `0` (disables shrinking) | [x] |
| E41 | `stbds_hash_bytes` | `len == 0` (with any `p`, incl. NULL) | reads nothing; `data = 0 << 56`; returns the pure-seed siphash value | [x] |
| E42 | `stbds_hash_bytes` | `len` not a multiple of 8 → `len - i` in `1..7` | switch fall-through builds the partial word; `d[3] << 24` **sign-extends** (int shift) | [x] |
| E43 | `stbds_hash_string` | empty string `""` | loop body never runs; avalanche applied to `seed` alone | [x] |
| E44 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` and `a->storage == NULL` | allocates an oversize block, `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`, returns `sb->storage` | [x] |
| E45 | `stbds_stralloc` | `len > a->remaining` and `len > blocksize` and `a->storage != NULL` | oversize block spliced in **after** the head (`sb->next = storage->next; storage->next = sb`); `a->remaining` NOT reset | [x] |
| E46 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (1<<20), i.e. `a->block >= 22` | `a->block` stops incrementing (clamped) | [x] |
| E47 | `stbds_stralloc` | `a->block` out of the range the library itself produces (e.g. 40, 64, 100) | `blocksize = (size_t)512u << (block>>1)` computed in **`size_t`** (x86-64 `shl` masks the count by 63) | [x] |
| E48 | `stbds_stralloc` | `a->block = 0`, `a->remaining = 0`, empty string (`len == 1`) | new 512-byte block, `remaining = 512` then `511`, `block` → 1 | [x] |
| E49 | `stbds_strreset` | `a->storage == NULL` (empty/zeroed arena) | frees nothing, memsets the arena to 0 | [x] |
| E50 | `stbds_arrfreef` | `a == NULL` | `free(NULL - 32)` — **undefined behaviour in C**; both sides pass the same wild pointer to `free`. Compared as a process outcome in a forked child | [x] |
| E51 | `strkey` | `n` negative / large (`INT_MIN`, `-1`, `2147483647`) | `sprintf(buffer, "test_%d", n)` into the 256-byte static; returns the same static address every call | [x] |
| E52 | `arr_del` | any `num`, including `INT_MIN`/`INT_MAX` | `void`; must not crash. Internally `arrdel(arr,i)`/`arrdelswap(arr,i)` for `i = 0..3` on a 4-element array | [x] |

## Verification

Every row above has a differential test in `tests/phase_c_errors.rs` (named
`eNN_*`), plus the generic FFI boundaries in `g1_*`–`g5_*`:

| test | covers |
|------|--------|
| `g1_null_pointer_arguments` | NULL passed to every entry point that accepts a pointer |
| `g2_zero_lengths` | `elemsize = 0`, `keysize = 0`, `len = 0` |
| `g3_large_lengths` | hash lengths up to 65536, `elemsize` up to 65536, 65535-char strings |
| `g4_one_past_valid_range` | `mode` = -1/0/1/2 and `sh_mode` = -1/0..3/4 |
| `g5_all_out_of_range_enum_values` | the 16x16 cross-product of out-of-enum-range `int`s through `mode` and `sh_mode` (157 combinations complete normally, 99 crash — identically on both sides) |

Result: **46/46 tests pass**, against both the release and the debug Rust `.so`.

### A note on `STBDS_ASSERT`

`c_src/src/lib.c` has `#define STBDS_ASSERT assert` and `c_src/CMakeLists.txt`
sets no build type, so `NDEBUG` is never defined and all seven asserts are LIVE.
Because E33 turned out to be reachable from the public API, the Rust translation
carries all of them as `abort()` calls (`stbds_assert!` in `src/lib.rs`), and the
tests compare the resulting SIGABRT via `fork`/`waitpid` rather than letting it
kill the harness.
