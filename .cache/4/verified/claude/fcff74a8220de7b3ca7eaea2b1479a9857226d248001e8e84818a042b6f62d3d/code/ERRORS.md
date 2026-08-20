# ERRORS.md — Phase A/C error & rejection surface

Derived mechanically from `c_src/src/lib.c` by grepping every `return`,
`STBDS_ASSERT`, `== NULL` / `!= NULL` / `== 0` / `< 0` / `>= …` guard and every
min/max constant.  `stb_ds` has no error *enum*: it rejects input by returning a
sentinel (`-1` = `STBDS_INDEX_EMPTY`, `NULL`/`0`), by taking an early-out
branch, or by tripping an `assert` (asserts are **live** in the shipped C `.so`
— it imports `__assert_fail`, i.e. `NDEBUG` is not defined).

Legend
* `temp` = `stbds_header(raw_array)->temp`, the value the `stbds_hm*` macros read back.
* `hdr`  = the `stbds_array_header` in front of the raw array.
* Test   = the differential test that pins the row; file `tests/errors_diff.rs`
  unless noted.  Run everything with `./scripts/verify.sh`.

Status: **44/44 executable rows checked**, all passing (7 `assert` rows and 3
rows are provably not executable — see the notes; they are marked as such
rather than faked).

## Sentinel / early-return rows

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| E1 | `stbds_arrgrowf` | `min_cap <= stbds_arrcap(a)` after the `min_len` fixup (L286) | returns `a` **unchanged** (and `NULL` when `a == NULL, min_cap == 0, addlen == 0`), no realloc | `err_arrgrowf_no_grow_returns_same_ptr` | [x] |
| E2 | `stbds_arrgrowf` | `a == NULL` (L300) | fresh alloc with `hdr.length=0`, `hdr.hash_table=0`, `hdr.temp=0` | `err_arrgrowf_null_input` | [x] |
| E3 | `stbds_arrgrowf` | `min_cap < 4` after the `2*cap` fixup (L291) | capacity bumped to exactly `4` | `err_arrgrowf_min_cap_below_4` | [x] |
| E4 | `stbds_arrgrowf` | `elemsize*min_cap + 32` wraps `size_t` (`elemsize = SIZE_MAX/2`) | unsigned wraparound → 28-byte `realloc`, `capacity = 4` | `err_arrgrowf_size_overflow_wraps` | [x] |
| E5 | `stbds_arrgrowf` | `stbds_arrlen(a) + addlen` wraps (L280, `addlen = SIZE_MAX`) | `min_cap = SIZE_MAX`, size wraps to 31 | `err_arrgrowf_addlen_overflow` | [x] |
| E6 | `stbds_hmfree_func` | `a == NULL` (L573) | returns immediately, no free, no crash (any `elemsize`, incl. 0 / `SIZE_MAX`) | `err_hmfree_null` | [x] |
| E7 | `stbds_hmfree_func` | `a != NULL` but `hdr.hash_table == NULL` (L574) | skips the strdup loop + `strreset`; `free(NULL)` + `free(hdr)` | `err_hmfree_no_table` | [x] |
| E8 | `stbds_hm_find_slot` | key absent, empty slot met in the *upper* half of the bucket (L609) | returns `-1` → `temp == -1` | `err_get_missing_key_returns_minus1` | [x] |
| E9 | `stbds_hm_find_slot` | key absent, empty slot met in the *wrapped* half (`i < limit`, L620) | returns `-1` (bucket crafted identically in both libs to force the branch) | `err_get_missing_key_wrapped_half` | [x] |
| E10 | `stbds_hmget_key_ts` | `a == NULL` (L634) | 1-elem zeroed array, `*temp = -1`, returns `arr+elemsize`, `length=1`, `capacity=4` | `err_hmget_ts_null_map` | [x] |
| E11 | `stbds_hmget_key_ts` | `a != NULL`, `hdr.hash_table == 0` (L644) | `*temp = -1`, returns `a` unchanged | `err_hmget_ts_no_table` | [x] |
| E12 | `stbds_hmget_key_ts` | `stbds_hm_find_slot() < 0` (L648) | `*temp = STBDS_INDEX_EMPTY` | `err_get_missing_key_returns_minus1` | [x] |
| E13 | `stbds_hmget_key` | any of E10..E12 | same value, additionally stored into `hdr.temp` (which `_ts` must NOT touch) | `err_hmget_key_writes_temp` | [x] |
| E14 | `stbds_hmput_default` | `a == NULL` (L669) | allocates, `hdr.length = 1`, elem0 zeroed, `capacity = 4` | `err_hmput_default_branches` | [x] |
| E15 | `stbds_hmput_default` | `a != NULL` **and** `hdr.length == 0` | re-grows, re-`memset`s elem0, `length = 1` | `err_hmput_default_branches` | [x] |
| E16 | `stbds_hmput_default` | `a != NULL`, `hdr.length != 0` | returns `a` unchanged, writes nothing | `err_hmput_default_branches` | [x] |
| E17 | `stbds_hmput_key` | `a == NULL` (L686) | 1 zeroed elem, `length=1`, then inserts (`temp = 0`, `length = 2`) | `err_hmput_key_null_map` | [x] |
| E18 | `stbds_hmput_key` | `hdr.hash_table == NULL` (L698/L707) | new 8-slot index, `string.mode = (mode >= 1) ? SH_DEFAULT : 0` | `err_hmput_key_mode_selects_string_mode` | [x] |
| E19 | `stbds_hmput_key` | `table->used_count >= table->used_count_threshold` (L698) | `slot_count*2`, rehash, old index freed (fires on the 7th insert, not the 6th) | `err_hmput_grow_at_threshold` | [x] |
| E20 | `stbds_hmput_key` | computed `hash < 2` (L719) | `hash += 2`, so a stored hash is never `0`/`1`. A siphash/string-hash of 0 or 1 cannot be constructed on purpose (2⁻⁶³), so the row is pinned by its **observable invariant** over randomized maps in both libs: `hash==0 ⇔ index==EMPTY`, `hash==1 ⇔ index==DELETED`, otherwise `index >= 0` | `err_hash_below_2_invariant` | [x] |
| E21 | `stbds_hmput_key` | `mode` out of enum range but `>= 1` (`2, 3, 4, 44, 1000, INT_MAX`) | full STRING behaviour: `stbds_hash_string` + `strcmp` + `temp_key` | `err_mode_out_of_range_string_side` | [x] |
| E22 | `stbds_hmput_key` | `mode` negative (`-1, -2, -1000, INT_MIN`) | BINARY behaviour: `stbds_hash_bytes` + `memcmp`, `string.mode = 0`, key `memcpy`ed | `err_mode_negative_is_binary` | [x] |
| E23 | `stbds_hmput_key` | `table->string.mode ∉ {SH_STRDUP, SH_ARENA, SH_DEFAULT}` (switch `default:`, L789) — `SH_NONE(0)`, `4`, `44`, `255`, `256→0`, `300→44`, `-1→255` | `memcpy(elem, key, keysize)`: the raw key **bytes**, not a pointer | `err_string_mode_default_branch`, `cfg_sh_none_copies_bytes` | [x] |
| E24 | `stbds_hmput_key` | duplicate key found in the *wrapped* half of a bucket (L747-751) | `temp = index` but **`temp_key` NOT refreshed** (asymmetric to L732-733); both halves crafted deterministically and compared | `err_dup_key_wrapped_half_no_temp_key` | [x] |
| E25 | `stbds_hmput_key` | a tombstone was seen before the empty slot (`tombstone >= 0`, L766) | insert lands in the tombstone slot, `--tombstone_count` | `err_insert_reuses_tombstone` | [x] |
| E26 | `stbds_hmdel_key` | `a == NULL` (L809) | returns `0` (**NULL**) for every elemsize/keysize/mode | `err_hmdel_null_map` | [x] |
| E27 | `stbds_hmdel_key` | `hdr.hash_table == 0` (L816) | `temp = 0`, returns `a` unchanged | `err_hmdel_no_table` | [x] |
| E28 | `stbds_hmdel_key` | key not found (`slot < 0`, L821) | `temp = 0`, returns `a`, map otherwise untouched | `err_hmdel_missing_found_and_last` | [x] |
| E29 | `stbds_hmdel_key` | key found | `temp = 1`, `hash[i]=1`, `index[i]=-2`, `--used_count`, `++tombstone_count`, `--hdr.length` | `err_hmdel_missing_found_and_last` | [x] |
| E30 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **exactly** && `string.mode == SH_STRDUP` (L836) | frees the key; with `mode = 2` (also "string" for hashing) the key is **not** freed — verified by reading the vacated slot's still-intact string | `err_hmdel_strdup_free_only_mode_1` | [x] |
| E31 | `stbds_hmdel_key` | `old_index == final_index` (L839) | no `memmove`, no slot re-point, survivors keep their indices | `err_hmdel_missing_found_and_last`, `cfg_del_last` | [x] |
| E32 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` (L854) | index halves (16→8 on the 4th of 7 deletes: `4 < 4` is false, `3 < 4` is true) | `err_hmdel_shrink` | [x] |
| E33 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (L858) — `1` for an 8-slot index | rebuild at the same size on the **2nd** tombstone, tombstones dropped | `err_hmdel_rebuild_on_tombstones` | [x] |
| E34 | `stbds_shmode_func` | `mode` out of enum range (`4, 5, 44, 127, 128, 255, 256, 257, 300, -1, -256, INT_MIN, INT_MAX`) | `string.mode = (unsigned char) mode` — silent truncation (`300 → 44`, `256 → 0`) | `err_shmode_out_of_range` | [x] |
| E35 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` (L893) | dedicated block; spliced *after* the head when one exists | `err_stralloc_oversized_string` | [x] |
| E36 | `stbds_stralloc` | same, but `a->storage == NULL` (L899) | `sb->next = 0; a->storage = sb; a->remaining = 0` | `err_stralloc_oversized_empty_arena` | [x] |
| E37 | `stbds_stralloc` | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)` (L890) | `a->block` stops incrementing (checked for block 0,1,2,19..25) | `err_stralloc_block_saturates` | [x] |
| E38 | `stbds_stralloc` | `a->block >= 110` → `512 << (block>>1)` overflows to 0; `block >= 128` → shift count > 63 | x86-64 `shl %cl` masks the count to 6 bits (verified in the C disassembly): `512 << ((block>>1) & 63)`; blocksize 0 ⇒ dedicated-block path. Rust uses `wrapping_shl` to match. Checked for block 110,111,126..131,250,254,255 | `err_stralloc_huge_block_masked_shift` | [x] |
| E39 | `stbds_hash_bytes` | `len == 0` (with `p == NULL` **and** `p != NULL`) | no byte read; `switch(0)` → `break`; value depends only on `len`/`seed` | `err_hash_bytes_null_zero_len` | [x] |
| E40 | `stbds_hash_bytes` | `len - i ∈ 1..7` (tail `switch`, L532) | fall-through accumulation; `d[3]<<24` / `d[7]<<24` **sign-extend** into `size_t` | `err_hash_bytes_tail_sign_extension`, `cfg_hash_bytes_high_bit_patterns` | [x] |
| E41 | `stbds_hash_string` | empty string `""` | loop skipped, finaliser applied to `seed` (64 random seeds + extremes) | `err_hash_string_empty` | [x] |
| E43 | `strkey` | `n = 0, ±1, ±9/10, 99/100, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1`, 256 random | `sprintf(buffer,"test_%d",n)`; worst case 17 bytes fits the 256-byte static buffer; the returned address is stable | `cfg_strkey_matrix` (`tests/demo_diff.rs`) | [x] |
| E44 | `helxo` | `letter = 0, -1, -128, 127, 200-as-char, '\n', '%', '\t'` + 64 random | `%c` of a `char` promoted to `int`: `0` emits a NUL byte, `-128` emits `0x80`; stdout compared byte for byte | `cfg_helxo_letters` (`tests/demo_diff.rs`) | [x] |

## Rows that are provably NOT executable (documented, not faked)

| # | condition | why it cannot be tested |
|---|-----------|-------------------------|
| E42 | `stbds_is_key_equal` with `mode >= 1` on a *binary*-populated element | `strcmp` dereferences the element's first 8 bytes as a `char *` → wild pointer. Undefined in C and address dependent, so "same behaviour" is not observable. Reachable only by mixing modes on one map; excluded from the suite. |
| E38b | `stbds_stralloc` with `a->block` such that `512 << ((block>>1)&63)` is huge but non-zero (e.g. 109 → 2⁶³, 200 → 2⁴⁵) | the C `malloc` fails and the *next* statement dereferences the NULL result → SIGSEGV in **both** libraries; the test process cannot survive it. |
| G10 | `elemsize == 0` into `hmput_key` | `arr_to_hash(a,0) == a`, so every element aliases the header; the resulting corruption is allocator/address dependent. |
| — | `stbds_arrfreef(NULL)` | computes `free((char*)NULL - 32)` → invalid free / abort in **both** libraries. Exercised nowhere; `tests/array_diff.rs` explicitly guards against it (with a comment). |

## `assert` rows (live in the shipped C `.so`, `abort()` on failure)

`STBDS_ASSERT` is `assert` and the C `.so` is built without `NDEBUG` (it imports
`__assert_fail`).  The Rust translation compiles `stbds_assert!` to a no-op,
which is only observable if an assert can actually fail — the table below shows
that none can be reached through the public API, so no divergence exists.  None
is executed: a fired C assert would kill the test process and prove nothing
about the Rust side.

| # | function | assert | reachable from the public API? |
|---|----------|--------|--------------------------------|
| A1 | `stbds_make_hash_index` L401 | `used_count_threshold + tombstone_count_threshold < slot_count` | No. `slot_count` is only ever `8`, `slot_count*2` or `slot_count>>1` (floored at 8) ⇒ `3n/4 + 3n/16 < n` holds for every `n >= 8`. Static function, not exported. |
| A2 | `stbds_hmput_key` L778 | `(size_t) i+1 <= stbds_arrcap(a)` | No: the `if` on L774 grows the array first. |
| A3 | `stbds_hmdel_key` L828 | `slot < (ptrdiff_t) table->slot_count` | No: `stbds_hm_find_slot` masks `pos` with `slot_count-1`. |
| A4 | `stbds_hmdel_key` L832 | `table->used_count >= 0` | Tautology (`size_t`). |
| A5 | `stbds_hmdel_key` L846 | `slot >= 0` after the re-find of the moved last element | Only with a mode-mismatched re-find (`mode >= 2` + string keys makes L842 take the *binary* branch and hash the key **pointer bytes**) — address dependent, same class as E42. The suite therefore deletes only the *last* element for `mode != 1` (see `err_mode_matrix_binary`). |
| A6 | `stbds_hmdel_key` L849 | `b->index[i] == final_index` | Same as A5. |
| A7 | `stbds_stralloc` L913 | `len <= a->remaining` | No: the `if` on L885 guarantees it (a corrupt arena with `remaining > 0, storage == NULL` NULL-derefs before reaching the assert, in both libraries). |

## Generic FFI-boundary rows (required by Phase C)

| # | condition | expected C result | test | ✔ |
|---|-----------|-------------------|------|---|
| G1 | NULL map pointer into every entry point that takes one (`arrgrowf`, `hmfree_func`, `hmget_key`, `hmget_key_ts`, `hmput_default`, `hmput_key`, `hmdel_key`) | see E1..E28 | `err_all_null_map_entry_points` | [x] |
| G2 | NULL data pointer + `len == 0` into `stbds_hash_bytes` | pure function of `len`/`seed` | `err_hash_bytes_null_zero_len` | [x] |
| G3 | `keysize == 0` (binary mode) | `memcmp(...,0) == 0` ⇒ *every* key compares equal; the map can only hold one entry; one delete empties it | `err_keysize_zero` | [x] |
| G4 | `keysize > elemsize` | the `memcpy` overruns into the next element (deterministic); the clobbered key becomes unfindable in both libraries | `err_keysize_gt_elemsize` | [x] |
| G5 | oversized `elemsize`/`addlen` (`SIZE_MAX`, `SIZE_MAX/2`) | unsigned wraparound (E4/E5) | `err_arrgrowf_size_overflow_wraps`, `err_arrgrowf_addlen_overflow` | [x] |
| G6 | out-of-range `mode` **enum** values across FFI (`INT_MIN, -1, 0, 1, 2, 3, 4, 44, 1000, INT_MAX`) through `hmput_key`/`hmget_key`/`hmget_key_ts`/`hmdel_key` | `>= 1` ⇒ string side, `<= 0` ⇒ binary side; `hmdel_key`'s strdup-free uses `== 1` exactly | `err_mode_matrix_binary`, `err_mode_out_of_range_string_side`, `err_mode_negative_is_binary`, `err_hmdel_strdup_free_only_mode_1` | [x] |
| G7 | out-of-range `shmode_func` enum (valid range is `0..3`) | `(unsigned char)` truncation, then switch `default:` | `err_shmode_out_of_range` | [x] |
| G8 | one step past the growth threshold (6 vs 7 inserts on an 8-slot index) | growth on the 7th (`>=`, evaluated *before* the increment) | `err_hmput_grow_at_threshold` | [x] |
| G9 | one step past the tombstone threshold (`== 1` for 8 slots) and the shrink threshold | rebuild on the 2nd delete; shrink when `used_count` drops *below* `slot_count>>2` | `err_hmdel_rebuild_on_tombstones`, `err_hmdel_shrink` | [x] |
