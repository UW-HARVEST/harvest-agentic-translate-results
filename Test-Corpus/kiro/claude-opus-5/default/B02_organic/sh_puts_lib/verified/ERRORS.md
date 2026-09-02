# ERRORS.md — error / rejection surface table

`c_src/src/lib.c` is stb_ds. It is a "trust the caller" C library: it has **no
error enum, no `RETURN_ERROR` macro, and no function that returns an error
code**. Its entire rejection surface consists of

1. **null-pointer / empty-state early returns** (`return;`, `return 0;`,
   `return -1;`, `if (a == NULL) …`),
2. **sentinel returns** (`STBDS_INDEX_EMPTY == -1`, `STBDS_INDEX_DELETED == -2`,
   `STBDS_HASH_EMPTY == 0`, `STBDS_HASH_DELETED == 1`),
3. **`STBDS_ASSERT` (= `assert`, LIVE because CMake does not define `NDEBUG`)**,
4. **implicit range/threshold decisions** on `used_count_threshold`,
   `used_count_shrink_threshold`, `tombstone_count_threshold`,
   `STBDS_STRING_ARENA_BLOCKSIZE_MIN/MAX`,
5. **out-of-range `int mode` values crossing the FFI boundary** — the `mode`
   and string-mode parameters are plain `int`, tested with `>=` / `==` / a
   `switch` with a `default`, so *every* `int` is an accepted input and each
   comparison partitions the space differently.

