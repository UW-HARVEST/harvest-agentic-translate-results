# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Every early-return, `STBDS_ASSERT`,
sentinel return, null check and min/max constant in the file is one row.

This library is stb-style: it has **no error enum and no error codes**. Its
entire rejection surface consists of

1. null-pointer early returns,
2. sentinel returns (`-1` = `STBDS_INDEX_EMPTY`, `0`/`NULL`),
3. live `assert()`s (`STBDS_ASSERT` = `assert`; `NDEBUG` is never defined by
   `c_src/CMakeLists.txt`, which sets no `CMAKE_BUILD_TYPE`, so the asserts are
   compiled in and abort the process),
4. arithmetic saturation / clamping constants.

An `assert` firing is observable as `SIGABRT` + a stderr message; the Rust side
must abort too (via `__assert_fail`). Rows whose trigger is *unreachable through
the public ABI* or whose trigger is memory-unsafe in C (dereferencing a wild
pointer) are marked as such and are covered by inspection rather than by a live
differential test — noted per row.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` (lib.c:290) | `min_cap <= stbds_arrcap(a)`, i.e. nothing to do (incl. `a=NULL, addlen=0, min_cap=0`) | returns `a` unchanged (may be `NULL`); **no allocation** | `err_01_arrgrowf_noop` |
| 2 | `stbds_arrgrowf` (lib.c:295) | `min_cap < 4` after clamping (e.g. `addlen=1`, `min_cap=0`, `a=NULL`) | `min_cap` forced to 4; header `length=0`, `capacity=4`, `hash_table=NULL`, `temp=0` | `err_02_arrgrowf_min_cap_4` |
| 3 | `stbds_arrgrowf` (lib.c:293) | `min_cap < 2*arrcap(a)` on a non-empty array | `min_cap` forced to `2*arrcap(a)` (doubling, not the requested value) | `err_03_arrgrowf_doubling` |
| 4 | `stbds_arrgrowf` (lib.c:284,300) | `a == NULL` (the "no array yet" rejection path) | fresh block; `length/hash_table/temp` zero-initialised (they are **not** initialised when `a != NULL`) | `err_04_arrgrowf_null_init` |
| 5 | `stbds_arrgrowf` | `elemsize * min_cap + sizeof(header)` overflows `size_t` (e.g. `elemsize = SIZE_MAX/2`) | wrap-around size passed to `realloc`; `realloc` fails, returns `NULL`, and the code **still writes** through `NULL+32` → SIGSEGV | inspection only (memory-unsafe in C; Rust reproduces the same wrapping arithmetic — verified by reading the code, not executed) |
| 6 | `stbds_make_hash_index` (lib.c:401) | `used_count_threshold + tombstone_count_threshold >= slot_count`; happens for `slot_count < 8` — e.g. `slot_count = 1` gives `1 + 0 >= 1` | `assert` fires → `SIGABRT` | unreachable via public ABI: every call site passes `STBDS_BUCKET_LENGTH` (8) or `slot_count*2` / `slot_count>>1` of an already-`>=8` value. Covered by inspection + row 22. |
| 7 | `stbds_hmfree_func` (lib.c:573) | `a == NULL` | returns immediately, frees nothing | `err_07_hmfree_null` |
| 8 | `stbds_hmfree_func` (lib.c:574) | `stbds_hash_table(a) == NULL` (array built by `arrgrowf`, never given a hash index) | skips the strdup-free loop and `strreset`; still frees `hash_table` (`NULL`, no-op) and the header | `err_08_hmfree_no_table` |
| 9 | `stbds_hm_find_slot` (lib.c:610) | probe reaches a slot with `bucket->hash[i] == STBDS_HASH_EMPTY` (0) in the *upper* scan → key absent | returns `-1` | `err_09_10_find_slot_miss` |
| 10 | `stbds_hm_find_slot` (lib.c:621) | same, in the *wrapped* (`i < limit`) scan → key absent | returns `-1` | `err_09_10_find_slot_miss` (randomised keys force both scans) |
| 11 | `stbds_hmget_key_ts` (lib.c:634) | `a == NULL` | allocates a 1-element zeroed array, `*temp = -1`, returns `arr+elemsize` (non-NULL) | `err_11_hmget_ts_null` |
| 12 | `stbds_hmget_key_ts` (lib.c:642) | `a != NULL` but `hash_table == 0` (e.g. array from `hmput_default`) | `*temp = -1`, returns `a` unchanged | `err_12_hmget_ts_no_table` |
| 13 | `stbds_hmget_key_ts` (lib.c:645) | key not found (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY` (-1) | `err_13_hmget_ts_missing_key` |
| 14 | `stbds_hmget_key` (lib.c:660) | any of rows 11–13 | same as 11–13 **and** writes `temp` into `header(p-elemsize)->temp` | `err_14_hmget_key_temp` |
| 15 | `stbds_hmput_default` (lib.c:669) | `a == NULL` | allocates 1-element zeroed array, returns `arr+elemsize` | `err_15_hmput_default_null` |
| 16 | `stbds_hmput_default` (lib.c:669) | `a != NULL` but `header(a-elemsize)->length == 0` | grows/zero-fills element 0, `length += 1` | `err_16_hmput_default_len0` |
| 17 | `stbds_hmput_key` (lib.c:686) | `a == NULL` | bootstraps a 1-element zeroed array first, then proceeds (never returns an error) | `err_17_hmput_null_bootstrap` |
| 18 | `stbds_hmput_key` (lib.c:698) | `table == NULL` **or** `used_count >= used_count_threshold` (load factor 3/4 reached) | rehash into `slot_count*2` (or fresh 8) — an implicit "capacity exceeded" branch | `err_18_hmput_grow_threshold` |
| 19 | `stbds_hmput_key` (lib.c:778) | `(size_t)i+1 > arrcap(a)` still true after `arrgrowf` (i.e. `arrgrowf` failed to grow) | `assert` fires → `SIGABRT` | unreachable while `realloc` succeeds; inspection only |
| 20 | `stbds_hmput_key` (lib.c:790 `default:`) | `table->string.mode` is not one of `STRDUP`/`ARENA`/`DEFAULT` — includes **out-of-range enum values** reaching `shmode_func` and being truncated to `unsigned char` | falls into `memcpy(..., keysize)` binary-key path regardless of `mode` | `err_20_out_of_range_shmode` |
| 21 | `stbds_hmdel_key` (lib.c:809) | `a == NULL` | returns `0` (`NULL`) — the one true sentinel-error return in the library | `err_21_hmdel_null` |
| 22 | `stbds_hmdel_key` (lib.c:813) | `hash_table == 0` | sets `temp = 0` then returns `a` unchanged (no deletion) | `err_22_hmdel_no_table` |
| 23 | `stbds_hmdel_key` (lib.c:819) | `stbds_hm_find_slot` returns `< 0` → key absent | `temp` left at `0`, returns `a`, length unchanged | `err_23_hmdel_missing_key` |
| 24 | `stbds_hmdel_key` (lib.c:828) | `slot >= (ptrdiff_t)table->slot_count` | `assert` → `SIGABRT` | unreachable: `find_slot` masks `pos` with `slot_count-1`; inspection only |
| 25 | `stbds_hmdel_key` (lib.c:832) | `table->used_count >= 0` on a `size_t` | tautology; never fires (GCC may warn) | inspection only |
| 26 | `stbds_hmdel_key` (lib.c:846) | re-lookup of the moved final element returns `slot < 0` | `assert` → `SIGABRT` | unreachable when the table is consistent; inspection only |
| 27 | `stbds_hmdel_key` (lib.c:849) | `b->index[i] != final_index` for the moved element | `assert` → `SIGABRT` | unreachable when the table is consistent; inspection only |
| 28 | `stbds_hmdel_key` (lib.c:857) | `used_count < used_count_shrink_threshold && slot_count > 8` | shrink rehash to `slot_count>>1` | `err_28_hmdel_shrink` |
| 29 | `stbds_hmdel_key` (lib.c:860) | `tombstone_count > tombstone_count_threshold` | same-size rebuild rehash | `err_29_hmdel_tombstone_rebuild` |
| 30 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **and** `string.mode == STRDUP`: the duped key is freed | freed; a later read of it is use-after-free | `err_30_hmdel_strdup_free` (behavioural: length/temp/order compared, plus forked runs so a bad free shows as a differing signal) |
| 30b | `stbds_hmput_key` (lib.c:754 vs lib.c:745) | duplicate key found in the **wrapped** probe scan: unlike the upper scan, that branch does NOT refresh `stbds_temp_key`, so `stbds_shputs` stores a **stale** pointer and `stbds_hmfree` later double-frees it | glibc `double free or corruption (fasttop)` → `SIGABRT` | `err_30b_shputs_stale_temp_key_double_free` (both libraries abort identically) |
| 31 | `stbds_stralloc` (lib.c:913) | `STBDS_ASSERT(len <= a->remaining)`. **Unreachable through a well-formed arena** — the oversized branch returns early and the new-block branch sets `remaining = blocksize >= len`. Verified by a 600-step randomised positive control. The *reachable* failure is an FFI caller passing `remaining > 0` with `storage == NULL`: the whole `if` is skipped and `a->storage->storage` is dereferenced | `SIGSEGV` (not the assert) | `err_31_stralloc_inconsistent_arena` |
| 32 | `stbds_stralloc` (lib.c:894) | `len > blocksize` (string longer than the current geometric block) | dedicated oversized block; `a->block` still incremented; `remaining` set to 0 when `storage == NULL`, **left untouched** when `storage != NULL` | `err_32_stralloc_oversized` |
| 33 | `stbds_stralloc` (lib.c:891) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (1<<20) — reached at `a->block == 22`, since `blocksize = 1 << (9 + block/2)` | `a->block` stops incrementing (saturates at 22) | `err_33_stralloc_block_saturate` |
| 34 | `stbds_stralloc` (lib.c:889) | `a->block >= 128` (only reachable by an FFI caller handing over a crafted arena, since natural growth saturates at 22) makes `512 << (block>>1)` shift by ≥ 64 — UB in C; gcc/x86 emits `shl`, i.e. count mod 64. For `block ∈ 100..127` the resulting blocksize is astronomically large, `realloc` returns `NULL`, and `sb->next = …` faults | `SIGSEGV` | `err_34_stralloc_shift_overflow` (exit-code-encoded result compared for the non-faulting `block` values) |
| 35 | `stbds_stralloc` | empty string `""` (`len == 1`) | allocates a 512-byte block, `remaining` 511, returns pointer to `""` | `err_35_stralloc_empty_string` |
| 36 | `stbds_strreset` (lib.c:934) | `a->storage == NULL` (already-reset or fresh arena) | loop body never runs, arena memset to 0 | `err_36_strreset_empty` |
| 37 | `stbds_hash_string` (lib.c:472) | empty string `""` | loop never runs; still mixes/returns `f(seed)`, never 0-length-special-cased | `err_37_hash_string_empty` |
| 38 | `stbds_hash_bytes` | `len == 0` | skips both the 8-byte loop and every `switch` case; `data = 0 << 56` | `err_38_hash_bytes_zero_len` |
| 39 | `stbds_hash_bytes` | `len` in 1..=7 (partial tail; each `case` is a distinct fall-through path) | each length hits a different fall-through chain | `err_39_hash_bytes_tail_lengths` |
| 40 | `stbds_hash_bytes` (lib.c:534) | `d[3] >= 0x80` in a full 8-byte block: `d[3]<<24` is a **negative `int`** that sign-extends into `size_t` | upper 32 bits of `data` all become 1 | `err_40_hash_bytes_sign_extension` |
| 41 | `stbds_hash_bytes` (lib.c:546 `case 4:`) | `len - i == 4..7` with `d[3] >= 0x80` | same sign extension in the tail | `err_41_hash_tail_sign_extension` |
| 42 | `stbds_hm_find_slot` / `hmput_key` | computed `hash < 2` (would collide with `HASH_EMPTY`=0 / `HASH_DELETED`=1) | `hash += 2` — the reserved-value rejection | `err_42_reserved_hash_values` (randomised search over seeds/keys) |
| 43 | `stbds_is_key_equal` (lib.c:559) | `mode >= STBDS_HM_STRING` — **any** int ≥ 1, including out-of-range enum values like 2, 99, `INT_MAX` | treated as a string compare (`strcmp`), never rejected | `err_43_mode_out_of_range_string` |
| 44 | `stbds_is_key_equal` / `hmput_key` | `mode < 0` (e.g. `-1`, `INT_MIN`) crossing the FFI boundary | `mode >= STBDS_HM_STRING` is false → binary/`memcmp` path, and `string.mode` set to `0` | `err_44_mode_negative` |
| 45 | `stbds_hmput_key` / `hmget_key` | `keysize == 0` with binary mode | `memcmp(...,0)` returns 0 → **every** key compares equal; only the hash distinguishes them | `err_45_keysize_zero` |
| 46 | `stbds_shmode_func` (lib.c:801) | `mode` out of the 0..3 enum range (e.g. 4, 255, 256, -1, `INT_MAX`) | `(unsigned char) mode` truncation; `string.mode` becomes `mode & 0xff` | `err_46_shmode_truncation` |
| 47 | `stbds_arrfreef` | `a == NULL` | `free(NULL - 32)` — invalid free, UB/crash. No null check exists | inspection only (memory-unsafe in C; Rust does the identical unchecked `stbds_header(a)` + `free`) |
| 48 | `strkey` (lib.c:947) | `n` such that `"test_%d"` exceeds the 256-byte `static char buffer[256]` | impossible for `int` (max 16 bytes); `INT_MIN` → `"test_-2147483648"` | `err_48_strkey_extremes` |
| 49 | `str_dups` (lib.c:955) | `num <= 0` | the `stralloc` loop body never runs; the strmap block still runs and still prints | `err_49_str_dups_nonpositive` |
| 50 | `str_dups` (lib.c:960-962) | the three post-`shputs` asserts | never fire for the fixed key `"a"`; both must behave identically | `err_50_str_dups_asserts_hold` |

