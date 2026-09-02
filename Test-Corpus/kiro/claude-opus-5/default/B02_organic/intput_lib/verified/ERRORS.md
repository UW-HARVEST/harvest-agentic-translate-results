# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from every `STBDS_ASSERT`, every `return -1` / `return 0` /
`return NULL` early-out, every explicit range / null / mode test, and every
min/max constant in `c_src/src/lib.c`.

Grep basis:

```
$ grep -n 'STBDS_ASSERT\|return -1\|return 0;\|return NULL\|if (a == NULL\|if (table == 0\|slot < 0\|mode >=\|mode ==\|_MIN\|_MAX' c_src/src/lib.c
```

`STBDS_ASSERT` is `assert` and the CMake build defines no `NDEBUG`, so a failing
assert calls `abort()` → the process dies with **SIGABRT (6)**. The Rust side
must die with the same signal.

Constants that bound behaviour: `STBDS_BUCKET_LENGTH 8`,
`STBDS_INDEX_EMPTY -1`, `STBDS_INDEX_DELETED -2`, `STBDS_HASH_EMPTY 0`,
`STBDS_HASH_DELETED 1`, `STBDS_HM_BINARY 0`, `STBDS_HM_STRING 1`,
`STBDS_SH_NONE/DEFAULT/STRDUP/ARENA 0..3`,
`STBDS_STRING_ARENA_BLOCKSIZE_MIN 512`, `STBDS_STRING_ARENA_BLOCKSIZE_MAX 1<<20`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` (line 285) | `min_cap <= stbds_arrcap(a)` — request already satisfied | early `return a`: the *same* pointer, header untouched (no realloc) |
| 2 | `stbds_arrgrowf` | `a == NULL, addlen == 0, min_cap == 0` — degenerate zero request | `min_cap (0) <= stbds_arrcap(NULL) (0)` ⇒ the row-1 early-out fires and **`NULL` is returned** (no allocation at all) |
| 2b | `stbds_arrgrowf` | `a == NULL`, `addlen + min_cap` resolves to `< 4` but `> 0` | `min_cap` forced to 4; fresh block, `length=0, capacity=4, hash_table=NULL, temp=0` |
| 3 | `stbds_hm_find_slot` (line 610, forward half of bucket scan) | probed slot has `hash == STBDS_HASH_EMPTY` before a hash match → key absent | `return -1` |
| 4 | `stbds_hm_find_slot` (line 621, wrap-around half of bucket scan) | same, but found in the `z < limit` half | `return -1` |
| 5 | `stbds_hmget_key_ts` (line 634) | `a == NULL` (no map yet) | allocates a 1-element zeroed map, `*temp = STBDS_INDEX_EMPTY (-1)`, returns non-NULL hash pointer |
| 6 | `stbds_hmget_key_ts` (line 646) | `a != NULL` but `header->hash_table == 0` (array made by `arrgrowf`/`hmput_default`, never `hmput_key`) | `*temp = -1`, returns `a` unchanged |
| 7 | `stbds_hmget_key_ts` (line 649) | key not present (`stbds_hm_find_slot` < 0) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` |
| 8 | `stbds_hmget_key` | any of rows 5–7 | additionally stores that `-1` into `stbds_header(raw_a)->temp` |
| 9 | `stbds_hmget_key_ts` / `stbds_hmget_key` | `mode` out of enum range, negative (e.g. `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING` false → **binary** `memcmp` path, identical to `mode == 0` |
| 10 | `stbds_hmget_key_ts` / `stbds_hmget_key` | `mode` out of enum range, `>= 2` (e.g. `2`, `7`, `INT_MAX`) | `mode >= STBDS_HM_STRING` true → **string** `strcmp`/`hash_string` path, identical to `mode == 1` |
| 11 | `stbds_hmput_default` (line 666) | `a == NULL` | allocates 1-element zeroed map, returns hash pointer |
| 12 | `stbds_hmput_default` | `a != NULL` and `header(raw_a)->length == 0` | grows/zeroes element 0, `length` becomes 1 |
| 13 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` unchanged (no allocation) |
| 14 | `stbds_hmput_key` (line 683) | `a == NULL` | bootstraps a 1-element zeroed array before doing the insert |
| 15 | `stbds_hmput_key` (line 778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | vacuously satisfied — `arrgrowf` above guarantees capacity; not reachable through the public API (no abort) |
| 16 | `stbds_hmput_key` | `mode >= 1` but `table->string.mode == STBDS_SH_NONE` (table first created with `mode 0`, later `hmput_key` with `mode 1`) | `switch(string.mode)` hits `default:` → `memcpy(key, keysize)` — the key is copied as raw bytes even in "string" mode |
| 17 | `stbds_hmput_key` | `mode` negative / `>= 2` | `mode >= STBDS_HM_STRING` decides hash+compare; `string.mode` on a fresh table becomes `STBDS_SH_DEFAULT` for any `mode >= 1`, `0` otherwise |
| 18 | `stbds_shmode_func` | `mode` out of enum range (`-1`, `4`, `255`, `256`, `999`, `INT_MAX`) | stores `(unsigned char) mode` into `string.mode`; a later `hmput_key` `switch` falls to `default:` → raw `memcpy` key path |
| 19 | `stbds_hmdel_key` (line 810) | `a == NULL` | `return 0` (NULL) — and no `temp` is written anywhere |
| 20 | `stbds_hmdel_key` (line 816) | `header(raw_a)->hash_table == 0` (no hash index) | sets `temp = 0`, `return a` |
| 21 | `stbds_hmdel_key` (line 822) | key absent (`stbds_hm_find_slot` < 0) | `temp` stays `0`, `return a`, `length` unchanged |
| 22 | `stbds_hmdel_key` (line 828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | vacuously satisfied (`find_slot` masks with `slot_count-1`); not reachable (no abort) |
| 23 | `stbds_hmdel_key` (line 832) | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t` ⇒ comparison is **always true in C**; can never abort, even if `--used_count` wrapped to `SIZE_MAX` |
| 24 | `stbds_hmdel_key` (line 846) | `STBDS_ASSERT(slot >= 0)` after re-finding the swapped-in last element | satisfied whenever the map invariants hold; not reachable (no abort) |
| 25 | `stbds_hmdel_key` (line 849) | `STBDS_ASSERT(b->index[i] == final_index)` | ditto; not reachable (no abort) |
| 26 | `stbds_hmdel_key` | `mode == 2` (`STBDS_HM_PTR_TO_STRING`) with `string.mode == STBDS_SH_STRDUP` | `mode == STBDS_HM_STRING` is **false** ⇒ the strdup'd key is **not** freed (leak), while `mode == 1` frees it |
| 27 | `stbds_hmdel_key` | delete the only real element (`old_index == final_index`) | no `memmove`, no slot re-find, `length` → 1, `temp == 1` |
| 28 | `stbds_make_hash_index` (line 401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | for every `slot_count` the public API can produce (8, 16, 32, …) this holds (8→6+1<8); not reachable (no abort) |
| 29 | `stbds_stralloc` (line 913) | `STBDS_ASSERT(len <= a->remaining)` | unreachable: the `len > remaining` branch either returns early (`len > blocksize`) or sets `remaining = blocksize >= len` |
| 30 | `stbds_stralloc` | `strlen(str)+1 > blocksize` where `blocksize = 512 << (block>>1)` and arena `storage == NULL` | dedicated over-sized block; `a->remaining` forced to `0`, returns `sb->storage` |
| 31 | `stbds_stralloc` | `strlen(str)+1 > blocksize` and arena `storage != NULL` | over-sized block spliced in as `storage->next`; `remaining` left untouched |
| 32 | `stbds_stralloc` | `a->block` already ≥ the value where `512 << (block>>1)` reaches `1<<20` (`block >= 22`) | `++a->block` is skipped (`blocksize < MAX` false) — `block` saturates |
| 33 | `stbds_stralloc` | empty string `""` (`len == 1`, minimal) | still consumes 1 byte of the arena |
| 34 | `stbds_hmfree_func` (line 585) | `a == NULL` | immediate `return`, nothing freed |
| 35 | `stbds_hmfree_func` | `header(a)->hash_table == NULL` | skips the STRDUP sweep and `strreset`, still `free`s `hash_table` (NULL) and the header |
| 36 | `stbds_hash_bytes` | `len == 0` | no main-loop iteration, `data = 0 << 56`, only the `case 0: break` tail — a well-defined hash of the empty input |
| 37 | `stbds_hash_bytes` | `len` not a multiple of `sizeof(size_t)`, high bit set in tail byte 3 | `case 4: data |= (d[3] << 24)` overflows `int` → sign-extended into `size_t` (gcc: wraps) |
| 38 | `stbds_hash_string` | empty string `""` | loop body never runs; result is the avalanche of `seed ^ seed == 0` plus `seed` |
| 39 | `stbds_hash_string` | bytes `>= 0x80` | `(unsigned char) *str` — no sign extension of the character |
| 40 | `intput` | `num == 9` | `hmget(intmap, num) == 7` fails (value is `9`) → `assert` → **SIGABRT** |
| 41 | `intput` | `num == 11` | `hmget(intmap, num) == 7` fails (value is `3`) → `assert` → **SIGABRT** |
| 42 | `intput` | any `num ∉ {9, 11}` (incl. `0`, `7`, `-1`, `INT_MIN`, `INT_MAX`) | all three asserts hold → returns normally (exit 0) |
| 43 | `stbds_arrfreef` | `a == NULL` | `free((char*)NULL - 32)` → invalid free, undefined behaviour / crash in both implementations (not exercised) |
| 44 | `stbds_hash_string` / `stbds_stralloc` / `stbds_hmput_key(mode>=1)` | `key == NULL` | dereferences NULL → SIGSEGV in both implementations (not exercised) |

Rows 15, 22, 23, 24, 25, 28, 29 are asserts that the C code *contains* but that
cannot fire through the public API; they are listed because Phase C must confirm
the Rust translation does not abort where C does not. Rows 43–44 are
undefined-behaviour crashes in the C original and are documented, not executed.

## Phase C status — every row has a passing differential test

All tests live in `tests/phase_c_errors.rs` and call both `.so`s through
`libloading`. `cargo test --release --test phase_c_errors` → **24 passed**.

| rows | test |
|------|------|
| 1 | `err01_arrgrowf_request_already_satisfied` |
| 2, 2b | `err02_arrgrowf_zero_request_returns_null` |
| 3, 4, 7, 8 | `err03_err04_find_slot_absent_key` |
| 5 | `err05_hmget_key_ts_null_map` |
| 6 | `err06_hmget_no_hash_table` |
| 9 | `err09_mode_below_range_is_binary` (`0, -1, -2, -7, -1000, INT_MIN, INT_MIN+1`) |
| 10 | `err10_mode_above_range_is_string` (`1, 2, 3, 7, 1000, INT_MAX`) |
| 11, 12, 13 | `err11_err12_err13_hmput_default` |
| 14, 15 | `err14_err15_hmput_key_bootstrap` |
| 16 | `err16_string_mode_on_sh_none_table` |
| 17 | `err09_…` + `err10_…` (both sides of the `mode >= 1` split) |
| 18 | `err18_shmode_func_out_of_range` (`-1, -2, 4, 5, 100, 255, 256, 257, 511, 512, 1000, INT_MAX`) |
| 19 | `err19_hmdel_key_null_map` |
| 20 | `err20_hmdel_key_no_hash_table` |
| 21, 27 | `err21_err27_hmdel_key_absent_and_last` |
| 22, 24, 25, 28 | `err22_err24_err25_err28_unreachable_asserts` (6 × 3000-op randomised churn; either side aborting kills the test process) |
| 23 | `err23_used_count_wraparound_must_not_abort` |
| 26 | `err26_mode2_strdup_key_not_freed` |
| 29, 30, 31, 32, 33 | `err29_err30_err31_err32_err33_stralloc_boundaries` |
| 34, 35 | `err34_err35_hmfree_func_degenerate` |
| 36, 37 | `err36_err37_hash_bytes_boundaries` |
| 38, 39 | `err38_err39_hash_string_boundaries` |
| 40, 41 | `err40_err41_intput_aborts_on_9_and_11` (subprocess; both must die with signal 6) |
| 42 | `err42_intput_returns_for_other_values` (subprocess; both must exit 0) |
| 43, 44 | documented only — undefined behaviour (invalid `free`, NULL deref) in the C original, identical in both, not executed |

### Bug found and fixed by this phase

Row 23 was a real divergence. The C source has

```c
STBDS_ASSERT(table->used_count >= 0);
```

on a `size_t`, which is vacuously true and can never fire. The Rust translation
had rendered it as `(*table).used_count as isize >= 0`, which **aborts** once
`--used_count` wraps to `SIZE_MAX`. `err23_…` reaches that state and caught it:
before the fix the Rust child died with SIGABRT while C returned normally.
