# ERRORS.md — Phase C error / rejection surface table

Derived **mechanically** from `c_src/src/lib.c`: every `STBDS_ASSERT`, every
early `return`, every null / zero / range test, and every sentinel value
(`STBDS_INDEX_EMPTY = -1`, `STBDS_INDEX_DELETED = -2`, `STBDS_HASH_EMPTY = 0`,
`STBDS_HASH_DELETED = 1`) that the C code uses to reject or short-circuit an
input.

Grep basis:

```
grep -n 'STBDS_ASSERT' c_src/src/lib.c          ->  lines 401 778 828 832 846 849 913 960 961 962
grep -n 'return 0;\|return -1\|return a;'       ->  lines 287 610 621 655 675 810 817 822 864
grep -n '== NULL\|!= NULL\|== 0)'               ->  lines 300 573 574 634 644 669 686 698 702 809 816
```

Legend for "expected C result": what an external caller can observe through
the FFI boundary (return value, `*temp` out-param, the `temp` field of the
array header, or process abort).

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `stbds_arrgrowf` | `a == NULL`, `addlen == 0`, `min_cap == 0` → `min_len(0) > min_cap(0)` false, `min_cap(0) <= arrcap(NULL)==0` true (line 286) | returns `NULL` unchanged; **no allocation** | `err_01_arrgrowf_null_zero_returns_null` |
| 2 | `stbds_arrgrowf` | `a != NULL` and requested `min_cap`/`addlen` already fit: `min_cap <= arrcap(a)` (line 286) | returns the *same* pointer `a`, header untouched (length/capacity/temp/hash_table unchanged) | `err_02_arrgrowf_noop_when_cap_sufficient` |
| 3 | `stbds_arrgrowf` | `a == NULL`, `min_cap` and `addlen` both `< 4` but non-zero → `min_cap < 4` clamp (line 291) | fresh block with `capacity == 4`, `length == 0`, `hash_table == NULL`, `temp == 0` | `err_03_arrgrowf_min_cap_clamped_to_4` |
| 4 | `stbds_arrgrowf` | `elemsize == 0` (degenerate/zero element size) | `realloc(NULL, 0*min_cap + 32)` succeeds; capacity set, no crash | `err_04_arrgrowf_zero_elemsize` |
| 5 | `stbds_arrgrowf` | `addlen == SIZE_MAX` (oversized length) → `min_len` wraps, `elemsize*min_cap+32` wraps → `realloc` of a small/absurd size | both sides must compute the **same** wrapped `min_cap`; observable via returned header `capacity` | `err_05_arrgrowf_oversized_addlen_wraps` |
| 6 | `stbds_hmfree_func` | `a == NULL` (line 573) | returns immediately, no-op, no crash | `err_06_hmfree_null_is_noop` |
| 7 | `stbds_hmfree_func` | `a != NULL` but `stbds_header(a)->hash_table == NULL` (line 574 false) | skips the STRDUP sweep and `strreset`; still `free(NULL)` + `free(header)`; no crash | `err_07_hmfree_no_hash_table` |
| 8 | `stbds_hm_find_slot` (via `stbds_hmget_key_ts`) | probe hits a bucket slot with `hash == STBDS_HASH_EMPTY (0)` before finding the key — first inner loop (line 609/610) | slot `-1` → `*temp = STBDS_INDEX_EMPTY (-1)` | `err_08_09_get_missing_key_returns_minus1` |
| 9 | `stbds_hm_find_slot` (via `stbds_hmget_key_ts`) | same, but the empty slot is found in the wrap-around loop `i < limit` (line 620/621) | slot `-1` → `*temp = -1` | `err_08_09_get_missing_key_returns_minus1` (2000 randomized misses per table size, so both inner loops are hit) |
| 10 | `stbds_hmget_key_ts` | `a == NULL` (line 634) | allocates a 1-element array (`length == 1`, zeroed elem), sets `*temp = -1`, returns `a + elemsize` | `err_10_hmget_key_ts_null_a` |
| 11 | `stbds_hmget_key_ts` | `a != NULL` but `hash_table == 0` (line 644) — e.g. an array built by `stbds_hmput_default` / `stbds_hmget_key_ts(NULL,..)` and never `put` | `*temp = -1`, returns `a` unchanged | `err_11_hmget_key_ts_no_table` |
| 12 | `stbds_hmget_key` | `a == NULL` | header `temp` field of the new array = `-1` | `err_12_hmget_key_null_a` |
| 13 | `stbds_hmget_key` | key absent from a populated table | header `temp` field = `-1` | `err_13_hmget_key_missing` |
| 14 | `stbds_hmput_default` | `a == NULL` (line 669) | grows to `capacity == 4`, `length == 1`, element zeroed, returns `a + elemsize` | `err_14_15_16_hmput_default_paths` |
| 15 | `stbds_hmput_default` | `a != NULL` and `stbds_header(a-elemsize)->length == 0` (line 669, 2nd disjunct) | grows again, `length` becomes 1 | `err_14_15_16_hmput_default_paths` |
| 16 | `stbds_hmput_default` | `a != NULL` and `length != 0` | returns `a` **unchanged** (no allocation, no zeroing) | `err_14_15_16_hmput_default_paths` |
| 17 | `stbds_hmput_key` | `a == NULL` (line 686) | bootstraps a 1-element array first, then inserts | `err_17_hmput_key_null_a` |
| 18 | `stbds_hmput_key` | `table == NULL` (line 698) and `mode < STBDS_HM_STRING` | new table with `string.mode = 0` (`STBDS_SH_NONE`) → default `memcpy` key path | `err_18_19_hmput_key_initial_string_mode` |
| 19 | `stbds_hmput_key` | `table == NULL` (line 698) and `mode >= STBDS_HM_STRING` | new table with `string.mode = STBDS_SH_DEFAULT (1)` → key pointer stored verbatim | `err_18_19_hmput_key_initial_string_mode` |
| 20 | `stbds_hmput_key` | duplicate key hit in the **first** inner loop (line 730) | returns without inserting; `length` unchanged; header `temp` = existing index; `hash_table->temp_key` updated **only** when `mode >= STBDS_HM_STRING` | `err_20_21_hmput_duplicate_key` |
| 21 | `stbds_hmput_key` | duplicate key hit in the **wrap-around** loop (line 748) | returns without inserting; header `temp` = existing index; `temp_key` **NOT** updated (asymmetry with row 20 — replicate, do not fix) | `err_20_21_hmput_duplicate_key` + `cfg43_string_duplicates_temp_key` (which snapshots `temp_key` after every duplicate put) |
| 22 | `stbds_hmput_key` | insert into a slot chain containing a tombstone (`index == STBDS_INDEX_DELETED (-2)`, line 740/756) | reuses the tombstone: `--tombstone_count`, `++used_count` | `err_22_hmput_reuses_tombstone` |
| 23 | `stbds_hmput_key` | `used_count >= used_count_threshold` (line 698) | table doubles: `slot_count *= 2`, all live entries rehashed, old table freed | `err_23_hmput_grows_table` |
| 24 | `stbds_hmput_key` (assert, line 778) | `(size_t)i+1 <= stbds_arrcap(a)` — cannot be violated after the preceding `arrgrowf`; unreachable via the public API | assert never fires | `err_24_hmput_capacity_assert_unreachable` |
| 25 | `stbds_shmode_func` | `mode` out of the `{0,1,2,3}` enum range, e.g. `4`, `7`, `255`, `256`, `-1`, `INT_MIN`, `INT_MAX` (line 803 `(unsigned char) mode`) | `string.mode = (unsigned char)mode` (truncated, **no** validation); `256 -> 0`, `-1 -> 255`, `INT_MAX -> 255` | `err_25_shmode_out_of_range_enum` |
| 26 | `stbds_shmode_func` | `elemsize == 0` | `arrgrowf(0,0,0,1)` → cap 4, `memset(a,0,0)`, `length = 1`; returns `a + 0` (== `a`) | `err_26_shmode_zero_elemsize` |
| 27 | `stbds_hmdel_key` | `a == NULL` (line 809/810) | returns `0` (`NULL`) | `err_27_hmdel_null_a_returns_null` |
| 28 | `stbds_hmdel_key` | `a != NULL` but `hash_table == 0` (line 816/817) | sets header `temp = 0`, returns `a` unchanged | `err_28_hmdel_no_table` |
| 29 | `stbds_hmdel_key` | key not present → `stbds_hm_find_slot` returns `< 0` (line 821/822) | header `temp = 0` (the "0 deleted" sentinel), returns `a`, `length` unchanged | `err_29_hmdel_missing_key` |
| 30 | `stbds_hmdel_key` | key present (line 831) | header `temp = 1`, `--used_count`, `++tombstone_count`, slot hash = `STBDS_HASH_DELETED (1)`, slot index = `STBDS_INDEX_DELETED (-2)`, `--length` | `err_30_31_hmdel_present_key` |
| 31 | `stbds_hmdel_key` | deleting the **last** element so `old_index == final_index` (line 839 false) | no `memmove`, no re-`find_slot` | `err_30_31_hmdel_present_key` (the `reverse = true` pass deletes in reverse insertion order, so `old_index == final_index` every time) |
| 32 | `stbds_hmdel_key` (assert, line 828) | `slot < (ptrdiff_t) table->slot_count` — `find_slot` masks with `slot_count-1`, unreachable | assert never fires | `err_32_34_35_hmdel_asserts_unreachable` |
| 33 | `stbds_hmdel_key` (assert, line 832) | `table->used_count >= 0` — `used_count` is `size_t`, so the comparison is vacuously true even after wrap | assert never fires, **even when `used_count` wraps** to `SIZE_MAX` (delete on an empty-but-tabled map) | `err_33_hmdel_used_count_assert_vacuous` |
| 34 | `stbds_hmdel_key` (assert, line 846) | `slot >= 0` for the moved element's re-lookup. **REACHABLE**: (a) `keyoffset != 0`, where the key compared at `elem+keyoffset` can match by coincidence but the re-lookup then fails; (b) `mode >= 2`, where line 845 hashes the *address* of the moved element instead of its key string | **`__assert_fail` -> SIGABRT** (the C build has no `-DNDEBUG`; `nm -D` shows `U __assert_fail`). The Rust carries the same assert and must abort with the same signal | `err_39_hmdel_nonzero_keyoffset`, `err_34_hmdel_mode_ge_2_mid_delete_aborts` (both fork a child per implementation and compare the termination signal) |
| 35 | `stbds_hmdel_key` (assert, line 849) | `b->index[i] == final_index` for the moved element | assert never fires for well-formed maps | `err_32_34_35_hmdel_asserts_unreachable` (6 x 400 randomized insert/delete ops) |
| 36 | `stbds_hmdel_key` | `used_count < used_count_shrink_threshold && slot_count > 8` (line 854) | table halves; old table freed | `err_36_hmdel_shrinks_table` |
| 37 | `stbds_hmdel_key` | `tombstone_count > tombstone_count_threshold` (line 858) | table rebuilt at the same `slot_count` | `err_37_hmdel_rebuilds_table` |
| 38 | `stbds_hmdel_key` | `mode == STBDS_HM_STRING` **and** `string.mode == STBDS_SH_STRDUP` (line 836) — note `==`, not `>=`: mode 2 does **not** free | strdup'd key freed only for `mode == 1` | `err_38_hmdel_strdup_free_only_mode_eq_1` |
| 39 | `stbds_hmdel_key` | `keyoffset != 0` (non-zero key offset, used by the `hmdel`/`shdel` macros via `STBDS_OFFSETOF`) | key compared/looked up at `elem + keyoffset` | `err_39_hmdel_nonzero_keyoffset` |
| 40 | `stbds_make_hash_index` (assert, line 401) | `used_count_threshold + tombstone_count_threshold < slot_count`; for every reachable `slot_count` (8,16,32,…) this is `sc-sc/4 + sc/8+sc/16 < sc` → true. `slot_count == 0` would fire but is unreachable (`shmode_func` uses 8, `hmput_key` uses 8 or `2*sc`, `hmdel_key` uses `sc>>1` only when `sc > 8`) | assert never fires | `err_40_make_hash_index_assert_unreachable` |
| 41 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` **and** `a->storage == NULL` (line 893, 898) | dedicated oversized block, `sb->next = 0`, `a->storage = sb`, `a->remaining = 0`; returns `sb->storage` | `err_41_stralloc_oversized_first` |
| 42 | `stbds_stralloc` | `len > a->remaining` **and** `len > blocksize` **and** `a->storage != NULL` (line 896) | oversized block spliced **after** the head (`sb->next = head->next; head->next = sb`); `a->remaining` left untouched | `err_42_stralloc_oversized_splice` |
| 43 | `stbds_stralloc` | `a->block` large enough that `512 << (block>>1) >= 1<<20` (line 890) | `a->block` is **not** incremented (saturates) | `err_43_stralloc_block_saturates` |
| 44 | `stbds_stralloc` | `a->block >= 128` → `block>>1 >= 64` → `512u << 64+` is UB in C; x86-64 masks the shift count to 6 bits | both must produce the identical (masked) `blocksize`, hence the identical branch | `err_44_stralloc_shift_overflow_ub` |
| 45 | `stbds_stralloc` (assert, line 913) | `len <= a->remaining` after the growth block. Reachable only with a corrupted arena (`storage == NULL` but `remaining >= len`), which dereferences `NULL` before the assert matters — not exercised | assert never fires on well-formed arenas | `err_45_stralloc_assert_holds` |
| 46 | `stbds_stralloc` | empty string `""` → `len == 1` | 1 byte consumed, `remaining` decremented by 1 | `err_46_stralloc_empty_string` |
| 47 | `stbds_strreset` | `a->storage == NULL` (line 924 loop not entered) | just `memset(a, 0, sizeof)`; no frees | `err_47_strreset_empty_arena` |
| 48 | `stbds_hash_bytes` | `len == 0` (and even `p == NULL`, since no byte is read) | hashes only `len << 56`; deterministic value | `err_48_hash_bytes_zero_len` |
| 49 | `stbds_hash_bytes` | `len % 8 == 7..1` tail with bytes `>= 0x80` → C promotes `d[3] << 24` to a **negative int** which is then sign-extended into `size_t` (lines 523-524, 536) | both must sign-extend identically | `err_49_hash_bytes_sign_extension` + `cfg09_hash_bytes_patterns` |
| 50 | `stbds_hash_string` | empty string `""` | `while(*str)` never runs; hash derived from `seed` alone | `err_50_hash_string_empty` |
| 51 | `stbds_hash_string` | bytes `>= 0x80` in the string — C casts to `(unsigned char)` before adding | no sign extension of the character | `err_51_hash_string_high_bit` |
| 52 | `stbds_hm_find_slot` | hash value `< 2` → `hash += 2` (lines 596, 719) so the `HASH_EMPTY(0)` / `HASH_DELETED(1)` sentinels are never used as real hashes | keys whose raw hash is 0 or 1 must still be findable | `err_52_hash_lt_2_bumped` |
| 53 | `stbds_hmget_key` / `stbds_hmput_key` / `stbds_hmdel_key` | `mode` out-of-range enum value across FFI: `2`, `3`, `255`, `-1`, `INT_MIN`, `INT_MAX`. `mode >= STBDS_HM_STRING(1)` selects the *string* path for `2,3,255,INT_MAX`; negatives (`-1`, `INT_MIN`) select the *binary* path | identical branch selection on both sides | `err_53_out_of_range_mode_enum` + `cfg50a`/`cfg50b`/`cfg50c` |
| 54 | `stbds_hmput_key` | `keysize == 0` in binary mode → `memcmp(...,0) == 0` always true → **every** key with a colliding hash compares equal; `memcpy(...,0)` copies nothing | identical degenerate behaviour | `err_54_zero_keysize_binary` |
| 55 | `stbds_hmput_key` | `keysize` larger than `elemsize` (oversized key) → `memcpy` overruns the element | must overrun identically (undefined but deterministic); tested with generous padding | `err_55_oversized_keysize` |
| 56 | `str_dups` | `num <= 0` (`0`, `-1`, `INT_MIN`) → the `for (i=0; i<num; ++i)` arena loop body never executes | no arena allocations; still runs the strdup-map block and prints `a <num>` | `err_56_57_str_dups_non_positive` |
| 57 | `str_dups` (asserts, lines 960-962) | `*strmap[0].key == 'a'`, `strmap[0].key != s.key`, `strmap[0].value == s.value` | all hold for `SH_STRDUP`; no abort for any `num` | `err_56_57_str_dups_non_positive` + `cfg53_str_dups_stdout` |
| 58 | `strkey` | `n` at `INT_MIN` / `INT_MAX` → `sprintf(buffer, "test_%d", n)` into a 256-byte static | `"test_-2147483648"` / `"test_2147483647"`, no overflow | `err_58_strkey_extremes` |
| 59 | `stbds_arrfreef` | `a == NULL` → `free((char*)NULL - 32)` | glibc rejects it (`free(): invalid pointer`) and **aborts**; both implementations must die with the same signal | `err_59_arrfreef_null_aborts_identically` (forked child per implementation) |
| 60 | `stbds_hash_string` | `str == NULL` → dereferences `NULL` | **SIGSEGV (11)** in both; this row caught a REAL divergence — see the notes below | `err_60_hash_string_null_aborts_identically` (forked child per implementation) |

## Notes

* The C library has **no** error-code return convention. Its only rejection
  signals are: the sentinel `-1` (`STBDS_INDEX_EMPTY`) in `*temp` / the header
  `temp` field, `0` vs `1` in the header `temp` field for `hmdel_key`,
  returning the input pointer unchanged, returning `NULL`, and `assert`
  aborts. Every row above is checked against one of those observables.
* `assert` is **live** in the C build (`c_src/CMakeLists.txt` sets no
  `NDEBUG`; `nm -D` shows `U __assert_fail`). Rows 24, 32, 33, 35, 40, 45 and 57
  prove no reachable input makes the C side abort there. Row 34 is the exception:
  it **is** reachable, so the translation now carries every one of the C's ten
  `STBDS_ASSERT`s verbatim (`src/lib.rs`, `STBDS_ASSERT!` macro -> write to fd 2
  + `abort()`), and rows 34/39/59/60 compare the termination signal of a forked
  child per implementation.

## Divergences this table found (and how they were fixed)

1. **Row 34 / 39 - missing `STBDS_ASSERT`s.** The Rust omitted all ten of the
   C's asserts. With `keyoffset != 0`, or with `mode >= 2` on a string map, the
   C's live `STBDS_ASSERT(slot >= 0)` (c_src/src/lib.c:846) really does fire and
   `abort()`s; the Rust instead computed `storage.offset(-1)` and wrote
   `b->index[7] = old_index` into the `stbds_hash_index` header - silent memory
   corruption where the C had a hard stop. Fixed by transliterating all ten
   asserts, including the ones that are vacuous in C (`used_count >= 0` on a
   `size_t`, which is documented in place rather than emitted).
2. **Row 60 - Rust's debug-only UB checks.** `stbds_hash_string(NULL, seed)`
   segfaults in C (SIGSEGV/11) but the Rust `.so` built with
   `debug-assertions = on` aborted (SIGABRT/6) from rustc's injected
   null-pointer-dereference check. The same class of check also turned the C's
   legal-on-x86 *unaligned* `char *` store (any `elemsize` that is not a
   multiple of 8, c_src/src/lib.c:786-788) into an abort. Since neither check is
   C semantics, `Cargo.toml` now sets `debug-assertions = false` and
   `overflow-checks = false` for the `dev`/`test` profiles, so every profile
   behaves like the release artifact - and the differential suite is run against
   BOTH the debug and the release `.so` (see `run_all_configs.sh`).