## Checklist

- [x] 1  `err_01_arrgrowf_noop`
- [x] 2  `err_02_arrgrowf_min_cap_4`
- [x] 3  `err_03_arrgrowf_doubling`
- [x] 4  `err_04_arrgrowf_null_init`
- [x] 5  inspection (memory-unsafe in C)
- [x] 6  inspection (unreachable via public ABI)
- [x] 7  `err_07_hmfree_null`
- [x] 8  `err_08_hmfree_no_table`
- [x] 9  `err_09_10_find_slot_miss`
- [x] 10 `err_09_10_find_slot_miss`
- [x] 11 `err_11_hmget_ts_null`
- [x] 12 `err_12_hmget_ts_no_table`
- [x] 13 `err_13_hmget_ts_missing_key`
- [x] 14 `err_14_hmget_key_temp`
- [x] 15 `err_15_hmput_default_null`
- [x] 16 `err_16_hmput_default_len0`
- [x] 17 `err_17_hmput_null_bootstrap`
- [x] 18 `err_18_hmput_grow_threshold`
- [x] 19 inspection (unreachable)
- [x] 20 `err_20_out_of_range_shmode`
- [x] 21 `err_21_hmdel_null`
- [x] 22 `err_22_hmdel_no_table`
- [x] 23 `err_23_hmdel_missing_key`
- [x] 24 inspection (unreachable)
- [x] 25 inspection (tautology)
- [x] 26 inspection (unreachable)
- [x] 27 inspection (unreachable)
- [x] 28 `err_28_hmdel_shrink`
- [x] 29 `err_29_hmdel_tombstone_rebuild`
- [x] 30 `err_30_hmdel_strdup_free`
- [x] 30b `err_30b_shputs_stale_temp_key_double_free`
- [x] 31 `err_31_stralloc_inconsistent_arena`
- [x] 32 `err_32_stralloc_oversized`
- [x] 33 `err_33_stralloc_block_saturate`
- [x] 34 `err_34_stralloc_shift_overflow`
- [x] 35 `err_35_stralloc_empty_string`
- [x] 36 `err_36_strreset_empty`
- [x] 37 `err_37_hash_string_empty`
- [x] 38 `err_38_hash_bytes_zero_len`
- [x] 39 `err_39_hash_bytes_tail_lengths`
- [x] 40 `err_40_hash_bytes_sign_extension`
- [x] 41 `err_41_hash_tail_sign_extension`
- [x] 42 `err_42_reserved_hash_values`
- [x] 43 `err_43_mode_out_of_range_string`
- [x] 44 `err_44_mode_negative`
- [x] 45 `err_45_keysize_zero`
- [x] 46 `err_46_shmode_truncation`
- [x] 47 inspection (memory-unsafe in C)
- [x] 48 `err_48_strkey_extremes`
- [x] 49 `err_49_str_dups_nonpositive`
- [x] 50 `err_50_str_dups_asserts_hold`

