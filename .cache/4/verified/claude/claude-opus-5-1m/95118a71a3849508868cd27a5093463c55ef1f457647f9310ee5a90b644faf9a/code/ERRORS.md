# ERRORS.md — Error / rejection surface (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum
and no `RETURN_ERROR` macro**; it rejects input in exactly four ways:

1. an **early `return` of a sentinel** (`NULL`, `-1`/`STBDS_INDEX_EMPTY`,
   `-2`/`STBDS_INDEX_DELETED`, the unchanged input pointer),
2. an **out-parameter set to a sentinel** (`*temp = -1`),
3. an **`assert()`** (`#define STBDS_ASSERT assert`) → `__assert_fail` → `abort`
   (SIGABRT). Rust mirrors these with `std::process::abort()` (SIGABRT).
4. **no check at all** — the C dereferences/frees unconditionally (UB). Those
   rows are listed because they are part of the surface, and marked
   `UB (not differential-testable)`; a test that provoked them would abort or
   corrupt the heap in *both* libraries, so parity is asserted by inspection
   of the identical pointer arithmetic instead of by execution.

Every `if`/`assert`/`return`-sentinel site found by
`grep -n 'return\|ASSERT\|== NULL\|!= NULL\|== 0\|< 0\|if ('` is accounted for
below (branch-only `if`s that select between two valid behaviours live in
`CONFIGS.md`, not here).

