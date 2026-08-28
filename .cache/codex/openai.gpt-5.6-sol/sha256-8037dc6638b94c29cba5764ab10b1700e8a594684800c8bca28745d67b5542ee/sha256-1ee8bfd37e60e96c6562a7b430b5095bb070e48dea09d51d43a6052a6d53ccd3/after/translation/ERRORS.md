# Error Surface

The C API has no error enum or `RETURN_ERROR` macro. Its explicit rejection
surface consists of not-found sentinels, null-map handling, and assertions.
Rows include internal assertions because they are mechanically present in the
C source; tests reach them through the exported operation that owns the state.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `stbds_hmfree_func` | `a == NULL` (line 573) | Return immediately; no effect | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` (line 634) | Allocate a zero default element and set `*temp = -1` | [x] |
| 3 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` (line 644) | Return the map and set `*temp = -1` | [x] |
| 4 | `stbds_hm_find_slot` via get/delete | an empty hash is reached in the initial bucket suffix (lines 609-610) | Return `-1`; public get exposes `temp == -1`, delete reports no deletion | [x] |
| 5 | `stbds_hm_find_slot` via get/delete | an empty hash is reached in the wrapped bucket prefix (lines 620-621) | Return `-1`; public get exposes `temp == -1`, delete reports no deletion | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` (lines 809-810) | Return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | map exists but `hash_table == NULL` (lines 816-817) | Return the map unchanged with header `temp == 0` | [x] |
| 8 | `stbds_hmdel_key` | key is absent, so `slot < 0` (lines 820-822) | Return the map unchanged with header `temp == 0` | [x] |
| 9 | `stbds_make_hash_index` via map creation/growth | `used_count_threshold + tombstone_count_threshold >= slot_count` (line 401) | `assert` failure (`SIGABRT`) | [x] |
| 10 | `stbds_hmput_key` | growth returns capacity smaller than `i + 1` (line 778) | `assert` failure (`SIGABRT`) | [x] |
| 11 | `stbds_hmdel_key` | found `slot >= table->slot_count` (line 828) | `assert` failure (`SIGABRT`) | [x] |
| 12 | `stbds_hmdel_key` | decrement leaves `table->used_count < 0` (line 832; impossible for unsigned `size_t`) | `assert` expression is always true; no rejection is observable | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`, line 846) | `assert` failure (`SIGABRT`) | [x] |
| 14 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` (line 849) | `assert` failure (`SIGABRT`) | [x] |
| 15 | `stbds_stralloc` | after block selection, `len > a->remaining` (line 913) | `assert` failure (`SIGABRT`) | [x] |
| 16 | `sh_puts` | copied arena key's first byte is not `'a'` (line 959) | `assert` failure (`SIGABRT`) | [x] |
| 17 | `sh_puts` | arena key pointer equals source literal pointer (line 960) | `assert` failure (`SIGABRT`) | [x] |
| 18 | `sh_puts` | copied value differs from input `num` (line 961) | `assert` failure (`SIGABRT`) | [x] |

Generic FFI boundaries not explicitly guarded by C are tested separately:
null data/key/string/arena/temp pointers, zero element/key/byte lengths,
oversized lengths, and mode values outside the named `0`/`1` and `0..3`
constants. For unchecked invalid pointers or impossible allocation sizes, the
C process signal/exit status is the expected result.

Rows 13 and 14 are triggered exactly by corrupting an otherwise valid map in
an isolated child process; C and Rust both terminate with `SIGABRT`. Rows 9-12
and 15-18 are allocator/arithmetic or postcondition invariants that cannot be
made false through the exported API without prior undefined behavior or fault
injection. Their owning operations are exercised repeatedly, and the Rust
translation now contains matching abort checks at each non-tautological site.
