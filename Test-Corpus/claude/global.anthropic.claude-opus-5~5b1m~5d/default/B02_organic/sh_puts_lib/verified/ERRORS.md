# ERRORS.md — Phase A: the ERROR-SURFACE TABLE

Every distinct way `c_src/src/lib.c` rejects / short-circuits / errors on its
input, obtained by grepping the C source for **every** `return` that is not the
single normal exit, every `STBDS_ASSERT`, every explicit range/null check and
every min/max constant:

```sh
grep -n 'return\|ASSERT\|== NULL\|!= NULL\|== 0\|NULL)\|< 0\|>= \|<= \|MIN\|MAX' c_src/src/lib.c
```

This library has **no error enum and no error return codes**. Its rejection
vocabulary is exactly three things:

* `NULL` / unchanged-pointer returns from the `void *`-returning entry points,
* the `-1` (`STBDS_INDEX_EMPTY`) / `-2` (`STBDS_INDEX_DELETED`) sentinels written
  into `stbds_array_header::temp` or `*temp`,
* `assert()` (live in the C `.so` — `CMakeLists.txt` defines no `NDEBUG`).

Each row is a distinct rejection branch. Rows are checked off when a
differential test constructs that exact condition, calls **both** `.so`s and
asserts the *same* sentinel / pointer / `temp` value comes back (not merely
"both failed").

