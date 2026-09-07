# Error-surface table

Rows are mechanically derived from every `return -1`, `return 0`/null rejection,
null/range check, and `STBDS_ASSERT` in `c_src/src/lib.c`. Internal assertions
are included even where the trigger requires corrupting private state.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| E01 | `stbds_hm_find_slot` via get/delete | probe reaches an empty slot in the suffix of the current bucket | index `-1` | [x] |
| E02 | `stbds_hm_find_slot` via get/delete | wrapped probe reaches an empty slot before the initial bucket offset | index `-1` | [x] |
| E03 | `stbds_hmget_key_ts` | map pointer is null | allocate default entry, write `temp = -1` | [x] |
| E04 | `stbds_hmget_key_ts` | map exists but has no hash table | return same map, write `temp = -1` | [x] |
| E05 | `stbds_hmget_key_ts` | key is absent from a populated table | return same map, write `temp = -1` | [x] |
| E06 | `stbds_hmdel_key` | map pointer is null | return null | [x] |
| E07 | `stbds_hmdel_key` | map exists but has no hash table | return same map, deletion flag `temp = 0` | [x] |
| E08 | `stbds_hmdel_key` | key is absent from a populated table | return same map, deletion flag `temp = 0` | [x] |
| E09 | `stbds_make_hash_index` | thresholds do not satisfy `used + tombstone < slot_count` | assertion abort | [x] |
| E10 | `stbds_hmput_key` | post-growth capacity is smaller than `old_length + 1` | assertion invariant preserved across boundary/growth stress | [x] |
| E11 | `stbds_hmdel_key` | located slot is outside `table->slot_count` | assertion abort | [x] |
| E12 | `stbds_hmdel_key` | decrementing `used_count` violates its asserted invariant | assertion is always true because `used_count` is unsigned | [x] |
| E13 | `stbds_hmdel_key` | moved final element cannot be found in the hash table | assertion abort | [x] |
| E14 | `stbds_hmdel_key` | moved element's hash slot does not contain `final_index` | assertion abort | [x] |
| E15 | `stbds_stralloc` | allocation path leaves `len > arena.remaining` | assertion invariant preserved across boundary/cap stress | [x] |
| E16 | `sh_geti` | initial lookup of `"foo"` does not return `-1` | assertion abort | [x] |
| E17 | `sh_geti` | lookup of `"foo"` after string-map mode setup does not return `-1` | assertion abort | [x] |
| E18 | `sh_geti` | lookup of `"foo"` after setting default does not return `-1` | assertion abort | [x] |
| E19 | `sh_geti` | odd-key lookup does not return default `-2` | assertion abort | [x] |
| E20 | `sh_geti` | present even-key lookup does not return `i * 3` | assertion abort | [x] |
| E21 | `sh_geti` | post-delete key with `i & 3 != 0` does not return `-2` | assertion abort | [x] |
| E22 | `sh_geti` | retained key with `i & 3 == 0` does not return `i * 3` | assertion abort | [x] |
| E23 | `sh_geti` | lookup after deleting every key does not return `-2` | assertion abort | [x] |
| E24 | `stbds_hash_string` | string pointer is null | process faults while dereferencing it | [x] |
| E25 | `stbds_hash_bytes` | data pointer is null and length is nonzero | process faults while reading it | [x] |
| E26 | `stbds_hmget_key_ts` | `temp` pointer is null | process faults while writing it | [x] |
| E27 | `stbds_stralloc` | arena pointer is null | process faults while reading it | [x] |
| E28 | `stbds_stralloc` | string pointer is null | process faults in `strlen` | [x] |
| E29 | `stbds_strreset` | arena pointer is null | process faults while reading it | [x] |
| E30 | hash-map APIs | enum/mode is one step below valid binary mode (`-1`) | C treats it as binary mode | [x] |
| E31 | hash-map APIs | enum/mode is one step above arena mode (`4`) | C uses string hashing and the switch default copy path | [x] |
| E32 | length-taking APIs | zero element/key length | preserve C's zero-byte allocation/hash/copy behavior | [x] |
| E33 | length-taking APIs | oversized length (`SIZE_MAX`) | preserve C overflow/allocation rejection or process termination | [x] |