Every row below was derived from a specific line of `lib.c` (line numbers
given). Rows are checked off once `translation/tests/errors.rs` proves C and
Rust agree.

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `stbds_hmfree_func` (l.573 `if (a == NULL) return;`) | `a == NULL` | returns immediately, no free, no crash | [x] |
| E2 | `stbds_hmfree_func` (l.574) | `a != NULL` but `stbds_header(a)->hash_table == NULL` (array made by `stbds_arrgrowf`, never `hmput`) | skips string/arena cleanup, `free(NULL)` on hash_table, frees header | [x] |
| E3 | `stbds_hmdel_key` (l.810 `return 0;`) | `a == NULL` | returns `NULL` (0), *not* the input | [x] |
| E4 | `stbds_hmdel_key` (l.816 `if (table == 0) return a;`) | `a != NULL` but no hash table yet | returns `a` unchanged, and sets `stbds_temp(raw_a) = 0` first (l.815) | [x] |
| E5 | `stbds_hmdel_key` (l.821 `if (slot < 0) return a;`) | key absent from a populated table | returns `a`; `stbds_temp(raw_a) == 0` (the "not deleted" answer) | [x] |
| E6 | `stbds_hmdel_key` (l.838) | `mode == STBDS_HM_STRING` (exactly 1) **and** `table->string.mode == STBDS_SH_STRDUP` | frees the stored strdup'd key | [x] |
| E7 | `stbds_hmdel_key` (l.838, `==` not `>=`) | `mode == 2` (i.e. the old `STBDS_HM_PTR_TO_STRING`) on a STRDUP table | does **not** free the key, but *does* still use string comparison in `stbds_hm_find_slot` (which tests `mode >= STBDS_HM_STRING`) — asymmetric, must be reproduced | [x] |
| E8 | `stbds_hm_find_slot` (l.610 / l.621 `return -1;`) | probe reaches a slot with `hash == STBDS_HASH_EMPTY (0)` | returns `-1` (key-not-found sentinel) | [x] |
| E9 | `stbds_hmget_key_ts` (l.634) | `a == NULL` | allocates a 1-element array, `length = 1`, zeroes elem 0, writes `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` | [x] |
| E10 | `stbds_hmget_key_ts` (l.645 `*temp = -1`) | `a != NULL`, `hash_table == NULL` | `*temp = -1`; key never hashed (so a NULL `key` is harmless here) | [x] |
| E11 | `stbds_hmget_key_ts` (l.648) | key not present in a populated table | `*temp = STBDS_INDEX_EMPTY (-1)` | [x] |
| E12 | `stbds_hmget_key` (l.660) | same as E9/E10/E11 | additionally stores that `-1` into `stbds_header(p-elemsize)->temp`, so `hmgeti` yields `-1` and `hmgetp_null` yields NULL | [x] |
| E13 | `stbds_hmput_default` (l.669) | `a == NULL` **or** existing `length == 0` | grows/creates a 1-element array (the `t[-1]` default slot) instead of touching `a[-1]` of an empty array | [x] |
| E14 | `STBDS_ASSERT` l.401 `used_count_threshold + tombstone_count_threshold < slot_count` | `stbds_make_hash_index(slot_count, …)` with a `slot_count` that violates it | `abort()` via `__assert_fail`. **Unreachable from the public API**: `slot_count` is only ever `STBDS_BUCKET_LENGTH (8)` or `table->slot_count*2` or `>>1`, i.e. a power of two `>= 8`; for those, `(sc - sc/4) + (sc/8 + sc/16) < sc` always holds. Invariant checked on both sides for every reachable `slot_count` 8…4096 | [x] |
| E15 | `STBDS_ASSERT` l.778 `(size_t) i+1 <= stbds_arrcap(a)` | array not grown enough before an insert | `abort()`. Unreachable: guarded by the `if ((size_t) i+1 > stbds_arrcap(a)) arrgrowf(…)` on l.775. Invariant `length <= capacity` checked after every insert | [x] |
| E16 | `STBDS_ASSERT` l.828 `slot < (ptrdiff_t) table->slot_count` | `stbds_hm_find_slot` returned a slot past the table | `abort()`. Unreachable: `find_slot` returns `(pos & ~7) + i` with `pos < slot_count` | [x] |
| E17 | `STBDS_ASSERT` l.832 `table->used_count >= 0` | — | tautology (`used_count` is `size_t`); never fires even after the `--table->used_count` on l.830 wraps. Deliberately omitted from the Rust translation | [x] |
| E18 | `STBDS_ASSERT` l.846 `slot >= 0` | **REACHABLE.** `stbds_hmdel_key(…, mode)` with `mode > STBDS_HM_STRING` (e.g. `2`) deleting a **non-final** entry. The key-reload on l.841 tests `mode == STBDS_HM_STRING`, so for `mode == 2` the `else` branch hands `find_slot` a pointer to the *element* while `find_slot` (which tests `mode >= STBDS_HM_STRING`) `strcmp`/hashes it as a string — it hashes the raw pointer bytes, misses, returns `-1`, and the assert fires | `abort()` / SIGABRT (exit 134). Verified in a child process: **both** libraries die with SIGABRT on the same input, and both survive when the final entry is deleted instead (`old_index == final_index` skips the block) | [x] |
| E19 | `STBDS_ASSERT` l.849 `b->index[i] == final_index` | re-lookup found a different entry | `abort()`. Unreachable for a consistent table: whenever E18's re-lookup succeeds it can only have found the moved element | [x] |
| E20 | `STBDS_ASSERT` l.913 `len <= a->remaining` | `stbds_stralloc` fell through the grow block without enough room | `abort()`. Unreachable: the `len > blocksize` branch returns early (l.905) and the else branch sets `remaining = blocksize >= len`. Invariant checked over 3000 randomized allocations | [x] |
| E21 | `STBDS_ASSERT` l.959–961 in `sh_puts` | arena-mode `shputs` failed to copy the key / value | `abort()`. Must hold for every `num`, incl. `0`, negative and `INT_MIN` | [x] |
| E22 | `stbds_stralloc` (l.882 `strlen(str)+1`) | `str == ""` → `len == 1` | never zero-length; always consumes 1 byte for the NUL | [x] |
| E23 | `stbds_stralloc` (l.896 `if (len > blocksize)`) | a string longer than the current block size (e.g. `len > 512` on a fresh arena) | allocates a *dedicated* block, splices it in as `a->storage->next` (or as `a->storage` with `remaining = 0` when the arena was empty), returns `sb->storage`, and **does not** decrement `remaining` | [x] |
| E24 | `stbds_stralloc` (l.890 `blocksize < BLOCKSIZE_MAX`) | enough allocations to drive `a->block` up to the cap | `a->block` stops incrementing once `512 << (block>>1) >= (1<<20)`, i.e. at `block == 22`; shift amount therefore never exceeds 11 | [x] |
| E25 | `stbds_strreset` (l.937 `while (x)`) | already-zeroed / never-used arena (`storage == NULL`) | frees nothing, memsets the arena to 0; idempotent | [x] |
| E26 | `stbds_arrgrowf` (l.288 `if (min_cap <= stbds_arrcap(a)) return a;`) | request that already fits | returns `a` **unchanged**, no realloc | [x] |
| E27 | `stbds_arrgrowf` (l.291–293) | `min_cap` below both `2*cap` and `4` | clamps up to `2*cap`, else to `4`; note the `else if` means a non-empty array is *never* clamped to 4 | [x] |
| E28 | `stbds_arrgrowf` (l.284) | `addlen == 0 && min_cap == 0` on a NULL array | `min_len = 0`, `min_cap = 0`, `0 <= arrcap(NULL) == 0` → **returns NULL** without allocating | [x] |
| E29 | `stbds_arrgrowf` (l.297) | `elemsize == 0` | `realloc(NULL, 0*min_cap + 32)` → 32-byte header-only allocation; succeeds | [x] |
| E30 | `stbds_hash_bytes` (l.522 loop / l.535 `switch (len - i)`) | `len == 0` (with any `p`, incl. NULL) | main loop and every `case` are skipped, `p` is never dereferenced → returns a well-defined hash of "length 0" | [x] |
| E31 | `stbds_hash_bytes` `switch (len - i)` fallthrough (l.536–543) | `len % 8 == 1..7` (7 distinct tails) | each tail case ORs in a different set of bytes; `case 4` (`data |= (d[3] << 24)`) sign-extends when `d[3] >= 0x80` | [x] |
| E32 | `stbds_hash_string` (l.474 `while (*str)`) | `str == ""` | loop body never runs → returns `finalize(seed) + seed` | [x] |
| E33 | `stbds_hm_find_slot` / `stbds_hmput_key` (l.617 / l.717 `if (hash < 2) hash += 2;`) | a key whose hash is `0` (`STBDS_HASH_EMPTY`) or `1` (`STBDS_HASH_DELETED`) | hash is bumped by 2 so it can never alias the empty/deleted sentinels | [x] |
| E34 | `stbds_hmput_key` (l.698) | `used_count >= used_count_threshold` (6 of 8 slots) | rebuild at `slot_count*2` before inserting | [x] |
| E35 | `stbds_hmdel_key` (l.854) | `used_count < used_count_shrink_threshold && slot_count > 8` | shrink to `slot_count>>1` | [x] |
| E36 | `stbds_hmdel_key` (l.858) | `tombstone_count > tombstone_count_threshold` (and not shrinking) | rebuild at the same `slot_count`, clearing tombstones | [x] |
| E37 | `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hm_find_slot` (`mode >= STBDS_HM_STRING`) | **out-of-range `mode`**: `-1`, `INT_MIN`, `2`, `7`, `INT_MAX` | `mode >= 1` → string hashing/`strcmp`; `mode <= 0` → `memcmp` of `keysize` bytes. Every `int` is accepted; no validation | [x] |
| E38 | `stbds_hmput_key` `switch (table->string.mode)` `default:` (l.789) | `stbds_shmode_func(elemsize, mode)` with `mode` whose **low byte** is outside `{1,2,3}`: `0`, `4`, `5`, `127`, `128`, `255`, `256`, `-1`, `-256`, `1000`, `INT_MAX`, `INT_MIN` | `mode` is stored as `(unsigned char) mode`; the `default:` arm does `memcpy(elem, key, keysize)` — i.e. an out-of-range shmode silently degrades to *binary* key storage. Note `(unsigned char)256 == 0`, `(unsigned char)-1 == 255`, `(unsigned char)INT_MAX == 255`, `(unsigned char)INT_MIN == 0` — but `257/258/259` **alias** `SH_DEFAULT/SH_STRDUP/SH_ARENA` and take those arms instead | [x] |
| E39 | `stbds_shmode_func` (l.798) | any `mode` | always allocates and always installs a fresh hash index with `slot_count = 8`; never returns NULL | [x] |
| E40 | `stbds_hmput_key` (l.687) | `a == NULL` | bootstraps the `t[-1]` default element *before* inserting; the first real entry therefore lands at array index 1 and gets `bucket->index = i-1 = 0` | [x] |
| E41 | `stbds_hmdel_key` (l.826 `final_index = arrlen(raw_a)-1-1`) | deleting the **last** entry (`old_index == final_index`) | skips the move-and-repatch block entirely | [x] |
| E42 | `stbds_hmdel_key` (l.834) | deleting a non-last entry | moves the final element into the hole and repatches its slot index | [x] |
| E43 | `stbds_hmdel_key` (l.812, `keyoffset`) | `keyoffset != 0` (e.g. key not the first struct member) | key is read/compared at `elem + keyoffset`; used for the post-move re-lookup | [x] |
| E44 | `sh_puts` (l.953 `for (i=0; i < num; ++i)`) | `num <= 0` (incl. `INT_MIN`) | arena loop never runs; the rest of the function still runs and prints one line | [x] |
| E45 | `strkey` (l.945 `sprintf(buffer,"test_%d",n)`) | `n == INT_MIN`, `n < 0`, `n == 0` | `"test_-2147483648"`, `"test_-<mag>"`, `"test_0"` in the shared 256-byte static buffer (returned pointer is always that same buffer) | [x] |
| E46 | `stbds_arrfreef` (l.315 `free(stbds_header(a))`) | `a == NULL` | computes `(header*)NULL - 1` and calls `free` on it → **glibc abort / SIGSEGV**. Genuine C UB; both implementations perform the identical arithmetic. Not exercised in-process (it would kill the test runner); asserted structurally instead | [x] |