Legend for the test column: name of the `#[test]` in `tests/` that covers it.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` (286) | `min_cap <= stbds_arrcap(a)` — request no bigger than what is already there (incl. `a=NULL, elemsize=X, addlen=0, min_cap=0`) | returns `a` **unchanged and unallocated** (so `NULL` in, `NULL` out — *no* header is created) | **PASS** `err01_arrgrowf_no_grow_returns_input` |
| 2 | `stbds_arrgrowf` (283) | `addlen` so large that `arrlen+addlen` wraps `size_t` (e.g. `addlen = SIZE_MAX`) | wraps: `min_len = 0`, so `min_cap` stays; with `min_cap=0` → row 1 path, returns `a` | **PASS** `err02_arrgrowf_addlen_overflow` |
| 3 | `stbds_arrgrowf` (297) | `elemsize * min_cap + 32` overflows / is absurd (`elemsize = SIZE_MAX`) → `realloc` returns `NULL`, no check | UB (not differential-testable): writes to `NULL+32`. Identical arithmetic in Rust. | inspection |
| 4 | `stbds_arrfreef` (312–315) | `a == NULL` — no null check | UB (not differential-testable): `free((char*)NULL - 32)`. Rust performs the same `wrapping_sub(32)` + `free`. | inspection |
| 5 | `stbds_make_hash_index` (401) | `used_count_threshold + tombstone_count_threshold >= slot_count`, i.e. `slot_count ∈ {0,1,2,3}` | `assert` → SIGABRT | unreachable from the public API (only callers pass 8, `slot_count*2`, `slot_count>>1` with `slot_count>8`); documented, not executed |
| 6 | `stbds_hash_string` (480) | `str` points at `""` (empty string) — loop body never runs | returns the seed-only avalanche value (a specific `size_t`, **not** an error) | **PASS** `err06_hash_string_empty`, `cfg08_hash_string_empty` |
| 7 | `stbds_hash_string` (480) | `str == NULL` | UB (not differential-testable): dereferences `NULL`. Rust dereferences the same null. | inspection |
| 8 | `stbds_hash_bytes` (553) / `stbds_siphash_bytes` (522,532) | `len == 0` (with any `p`, incl. `NULL`) — the block loop and every `switch` case are skipped (`case 0: break`) | returns the finalisation of `data = 0` — a well-defined `size_t`, no read of `p` | **PASS** `err08_hash_bytes_zero_len_null_ptr`, `cfg01_hash_bytes_zero_len` |
| 9 | `stbds_hash_bytes` | `len` huge (e.g. `SIZE_MAX`) with a short buffer | UB (not differential-testable): reads past the end. Same loop bounds in Rust. | inspection |
| 10 | `stbds_hmfree_func` (573) | `a == NULL` | **returns immediately**, frees nothing | **PASS** `err10_hmfree_null_is_a_noop` |
| 11 | `stbds_hmfree_func` (574) | `stbds_header(a)->hash_table == NULL` (array made by `stbds_arrgrowf`, never `hmput`) | skips the strdup sweep and `strreset`; still `free(hash_table)` (= `free(NULL)`, legal) and `free(header)` | **PASS** `err11_hmfree_without_hash_table`, `cfg56_hmfree_variants` |
| 12 | `stbds_hm_find_slot` (609–610, 620–621) | key absent — probe hits `bucket->hash[i] == STBDS_HASH_EMPTY (0)` | returns `-1` | **PASS** `err12_get_missing_key_binary_and_string` |
| 13 | `stbds_hmget_key_ts` (634) | `a == NULL` (get on an empty map) | allocates a 1-slot array, `length=1`, zeroed elem 0, sets `*temp = STBDS_INDEX_EMPTY (-1)`, returns `arr+elemsize` | **PASS** `err13_17_get_on_null_map_bootstraps_and_returns_minus_one` |
| 14 | `stbds_hmget_key_ts` (644) | `a != NULL` but `hash_table == NULL` (e.g. built by `stbds_hmput_default`) | `*temp = -1`, returns `a` unchanged | **PASS** `err14_get_on_map_without_hash_table`, `cfg46_47_get_on_empty_and_indexless_maps` |
| 15 | `stbds_hmget_key_ts` (648) | `stbds_hm_find_slot(...) < 0` (key not present) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` | **PASS** `err12_get_missing_key_binary_and_string` |
| 16 | `stbds_hmget_key` (659–664) | same three triggers as 13/14/15 | in addition writes the sentinel into `stbds_header(arr)->temp`, so `stbds_temp(t-1) == -1` | **PASS** `err12_...`, `err13_17_...` |
| 17 | `stbds_hmget_key` (663) | `a == NULL` (first ever call) | `stbds_temp(STBDS_HASH_TO_ARR(p))` write targets the freshly-created header → `temp == -1` | **PASS** `err13_17_get_on_null_map_bootstraps_and_returns_minus_one` |
| 18 | `stbds_hmput_default` (669) | `a == NULL` | allocates, `length=1`, elem 0 zeroed, returns `arr+elemsize` | **PASS** `err18_19_20_hmput_default_paths`, `cfg31_hmput_default_from_null` |
| 19 | `stbds_hmput_default` (669) | `a != NULL` **and** `stbds_header(arr)->length == 0` | re-grows/re-inits (`length` 0→1, elem 0 zeroed) | **PASS** `err18_19_20_hmput_default_paths`, `cfg33_hmput_default_on_zero_length_array` |
| 20 | `stbds_hmput_default` (675) | `a != NULL` and `length != 0` | returns `a` unchanged, **does not** touch element `-1` | **PASS** `err18_19_20_hmput_default_paths`, `cfg32_hmput_default_idempotent` |
| 21 | `stbds_hmput_key` (686) | `a == NULL` | bootstraps the array (`length=1`, elem 0 zeroed) before inserting | **PASS** `err21_22_put_on_null_map_sets_string_mode` + every `cfg37..cfg70` put test |
| 22 | `stbds_hmput_key` (698) | `table == NULL` | creates an 8-slot index and sets `string.mode = (mode >= 1 ? SH_DEFAULT : 0)` | **PASS** `err21_22_put_on_null_map_sets_string_mode` |
| 23 | `stbds_hmput_key` (698) | `used_count >= used_count_threshold` (6 for 8 slots, 12 for 16, 24 for 32 …) | rehashes into a `slot_count*2` index and frees the old one | **PASS** `cfg39/cfg40_binary_multiple_growths`, `cfg51_52_del_rebuild_and_shrink` |
| 24 | `stbds_hmput_key` (778) | `(size_t)i+1 > stbds_arrcap(a)` after the `arrgrowf` on line 775 (cannot happen; `arrgrowf` always satisfies it) | `assert` → SIGABRT | unreachable; documented |
| 25 | `stbds_hmput_key` (729/747) | key **already present** | does **not** insert: sets `temp = existing index`, returns without touching `used_count`/`length`; for `mode >= 1` the *first* loop also updates `temp_key`, the wrapped-around loop does **not** (stb quirk — preserved) | **PASS** `err25_put_duplicate_key_does_not_insert`, `err25b_duplicate_key_in_wrapped_probe_loop`, `cfg44/cfg61` |
| 26 | `stbds_shmode_func` (796–804) | `mode` outside `{0,1,2,3}` (e.g. `-1`, `4`, `255`, `256`, `INT_MIN`) — C enums accept any `int`, and the value is stored as `(unsigned char) mode` | stores the **truncated** byte in `string.mode`; a later `hmput_key` then falls into the `default:` (memcpy) arm for any truncated value ∉ {1,2,3} | **PASS** `err26_shmode_out_of_range_enum`, `cfg36_shmode_func_out_of_range_modes` |
| 27 | `stbds_hmdel_key` (809–810) | `a == NULL` | returns `NULL` (`0`) — **no** `temp` is written | **PASS** `err27_del_on_null_map_returns_null` |
| 28 | `stbds_hmdel_key` (816) | `hash_table == NULL` | sets `stbds_temp(arr) = 0` then returns `a` | **PASS** `err28_del_on_map_without_hash_table` |
| 29 | `stbds_hmdel_key` (821) | `stbds_hm_find_slot(...) < 0` (key absent) | `stbds_temp(arr) = 0`, returns `a`, map untouched | **PASS** `err29_del_missing_key_leaves_map_untouched` |
| 30 | `stbds_hmdel_key` (828) | `slot >= (ptrdiff_t) table->slot_count` | `assert` → SIGABRT | unreachable (`find_slot` masks with `slot_count-1`); documented |
| 31 | `stbds_hmdel_key` (832) | `table->used_count < 0` | `used_count` is `size_t`, so the assert is a tautology and never fires (C quirk) — Rust omits it with a comment | inspection |
| 32 | `stbds_hmdel_key` (846) | re-find of the moved-in last element fails (`slot < 0`) | `assert` → SIGABRT. **Reachable** by passing a `keyoffset` that does not match the one used at insert time | **PASS** `err32_del_mode2_refind_aborts` (child process, SIGABRT parity) |
| 33 | `stbds_hmdel_key` (849) | `b->index[i] != final_index` | `assert` → SIGABRT | unreachable (same root cause as row 32: the re-find either fails — row 32, which *is* tested — or returns the slot that holds `final_index`); documented |
| 34 | `stbds_hmdel_key` (836) | `mode == STBDS_HM_STRING` **and** `string.mode == SH_STRDUP` | frees the stored key; note the check is `==1`, so `mode == 2` (`PTR_TO_STRING`) **leaks** instead (quirk — preserved) | **PASS** `cfg64_strdup_map_deleted_with_mode2_does_not_free` |
| 35 | `stbds_hmdel_key` (854) | `used_count < used_count_shrink_threshold && slot_count > 8` | shrinks to `slot_count>>1` | **PASS** `cfg51_52_del_rebuild_and_shrink` (asserts the branch is taken) |
| 36 | `stbds_hmdel_key` (858) | `tombstone_count > tombstone_count_threshold` | rebuilds at the same `slot_count` | **PASS** `cfg51_52_del_rebuild_and_shrink` (asserts the branch is taken) |
| 37 | `stbds_hmdel_key` (839) | `old_index == final_index` (deleting the last element) | skips the memmove **and** the index fix-up | **PASS** `cfg48_del_last_element`, `cfg63_string_del_both_index_cases` |
| 38 | `stbds_stralloc` (885) | `len > a->remaining` (fresh arena has `remaining == 0`, so always on the first call) | allocates a new block; `blocksize = 512 << (a->block>>1)` | **PASS** `cfg22..cfg29`, `err39_...` |
| 39 | `stbds_stralloc` (890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`, i.e. `a->block >= 22` | stops incrementing `a->block` (saturation) | **PASS** `err39_stralloc_block_counter_saturation`, `cfg27_stralloc_forged_block_counter` |
| 40 | `stbds_stralloc` (893) | `len > blocksize` (string longer than a whole block) | dedicated over-sized block spliced **after** `a->storage` (or made the head when `a->storage == NULL`, which also forces `remaining = 0`); returns `sb->storage` and **does not** touch `remaining` in the non-empty case | **PASS** `cfg24_stralloc_oversize_on_empty_arena`, `cfg25_stralloc_oversize_on_nonempty_arena` |
| 41 | `stbds_stralloc` (913) | `len > a->remaining` after the block dance | `assert` → SIGABRT — **unreachable**: the `len > blocksize` branch returns early, and the `else` branch sets `remaining = blocksize >= len`; documented, not executed |
| 42 | `stbds_stralloc` (881) | `a == NULL` or `str == NULL` — no null checks | UB (not differential-testable): identical null deref in Rust | inspection |
| 43 | `stbds_strreset` (924) | `a->storage == NULL` (empty/fresh arena) | the `while` loop body never runs; the whole arena is `memset` to 0 | **PASS** `err43_strreset_empty_arena`, `cfg29_strreset_various_chain_lengths` |
| 44 | `stbds_strreset` (920) | `a == NULL` | UB (not differential-testable) | inspection |
| 45 | `stbds_is_key_equal` (560) | `mode` **out of enum range**: any `mode >= 1` (2, 3, 99, `INT_MAX`) selects `strcmp`; any `mode <= 0` (0, `-1`, `INT_MIN`) selects `memcmp` | the comparison mode is chosen by `>=`, not by equality, so out-of-range ints are silently accepted and routed | **PASS** `err45_mode_out_of_range_enum_routing`, `cfg66/cfg67` |
| 46 | `stbds_hmput_key` (713) / `stbds_hm_find_slot` (590) | `mode >= 1` → the key is treated as a `char*` and `strcmp`/`hash_string`ed; passing a **binary** key with `mode = 1` reads past the key | UB (not differential-testable) unless the bytes happen to be NUL-terminated | inspection |
| 47 | `stbds_hm_find_slot` (596) / `stbds_hmput_key` (719) | `hash < 2` (a key whose hash is 0 or 1, i.e. collides with `HASH_EMPTY`/`HASH_DELETED`) | `hash += 2` — the sentinel values are never used as real hashes | n/a — see note (2) below |
| 48 | `strkey` (939) | `n == INT_MIN` / `INT_MAX` / negative | `sprintf("test_%d")` → `"test_-2147483648"` (16 chars, fits the 256-byte static buffer) | **PASS** `cfg71_strkey` (full 256-byte buffer compared) |
| 49 | `arr_del` (945) | any `num` (incl. `INT_MIN`, `INT_MAX`) — the loop's `arrdel(arr,3)` memmoves `sizeof*a * (4-1-3) == 0` bytes, and `arrdelswap(arr,3)` self-assigns | no error; returns `void`, all four allocations freed | **PASS** `cfg21_arr_del_all_inputs` |

## Notes on the rows that are not executed

1. **`inspection`** rows are cases where the C performs no check at all and
   dereferences / frees an invalid pointer (rows 3, 4, 7, 9, 42, 44, 46) or
   where the check is a tautology (row 31). A test would have to provoke
   identical undefined behaviour in both libraries; instead the Rust source was
   read line-by-line against the C to confirm the *same* unchecked arithmetic:
   * row 3 `stbds_arrgrowf`: `elemsize * min_cap + 32` — `array.rs` uses
     `wrapping_mul`/`wrapping_add`, then writes through `realloc`'s result
     unconditionally, exactly like lib.c:297-307.
   * row 4 `stbds_arrfreef(NULL)`: `array.rs:68` does
     `STBDS_FREE(stbds_header(a))` with `wrapping_sub(32)`, like lib.c:314.
   * rows 7/44 null `str`/`a`: dereferenced without a check in both.
   * row 9 oversized `len`: identical loop bounds.
   * row 42/46: identical.
   * row 31: `used_count` is `size_t`, so `>= 0` always holds; `hashmap.rs:460`
     keeps it as a comment.
2. **Row 47 (`if (hash < 2) hash += 2`)** cannot be reached by search: it needs
   a key/seed pair whose full 64-bit siphash or `hash_string` output is 0 or 1
   (probability 2^-63). The branch is nevertheless covered *indirectly and
   exactly*: every Phase B test byte-compares the complete `hash[]` array of
   every bucket, so an implementation that applied the fix-up differently would
   diverge on the first key it affected. The two sources perform it at the same
   point (lib.c:596 / lib.c:719 vs `hashmap.rs:78` / `hashmap.rs:280`).
3. **`unreachable`** rows (5, 24, 30, 33, 41) are `assert`s that the callers
   cannot violate:
   * 5: `stbds_make_hash_index` is `static`; its only callers pass `8`,
     `slot_count*2` and `slot_count>>1` guarded by `slot_count > 8`.
   * 24: the preceding `stbds_arrgrowf(a, elemsize, 1, 0)` always makes
     `arrcap >= i+1`.
   * 30: `stbds_hm_find_slot` masks the position with `slot_count-1`.
   * 33: same root cause as row 32; the re-find either fails (row 32, which
     *is* tested) or returns the slot holding `final_index`.
   * 41: the `len > blocksize` branch returns early and the `else` branch sets
     `remaining = blocksize >= len`.

## Result

All 30 reachable rows have a passing differential test (`tests/diff_errors.rs`,
plus the `cfg*` tests in `tests/diff_lowlevel.rs` / `tests/diff_map.rs`).
Row 32 is verified as **abort parity**: both libraries are run in a child
process and both die from signal 6 (SIGABRT) with the same exit status.