## R — value/sentinel rejections (all differentially tested)

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| R1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` — growth request already satisfied (`lib.c:286 return a;`) | returns `a` **unchanged and unreallocated** (identical pointer; header untouched) | [x] |
| R2 | `stbds_arrgrowf` | `a == NULL, addlen == 0, min_cap == 0` → `min_len==0`, `0 <= arrcap(NULL)==0` | returns `NULL`; **no allocation at all** | [x] |
| R3 | `stbds_arrgrowf` | `a == NULL` (fresh array) | fresh block; `length = 0`, `hash_table = NULL`, `temp = 0`, `capacity = max(min_len, min_cap, 2*0, 4)` | [x] |
| R4 | `stbds_hmfree_func` | `a == NULL` (`lib.c:573 if (a == NULL) return;`) | returns immediately, no `free()`, no crash | [x] |
| R5 | `stbds_hmfree_func` | `stbds_hash_table(a) == NULL` (array made by `arrgrowf`, never `hmput_key`) | skips the STRDUP sweep and `strreset`; still `free(hash_table)` (a no-op `free(NULL)`) and `free(header)` | [x] |
| R6 | `stbds_hm_find_slot` | probe reaches a bucket entry whose `hash == STBDS_HASH_EMPTY (0)` in the **first** (`i = pos&7 .. 7`) scan → key absent (`lib.c:610 return -1;`) | `-1` | [x] |
| R7 | `stbds_hm_find_slot` | same, detected in the **second** (`i = 0 .. pos&7`) wrap-around scan (`lib.c:621 return -1;`) | `-1` | [x] |
| R8 | `stbds_hmget_key_ts` | `a == NULL` (`lib.c:634`) | allocates the 1-element sentinel array, `*temp = STBDS_INDEX_EMPTY == -1`, returns `ARR_TO_HASH(a)` | [x] |
| R9 | `stbds_hmget_key_ts` | `a != NULL` but `header(raw_a)->hash_table == 0` (never inserted; e.g. after `stbds_hmput_default` only) (`lib.c:644`) | `*temp = -1`, returns `a` unchanged | [x] |
| R10 | `stbds_hmget_key_ts` | key not present → `stbds_hm_find_slot() < 0` (`lib.c:648`) | `*temp = STBDS_INDEX_EMPTY == -1`, returns `a` unchanged | [x] |
| R11 | `stbds_hmget_key` | all three of R8/R9/R10 | same as above **and** `header(HASH_TO_ARR(p))->temp == -1` (this is what `hmgeti`/`hmgetp_null` read to decide "absent") | [x] |
| R12 | `stbds_hmdel_key` | `a == NULL` (`lib.c:809 return 0;`) | returns `NULL` (**not** `a`-shaped), no side effects | [x] |
| R13 | `stbds_hmdel_key` | `header(raw_a)->hash_table == 0` (`lib.c:816`) | sets `temp = 0` **first**, then returns `a` unchanged; `length` unchanged | [x] |
| R14 | `stbds_hmdel_key` | key absent → `slot < 0` (`lib.c:821`) | `temp == 0`, returns `a`, `length`/`used_count`/`tombstone_count` all unchanged | [x] |
| R15 | `stbds_hmdel_key` | `keyoffset` non-zero while the key was stored at offset 0 → `stbds_is_key_equal` compares the wrong bytes → `slot < 0` | `temp == 0`, returns `a` unchanged (same as R14) — the deletion is silently rejected | [x] |
| R16 | `stbds_hmput_default` | `a == NULL` **or** `header(HASH_TO_ARR(a))->length == 0` (`lib.c:669`) | allocates/extends and zero-fills element 0; returns `ARR_TO_HASH`; the *non*-triggering case returns `a` byte-identical | [x] |
| R17 | `stbds_hash_bytes` | `len == 0` (with `p == NULL`) — no byte is dereferenced (`i+8<=0` false, `switch(0)` → `break`) | `data = 0`, returns the pure-seed siphash; **must not** read `*p` | [x] |
| R18 | `stbds_hash_string` | empty string `""` | `while(*str)` never runs; result is the pure-seed avalanche | [x] |
| R19 | `stbds_hm_find_slot` / `stbds_hmput_key` | computed `hash < 2` (collides with `STBDS_HASH_EMPTY==0` / `STBDS_HASH_DELETED==1`) → `if (hash < 2) hash += 2;` (`lib.c:596`, `lib.c:719`) | hash is bumped to 2/3; empty/deleted slots stay distinguishable | [x] |
| R20 | `stbds_hmput_key` | `mode` out of the `{0,1}` enum range, e.g. `2`, `5`, `INT_MAX` → `mode >= STBDS_HM_STRING` is **true** | takes the *string* hash/compare path even though no such enumerator exists | [x] |
| R21 | `stbds_hmput_key` | `mode` **negative** (`-1`, `INT_MIN`) → `mode >= STBDS_HM_STRING` is **false** | takes the *binary* `memcmp`/`hash_bytes` path; `nt->string.mode = 0` | [x] |
| R22 | `stbds_hmdel_key` | `mode >= 2` (string-ish) → `mode == STBDS_HM_STRING` is **false** | the `STBDS_SH_STRDUP` key is **not** freed and the binary `keyoffset` re-lookup branch is taken (asymmetric with `hmput_key`) — reproduced verbatim | [x] |
| R23 | `stbds_shmode_func` | `mode` outside `{0,1,2,3}`: `4`, `255`, `256` (→ truncates to `0`), `-1` (→ `255`) | `h->string.mode = (unsigned char) mode`; `hmput_key`'s `switch` then hits `default:` → raw `memcpy(key, keysize)` | [x] |
| R24 | `stbds_hmput_key` | `table->string.mode` has no matching `case` (`STBDS_SH_NONE==0`, `4`, `255`, …) | `default:` → `memcpy((char*)a + elemsize*i, key, keysize)`; `temp_key` is **not** written | [x] |
| R25 | `stbds_stralloc` | `len > blocksize` (string longer than the next arena block) **and** `a->storage != NULL` | dedicated over-sized block spliced in as `a->storage->next`; `a->remaining` left **unchanged**; returns `sb->storage` | [x] |
| R26 | `stbds_stralloc` | `len > blocksize` **and** `a->storage == NULL` | over-sized block becomes `a->storage`, `sb->next = NULL`, `a->remaining = 0` | [x] |
| R27 | `stbds_stralloc` | `a->block` so large that `512 << (block>>1)` shifts by ≥ 64 (e.g. `block = 255` → shift 127) — C undefined behaviour, x86-64 masks the count to 6 bits | `blocksize = 512 << 63 == 0`; `blocksize < 1<<20` so `++a->block` (`255 → 0`); `len > 0` so the over-sized-block path is taken | [x] |
| R28 | `stbds_stralloc` | `a->block` at its natural ceiling (22): `512 << 11 == 1<<20` is **not** `< STBDS_STRING_ARENA_BLOCKSIZE_MAX` | `a->block` is **not** incremented — blocks stop growing at 1 MiB | [x] |
| R29 | `stbds_strreset` | already-empty arena (`a->storage == NULL`) | frees nothing, zeroes all 24 bytes of the arena | [x] |
| R30 | `stbds_strreset` | called twice in a row / on an arena whose blocks were spliced by R25 | idempotent; walks the whole `next` chain exactly once | [x] |
| R31 | `strkey` | `n == INT_MIN` (`-2147483648`) — the classic `abs()` overflow input | `sprintf("test_%d")` → `"test_-2147483648"` (16 chars + NUL, fits the 256-byte static buffer) | [x] |
| R32 | `strkey` | `n < 0` generally, and `n == 0` | `"test_-1"`, `"test_0"`, … | [x] |
| R33 | `sh_puts` | `num <= 0` (`for (i=0; i<num; ++i)` never runs) | no `stralloc` calls; still prints exactly one line `"a <num>\n"` | [x] |
| R34 | `sh_puts` | `num == INT_MIN` / very negative | loop skipped; prints `"a -2147483648\n"` | [x] |
| R35 | `stbds_hmget_key_ts` / `stbds_hmdel_key` | `key` bytes that hash into a bucket full of `STBDS_HASH_DELETED (1)` entries (tombstones) — probe must **keep going**, not report absent | tombstones are skipped (`hash==1 != 0`), probe continues to the next bucket | [x] |
| R36 | `stbds_hmput_key` | insertion lands on a reclaimable tombstone (`tombstone >= 0` at `found_empty_slot`) | `pos = tombstone; --table->tombstone_count;` — the freed slot is reused instead of the empty one | [x] |

## A — `assert()` rows (live in the C `.so`; proved unreachable via the API)

`STBDS_ASSERT` is `assert`, and the C library is built **without** `NDEBUG`, so
each of these aborts the process. Rather than crash the test runner, each row is
discharged by proving the predicate holds for every input reachable through the
exported API, and (where cheap) by a *positive* differential test that drives
the code right up to the boundary and confirms both libraries agree.

| #  | site | assertion | why unreachable through the exported API | boundary test | [x] |
|----|------|-----------|------------------------------------------|---------------|-----|
| A1 | `stbds_make_hash_index` (`lib.c:401`) | `used_count_threshold + tombstone_count_threshold < slot_count` | `slot_count` is only ever `8` (from `hmput_key`/`shmode_func`), doubled by `hmput_key`, or halved by `hmdel_key` (guarded by `slot_count > 8`) — i.e. always a power of two ≥ 8. For `sc = 8,16,32,64,…`: `(sc − sc>>2) + (sc>>3 + sc>>4)` = `7,15,30,60,…` < `8,16,32,64,…`. | grow to 8/16/32/64/128 and shrink back | [x] |
| A2 | `stbds_hmput_key` (`lib.c:778`) | `(size_t) i+1 <= stbds_arrcap(a)` | guarded three lines above by `if ((size_t) i+1 > stbds_arrcap(a)) a = stbds_arrgrowf(a, elemsize, 1, 0);`, and `arrgrowf` always returns `capacity >= arrlen+1` | insert across every capacity doubling (4, 8, 16, …) | [x] |
| A3 | `stbds_hmdel_key` (`lib.c:828`) | `slot < (ptrdiff_t) table->slot_count` | `stbds_hm_find_slot` returns `(pos & ~7) + i` with `pos &= slot_count-1` and `i < 8`, so `slot < slot_count` by construction | delete from tables of slot_count 8/16/32 | [x] |
| A4 | `stbds_hmdel_key` (`lib.c:832`) | `table->used_count >= 0` | `used_count` is `size_t` — the comparison is vacuously true in C | — | [x] |
| A5 | `stbds_hmdel_key` (`lib.c:846`) | `slot >= 0` after re-finding the moved last element | the element just `memmove`d into `old_index` was in the table, hence findable — **provided** `mode` is `0` or exactly `1`. For `mode >= 2` the re-lookup hashes the *bytes of the key pointer* as a string and this assert **can** fire; that combination is therefore excluded from the delete-with-relocation tests (see R22) | delete non-last element for `mode ∈ {0,1}` | [x] |
| A6 | `stbds_hmdel_key` (`lib.c:849`) | `b->index[i] == final_index` | follows from A5 | same as A5 | [x] |
| A7 | `stbds_stralloc` (`lib.c:913`) | `len <= a->remaining` | either the `if` was not entered (`len <= remaining`), or a fresh block set `remaining = blocksize >= len`, or the `len > blocksize` branch already returned | strings of length exactly `blocksize`, `blocksize-1`, `blocksize+1` | [x] |
| A8 | `sh_puts` (`lib.c:959-961`) | `*strmap[0].key == 'a'`, `strmap[0].key != s.key`, `strmap[0].value == s.value` | `sh_new_arena` sets `string.mode = STBDS_SH_ARENA`, so `hmput_key` arena-copies `"a"` → a different pointer holding `"a"`; `shputs` then stores the whole struct so `value == num` | every `sh_puts` differential call | [x] |
| A9 | `stbds_arrfreef` / `stbds_hmfree_func` | *(no assert)* — `stbds_arrfreef(NULL)` computes `free((char*)NULL - 32)`, an invalid free that crashes **identically** in both libraries (`wrapping_sub`) | not a rejection the library performs; excluded from testing as it is UB in the C original | not tested (UB in C) | [x] |

## Generic FFI boundary cases (required in addition to the table)

| # | case | covered by |
|---|------|-----------|
| G1 | `NULL` pointer for every pointer parameter the C code actually null-checks (`a` in `arrgrowf`, `hmfree_func`, `hmget_key`, `hmget_key_ts`, `hmput_key`, `hmput_default`, `hmdel_key`) | R2–R4, R8, R12, R16 |
| G2 | `NULL` + `len == 0` for `stbds_hash_bytes` | R17 |
| G3 | zero lengths: `keysize == 0`, `addlen == 0`, `min_cap == 0`, `elemsize == 0`, empty string keys | R2, R17, R18 + `errors.rs::keysize_zero`, `elemsize_zero` |
| G4 | over-sized lengths: `len` one past the arena block, `addlen`/`min_cap` at the capacity doubling boundary | R25–R28, A2 |
| G5 | one step past a documented range: `slot_count` boundary `6→7` inserts (`used_count_threshold`), `tombstone_count_threshold+1` deletes, `used_count_shrink_threshold-1` | A1, R36 |
| G6 | **out-of-range enum values across FFI**: `mode` (`STBDS_HM_*`) = `-1, 0, 1, 2, 3, 5, 255, 256, INT_MIN, INT_MAX`; `mode` (`STBDS_SH_*`) = `-1, 0..4, 255, 256, INT_MIN, INT_MAX` | R20–R24 |

---

## Row → test mapping (every checkbox is auditable)

Run with `cargo test --offline --release -- --test-threads=1` (the two libraries
share process-global state — the `stbds_hash_seed` static and `strkey`'s static
buffer — so the harness serialises tests through `common::lock()`).

| rows | test |
|------|------|
| R1 | `c_errors::r1_arrgrowf_early_out_returns_input_pointer_unchanged`, `b_array::c18_early_out_returns_same_pointer` |
| R2 | `c_errors::r2_arrgrowf_null_zero_zero_returns_null`, `b_array::c14_null_zero_zero_returns_null` |
| R3 | `c_errors::r3_arrgrowf_fresh_array_header_is_fully_initialised` |
| R4 | `c_errors::r4_hmfree_func_null_is_a_noop`, `b_map_binary::c50_hmfree_states` |
| R5 | `c_errors::r5_hmfree_func_with_null_hash_table`, `b_map_binary::c50_hmfree_states` |
| R6, R7, R35 | `c_errors::r6_r7_r35_find_slot_returns_minus_one_from_both_scans` (prints the branch-hit counters: first-scan 3767, wrap-around 433, via-tombstones 784, multi-bucket 83) |
| R8 | `c_errors::r8_hmget_key_ts_on_null_map` |
| R9 | `c_errors::r9_hmget_key_ts_with_null_hash_table` |
| R10, R11 | `c_errors::r10_r11_absent_key_sentinels`, `b_map_binary::c40_c41_lookup_states` |
| R12 | `c_errors::r12_hmdel_key_null_returns_null` |
| R13 | `c_errors::r13_hmdel_key_with_null_hash_table_sets_temp_zero` |
| R14 | `c_errors::r14_hmdel_key_absent_leaves_everything_unchanged` |
| R15 | `c_errors::r15_hmdel_key_nonzero_keyoffset_is_rejected`, `b_map_binary::c47_c48_geometry_and_keyoffset` |
| R16 | `c_errors::r16_hmput_default_branches`, `b_map_binary::c49_hmput_default` |
| R17 | `c_errors::r17_hash_bytes_len_zero_never_dereferences`, `b_pure::c1_hash_bytes_len0_null` |
| R18 | `c_errors::r18_hash_string_empty`, `b_pure::c10_c11_hash_string` |
| R19 | `c_errors::r19_hash_below_two_is_bumped` (constructs a seed that makes the raw hash exactly 0 and 1) |
| R20 | `c_errors::r20_out_of_range_mode_takes_the_string_path`, `b_enums::c61_stringish_modes_put_get` |
| R21 | `c_errors::r21_negative_mode_takes_the_binary_path`, `b_enums::c62_binaryish_modes_put_get` |
| R22 | `c_errors::r22_hmdel_stringish_mode_delete_last`, `b_enums::c63_stringish_delete_last_only` |
| R23 | `c_errors::r23_shmode_func_truncates_mode_to_unsigned_char`, `b_enums::c65_shmode_func_truncation` |
| R24 | `c_errors::r24_switch_default_memcpys_raw_key_bytes`, `b_map_string::c54_sh_none_with_string_mode` |
| R25 | `c_errors::r25_oversized_string_into_a_non_empty_arena`, `b_arena::c26_c30_boundaries` |
| R26 | `c_errors::r26_oversized_string_into_a_fresh_arena` |
| R27 | `c_errors::r27_block_field_shift_of_64_or_more`, `b_arena::c31_c32_preset_block_field` |
| R28 | `c_errors::r28_block_stops_growing_at_the_1mib_ceiling` |
| R29, R30 | `c_errors::r29_r30_strreset_is_idempotent_and_walks_the_whole_chain`, `b_arena::c34_strreset_shapes` |
| R31, R32 | `c_errors::r31_r32_strkey_negative_and_int_min`, `b_pure::c13_strkey` (3018 values) |
| R33, R34 | `c_errors::r33_r34_sh_puts_non_positive_num`, `b_shputs::c72_negative_num` |
| R36 | `c_errors::r36_insertion_reuses_a_tombstone` (prints 2164 verified reclamations), `b_map_binary::c46_tombstone_reuse` |
| A1 | `d_rehash::make_hash_index_rehash_matches_an_independent_model`, `c_errors::g5_threshold_boundaries` |
| A2 | `b_map_binary::c36_c37_growth_chain`, `b_array::c21_c22_append_one_at_a_time` |
| A3, A4, A5, A6 | `b_map_binary::c42_c43_delete_last_and_middle`, `c_errors::r22_hmdel_stringish_mode_delete_last` |
| A7 | `b_arena::c26_c30_boundaries` (`len` == `blocksize`−1, `blocksize`, `blocksize`+1 at every ladder step) |
| A8 | every `b_shputs` test (the three asserts hold on every call) |
| A9 | not tested — `stbds_arrfreef(NULL)` is `free((char*)NULL - 32)`, undefined behaviour in the C original that crashes both libraries identically |
| G1 | R2, R3, R4, R8, R12, R16 |
| G2 | R17 |
| G3 | `c_errors::g3_keysize_zero`, `c_errors::g3_elemsize_zero` |
| G4 | R25–R28, `b_array::c19_c20_growth_rules_on_existing_array` |
| G5 | `c_errors::g5_threshold_boundaries` (n = 5,6,7,11,12,13,23,24,25,47,48,49 → the exact `slot_count` ladder 8/16/32/64/128) |
| G6 | `b_enums::*` and `c_errors::r20`–`r24` (`mode` ∈ {−2^31, −256, −255, −2, −1, 0, 1, 2, 3, 5, 42, 255, 256, 65536, 2^31−2, 2^31−1}; `STBDS_SH_*` ∈ {−2^31, −256, −255, −2, −1, 0…6, 127, 128, 254, 255, 256, 257, 511, 512, 65535, 65536, 2^31−1}) |

### Two C behaviours that are undefined and therefore deliberately not tested

Both are reproduced faithfully by the Rust (identical arithmetic, identical
crash), but exercising them would abort the test process rather than reveal a
divergence:

1. `stbds_stralloc` with `a->block` such that `512 << (block>>1)` is between
   2 GiB and 8 EiB: the C asks `realloc` for a multi-terabyte block, gets `NULL`
   and dereferences it (`sb->next = a->storage`). See the filter in
   `b_arena::c31_c32_preset_block_field`.
2. `STBDS_SH_NONE`-style key storage combined with `mode >= STBDS_HM_STRING`
   *and* a hash match: `stbds_is_key_equal` does
   `strcmp(key, *(char **) element)`, dereferencing raw key bytes as a pointer.
   `b_map_string::c54_sh_none_with_string_mode` therefore only inserts distinct
   keys and only looks up absent ones.
