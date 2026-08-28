# Error Surface

The C API does not define error enums or use `RETURN_ERROR`, `return -1` from
an exported function, or `return NULL` spelling. The rows below mechanically
cover its exported null/missing-key rejection paths and every source `assert`.
Rows marked "internal invariant" cannot be requested through a well-formed
public call; violating them requires allocator failure or corruption of the
library's private allocation headers.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `stbds_hmget_key_ts` | `a == NULL` | Allocate a zero default element, set `*temp = -1`, return the hash-array pointer | [x] |
| 2 | `stbds_hmget_key_ts` | Non-null map whose `hash_table == NULL` | Set `*temp = -1`, return `a` unchanged | [x] |
| 3 | `stbds_hmget_key_ts` | Key is absent and the first scanned probe segment reaches `STBDS_HASH_EMPTY` | Set `*temp = -1`, return `a` | [x] |
| 4 | `stbds_hmget_key_ts` | Key is absent and the wrapped probe segment reaches `STBDS_HASH_EMPTY` | Set `*temp = -1`, return `a` | [x] |
| 5 | `stbds_hmget_key` | Any missing-key condition from rows 1-4 | Return the map and store `-1` in the private header `temp` field | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | Return `NULL` (`0`) | [x] |
| 7 | `stbds_hmdel_key` | Non-null map whose `hash_table == NULL` | Return `a` unchanged with header `temp = 0` | [x] |
| 8 | `stbds_hmdel_key` | Key is absent (`stbds_hm_find_slot < 0`) | Return `a` unchanged with header `temp = 0` | [x] |
| 9 | `stbds_hmfree_func` | `a == NULL` | Return without action | [x] |
| 10 | `stbds_hash_bytes` | `p == NULL && len == 0` | Return the defined empty-input hash without dereferencing `p` | [x] |
| 11 | `stbds_hmget_key*` / `stbds_hmput_key` / `stbds_hmdel_key` | `mode < STBDS_HM_STRING` (including out-of-range negative values) | Treat the key as binary | [x] |
| 12 | `stbds_hmget_key*` / `stbds_hmput_key` / `stbds_hmdel_key` | `mode > STBDS_HM_STRING` | Treat the key as a C string | [x] |
| 13 | `stbds_shmode_func` | `mode` is outside `0..=3` (`STBDS_SH_STRDUP == 2`, `STBDS_SH_ARENA == 3`) | Store `(unsigned char)mode`; allocation succeeds (later put behavior follows the C switch default) | [x] |
| 14 | `stbds_make_hash_index` (internal) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 15 | `stbds_hmput_key` (internal) | Growth returns capacity smaller than `i + 1` | `assert` abort | [x] |
| 16 | `stbds_hmdel_key` (internal) | Found `slot >= table->slot_count` | `assert` abort | [x] |
| 17 | `stbds_hmdel_key` (internal) | `table->used_count < 0` after decrement | `assert` abort; condition is unreachable because the field is unsigned `size_t` | [x] |
| 18 | `stbds_hmdel_key` (internal) | Moved final element cannot be found (`slot < 0`) | `assert` abort | [x] |
| 19 | `stbds_hmdel_key` (internal) | Moved element's bucket index is not `final_index` | `assert` abort | [x] |
| 20 | `stbds_stralloc` (internal) | After block selection, `len > a->remaining` | `assert` abort | [x] |
| 21 | `arr_push` (internal) | Initial `arrlen(NULL) != 0` | `assert` abort; condition is unreachable by construction | [x] |
| 22 | `stbds_stralloc` | Empty arena or exhausted current block (`len > remaining`); `STBDS_STRING_ARENA_BLOCKSIZE_MIN == 512` | Select a block of at least 512 bytes before copying | [x] |
| 23 | `stbds_stralloc` | Scheduled block size reaches `STBDS_STRING_ARENA_BLOCKSIZE_MAX == 1 << 20` | Stop incrementing `a->block`; later scheduled blocks remain capped at the maximum | [x] |

Other null pointers, an oversized allocation whose size arithmetic wraps, and
nonzero lengths paired with invalid pointers invoke undefined C behavior; the
C source does not reject them with a stable result. They are therefore not
claimed as error-result rows.
