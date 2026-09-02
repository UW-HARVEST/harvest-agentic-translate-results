# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by enumerating **every**
early-return, sentinel return, `STBDS_ASSERT`, null check, range check and
min/max constant. Line numbers refer to `c_src/src/lib.c`.

This library has no error enum and no `errno`; it rejects input by

* returning the input pointer unchanged (no-op),
* returning `NULL` / `0`,
* writing a sentinel index `-1` (`STBDS_INDEX_EMPTY`) into `temp` / the array
  header's `temp` field,
* writing `0` vs `1` into the header `temp` field (delete "did nothing" flag),
* `assert()` → `SIGABRT` (the build has no `-DNDEBUG`, asserts are live).

Every row has a differential test in `tests/phase_c_errors.rs` (`[x]` = passing
against both `.so` files). Rows that abort, fault or corrupt the heap are
compared on child-process termination status plus piped observations, via
`common::assert_same_outcome` / `assert_same_capture`.

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `stbds_arrgrowf` (L287) | `min_cap <= stbds_arrcap(a)` after `min_cap = max(min_cap, arrlen(a)+addlen)` | returns `a` **unchanged and unreallocated** (same pointer, incl. `NULL`) | `err01_arrgrowf_noop_returns_input_unchanged` | [x] |
| 2 | `stbds_arrgrowf` (L280) | `a == NULL` | len/cap treated as 0; `length`/`hash_table`/`temp` explicitly zeroed | `err02_arrgrowf_null_input_initialises_header` | [x] |
| 3 | `stbds_arrgrowf` (L283) | `arrcap < min_cap < 2*arrcap` | clamped up to `2*arrcap` | `err03_04_arrgrowf_clamp_branches` | [x] |
| 4 | `stbds_arrgrowf` (L285) | `min_cap >= 2*arrcap` **and** `min_cap < 4` | clamped up to `4` | `err03_04_arrgrowf_clamp_branches` | [x] |
| 5 | `stbds_arrgrowf` | `elemsize == 0` | header-only allocation; `capacity = min_cap` | `err05_arrgrowf_elemsize_zero` | [x] |
| 6 | `stbds_arrgrowf` | `elemsize*min_cap + 32` overflows `size_t` | wraps; allocation may succeed with far too little space, or fail and be dereferenced | `err06a` (wraps big), `err06c` (wraps below header size, child-captured), `err06b` (alloc fails → fault) | [x] |
| 7 | `stbds_make_hash_index` (L401) | `slot_count ∈ {0,1,2}` → `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` → abort. **Unreachable via the public API** | `err07_slot_count_is_always_a_power_of_two_at_least_8` proves `slot_count` is always a power of two ≥ 8 through grow *and* shrink | [x] |
| 8 | `stbds_hash_bytes` | `len == 0` (`p` may be `NULL`) | no bytes read; hashes the length-only tail word | `err08_hash_bytes_zero_length_null_pointer` | [x] |
| 9 | `stbds_siphash_bytes` (L534) | `len % 8 == r` for each `r ∈ 0..7` | switch fall-through reads exactly `r` tail bytes; `case 4` sign-extends `d[3]<<24` | `err09_hash_bytes_every_tail_length`, `row03_hash_bytes_high_bit_bytes` | [x] |
| 10 | `stbds_hash_string` (L484) | empty string | loop body never runs; avalanche applied to `seed` alone | `err10_hash_string_empty` | [x] |
| 11 | `stbds_is_key_equal` (L560) | `mode >= 1` — **any** int ≥ 1, incl. `2`, `99`, `INT_MAX` | `strcmp` through the *pointer* at `a+elemsize*i+keyoffset`; `keysize` ignored | `err11_string_lookup_accepts_any_mode_at_or_above_one` | [x] |
| 12 | `stbds_is_key_equal` (L563) | `mode < 1`, incl. `-1`, `-1000`, `INT_MIN` | `memcmp` over `keysize` bytes | `err11_12_binary_lookup_accepts_any_mode_below_one` | [x] |
| 13 | `stbds_hmfree_func` (L573) | `a == NULL` | returns immediately, no free | `err13_hmfree_func_null_is_a_noop` | [x] |
| 14 | `stbds_hmfree_func` (L575) | `hash_table == NULL` | skips key freeing and `strreset`; still frees `hash_table` (NULL) and the header | `err14_hmfree_func_without_hash_table` | [x] |
| 15 | `stbds_hm_find_slot` (L610, L621) | probe reaches `hash == STBDS_HASH_EMPTY` — key absent | returns `-1` | `err15_18_19_lookup_miss_yields_minus_one` | [x] |
| 16 | `stbds_hmget_key_ts` (L633) | `a == NULL` | allocates a 1-element zeroed array, `length=1`, `*temp=-1`, never returns `NULL` | `err16_hmget_key_ts_on_null_bootstraps` | [x] |
| 17 | `stbds_hmget_key_ts` (L648) | `a != NULL` but `hash_table == NULL` | `*temp=-1`, returns `a` unchanged, **key never dereferenced** | `err17_hmget_without_index_never_dereferences_the_key` (passes `NULL` as the key) | [x] |
| 18 | `stbds_hmget_key_ts` (L651) | key not present | `*temp = STBDS_INDEX_EMPTY` = `-1` | `err15_18_19_lookup_miss_yields_minus_one` | [x] |
| 19 | `stbds_hmget_key` (L663) | any of #16-#18 | mirrors `temp` into the array header's `temp` | `err15_18_19...`, `row25_hmget_key_vs_ts_temp_semantics` | [x] |
| 20 | `stbds_hmput_default` (L669) | `a == NULL` | grows from `NULL`, `length=1`, zeroed | `err20_21_22_hmput_default_branches` | [x] |
| 21 | `stbds_hmput_default` (L669) | `a != NULL` but `length == 0` | grows again and re-zeroes element 0 | `err20_21_22_hmput_default_branches` | [x] |
| 22 | `stbds_hmput_default` (L669) | `a != NULL` and `length != 0` | returns `a` **unchanged**, no realloc | `err20_21_22_hmput_default_branches` | [x] |
| 23 | `stbds_hmput_key` (L692) | `a == NULL` | bootstraps a 1-element array before inserting | `err23_24_hmput_key_bootstraps_and_creates_index` | [x] |
| 24 | `stbds_hmput_key` (L703) | `table == NULL` | fresh 8-slot index; `string.mode = (mode>=1 ? SH_DEFAULT : 0)` | `err23_24_hmput_key_bootstraps_and_creates_index` | [x] |
| 25 | `stbds_hmput_key` (L703) | `used_count >= used_count_threshold` | rehash into `slot_count*2`, old table freed, `string`/`seed` carried over | `err25_hmput_key_rehash_at_used_count_threshold` (also checks all three threshold formulas) | [x] |
| 26 | `stbds_hmput_key` (L726) | key already present, found in the forward half of the bucket scan | early return with `temp` = existing index; for `mode>=1` also republishes `temp_key` | `err26_27_hmput_key_duplicate_key_early_returns` | [x] |
| 27 | `stbds_hmput_key` (L740) | key already present, found in the *wrap-around* half | same **but `temp_key` is NOT set** (C asymmetry, preserved) | `err26_27_hmput_key_duplicate_key_early_returns` (compares `temp_key` after every duplicate put) | [x] |
| 28 | `stbds_hmput_key` (L770) | a tombstone was seen before the empty slot | insert reuses the tombstone, `--tombstone_count` | `err28_hmput_key_reuses_tombstones` | [x] |
| 29 | `stbds_hmput_key` (L778) | `arrgrowf` failed to satisfy `i+1 <= arrcap` | `assert` → abort. **Unreachable** without an allocator failure: `arrgrowf(a,elemsize,1,0)` sets `min_cap >= arrlen+1` | covered indirectly by `err25` (asserts capacity always suffices across 300 inserts) | [x] |
| 30 | `stbds_hmput_key` (L786) | `table->string.mode` not in `{1,2,3}` — `0` or any other u8 (`4`, `255`, …) | `default:` → `memcpy(key, keysize)`, `temp_key` left stale | `err30_hmput_key_switch_default_branch_for_unknown_string_mode` | [x] |
| 31 | `stbds_shmode_func` (L800) | `mode` outside `0..3`, e.g. `4`, `-1`, `256`, `259`, `INT_MIN`, `INT_MAX` | `string.mode = (unsigned char) mode` — silently truncated, no validation | `err31_shmode_func_truncates_mode_to_u8` (20 values) | [x] |
| 32 | `stbds_hmdel_key` (L810) | `a == NULL` | returns `0` (`NULL`) | `err32_hmdel_key_null_returns_null` (× every `mode` in `MODES`) | [x] |
| 33 | `stbds_hmdel_key` (L817) | `hash_table == NULL` | forces header `temp = 0`, returns `a` unchanged, key never dereferenced | `err33_hmdel_key_without_index_sets_temp_zero`, `err17...` | [x] |
| 34 | `stbds_hmdel_key` (L822) | key absent | `temp` stays `0`, returns `a`, `length` unchanged | `err34_hmdel_key_absent_key_is_a_noop` (500 absent keys, full state unchanged) | [x] |
| 35 | `stbds_hmdel_key` (L828) | `slot >= table->slot_count` | `assert` → abort. **Unreachable**: `find_slot` masks `pos` by `slot_count-1` | `err07` (slot_count invariant) + `err41_42` (all slots stay in range across 400 deletes) | [x] |
| 36 | `stbds_hmdel_key` (L832) | `used_count < 0` | `assert`; `used_count` is `size_t`, so vacuously true even on underflow | vacuous in C; the Rust uses `wrapping_sub` to match — `err43`, `err34` | [x] |
| 37 | `stbds_hmdel_key` (L846) | re-locating the moved last element fails (`slot < 0`) | `assert` → abort | `row31_shdel_mode2_skips_strdup_free` part (b) and `errg3_del_with_nonzero_keyoffset` — both abort with `SIGABRT` in **both** builds | [x] |
| 38 | `stbds_hmdel_key` (L849) | relocated slot's index `!= final_index` | `assert` → abort | reached only after #37 passes; exercised by `err40`/`err41_42`/`row27` (thousands of relocating deletes, no abort in either build) | [x] |
| 39 | `stbds_hmdel_key` (L838) | `mode == STBDS_HM_STRING` **exactly 1** and `string.mode == SH_STRDUP` | frees the old key string; `mode == 2` does **not** free | `err39_strdup_free_only_at_mode_exactly_one`, `row31_shdel_mode2_skips_strdup_free` | [x] |
| 40 | `stbds_hmdel_key` (L841) | `old_index == final_index` | no memmove, no slot re-point | `err40_hmdel_key_deleting_the_last_element_skips_relocation` | [x] |
| 41 | `stbds_hmdel_key` (L856) | `used_count < used_count_shrink_threshold` **and** `slot_count > 8` | rebuild at `slot_count>>1` | `err41_42_hmdel_key_shrink_and_rebuild` (≥5 shrinks observed) | [x] |
| 42 | `stbds_hmdel_key` (L859) | `tombstone_count > tombstone_count_threshold` | rebuild at the same `slot_count` | `err41_42_hmdel_key_shrink_and_rebuild`, `row28_hmdel_tombstone_rebuild` | [x] |
| 43 | `stbds_hmdel_key` | delete on a `slot_count == 8` table | neither branch fires (`shrink_threshold` forced to 0 for `slot_count <= 8`) | `err43_hmdel_on_default_only_map_never_shrinks` | [x] |
| 44 | `stbds_stralloc` (L913) | `len > a->remaining` after block selection | `assert(len <= a->remaining)` → abort. **Unreachable**: either `len > blocksize` returns early, or a block of `>= len` bytes is installed | `err44_stralloc_remaining_invariant_holds_everywhere` (3200 randomized allocations, invariant holds in both) | [x] |
| 45 | `stbds_stralloc` (L897) | `len > blocksize` and `a->storage == NULL` | dedicated block, `sb->next = 0`, `a->remaining = 0`, returns `sb->storage` | `err45_46_stralloc_dedicated_block_paths` | [x] |
| 46 | `stbds_stralloc` (L897) | `len > blocksize` and `a->storage != NULL` | dedicated block spliced in *after* the head; `a->remaining` **left unchanged** | `err45_46_stralloc_dedicated_block_paths` | [x] |
| 47 | `stbds_stralloc` (L890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (1<<20) | `a->block` stops incrementing (saturates at 22) | `err47_stralloc_block_counter_saturates_at_22` | [x] |
| 48 | `stbds_stralloc` (L888) | caller-supplied `a->block` outside the reachable `0..22` — any value in `0..=255` | shift count `block>>1` can exceed the width of `size_t` (C UB). x86-64 gcc emits a `shl` whose count is taken **mod 64**; for `block >= ~46` the resulting multi-GB allocation fails and the C dereferences `NULL` | `err48_stralloc_with_out_of_range_block_counter` — **all 256 values × 2 lengths**, each in its own child pair, comparing captured arena state *and* termination. The Rust masks the shift explicitly (`src/strings.rs`) so this is not left to the Rust backend | [x] |
| 49 | `stbds_stralloc` | empty string `""` (`len == 1`) | consumes 1 byte of the arena | `err49_stralloc_empty_string` (600 in a row) | [x] |
| 50 | `stbds_strreset` (L921) | zeroed / already-reset arena | loop skipped, arena re-zeroed (**including `block` and `mode`**) | `err50_strreset_on_zeroed_arena` | [x] |
| 51 | `stbds_arrfreef` | `a == NULL` | `free((char*)NULL - 32)` — wild free | `err51_arrfreef_null_faults_identically` (both die on the same signal) | [x] |
| 52 | `strkey` (L940) | `n < 0`, incl. `INT_MIN` | `"test_-2147483648"` | `err52_53_strkey_negative_and_shared_buffer`, `row35_strkey` (4019 values) | [x] |
| 53 | `strkey` | any `n` | always the same `static` 256-byte buffer; the previous result is clobbered | `err52_53_strkey_negative_and_shared_buffer` | [x] |
| 54 | `hm_geti` (L947) | `num <= 0` | all loops skipped; the four leading asserts still run and must hold | `err54_55_hm_geti_degenerate_num` (`INT_MIN`, -1000, -2, -1, 0, 1) | [x] |
| 55 | `hm_geti` (L952) | any `num` | 13 distinct live `STBDS_ASSERT`s; any mismatch aborts | `row36_hm_geti_end_to_end` (18 `num` values × 5 seeds, child-compared) | [x] |

## Generic FFI-boundary boundaries

| # | case | expectation | test | [x] |
|---|------|-------------|------|-----|
| G1 | `stbds_hash_bytes(NULL, 0, seed)` | no deref; equal hashes | `err08_hash_bytes_zero_length_null_pointer` | [x] |
| G2 | `keysize == 0`, binary mode | `memcmp(_,_,0) == 0` and a constant hash → every key compares equal | `errg2_keysize_zero_makes_every_key_equal` | [x] |
| G3 | `mode` = `INT_MIN`, `-1000`, `-2`, `-1`, `0`, `1`, `2`, `3`, `4`, `99`, `1000`, `INT_MAX` across all four `hm*` entry points | binary iff `mode < 1`; `hmdel_key`'s strdup-free and relocate only at `mode == 1` | `err11_12_*`, `err11_*`, `err32_*`, `err39_*` | [x] |
| G3b | `hmdel_key` with `keyoffset != 0` (a value the macros never generate) | compares the wrong bytes; misses or aborts on the relocate assert | `errg3_del_with_nonzero_keyoffset` (child-compared) | [x] |
| G3c | `hmget_key_ts` with `temp == NULL` | writes through the NULL pointer → fault | `errg3_null_temp_pointer_faults_identically` | [x] |
| G4 | `shmode_func` mode = `-1`, `0`..`5`, `127`, `128`, `255`, `256`..`260`, `1000`, `INT_MIN`, `INT_MAX` | `string.mode = mode as u8` | `err31_shmode_func_truncates_mode_to_u8` | [x] |
| G5 | `stbds_rand_seed(0)` / `(usize::MAX)` / `1<<63` then build tables | the first table records the seed verbatim; the global advances by `seed*a+b` | `errg5_rand_seed_extremes`, `row05_rand_seed_and_table_seed_chain` | [x] |
| G6 | `arrgrowf` with `addlen`/`min_cap` = `0`,`1`,`3`,`4`,`5`,`1<<20`,`usize::MAX` | identical header and identical no-op-vs-grow decision | `errg6_arrgrowf_extreme_arguments`, `err06a/b/c` | [x] |
| G7 | `string.mode == 4` (one past `SH_ARENA`) reaching `hmput_key`'s `switch` | `default:` memcpy branch | `err30_hmput_key_switch_default_branch_for_unknown_string_mode` | [x] |

## Notable ground-truth quirks confirmed (and reproduced, not fixed)

* `stbds_hash_bytes` **ignores its `seed`**: `stbds_siphash_bytes` XORs `seed`
  into each state word twice (`v0 = K0 ^ seed; v0 ^= C0 ^ seed;`), so it cancels.
  Binary-key bucket layouts are therefore seed-independent —
  `row37a`/`row37d` pin this down. `stbds_hash_string` *does* use the seed
  (`row37b`/`row37c`).
* `stbds_hmput_key` publishes `temp_key` only in the forward half of the bucket
  scan (row 27).
* `stbds_hmdel_key` tests `mode == STBDS_HM_STRING` exactly, while everything
  else tests `mode >= STBDS_HM_STRING`; with `mode >= 2` a relocating delete
  hashes the raw element bytes and aborts on `assert(slot >= 0)` (row 37/39).
* `stbds_arrfreef(NULL)` and `stbds_arrgrowf`'s unchecked `realloc` result are
  both reproduced verbatim.