## Deliberately NOT tested (would abort/segfault the shared test process)

- E46 `stbds_arrfreef(NULL)`.
- `stbds_stralloc` on a hand-forged arena with `storage == NULL` but
  `remaining >= len` (dereferences NULL in both).
- `stbds_stralloc` on a hand-forged arena with `block > 127` — `512 << (block>>1)`
  is a shift-count overflow (UB) in C. The Rust translation masks the shift to
  match the x86-64 `shl` semantics gcc emits, so it does not panic; see
  `translation/src/lib.rs::stbds_stralloc`.
- A **string** lookup (`mode >= 1`) against a table whose `string.mode` hits the
  `default:` arm (E38): `default:` `memcpy`s the *string's bytes* into the
  element, and a later `is_key_equal` reinterprets those bytes as a `char *` and
  dereferences it. Both libraries do the identical wild read. Covered only up to
  the (well-defined) insert, in `maps.rs::c28b_*`.
- The same `default:` arm also reads `keysize` bytes from a key string shorter
  than `keysize` (a heap over-read present in the original C). Tests use keys of
  at least `keysize` bytes.
- E18's abort *is* tested, but only in a child process
  (`errors.rs::e18_mode2_nonlast_delete_aborts_in_both`).

## The C asserts are LIVE, and so are the Rust ones

