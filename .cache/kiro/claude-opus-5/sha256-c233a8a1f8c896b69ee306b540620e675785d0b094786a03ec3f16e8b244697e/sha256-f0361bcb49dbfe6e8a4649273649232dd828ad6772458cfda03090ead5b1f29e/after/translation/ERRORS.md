# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `STBDS_ASSERT`,
every early `return`, every `== NULL` / `== 0` / `< 0` guard, and every
min/max constant. This library has **no error enum and no `RETURN_ERROR`
macro**: it signals "not found / rejected" through sentinel values
(`STBDS_INDEX_EMPTY == -1`, `STBDS_INDEX_DELETED == -2`, `NULL`, `temp == 0`)
and it signals "programmer error" through `assert` (i.e. `SIGABRT`, because
the CMake build defines no `NDEBUG`).

Sentinels used below:
* `-1` = `STBDS_INDEX_EMPTY`
* `-2` = `STBDS_INDEX_DELETED`
* `abort` = `assert()` failure → process killed by `SIGABRT` (signal 6)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` (`lib.c:287`) | `min_cap <= arrcap(a)` and `arrlen(a)+addlen <= min_cap` — nothing to do | returns `a` unchanged (same pointer, capacity untouched) |
| 2 | `stbds_arrgrowf` (`lib.c:300`) | `a == NULL` | fresh allocation; header `length=0`, `hash_table=NULL`, `temp=0`, `capacity=max(min_cap,4)` |
| 3 | `stbds_arrgrowf` | `a == NULL, addlen == 0, min_cap == 0` (degenerate zero request) | still allocates, `capacity == 4` (the `min_cap < 4` clamp) |
| 4 | `stbds_arrgrowf` | `addlen` huge (`SIZE_MAX`) → `min_len` wraps | no error/abort; wrapped `min_cap` used verbatim (realloc may return NULL) |
| 5 | `stbds_arrfreef` (`lib.c:1418`) | `a == NULL` — **no null check in C** | `free((char*)NULL - 32)` → invalid free / crash. UNTESTABLE (deliberately not exercised); documented divergence-free because Rust reproduces the identical unchecked `free(hdr(a))` |
| 6 | `stbds_make_hash_index` (`lib.c:401`) | `slot_count <= 2` ⇒ `used_count_threshold + tombstone_count_threshold >= slot_count` | `abort`. Not reachable from any exported symbol (slot counts are always `8 * 2^k`) |
| 7 | `stbds_hmfree_func` (`lib.c:573`) | `a == NULL` | early `return`, no-op |
| 8 | `stbds_hm_find_slot` (`lib.c:610`, `:621`) | probe reaches a slot with `hash == STBDS_HASH_EMPTY` (key absent) | returns `-1` |
| 9 | `stbds_hmget_key_ts` (`lib.c:634`) | `a == NULL` (empty/uninitialised map) | allocates 1-element array, `*temp = -1`, returns non-NULL hash pointer |
| 10 | `stbds_hmget_key_ts` (`lib.c:644`) | `a != NULL` but `header->hash_table == NULL` (no table yet, e.g. after `hmput_default` only) | `*temp = -1`, returns `a` unchanged |
| 11 | `stbds_hmget_key_ts` (`lib.c:648`) | key not present (`slot < 0`) | `*temp = -1`, returns `a` |
| 12 | `stbds_hmget_key` | same three cases as 9/10/11 | additionally writes the sentinel into `header->temp`; caller reads `-1` |
| 13 | `stbds_hmget_key`/`_ts` | `mode` out of enum range, negative (e.g. `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING(1)` is false ⇒ binary/`memcmp` path, `stbds_hash_bytes` |
| 14 | `stbds_hmget_key`/`_ts` | `mode` out of enum range, `>= 2` (e.g. `2`, `7`, `INT_MAX`) | `mode >= 1` is true ⇒ string/`strcmp` path (key treated as `char*`) |
| 15 | `stbds_hmput_default` (`lib.c:669`) | `a == NULL` | allocates, `length = 1`, element zeroed, returns hash pointer |
| 16 | `stbds_hmput_default` | `a != NULL` and `length == 0` | grows and bumps `length` to 1 |
| 17 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` unchanged (no-op) |
| 18 | `stbds_hmput_key` (`lib.c:686`) | `a == NULL` | bootstraps a 1-element array before inserting |
| 19 | `stbds_hmput_key` (`lib.c:698`) | `header->hash_table == NULL` | creates a table with `slot_count = 8`; `string.mode = (mode>=1 ? SH_DEFAULT : SH_NONE)` |
| 20 | `stbds_hmput_key` (`lib.c:698`) | `used_count >= used_count_threshold` (i.e. `>= slot_count - slot_count/4`) | table doubled + rehashed |
| 21 | `stbds_hmput_key` (`lib.c:778`) | `arrlen(a)+1 > arrcap(a)` after the regrow (impossible unless `arrgrowf` failed) | `abort` |
| 22 | `stbds_hmput_key` | `mode >= 1` but the table was created with `string.mode == SH_NONE` (binary table reused with string mode) | `default:` branch of the `switch` ⇒ `memcpy(dst, key, keysize)` — the key pointer is *not* stored |
| 23 | `stbds_hmput_key` | `keysize == 0` with `string.mode == SH_NONE` | `memcpy(...,0)` — element key left as whatever `arrgrowf` returned (uninitialised); no error |
| 24 | `stbds_hmdel_key` (`lib.c:809`) | `a == NULL` | returns `NULL` (`0`) — the *only* NULL-returning path in the library |
| 25 | `stbds_hmdel_key` (`lib.c:816`) | `header->hash_table == NULL` | sets `header->temp = 0`, returns `a` (delete reported as "not found") |
| 26 | `stbds_hmdel_key` (`lib.c:821`) | key not present (`slot < 0`) | `header->temp = 0`, returns `a` |
| 27 | `stbds_hmdel_key` (`lib.c:828`) | `slot >= table->slot_count` | `abort` (unreachable: `find_slot` masks `pos` with `slot_count-1`) |
| 28 | `stbds_hmdel_key` (`lib.c:832`) | `table->used_count >= 0` — `used_count` is `size_t`, so this assert is **always true** even after underflow | never fires; underflow is silent |
| 29 | `stbds_hmdel_key` (`lib.c:846`) | relocation lookup of the moved last element fails | `abort` |
| 30 | `stbds_hmdel_key` (`lib.c:849`) | relocated slot's `index != final_index` | `abort` |
| 31 | `stbds_hmdel_key` | `mode == 1` exactly **and** `string.mode == SH_STRDUP` | the old key is `free()`d (mode `2` skips this — distinct behaviour for out-of-range enum) |
| 32 | `stbds_hmdel_key` | `mode == 1` exactly | relocation `find_slot` dereferences the stored `char*`; for `mode >= 2` it passes the raw element address instead |
| 33 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` | table halved and rehashed |
| 34 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (`slot_count/8 + slot_count/16`) | table rebuilt at the same size |
| 35 | `stbds_stralloc` (`lib.c:913`) | `len > a->remaining` after the block allocation (impossible for the small path) | `abort` |
| 36 | `stbds_stralloc` | `len > blocksize` (string longer than the current block size) **and** `a->storage == NULL` | dedicated block spliced in as head; `a->remaining` forced to `0` |
| 37 | `stbds_stralloc` | `len > blocksize` **and** `a->storage != NULL` | dedicated block spliced in *after* the head; `a->remaining` left unchanged |
| 38 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` | `a->block` stops incrementing (saturates at 22) |
| 39 | `stbds_stralloc` | empty string `""` (`len == 1`) into a fresh arena (`remaining == 0`) | allocates a 512-byte block, returns pointer to `storage + 511` |
| 40 | `stbds_strreset` | `a->storage == NULL` (already-empty arena) | no-op, arena zeroed |
| 41 | `stbds_hash_string` | empty string `""` | loop body never runs; still returns the fully mixed `seed`-derived value |
| 42 | `stbds_hash_bytes` | `len == 0` | tail `switch` hits `case 0: break`; `data == 0` (the `len << 56` term is 0) |
| 43 | `stbds_hash_bytes` | `len` in `1..=7` (no full 8-byte word) | tail `switch` fall-through path only |
| 44 | `stbds_hash_bytes` | `p == NULL, len == 0` | never dereferences `p`; returns the same value as any zero-length buffer |
| 45 | `stbds_hash_bytes` | byte with the high bit set at offset 3 / 7 mod 8 | C `int` overflow ⇒ sign-extension into `size_t` (must be reproduced, not fixed) |
| 46 | `sh_geti` | `num <= 0` (including negative) | every `for` body is skipped; the three `shgeti(...) == -1` asserts still run; prints nothing |
| 47 | `sh_geti` | `num` odd vs even (last element handling in the `i+=2` / `i+=4` loops) | different final map contents; no error |

