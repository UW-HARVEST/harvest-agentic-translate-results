# Error surface

The C API declares no error enum and returns no conventional error code.
Caller-visible rejection is represented by null/unchanged pointers and the
`STBDS_INDEX_EMPTY` (`-1`) lookup sentinel. Assertions below are included
mechanically even when valid public calls cannot violate the invariant.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| E01 | `stbds_hmget_key_ts` | `a == NULL` (lookup in a null map) | allocate an empty/default entry and store `-1` in `*temp` | [x] |
| E02 | `stbds_hmget_key_ts` | map exists but `header->hash_table == NULL` | return `a` unchanged and store `-1` in `*temp` | [x] |
| E03 | `stbds_hmget_key_ts` / `stbds_hmget_key` | hash table exists but key search reaches an empty slot in the first probe segment | return map and report index `-1` | [x] |
| E04 | `stbds_hmget_key_ts` / `stbds_hmget_key` | hash table exists but key search reaches an empty slot in the wrapped probe segment | return map and report index `-1` | [x] |
| E05 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| E06 | `stbds_hmdel_key` | map exists but `header->hash_table == NULL` | return `a` unchanged with header `temp == 0` | [x] |
| E07 | `stbds_hmdel_key` | hash table exists but key is absent (`stbds_hm_find_slot < 0`) | return `a` unchanged with header `temp == 0` | [x] |
| E08 | `stbds_hmfree_func` | `a == NULL` | return normally without action | [x] |
| E09 | `stbds_make_hash_index` (via put/mode APIs) | internal invariant `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| E10 | `stbds_hmput_key` | internal post-grow invariant `(size_t)i + 1 > arrcap(a)` | `assert` abort | [x] |
| E11 | `stbds_hmdel_key` | located `slot >= table->slot_count` | `assert` abort | [x] |
| E12 | `stbds_hmdel_key` | internal post-decrement invariant `table->used_count < 0` | `assert` abort; condition is impossible because the field is `size_t` | [x] |
| E13 | `stbds_hmdel_key` | moved final element cannot be found (`slot < 0`) | `assert` abort | [x] |
| E14 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | `assert` abort | [x] |
| E15 | `stbds_stralloc` | internal post-allocation invariant `len > a->remaining` | `assert` abort | [x] |
| E16 | pointer-taking exports | required pointer is null where C performs an unconditional dereference (`stbds_arrfreef`, nonzero-length hash input, string input, arena input, key input, or `temp`) | undefined behavior, normally process `SIGSEGV`; compare in isolated child processes | [x] |
| E17 | length-taking exports | zero length (`stbds_hash_bytes`) or zero `addlen`/`min_cap` (`stbds_arrgrowf`) | valid boundary: hash empty input or allocate minimum capacity as selected by C | [x] |
| E18 | length-taking exports | length/capacity arithmetic exceeds `SIZE_MAX` | C unsigned arithmetic wraps; subsequent allocation/dereference may return a value, fail, or crash, so compare in isolated child processes | [x] |
| E19 | mode-taking exports | mode is one step outside named constants (`-1` or `4`) | no enum rejection: integer comparisons/cast are applied exactly as in C | [x] |

Rows E09-E15 describe internal consistency checks, not supported caller inputs.
Tests reach the externally constructible cases and document the impossible or
allocator-dependent cases explicitly rather than inventing a C error code.
`internal_assert_surface_is_preserved` audits E10-E12/E15, whose violating
conditions are unreachable through a valid public state; their non-aborting
paths are exercised repeatedly through both shared objects.
