# Error Surface

The C API has no error enum and no conventional error return. Its observable
rejections are `STBDS_INDEX_EMPTY` (`-1`) for missing lookups and null/no-op
returns for deletions/frees. Assertions guard private data-structure invariants;
they are listed because every C `STBDS_ASSERT` was mechanically inspected, but
they are not reachable from an uncorrupted public-API state.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | allocate the default entry and write `-1` to `temp` | [x] |
| 2 | `stbds_hmget_key_ts` | map exists but `hash_table == NULL` | return the same map and write `-1` to `temp` | [x] |
| 3 | `stbds_hmget_key_ts` | key is absent and the first probe scan reaches `STBDS_HASH_EMPTY` | return the same map and write `-1` to `temp` | [x] |
| 4 | `stbds_hmget_key_ts` | key is absent and the wrapped probe scan reaches `STBDS_HASH_EMPTY` | return the same map and write `-1` to `temp` | [x] |
| 5 | `stbds_hmget_key` | key is absent | return the map and store `-1` in the hidden header `temp` | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| 7 | `stbds_hmdel_key` | map exists but `hash_table == NULL` | return the same map, hidden header `temp = 0` | [x] |
| 8 | `stbds_hmdel_key` | key is absent | return the same map, hidden header `temp = 0` | [x] |
| 9 | `stbds_hmfree_func` | `a == NULL` | return without action | [x] |
| 10 | `stbds_make_hash_index` (via put/mode APIs) | invariant `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort; unreachable for internally generated power-of-two slot counts | [x] |
| 11 | `stbds_hmput_key` | invariant `new_length > capacity` after requested growth | `assert` abort; reachable only after allocator failure/private-header corruption | [x] |
| 12 | `stbds_hmdel_key` | invariant `slot >= table->slot_count` | `assert` abort; reachable only after private-table corruption | [x] |
| 13 | `stbds_hmdel_key` | invariant `used_count` underflows after finding a supposedly live key | `assert` abort; reachable only after private-table corruption | [x] |
| 14 | `stbds_hmdel_key` | moved final entry cannot be found (`slot < 0`) | `assert` abort; reachable only after private-table/entry corruption | [x] |
| 15 | `stbds_hmdel_key` | moved final entry's bucket index is not `final_index` | `assert` abort; reachable only after private-table corruption | [x] |
| 16 | `stbds_stralloc` | invariant `strlen(str) + 1 > a->remaining` after allocation path | `assert` abort; reachable only after allocator failure/private-arena corruption | [x] |
| 17 | `arr_ins` | inserted value is not present at insertion index | `assert` abort; internal self-test invariant | [x] |
| 18 | `arr_ins` | for insertion indexes 0 through 3, element 4 is not `4` | `assert` abort; internal self-test invariant | [x] |

Rows 1 through 9 are covered by `map_lookup_rejection_sentinels_match`,
`wrapped_probe_missing_key_sentinel_matches`, and the null-map free calls.
Rows 10, 14, and 15 are made reachable by controlled private-state corruption
in `reachable_assertion_guards_abort_identically`; C and Rust terminate with
the same signal. Rows 11 through 13 and 16 through 18 are allocator-independent
or mathematical internal invariants that cannot be falsified by API input
without an earlier undefined memory failure. Their guards are exercised by the
randomized growth/deletion tests, arena tests, and randomized `arr_ins` calls.
The invalid pointer and oversized-length process outcomes are covered by
`invalid_pointer_boundaries_fail_identically`.

## FFI Boundary Conditions

The C source does not validate pointer/size contracts. Null data with zero
length is accepted by `stbds_hash_bytes`; null map/array pointers are accepted
only where rows 1, 6, and 9 or valid construction paths say so. Null required
output/key/string/arena pointers, nonzero lengths with null data, zero element
sizes used as real containers, arithmetic-overflow lengths, and pointers not
created by this library have undefined C behavior rather than an error result.
Differential tests must isolate such calls in subprocesses when exercised.

The integer `mode` arguments are not C enums and have no rejected values:
comparisons use `mode >= STBDS_HM_STRING`, deletion uses
`mode == STBDS_HM_STRING`, and `stbds_shmode_func` truncates mode to
`unsigned char`. Out-of-range values are therefore valid branch inputs and are
covered in `CONFIGS.md`.
