# Error Surface

The C API has no error enum and no `RETURN_ERROR` macro. It generally assumes
pointer/size preconditions; invalid pointers that it dereferences are undefined
behavior, not a C rejection result. The rows below enumerate every explicit
sentinel return, null rejection/no-op, and assertion in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `stbds_hm_find_slot` via `stbds_hmget_key_ts` | lookup reaches `STBDS_HASH_EMPTY` in the probe bucket from the initial offset (`i = pos & 7 .. 7`) | slot `-1`; public `temp` is `STBDS_INDEX_EMPTY` (`-1`) | [x] |
| 2 | `stbds_hm_find_slot` via `stbds_hmget_key_ts` | wrapped lookup reaches `STBDS_HASH_EMPTY` in the same bucket (`i = 0 .. (pos & 7)-1`) | slot `-1`; public `temp` is `STBDS_INDEX_EMPTY` (`-1`) | [x] |
| 3 | `stbds_hmget_key_ts` | `a == NULL` | allocate a zero default entry, set `*temp = -1`, return pointer one element past that entry | [x] |
| 4 | `stbds_hmget_key_ts` | non-null map storage has `hash_table == NULL` | set `*temp = -1`, return input pointer unchanged | [x] |
| 5 | `stbds_hmget_key_ts` | initialized table does not contain `key` | set `*temp = -1`, return input pointer unchanged | [x] |
| 6 | `stbds_hmget_key` | key is absent (including a newly created/null map) | return map pointer and store `-1` in the array header `temp` field | [x] |
| 7 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 8 | `stbds_hmdel_key` | non-null map storage has `hash_table == NULL` | set header `temp = 0`, return input pointer unchanged | [x] |
| 9 | `stbds_hmdel_key` | initialized table does not contain `key` (`slot < 0`) | set header `temp = 0`, return input pointer unchanged | [x] |
| 10 | `stbds_hmfree_func` | `a == NULL` | return without action | [x] |
| 11 | `stbds_make_hash_index` (reached by put/mode/grow/shrink/rebuild) | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 12 | `stbds_hmput_key` | after growth, `(size_t)i + 1 > array capacity` | `assert` abort | [x] |
| 13 | `stbds_hmdel_key` | found slot is not below `table->slot_count` | `assert` abort | [x] |
| 14 | `stbds_hmdel_key` | deletion makes `table->used_count < 0` | `assert` abort (the field is unsigned, so this invariant is mechanically always true) | [x] |
| 15 | `stbds_hmdel_key` | moved final element cannot be found after `memmove` (`slot < 0`) | `assert` abort | [x] |
| 16 | `stbds_hmdel_key` | moved element's hash slot does not point at `final_index` | `assert` abort | [x] |
| 17 | `stbds_stralloc` | normal-block path reaches copy with `len > a->remaining` | `assert` abort | [x] |
| 18 | mode-taking hash APIs | `mode` is outside the named values (`0` binary, `1` string; or string storage modes `0..3`) | no rejection: comparisons use the raw `int`; `stbds_shmode_func` stores its low 8 bits | [x] |
| 19 | byte-buffer APIs | null data with zero length (`stbds_hash_bytes(NULL, 0, seed)`) | accepted; returns the zero-length hash | [x] |

Rows 11-17 are internal consistency assertions. No well-formed public call is
intended to trigger them; tests exercise the public transitions that reach each
assert site and verify that neither implementation aborts. Null pointers for
required strings, arenas, output pointers, or positive-length byte buffers are
dereferenced by C and therefore have no defined C result to reproduce.
