# Error-surface table

The C API has no error enum or `RETURN_ERROR`/`return NULL` failure API. Its observable rejection surface is made of null special cases, missing-key sentinels, and internal assertions. Rows 1–10 are directly constructible through exported calls. Corrupt-state probes exercise rows 11, 13, and 14 in isolated child processes; rows 12 and 15 are tautological postcondition assertions, retained exactly in Rust and exercised at every boundary that establishes them. Rows 16–20 cover the generic FFI boundaries required by Phase C where the C implementation has no defensive check.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `stbds_hmfree_func` | `a == NULL` | returns immediately; no allocation is freed | [x] |
| 2 | `stbds_hmget_key_ts` | `a == NULL` | allocates the zero default entry, returns the hash-visible pointer, writes `temp = -1` | [x] |
| 3 | `stbds_hmget_key_ts` / `stbds_hmget_key` | array exists but `hash_table == NULL` | returns the same hash-visible pointer and reports/stores `-1` | [x] |
| 4 | `stbds_hmget_key_ts` / `stbds_hmget_key` | hash table exists but the key is absent and the first probe segment reaches `STBDS_HASH_EMPTY` | returns the same pointer and reports/stores `-1` | [x] |
| 5 | `stbds_hmget_key_ts` / `stbds_hmget_key` | hash table exists but the key is absent and the wrapped probe segment reaches `STBDS_HASH_EMPTY` | returns the same pointer and reports/stores `-1` | [x] |
| 6 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` | [x] |
| 7 | `stbds_hmdel_key` | array exists but `hash_table == NULL` | returns the same pointer and stores deletion result `temp = 0` | [x] |
| 8 | `stbds_hmdel_key` | hash table exists but key is absent (`slot < 0`) | returns the same pointer and stores deletion result `temp = 0` | [x] |
| 9 | mode-bearing hash APIs | `mode = -1`, one below binary mode `0` | accepted as binary mode (`mode >= 1` is false) | [x] |
| 10 | mode-bearing hash APIs | `mode = 2`, `INT_MAX`, or another value above string mode `1` | accepted as string mode (`mode >= 1` is true); `stbds_shmode_func` stores the low unsigned byte | [x] |
| 11 | internal `stbds_make_hash_index` via map growth | `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 12 | `stbds_hmput_key` | post-growth condition `new_length > capacity` | `assert` abort | [x] |
| 13 | `stbds_hmdel_key` | found slot is outside `table->slot_count` | `assert` abort | [x] |
| 14 | `stbds_hmdel_key` | moved entry cannot be found, or its bucket index is not the former final index | `assert` abort | [x] |
| 15 | `stbds_stralloc` | post-allocation `len > arena.remaining` | `assert` abort | [x] |
| 16 | `stbds_arrfreef` | `a == NULL` (no C null guard) | invalid free / process termination | [x] |
| 17 | `stbds_hash_string` | `str == NULL` | invalid dereference / process termination | [x] |
| 18 | `stbds_hash_bytes` | `p == NULL && len > 0`; boundary `len == 0` is valid | invalid dereference / process termination for positive length | [x] |
| 19 | `stbds_stralloc` | `arena == NULL` or `str == NULL` | invalid dereference / process termination | [x] |
| 20 | length-taking allocation APIs | zero length, very large length, and one-past-capacity arithmetic boundaries | exact C return/sentinel when allocation succeeds; otherwise the same process failure class | [x] |

Source constants/ranges included in the tests:

- hash bucket length/minimum slot count: `8`
- initial array minimum capacity: `4`
- hash sentinels: empty `0`, deleted `1`
- hash index sentinels: empty `-1`, deleted `-2`
- string modes: none `0`, default `1`, strdup `2`, arena `3`
- string arena block minimum `512`, maximum `1 << 20`