## Divergences found and fixed

Every row above now passes against both `.so`s. Two genuine C↔Rust divergences
were found while building the error-path tests; both were in the same class and
both were fixed on the Rust side.

| # | row | C result | Rust result (before fix) | fix |
|---|-----|----------|--------------------------|-----|
| 1 | 31 (`stralloc` with `remaining > 0`, `storage == NULL`) | `SIGSEGV` | `SIGABRT` | `translation/Cargo.toml`: `debug-assertions = false` |
| 2 | 34 (`stralloc` with `a->block ≥ 100`, `realloc` returning `NULL`) | `SIGSEGV` | `SIGABRT` | same |

Root cause: rustc inserts null-pointer-dereference debug assertions when
`debug-assertions` is on. Where the C code simply faults on a bad address, the
`dev`-profile Rust build panicked and (with `panic = "abort"`) raised `SIGABRT`
instead of `SIGSEGV`. The `release` profile already matched the C, because
`debug-assertions` defaults to off there. Turning the checks off in **both**
profiles — for the same reason the crate already sets `overflow-checks = false`
— makes the dev and release builds behave identically to the C library.

Behaviours that look like bugs but are faithful to the C and are therefore
asserted as-is rather than "fixed":

* `stbds_hmput_key` not refreshing `stbds_temp_key` in its wrapped probe scan,
  which makes `stbds_shputs` double-free on duplicate keys (row 30b).
* `stbds_hmdel_key` tripping `STBDS_ASSERT(slot >= 0)` at lib.c:846 whenever
  `mode >= 2` and the deleted element is not the last one (covered by
  `phase_b_delete_arena::row_40b_delete_ptr_to_string_mode_asserts`).
* `stbds_siphash_bytes` sign-extending `d[3] << 24` into the upper half of
  `data` (rows 40, 41).
* `stbds_hash_bytes` ignoring its `seed` entirely, because each `v0..v3`
  initialiser XORs it in twice (CONFIGS row 11).
* `printf("%s %d\n", strmap[z], strmap[z].value)` in `str_dups` passing a
  16-byte struct by value to a variadic function; under the SysV AMD64 ABI
  `%s` consumes `.key` and `%d` consumes `.value` (CONFIGS rows 52, 53).
