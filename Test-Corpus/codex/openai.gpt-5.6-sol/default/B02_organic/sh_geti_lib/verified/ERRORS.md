# Error Surface

This table is derived from every sentinel-return branch and every
`STBDS_ASSERT` in `src/lib.c`. The implementation has no error enum and no
`RETURN_ERROR`/`return NULL` branch other than the cases below. Rows E09-E20
are internal consistency assertions, not ordinary defensive API validation;
the C result if their condition is false is process termination through
`assert`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|---|
| E01 | `stbds_hmget_key_ts` | `a == NULL` (empty map lookup) | allocate the default element, write `temp = -1`, return map pointer | [x] |
| E02 | `stbds_hmget_key_ts` | map exists but `header.hash_table == NULL` | write `temp = -1`, return the same map pointer | [x] |
| E03 | `stbds_hmget_key_ts` | hash table exists but key is absent | write `temp = -1`, return the same map pointer | [x] |
| E04 | `stbds_hmget_key` | key is absent | return map and store `header.temp = -1` | [x] |
| E05 | `stbds_hmdel_key` | `a == NULL` | return `NULL` | [x] |
| E06 | `stbds_hmdel_key` | map exists but `header.hash_table == NULL` | return the same pointer and store `header.temp = 0` | [x] |
| E07 | `stbds_hmdel_key` | hash table exists but key is absent | return the same pointer and store `header.temp = 0` | [x] |
| E08 | `stbds_hmfree_func` | `a == NULL` | return without action | [x] |
| E09 | `stbds_make_hash_index` (via put/mode APIs), line 401 | `used_count_threshold + tombstone_count_threshold >= slot_count` | assertion failure (`SIGABRT`) | [x] |
| E10 | `stbds_hmput_key`, line 778 | after growth, `i + 1 > array capacity` | assertion failure (`SIGABRT`) | [x] |
| E11 | `stbds_hmdel_key`, line 828 | found slot is outside `table.slot_count` | assertion failure (`SIGABRT`) | [x] |
| E12 | `stbds_hmdel_key`, line 832 | decrement makes unsigned `used_count < 0` | assertion failure; condition is tautologically unreachable for `size_t` | [x] |
| E13 | `stbds_hmdel_key`, line 846 | moved final element cannot be found in the table | assertion failure (`SIGABRT`) | [x] |
| E14 | `stbds_hmdel_key`, line 849 | moved element's bucket index is not `final_index` | assertion failure (`SIGABRT`) | [x] |
| E15 | `stbds_stralloc`, line 913 | after block selection, `len > arena.remaining` | assertion failure (`SIGABRT`) | [x] |
| E16 | `sh_geti`, line 956 | initial lookup of `"foo"` does not return `-1` | assertion failure (`SIGABRT`) | [x] |
| E17 | `sh_geti`, line 961 | lookup after selecting strdup/arena mode does not return `-1` | assertion failure (`SIGABRT`) | [x] |
| E18 | `sh_geti`, line 963 | lookup after installing default does not return `-1` | assertion failure (`SIGABRT`) | [x] |
| E19 | `sh_geti`, lines 971-972 | pre-delete lookup differs from default `-2` for odd keys or `i*3` for even keys | assertion failure (`SIGABRT`) | [x] |
| E20 | `sh_geti`, lines 976-977 | selective-delete lookup differs from default except keys divisible by four | assertion failure (`SIGABRT`) | [x] |
| E21 | `sh_geti`, line 981 | lookup after all deletions differs from default `-2` | assertion failure (`SIGABRT`) | [x] |

## Generic FFI Boundaries

The C implementation does not validate pointer/length pairs before
dereferencing. Null `str`, null hash input with nonzero length, null `temp`,
zero `elemsize`, arithmetic-overflow sizes, and malformed opaque map pointers
therefore have undefined behavior rather than a defined rejection result.
They are recorded here rather than assigned an invented expected value.

Mode is a C `int`, not a validating enum. The actual branches are `mode < 1`
(binary), `mode >= 1` (string comparison/hash), and, during deletion,
`mode == 1` (string-specific ownership cleanup). Out-of-range values are
covered as configuration behavior where the C call has defined memory access.

## Differential Test Mapping

- E01-E08: exact pointer/sentinel/temp results in
  `valid_binary_map_surface_v20_v31_and_e01_e08`.
- E09, E13, E14: isolated child-process calls in
  `error_assertion_surface_e09_e21`; C and Rust both terminate with the same
  signal for malformed growth metadata, out-of-range moved-string mode, and a
  malformed moved bucket index.
- E10-E12, E15: the conditions are guaranteed or mathematically unreachable
  after the immediately preceding C branches. Their assertion-bearing paths
  run across randomized V20-V31 and V40-V43 inputs.
- E16-E21: exercised by randomized `sh_geti` workflows in V45-V46; these
  assertions test the implementation's own consistency and have no external
  input that directly falsifies only the assertion condition.
