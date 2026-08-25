# Error Surface

The C API has no error enum or conventional error return. Its rejection
surface consists of lookup/delete sentinels, null no-op paths, and assertions.
Rows 10-18 are internal consistency assertions reached through exported
operations; they are listed because every C `STBDS_ASSERT` is part of the
surface. Rows 10, 12, 14, and 15 are exercised with deliberately corrupted
private state. The other assertion predicates are established by immediately
preceding assignments or are tautological for their unsigned types, so the
tests exercise them through randomized valid operations rather than claiming
that an exported input can make an unreachable predicate false.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `stbds_hm_find_slot` via get/delete | first probe segment encounters `STBDS_HASH_EMPTY` before the key | [x] returns `-1`, exposed by get as temp `-1` and by delete as no deletion |
| 2 | `stbds_hm_find_slot` via get/delete | wrapped probe segment encounters `STBDS_HASH_EMPTY` before the key | [x] returns `-1`, exposed by get as temp `-1` and by delete as no deletion |
| 3 | `stbds_hmget_key_ts` | `a == NULL` | [x] creates the default entry, stores `STBDS_INDEX_EMPTY` (`-1`) in `*temp`, returns hash-array pointer |
| 4 | `stbds_hmget_key_ts` | non-null array has `hash_table == NULL` | [x] stores `-1` in `*temp`, returns the input pointer |
| 5 | `stbds_hmget_key_ts` | initialized table does not contain key (`slot < 0`) | [x] stores `-1` in `*temp`, returns the input pointer |
| 6 | `stbds_hmfree_func` | `a == NULL` | [x] returns without action |
| 7 | `stbds_hmdel_key` | `a == NULL` | [x] returns `NULL` |
| 8 | `stbds_hmdel_key` | non-null array has `hash_table == NULL` | [x] returns input pointer and leaves header temp at `0` |
| 9 | `stbds_hmdel_key` | initialized table does not contain key (`slot < 0`) | [x] returns input pointer and leaves header temp at `0` |
| 10 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | [x] `assert` fails and process aborts |
| 11 | `stbds_hmput_key` | after growth, `i + 1 > stbds_arrcap(a)` | [x] `assert` fails and process aborts |
| 12 | `stbds_hmdel_key` | located `slot >= table->slot_count` | [x] `assert` fails and process aborts |
| 13 | `stbds_hmdel_key` | decrement would make `table->used_count < 0` | [x] assertion is present but cannot reject on this build because `used_count` is unsigned `size_t` |
| 14 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | [x] `assert` fails and process aborts |
| 15 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | [x] `assert` fails and process aborts |
| 16 | `stbds_stralloc` | post-allocation `len > a->remaining` on the normal-block path | [x] `assert` fails and process aborts |
| 17 | `str_dups` | inserted duplicate key does not begin with `'a'` | [x] `assert` fails and process aborts |
| 18 | `str_dups` | strdup mode retained the source key pointer | [x] `assert` fails and process aborts |
| 19 | `str_dups` | inserted value differs from input `num` | [x] `assert` fails and process aborts |
| 20 | `stbds_hash_string` | `str == NULL` | [x] invalid pointer dereference; process terminates |
| 21 | `stbds_hash_bytes` | `p == NULL && len > 0` | [x] invalid pointer dereference; process terminates |
| 22 | `stbds_arrfreef` | `a == NULL` | [x] invalid header pointer passed to `free`; behavior is process-level rejection |
| 23 | `stbds_stralloc` | `a == NULL` or `str == NULL` | [x] invalid pointer dereference; process terminates |
| 24 | `stbds_strreset` | `a == NULL` | [x] invalid pointer dereference; process terminates |
| 25 | hash-map entry points | `elemsize == 0` | [x] no explicit C rejection; pointer arithmetic aliases entries and subsequent stateful operations may terminate |
| 26 | hash-map entry points | mode is outside binary/string values (`mode < 0` or `mode > 1`) | [x] no enum rejection: `< 1` follows binary branches and `>= 1` follows string branches |
| 27 | `stbds_shmode_func` | mode is outside `0..=3` | [x] no enum rejection: value is truncated to `unsigned char`; insertion uses the switch default unless it becomes `1`, `2`, or `3` |
| 28 | length-taking entry points | zero length/key size | [x] accepted; hashes/compares zero bytes and returns the normal result/sentinel |
| 29 | length-taking entry points | oversized length/capacity causing `size_t` arithmetic wrap or failed allocation | [x] no explicit error sentinel or allocation check; process-level rejection is permitted by C |
