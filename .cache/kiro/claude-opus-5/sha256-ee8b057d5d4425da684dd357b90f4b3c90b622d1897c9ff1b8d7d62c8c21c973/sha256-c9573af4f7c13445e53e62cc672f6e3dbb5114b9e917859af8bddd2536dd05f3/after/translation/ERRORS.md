# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping every early `return`,
every sentinel value, every `STBDS_ASSERT`, every explicit range/null check and
every min/max constant.  Line numbers refer to `c_src/src/lib.c`.

This library has **no error enum and no `RETURN_ERROR` macro**.  It signals
rejection in exactly four ways, and every row below is one of them:

* an early `return` of an unchanged / freshly-allocated pointer (a no-op),
* a `NULL` (`0`) return,
* a `-1` (`STBDS_INDEX_EMPTY`) / `-2` (`STBDS_INDEX_DELETED`) sentinel written
  to `*temp` or to `header->temp`,
* `assert()` → `SIGABRT` (the CMake build defines no `NDEBUG`, so asserts are
  live in the C `.so`).

`RES` below is shorthand for "the pointer the function returns".

| # | function | trigger (exact invalid input / condition) | expected C result |
|---|----------|-------------------------------------------|-------------------|
| 1 | `stbds_arrgrowf` (L287) | `min_cap <= arrcap(a)` after `min_cap = max(min_cap, arrlen(a)+addlen)` — i.e. growth not needed (`a` non-null, `addlen=0`, `min_cap<=cap`) | returns `a` **unchanged**, no realloc, header untouched |
| 2 | `stbds_arrgrowf` (L283) | `addlen` so large that `arrlen(a)+addlen` wraps `size_t` | wrapped `min_len` used; **no** overflow check — must wrap identically |
| 3 | `stbds_arrgrowf` (L291) | fresh array (`a == NULL`), `addlen==0`, `min_cap` in `1..3` | `min_cap` is bumped to **4** (`else if (min_cap < 4)`) |
| 4 | `stbds_arrgrowf` (L289) | `a` non-null and `min_cap < 2*arrcap(a)` | `min_cap` bumped to `2*arrcap(a)`; the `< 4` clamp is **skipped** |
| 5 | `stbds_arrgrowf` (L300) | `a == NULL` | `length`,`hash_table`,`temp` zero-initialised; when `a != NULL` they are **left as-is** |
| 6 | `stbds_arrfreef` (L311) | `a == NULL` | **no null check** — `free(header(NULL))` = `free((void*)-32)`; not exercised (would crash both) |
| 7 | `stbds_hmfree_func` (L573) | `p == NULL` | returns immediately, no free |
| 8 | `stbds_hmfree_func` (L574) | `header(p)->hash_table == NULL` (array built by `arrgrowf`, never `hmput`) | skips strdup-sweep and `strreset`; still `free()`s `hash_table` (NULL) + header |
| 9 | `stbds_hm_find_slot` (L610) | probe hits `bucket->hash[i] == STBDS_HASH_EMPTY` (0) in the forward half-scan → key absent | returns `-1` |
|10 | `stbds_hm_find_slot` (L621) | probe hits `STBDS_HASH_EMPTY` in the wrap-around half-scan (`i < pos & 7`) → key absent | returns `-1` |
|11 | `stbds_hm_find_slot` (L596) | computed `hash < 2` (collides with `HASH_EMPTY`=0 / `HASH_DELETED`=1) | `hash += 2` before probing |
|12 | `stbds_hmget_key_ts` (L634-639) | `a == NULL` | allocates a 1-element array, `*temp = -1`, returns a **non-NULL** hash pointer |
|13 | `stbds_hmget_key_ts` (L644) | `a != NULL` but `header(raw_a)->hash_table == 0` | `*temp = -1`, returns `a` unchanged |
|14 | `stbds_hmget_key_ts` (L648) | key not present (`find_slot < 0`) | `*temp = STBDS_INDEX_EMPTY` (`-1`), returns `a` unchanged |
|15 | `stbds_hmget_key_ts` | slot found but `bucket->index[slot&7]` is `-2` (`INDEX_DELETED`) | `*temp` = `-2` verbatim (no filtering) |
|16 | `stbds_hmget_key` (L661) | any of rows 12–15 | additionally writes the same sentinel into `header(RES-elemsize)->temp` |
|17 | `stbds_hmput_default` (L669) | `a == NULL` | grows a fresh 1-element zeroed array, returns non-NULL hash pointer |
|18 | `stbds_hmput_default` (L669) | `a != NULL` but `header(raw_a)->length == 0` | grows again and `length += 1` (so `length` becomes 1) |
|19 | `stbds_hmput_default` (L675) | `a != NULL` and `length != 0` | returns `a` unchanged (no allocation) |
|20 | `stbds_hmput_key` (L686) | `a == NULL` | bootstraps a 1-element zeroed array first, then proceeds |
|21 | `stbds_hmput_key` (L698) | `table == NULL` | builds an 8-slot index; `nt->string.mode = (mode>=1 ? SH_DEFAULT : 0)` |
|22 | `stbds_hmput_key` (L698) | `table->used_count >= table->used_count_threshold` (`slot_count - slot_count/4`) | rehashes into `slot_count*2` |
|23 | `stbds_hmput_key` (L719) | `hash < 2` | `hash += 2` |
|24 | `stbds_hmput_key` (L739,755) | probe passes a `bucket->index[i] == STBDS_INDEX_DELETED` tombstone | first tombstone remembered, reused, `--tombstone_count` |
|25 | `stbds_hmput_key` (L778) | `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` | abort if violated (unreachable through the public API) |
|26 | `stbds_hmput_key` (L787-791) | `table->string.mode` **not** in `{SH_STRDUP,SH_ARENA,SH_DEFAULT}` (i.e. `SH_NONE`, or an out-of-range value set via `shmode_func`) | `default:` branch → `memcpy(elem, key, keysize)`; `temp_key` **not** written |
|27 | `stbds_hmput_key` (L732) | duplicate key found in the *forward* half-scan with `mode >= STBDS_HM_STRING` | `temp_key` **is** updated |
|28 | `stbds_hmput_key` (L747-751) | duplicate key found in the *wrap-around* half-scan | `temp_key` is **NOT** updated (asymmetry in the C — preserved verbatim) |
|29 | `stbds_hmdel_key` (L809) | `a == NULL` | returns `0` (**NULL**) |
|30 | `stbds_hmdel_key` (L816) | `header(raw_a)->hash_table == 0` | `header->temp = 0`, returns `a` unchanged, length untouched |
|31 | `stbds_hmdel_key` (L821) | key not found (`find_slot < 0`) | `header->temp = 0`, returns `a`, length untouched |
|32 | `stbds_hmdel_key` (L828) | `STBDS_ASSERT(slot < (ptrdiff_t) table->slot_count)` | abort if violated |
|33 | `stbds_hmdel_key` (L832) | `STBDS_ASSERT(table->used_count >= 0)` | `used_count` is `size_t`, so always true — never fires |
|34 | `stbds_hmdel_key` (L846) | `STBDS_ASSERT(slot >= 0)` on the re-find of the moved last element | abort if the moved key cannot be found |
|35 | `stbds_hmdel_key` (L849) | `STBDS_ASSERT(b->index[i] == final_index)` | abort if the re-found slot does not point at the moved element |
|36 | `stbds_hmdel_key` (L836) | `mode == STBDS_HM_STRING` (**exact** `==`, not `>=`) **and** `string.mode == SH_STRDUP` | frees the old key; `mode == 2` does **not** free |
|37 | `stbds_hmdel_key` (L842) | `mode == STBDS_HM_STRING` on the re-find | key is *dereferenced* (`*(char**)elem`); for `mode == 2` the element address itself is passed while `find_slot` still does `strcmp` (PTR_TO_STRING semantics) |
|38 | `stbds_hmdel_key` (L858) | `used_count < used_count_shrink_threshold && slot_count > 8` | index rebuilt at `slot_count>>1` |
|39 | `stbds_hmdel_key` (L861) | `tombstone_count > tombstone_count_threshold` (`slot_count/8 + slot_count/16`) | index rebuilt at the same `slot_count` |
|40 | `stbds_hmdel_key` | delete of the *last* element (`old_index == final_index`) | the compaction `memmove` + re-find is **skipped** |
|41 | `stbds_make_hash_index` (L401) | `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold < slot_count)` | abort; holds for every `slot_count >= 8` reachable here |
|42 | `stbds_make_hash_index` (L398) | `slot_count <= STBDS_BUCKET_LENGTH` (8) | `used_count_shrink_threshold` forced to **0** (so an 8-slot table never shrinks) |
|43 | `stbds_make_hash_index` (L403) | `ot == NULL` | fresh `seed` from the global `stbds_hash_seed`, then the LCG advances the global; `ot != NULL` inherits `seed` + `string` and does **not** advance it |
|44 | `stbds_stralloc` (L913) | `STBDS_ASSERT(len <= a->remaining)` | abort if the arena bookkeeping is inconsistent |
|45 | `stbds_stralloc` (L885) | `len > a->remaining` | allocate a new block |
|46 | `stbds_stralloc` (L893) | `len > blocksize` (oversized string, `blocksize = 512 << (block>>1)`) | dedicated block spliced in; `a->remaining` set to 0 only when the arena was empty |
|47 | `stbds_stralloc` (L890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (`1<<20`) | `a->block` stops incrementing (so `block` saturates at 22) |
|48 | `stbds_stralloc` | `str` is `""` (`len == 1`) | still consumes 1 byte of the arena |
|49 | `stbds_strreset` (L924) | `a->storage == NULL` (zeroed arena) | loop body never runs; arena memset to 0 anyway |
|50 | `stbds_shmode_func` (L804) | `mode` outside the `STBDS_SH_*` enum (e.g. `4`, `255`, `256`, `-1`) | stored as `(unsigned char) mode` — **truncated**, no validation |
|51 | `stbds_is_key_equal` (L560) | `mode` out-of-range **negative** (e.g. `-1`, `INT_MIN`) | `mode >= STBDS_HM_STRING` false → `memcmp` (binary) path |
|52 | `stbds_is_key_equal` (L560) | `mode` out-of-range **positive** (e.g. `2`, `1000`, `INT_MAX`) | `mode >= STBDS_HM_STRING` true → `strcmp` path |
|53 | `stbds_hash_bytes` / `stbds_hmput_key` | `keysize == 0` | `memcmp(...,0)` returns 0 → *every* key compares equal; siphash of 0 bytes is well-defined |
|54 | `stbds_hash_string` (L466) | `str` points at `""` | loop skipped; still mixes and returns `hash+seed` |
|55 | `stbds_hash_string` (L468) | bytes with the high bit set (`>= 0x80`) | added as `(unsigned char)`, **not** sign-extended |
|56 | `stbds_hash_bytes` (L520-521) | any 8-byte block whose byte 3 has the high bit set | `d[3] << 24` is computed in `int` → the value is **sign-extended** into `size_t`, so the upper 32 bits become all-ones before being OR'd with the high word. Must be reproduced bit-for-bit |
|57 | `stbds_hash_bytes` (L529-537) | `len % 8` in `1..7` (tail `switch` fall-through), esp. `len%8 == 4` with `d[3] >= 0x80` | same `int` sign-extension in `case 4` |
|58 | `stbds_hash_bytes` | `len == 0` | loop and all `switch` cases skipped, `data = 0 << 56` |
|59 | `strkey` (L940) | `n` large / negative (`INT_MIN`) | `sprintf` into the 256-byte static `buffer`, no bounds check; returns the shared buffer pointer |
|60 | `arr_del` (L947) | any `int num`, incl. `INT_MIN`/`INT_MAX` | pure allocate/delete/free; observably a no-op that must not abort |

## Rows intentionally NOT turned into executable tests

**Asserts / missing null checks unreachable through the public API**
(rows **6, 25, 32, 33, 34, 35, 41, 44**).  Firing them would `SIGABRT` the C
`.so` and take the test harness down with it, which proves nothing about parity.
They are covered *indirectly and continuously*: the C `.so` is built with asserts
live (the CMake build defines no `NDEBUG`), so if the Rust and C bookkeeping ever
diverged in a way that violates one of these invariants, the C side would abort
during any Phase B/C test that drives `hmput_key` / `hmdel_key` / `stralloc`.

**Row 2 (partially)** — `arrlen + addlen` wrapping `size_t`.  The safe variant
(the wrapped `min_len` taking the `min_cap <= arrcap` early-out) is tested in
`b_arr::err_row_2_addlen_wraparound`, and `elemsize * min_cap` wrapping to
exactly 0 in `c_errors::zero_and_oversized_lengths`.  Wraps that land on a
*small non-zero* byte size are not executed: the C writes a 32-byte header
regardless of the size it asked `realloc` for, so both libraries overflow the
heap by the same amount and abort the process — matching behaviour, but it
destroys the harness.

**Rows 11 and 23** — the `if (hash < 2) hash += 2` guard.  Observing it needs a
siphash-2-4 or `stbds_hash_string` preimage of exactly 0 or 1, a 2^-63 event that
cannot be constructed.  `c_errors::err_rows_11_15_23_hash_below_two_premises`
instead pins the premise the guard rests on: 200 000 randomized inputs confirm
the two libraries produce bit-identical hashes, so they necessarily make the same
`< 2` decision (and confirm no hash `< 2` is reachable by sampling).

**Row 15** — `*temp` receiving `STBDS_INDEX_DELETED` (`-2`).  Structurally
impossible: a slot only matches when `bucket->hash[i] == hash`, every probe hash
is `>= 2` (row 11), and every tombstone stores `hash == 1`.  The same test
verifies this over a 600-op delete/insert churn: every bucket slot is either
`(hash 0, index -1)`, `(hash 1, index -2)`, or `(hash >= 2, index >= 0)`.

All other rows — 1, 3, 4, 5, 7, 8, 9, 10, 12, 13, 14, 16, 17, 18, 19, 20, 21,
22, 24, 26, 27, 28, 29, 30, 31, 36, 37, 38, 39, 40, 42, 43, 45, 46, 47, 48, 49,
50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60 — have executable differential tests.

## Row to test mapping

| rows | test |
|------|------|
| 1, 3, 4, 5 | `b_arr::row_16_grow_from_null_min_cap`, `row_18_grow_noop`, `row_19_20_grow_doubling_vs_exact` |
| 2 | `b_arr::err_row_2_addlen_wraparound`, `c_errors::zero_and_oversized_lengths` |
| 7, 8 | `b_arena::row_73_hmfree_func_shapes`, `c_errors::null_pointer_boundary` |
| 9, 10 | `c_errors::err_rows_9_10_find_slot_returns_minus_one` |
| 12, 13, 14, 16 | `b_hmap_binary::row_31_32_get_ts_present_absent`, `row_33_35_get_bootstrap_paths`, `c_errors::null_pointer_boundary` |
| 17, 18, 19 | `b_hmap_binary::row_36_39_hmput_default` |
| 20, 21, 22 | `b_hmap_binary::row_24_26_put_counts`, `c_errors::cfg_row_79_hmput_key_null_string_mode` |
| 24, 39 | `b_hmap_binary::row_44_45_tombstone_rebuild_and_reuse` |
| 26 | `b_hmap_string::row_52_sh_none_string_mode` |
| 27, 28 | `c_errors::err_rows_27_28_duplicate_temp_key_asymmetry` |
| 29, 30, 31, 40 | `b_hmap_binary::row_40_42_del_basics`, `c_errors::err_rows_30_31_del_rejections` |
| 36, 37 | `b_hmap_string::row_58_59_string_deletes`, `row_62_mode_two_strdup_delete_last` |
| 38, 42 | `b_hmap_binary::row_43_shrink_rebuild` |
| 43 | `b_hmap_binary::row_30_76_global_seed_variation` |
| 45, 46, 47, 48 | `b_arena::row_65_stralloc_fresh_short` .. `row_70_stralloc_empty_strings` |
| 49 | `b_arena::row_71_72_strreset` |
| 50 | `b_hmap_string::err_row_50_shmode_out_of_range`, `c_errors::err_row_50_shmode_full_sweep` |
| 51, 52 | `b_hmap_string::row_61_mode_two_default`, `row_63_negative_mode`, `c_errors::err_rows_51_52_out_of_range_mode_enum` |
| 53 | `b_hmap_binary::row_29_keysize_zero` |
| 54, 55 | `b_hash::row_11_12_hash_string_ascii`, `row_13_hash_string_high_bit` |
| 56, 57, 58 | `b_hash::row_9_hash_bytes_sign_extension`, `row_1_6_hash_bytes_all_lengths`, `row_7_hash_bytes_all_zero`, `row_8_hash_bytes_all_ff` |
| 59 | `b_arena::row_74_strkey` |
| 60 | `b_arena::row_75_arr_del` |