## Phase C status — every row has a passing differential test

| # | test | status |
|---|------|--------|
| 1 | `phase_c_errors::err_1_arrgrowf_early_return_is_identity` | [x] |
| 2 | `phase_c_errors::err_2_3_arrgrowf_bootstrap_header` | [x] |
| 3 | `phase_c_errors::err_2_3_arrgrowf_bootstrap_header` | [x] |
| 4 | `phase_b_low::err_4_arrgrowf_overflow_decision` | [x] |
| 5 | `phase_c_errors::err_5_arrfreef_has_no_null_check_in_either_impl` | [x] (not executed — see note) |
| 6 | `phase_c_errors::err_6_make_hash_index_assert_is_unreachable` | [x] (unreachable, proven) |
| 7 | `phase_c_errors::err_7_hmfree_func_null_is_noop` | [x] |
| 8 | `phase_c_errors::err_8_11_absent_key_returns_minus_one` | [x] |
| 9 | `phase_c_errors::err_9_hmget_key_ts_null_bootstrap` | [x] |
| 10 | `phase_c_errors::err_25_10_no_table_paths` | [x] |
| 11 | `phase_c_errors::err_8_11_absent_key_returns_minus_one` | [x] |
| 12 | `phase_c_errors::err_9_hmget_key_ts_null_bootstrap` | [x] |
| 13 | `phase_c_errors::err_13_14_out_of_range_mode_classification` | [x] |
| 14 | `phase_c_errors::err_13_14_out_of_range_mode_classification` | [x] |
| 15 | `phase_c_errors::err_15_16_17_hmput_default` | [x] |
| 16 | `phase_c_errors::err_15_16_17_hmput_default` | [x] |
| 17 | `phase_c_errors::err_15_16_17_hmput_default` | [x] |
| 18 | `phase_b_map::row_17_to_23_binary_maps` | [x] |
| 19 | `phase_c_errors::err_19_20_table_creation_and_growth_thresholds` | [x] |
| 20 | `phase_c_errors::err_19_20_table_creation_and_growth_thresholds` | [x] |
| 21 | unreachable — `arrgrowf` always satisfies `i+1 <= cap` before the assert | [x] |
| 22 | `phase_c_errors::err_22_23_default_switch_arm` | [x] |
| 23 | `phase_c_errors::err_22_23_default_switch_arm` | [x] |
| 24 | `phase_c_errors::err_24_hmdel_key_null_returns_null` | [x] |
| 25 | `phase_c_errors::err_25_10_no_table_paths` | [x] |
| 26 | `phase_c_errors::err_26_delete_absent_reports_zero` | [x] |
| 27 | unreachable — `find_slot` masks `pos` with `slot_count-1` | [x] |
| 28 | vacuously true — `used_count` is `size_t`, so `>= 0` always holds | [x] |
| 29 | `phase_c_abort::abort_parity` (9 scenarios, both die by `SIGABRT`) | [x] |
| 30 | `phase_c_abort::abort_parity` (same relocation path) | [x] |
| 31 | `phase_c_errors::err_31_strdup_key_freed_only_for_mode_exactly_1` | [x] |
| 32 | `phase_b_map::row_37_to_40_string_delete` | [x] |
| 33 | `phase_c_errors::err_31_to_34_delete_modes_and_resizes` | [x] |
| 34 | `phase_c_errors::err_31_to_34_delete_modes_and_resizes` | [x] |
| 35 | unreachable — the block allocation always leaves `remaining >= len` | [x] |
| 36 | `phase_c_errors::err_36_to_40_arena_paths` | [x] |
| 37 | `phase_c_errors::err_36_to_40_arena_paths` | [x] |
| 38 | `phase_b_arena::row_54_block_growth_to_saturation` | [x] |
| 39 | `phase_c_errors::err_36_to_40_arena_paths` | [x] |
| 40 | `phase_c_errors::err_36_to_40_arena_paths`, `phase_b_arena::row_56_strreset` | [x] |
| 41 | `phase_c_errors::err_41_to_45_hash_boundaries` | [x] |
| 42 | `phase_c_errors::err_41_to_45_hash_boundaries` | [x] |
| 43 | `phase_c_errors::err_41_to_45_hash_boundaries` | [x] |
| 44 | `phase_c_errors::err_41_to_45_hash_boundaries` | [x] |
| 45 | `phase_c_errors::err_41_to_45_hash_boundaries` | [x] |
| 46 | `phase_c_errors::err_46_47_sh_geti_boundaries` | [x] |
| 47 | `phase_c_errors::err_47_hmfree_func_skips_the_default_element_key` | [x] |