Because E18 is reachable, the translation cannot simply drop `STBDS_ASSERT`:
C would `abort()` where Rust would keep going with `slot == -1` and scribble
outside the bucket array. `translation/src/lib.rs` therefore reinstates every
non-tautological assert (l.401, l.778, l.828, l.846, l.849, l.913 and the three
in `sh_puts`) as a `stbds_assert!` that writes a diagnostic to fd 2 and calls
libc `abort()` — the same SIGABRT/exit-134 that `assert` produces. Only l.832
(`used_count >= 0` on a `size_t`) is omitted, as it can never fire.

## Row → test mapping

| rows | test |
|------|------|
| E1 | `errors.rs::e1_hmfree_null_is_noop` |
| E2 | `errors.rs::e2_hmfree_array_without_hash_table`, `maps.rs::c46_c47_hmfree` |
| E3 | `errors.rs::e3_hmdel_null_returns_null` |
| E4 | `errors.rs::e4_hmdel_without_hash_table` |
| E5 | `errors.rs::e5_hmdel_absent_key` |
| E6, E7 | `errors.rs::e6_e7_strdup_free_only_when_mode_equals_one`, `maps.rs::c41_mode2_on_strdup_table` |
| E8, E11, E12 | `errors.rs::e8_e11_e12_lookup_miss_sentinel` |
| E9 | `errors.rs::e9_get_from_null_bootstraps`, `maps.rs::c34_get_from_null` |
| E10 | `errors.rs::e10_get_without_hash_table_ignores_key` |
| E13 | `errors.rs::e13_hmput_default_paths`, `maps.rs::c15_c17_hmput_default` |
| E14, E15, E16 | `errors.rs::e14_threshold_invariant_over_all_reachable_slot_counts` |
| E17 | `errors.rs::e17_used_count_is_unsigned` |
| E18, E19 | `errors.rs::e18_mode2_nonlast_delete_aborts_in_both` + `e18_control_mode2_last_delete_succeeds_in_both` |
| E20, E24 | `errors.rs::e20_stralloc_remaining_invariant`, `errors.rs::e24_block_cap`, `leaf.rs::c49_stralloc_block_saturation` |
| E21, E44 | `errors.rs::e21_e44_sh_puts_edge_nums`, `leaf.rs::c53_sh_puts_stdout` |
| E22, E23, E25 | `errors.rs::e22_e23_e25_arena_edges`, `errors.rs::e23_dedicated_block_on_empty_arena`, `leaf.rs::c48/c50/c52` |
| E26–E29 | `errors.rs::e26_e29_arrgrowf_edges`, `leaf.rs::c11/c12/c14` |
| E30 | `errors.rs::e30_hash_bytes_len_zero_never_reads`, `leaf.rs::c1_hash_bytes_len0_null` |
| E31 | `errors.rs::e31_hash_bytes_every_tail_case`, `leaf.rs::c2_c7_hash_bytes_all_lengths_and_seeds` |
| E32 | `errors.rs::e32_hash_string_empty`, `leaf.rs::c8_hash_string` |
| E33 | `errors.rs::e33_no_occupied_slot_carries_a_sentinel_hash` |
| E34, E35, E36 | `errors.rs::e34_e36_threshold_transitions`, `maps.rs::c37_c44_c45_delete_all_random_order` |
| E37 | `errors.rs::e37_out_of_range_mode_partitioning`, `maps.rs::c22/c23` |
| E38, E39 | `errors.rs::e38_e39_shmode_func_out_of_range`, `maps.rs::c28a/c28b` |
| E40 | `errors.rs::e40_first_put_index` |
| E41, E42 | `errors.rs::e41_e42_delete_last_vs_middle`, `maps.rs::c35_c36_delete_last_and_middle` |
| E43 | `errors.rs::e43_nonzero_keyoffset`, `maps.rs::c42_keyoffset` |
| E45 | `errors.rs::e45_strkey_extremes`, `leaf.rs::c10_strkey` |
| E46 | `errors.rs::e46_arrfreef_null_is_undefined_in_both` (structural) |
