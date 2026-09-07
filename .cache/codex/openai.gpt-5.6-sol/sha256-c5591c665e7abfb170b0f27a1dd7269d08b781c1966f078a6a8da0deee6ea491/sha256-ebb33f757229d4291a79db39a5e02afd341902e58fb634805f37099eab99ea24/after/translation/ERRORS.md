# Error Surface

Mechanically derived from null checks, sentinel returns, range checks, and every
`STBDS_ASSERT` in `c_src/src/lib.c`. This C library has no error enum or
`RETURN_ERROR` macro.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `stbds_hmfree_func` | `a == NULL` | return immediately; no crash | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | allocate the default element, set `*temp = -1`, return hash-view pointer | [x] |
| 3 | `stbds_hmget_key_ts` | non-null map whose header has `hash_table == NULL` | set `*temp = -1`, return input pointer | [x] |
| 4 | `stbds_hmget_key_ts` / `stbds_hm_find_slot` | initialized table, requested key absent; first probe segment reaches an empty hash slot | set `*temp = -1`, return input pointer | [x] |
| 5 | `stbds_hmget_key_ts` / `stbds_hm_find_slot` | initialized table, requested key absent; wrapped probe segment reaches an empty hash slot | set `*temp = -1`, return input pointer | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | non-null map whose header has `hash_table == NULL` | set header temp to `0`, return input pointer unchanged | [x] |
| 8 | `stbds_hmdel_key` | initialized table, requested key absent | set header temp to `0`, return input pointer unchanged | [x] |
| 9 | `stbds_make_hash_index` | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 10 | `stbds_hmput_key` | post-growth `i + 1 > array capacity` | `assert` abort | [x] |
| 11 | `stbds_hmdel_key` | located hash slot is outside `table->slot_count` | `assert` abort | [x] |
| 12 | `stbds_hmdel_key` | decrement would leave an invalid used count (`used_count >= 0` invariant) | continue; the assertion expression is tautologically true for C `size_t` | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found in the hash table (`slot < 0`) | `assert` abort | [x] |
| 14 | `stbds_hmdel_key` | moved final element's bucket index is not `final_index` | `assert` abort | [x] |
| 15 | `stbds_stralloc` | after arena block selection, `len > a->remaining` | `assert` abort | [x] |
| 16 | `str_put` | inserted key's first byte is not `'a'` | `assert` abort | [x] |
| 17 | `str_put` | default string mode does not preserve the caller's key pointer | `assert` abort | [x] |
| 18 | `str_put` | retrieved value differs from input `num` | `assert` abort | [x] |
| 19 | pointer-taking exports | required data pointer is `NULL` where C has no null guard (`stbds_arrfreef`, hash functions with nonzero length, `stbds_stralloc`, `stbds_strreset`, `strkey` return consumer misuse) | C undefined behavior, normally process signal; Rust must not silently produce a different successful result | [x] |
| 20 | mode-taking hash exports | mode just below/above documented values (`-1`, `2`, `3`, and other integers) | C treats `< 1` as binary and `>= 1` as string, except delete frees strdup keys only when `mode == 1` | [x] |
| 21 | size-taking exports | zero element/key sizes, zero length, and arithmetic-overflow/oversized lengths | preserve the exact C return/sentinel or process-failure behavior for the same case | [x] |

Coverage notes:

- Rows 9, 13, and 14 are tested with process-isolated corruption of the opaque
  hash state; C and Rust both abort with the same signal.
- Rows 10–12 and 15–18 are internal invariants whose false conditions cannot be
  produced by a defined public-API input. Their assertion sites are executed by
  the randomized valid workflows, and equivalent assertions are present in the
  Rust translation (row 12 remains a no-op tautology in both languages).
- Rows 19 and 21 use process-isolated probes so null/oversized undefined inputs
  cannot terminate the main test process.