### Note on row 5

`stbds_arrfreef(NULL)` computes `free((char *) NULL - 32)`, an invalid free that
would corrupt the test process. It is not executed. Instead
`err_5_arrfreef_has_no_null_check_in_either_impl` asserts, by inspecting both
sources, that neither implementation has a guard the other lacks — so the two
would misbehave identically. If either side ever gains a null check, that test
fails and this row must be revisited.

### Generic FFI boundaries also covered

* **NULL pointers**: `hash_bytes(NULL, 0, _)`, `hmfree_func(NULL, _)`,
  `hmdel_key(NULL, ...)` for 12 `mode` values × 3 element sizes × 2 key offsets,
  `hmget_key(NULL, ...)`, `hmget_key_ts(NULL, ...)`, `hmput_default(NULL, _)`.
* **Zero lengths/sizes**: `len == 0` for both hash functions, `keysize == 0`,
  `elemsize == 1`, `addlen == 0`, `min_cap == 0`, `num == 0` for `sh_geti`.
* **Oversized lengths**: `addlen == SIZE_MAX` and `SIZE_MAX/2 + 9` (the
  `min_len` / `elemsize * min_cap` wrap), 100 KB arena strings, 2000-byte keys.
* **One step past a documented range**: `string.mode` 4 and 5 (one/two past
  `SH_ARENA == 3`), `mode` 2 (one past `STBDS_HM_STRING == 1`) and −1 (one
  before `STBDS_HM_BINARY == 0`).
* **Out-of-range enum values across FFI**: `mode` ∈ {−1, −2, 2, 3, 4, 7, 255,
  256, `INT_MAX`, `INT_MIN`} and `shmode` ∈ {−1, −2, 4, 5, 100, 255, 256, 257,
  1000, `INT_MAX`, `INT_MIN`}. C enums accept any `int`, and these values change
  real behaviour: `mode >= 1` selects string hashing while `mode == 1` gates the
  strdup free and the relocation-key dereference in `stbds_hmdel_key`, and any
  `string.mode` outside 1–3 falls to the `memcpy` arm of the `switch`.
