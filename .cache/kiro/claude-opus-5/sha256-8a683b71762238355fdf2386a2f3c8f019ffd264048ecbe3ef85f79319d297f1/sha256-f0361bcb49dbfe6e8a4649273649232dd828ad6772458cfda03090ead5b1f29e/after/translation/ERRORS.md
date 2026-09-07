# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every `STBDS_ASSERT`
(`#define STBDS_ASSERT assert`, and `NDEBUG` is **not** defined by
`c_src/CMakeLists.txt`, so all asserts are live), every early `return`
sentinel, every null check, every range check and every min/max constant.

This library has **no error-code enum and no `RETURN_ERROR` macro**. Its whole
rejection surface consists of (a) sentinel returns / sentinel out-params,
(b) live `assert()` aborts, (c) "silently do nothing / return input unchanged"
paths, and (d) constants that bound behaviour.

Legend for "expected C result":
* `SENTINEL` — a specific value is returned or written to an out-param.
* `NOP` — input is returned unchanged / function returns without effect.
* `ABORT` — live `assert()` fires → `SIGABRT` (test must run out-of-process).
* `UB/CRASH` — C dereferences/frees an invalid pointer; both libraries must
  behave the same way, verified out-of-process by exit status.

| #  | function | trigger (exact invalid input / condition) | expected C result | verified by | [x] |
|----|----------|-------------------------------------------|-------------------|-----------|-----|
| 1  | `stbds_arrgrowf` (L286) | `min_cap <= stbds_arrcap(a)` and `min_len <= min_cap` (nothing to grow) | `NOP` — returns `a` bit-identical, no realloc, header untouched | `err01_arrgrowf_nothing_to_grow_returns_input` | [x] |
| 2  | `stbds_arrgrowf` (L300) | `a == NULL` | fresh block; `length=0`, `hash_table=NULL`, `temp=0`, `capacity=min_cap` | `err02_arrgrowf_null_initialises_header` | [x] |
| 3  | `stbds_arrgrowf` (L280-283) | `min_cap` in `(arrcap, 2*arrcap)` | `min_cap` forced to `2*arrcap` | `err03_04_arrgrowf_capacity_policy` | [x] |
| 4  | `stbds_arrgrowf` (L282-283) | `2*arrcap <= min_cap < 4` (i.e. `a==NULL`/cap 0, `min_cap<4`) | `min_cap` forced to `4` | `err03_04_arrgrowf_capacity_policy` | [x] |
| 5  | `stbds_arrgrowf` | `elemsize == 0` (degenerate) | `realloc(NULL, 0*min_cap+32)` succeeds, capacity set, no crash | `err05_arrgrowf_zero_elemsize` | [x] |
| 6  | `stbds_arrgrowf` | `addlen` huge so `arrlen+addlen` overflows `size_t` | wrap-around `min_len`; must wrap identically (no panic in Rust) | `err06_arrgrowf_size_overflow_same_outcome` (out of process) | [x] |
| 7  | `stbds_arrfreef` (L295) | `a == NULL` — C computes `free((header*)NULL - 1)` = `free((void*)-32)` | `UB/CRASH` — observed `SIGSEGV` (glibc dereferences the bogus chunk header); both libraries must die identically | `err07_arrfreef_null_same_abort` (out of process) | [x] |
| 8  | `stbds_hmfree_func` (L573) | `a == NULL` | `NOP` — returns immediately, no free | `err08_hmfree_null_is_nop` | [x] |
| 9  | `stbds_hmfree_func` (L574) | `stbds_hash_table(a) == NULL` (array made by `arrgrowf`, never `hmput`) | skips strdup-free + `strreset`, still frees `hash_table` (NULL) and header | `err09_hmfree_tableless_array` | [x] |
| 10 | `stbds_hm_find_slot` (L610) | probe reaches a slot with `hash == STBDS_HASH_EMPTY` (0) in the upper half-scan → key absent | `SENTINEL` `-1` | `err10_11_find_slot_miss_both_scan_halves` | [x] |
| 11 | `stbds_hm_find_slot` (L621) | same, in the wrapped lower half-scan (`i < pos & 7`) | `SENTINEL` `-1` | `err10_11_find_slot_miss_both_scan_halves` | [x] |
| 12 | `stbds_hmget_key_ts` (L634-639) | `a == NULL` | `*temp = STBDS_INDEX_EMPTY (-1)`; returns a NEW 1-element hash array (`length==1`, `hash_table==NULL`) | `err12_13_14_15_hmget_sentinels` | [x] |
| 13 | `stbds_hmget_key_ts` (L644-645) | `a != NULL` but `hash_table == 0` | `*temp = -1`; returns `a` unchanged (`NOP`) | `err12_13_14_15_hmget_sentinels` | [x] |
| 14 | `stbds_hmget_key_ts` (L648-649) | key not present (`slot < 0`) | `*temp = STBDS_INDEX_EMPTY (-1)` | `err12_13_14_15_hmget_sentinels` | [x] |
| 15 | `stbds_hmget_key` (L668-673) | any of rows 12–14 | same as 12–14 **plus** `stbds_header(p-elemsize)->temp == -1` | `err12_13_14_15_hmget_sentinels` | [x] |
| 16 | `stbds_hmget_key_ts` | `temp == NULL` out-pointer | `UB/CRASH` (unconditional `*temp` store) — must match | `err16_hmget_key_ts_null_temp_same_fault` (out of process) | [x] |
| 17 | `stbds_hmput_default` (L669) | `a == NULL` | allocates 1-element array, `length==1`, element zeroed | `err17_18_19_hmput_default_paths` | [x] |
| 18 | `stbds_hmput_default` (L669) | `a != NULL` but raw `length == 0` | re-grows via `arrgrowf`, `length` becomes 1, element zeroed | `err17_18_19_hmput_default_paths` | [x] |
| 19 | `stbds_hmput_default` (L669) | `a != NULL`, raw `length != 0` | `NOP` — returns `a` unchanged | `err17_18_19_hmput_default_paths` | [x] |
| 20 | `stbds_hmput_key` (L686) | `a == NULL` | allocates, memset, `length=1`, then normal insert | `err20_21_22_hmput_key_table_creation_and_growth` | [x] |
| 21 | `stbds_hmput_key` (L698) | `table == NULL` | new index of `STBDS_BUCKET_LENGTH` (8) slots; `string.mode = mode>=1 ? SH_DEFAULT : 0` | `err20_21_22_hmput_key_table_creation_and_growth` | [x] |
| 22 | `stbds_hmput_key` (L698) | `used_count >= used_count_threshold` (6 of 8) | rehash into `slot_count*2` | `err20_21_22_hmput_key_table_creation_and_growth` | [x] |
| 23 | `stbds_hmput_key` (L719) / `find_slot` (L596) | key whose hash is `0` or `1` (collides with `HASH_EMPTY`/`HASH_DELETED`) | `hash += 2` fixup — insert+lookup must still round-trip | `err23_hash_below_2_fixup` | [x] |
| 24 | `stbds_hmput_key` (L740/756, L766-769) | insert lands on a tombstone (`index == STBDS_INDEX_DELETED`) | reuses tombstone slot, `--tombstone_count`, `++used_count` | `err24_tombstone_reuse` | [x] |
| 25 | `stbds_hmput_key` (L778) | `assert((size_t)i+1 <= stbds_arrcap(a))` after grow | `ABORT` if it ever fails (never in-spec; both must not abort) | `err25_33_42_45_live_asserts_never_fire` | [x] |
| 26 | `stbds_hmput_key` (L785-790) | `table->string.mode` not in {2,3,1} — i.e. `SH_NONE(0)` or any out-of-range value forced via `shmode_func` | `default:` branch → raw `memcpy` of `keysize` bytes (no strdup/arena) | `err26_default_memcpy_branch` | [x] |
| 27 | `stbds_hmput_key` | `mode` out-of-range enum, e.g. `2`, `3`, `1000`, `INT_MAX` | `mode >= STBDS_HM_STRING` is true → treated as STRING (hash_string, strcmp) | `err27_28_mode_boundaries` | [x] |
| 28 | `stbds_hmput_key` | `mode` negative, e.g. `-1`, `INT_MIN` | `mode >= 1` false → treated as BINARY (hash_bytes, memcmp) | `err27_28_mode_boundaries` | [x] |
| 29 | `stbds_hmput_key` | `keysize == 0`, BINARY mode | `hash_bytes(key,0,seed)` + `memcmp(...,0)` always equal → every key aliases slot 0 | `err29_keysize_zero_aliases_everything` | [x] |
| 30 | `stbds_hmdel_key` (L809-810) | `a == NULL` | `SENTINEL` returns `NULL` (`0`) | `err30_hmdel_null_returns_null` | [x] |
| 31 | `stbds_hmdel_key` (L816-817) | `hash_table == 0` | sets `header->temp = 0`, returns `a` (`NOP` delete) | `err31_hmdel_tableless_sets_temp_zero` | [x] |
| 32 | `stbds_hmdel_key` (L821-822) | key absent (`slot < 0`) | `header->temp = 0`, returns `a`, length unchanged | `err32_hmdel_absent_key_is_nop` | [x] |
| 33 | `stbds_hmdel_key` (L828) | `assert(slot < (ptrdiff_t) table->slot_count)` | `ABORT` if violated (never in-spec) | `err25_33_42_45_live_asserts_never_fire` | [x] |
| 34 | `stbds_hmdel_key` (L832) | `assert(table->used_count >= 0)` — `size_t`, tautologically true | never fires (Rust may legally omit it) | n/a — tautological (`size_t >= 0`); Rust legally omits it | [x] |
| 35 | `stbds_hmdel_key` (L846) | `assert(slot >= 0)` — re-find of the moved-in last element | `ABORT` if the moved key is not findable | `err35_36_hmdel_wrong_string_mode_same_abort` (out of process) | [x] |
| 36 | `stbds_hmdel_key` (L849) | `assert(b->index[i] == final_index)` | `ABORT` if index bookkeeping diverges | `err35_36_hmdel_wrong_string_mode_same_abort` (out of process) | [x] |
| 37 | `stbds_hmdel_key` (L837) | `mode == STBDS_HM_STRING` **exactly** (not `2`) and `string.mode == SH_STRDUP` | frees the duped key; with `mode==2` the dup is **leaked** — behaviour must match | `err37_mode2_skips_strdup_free` | [x] |
| 38 | `stbds_hmdel_key` (L856-857) | `used_count < used_count_shrink_threshold && slot_count > 8` | table shrunk to `slot_count>>1` | `err38_39_shrink_and_rebuild_branches` | [x] |
| 39 | `stbds_hmdel_key` (L859-860) | `tombstone_count > tombstone_count_threshold` | table rebuilt at same `slot_count` | `err38_39_shrink_and_rebuild_branches` | [x] |
| 40 | `stbds_hmdel_key` | delete the LAST element (`old_index == final_index`) | skips memmove + re-find entirely | `err40_41_delete_tail_and_single_entry` | [x] |
| 41 | `stbds_hmdel_key` | delete on a map of exactly 1 entry (so `final_index == 0`) | `length` 2→1, `used_count` 1→0 | `err40_41_delete_tail_and_single_entry` | [x] |
| 42 | `stbds_make_hash_index` (L401) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)` | holds for every power-of-two `slot_count >= 8`; `ABORT` otherwise | `err25_33_42_45_live_asserts_never_fire` | [x] |
| 43 | `stbds_make_hash_index` (L406-408) | `slot_count <= STBDS_BUCKET_LENGTH` (8) | `used_count_shrink_threshold` forced to `0` (never shrinks the 8-slot table) | `err20_21_22_hmput_key_table_creation_and_growth` | [x] |
| 44 | `stbds_shmode_func` | `mode` out of the `{0,1,2,3}` enum, e.g. `4`, `-1`, `255`, `256`, `259`, `INT_MIN` | `string.mode = (unsigned char) mode` — truncation, then row 26's `default:` unless the truncated byte is 1/2/3 | `err44_shmode_out_of_range_truncation` | [x] |
| 45 | `stbds_stralloc` (L913) | `assert(len <= a->remaining)` | `ABORT` if the block was not sized to fit | `err25_33_42_45_live_asserts_never_fire` | [x] |
| 46 | `stbds_stralloc` (L884, L899) | `len > a->remaining` and `len > blocksize` (oversized string) | dedicated block; when `a->storage == NULL` also sets `remaining = 0` | `err46_47_48_50_arena_boundaries` | [x] |
| 47 | `stbds_stralloc` (L890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX` (`1<<20`) | `a->block` stops incrementing (saturates at 23) | `err46_47_48_50_arena_boundaries` | [x] |
| 48 | `stbds_stralloc` | empty string `""` (`len == 1`) into a fresh arena (`remaining == 0`) | allocates a 512-byte block, returns pointer to `'\0'` | `err46_47_48_50_arena_boundaries` | [x] |
| 49 | `stbds_stralloc` | `a == NULL` | `UB/CRASH` (unconditional `a->remaining` read) | `err49_stralloc_null_arena_same_fault` (out of process) | [x] |
| 50 | `stbds_strreset` (L925) | `a->storage == NULL` (fresh/already-reset arena) | `NOP` free loop, then zeroes the whole arena | `err46_47_48_50_arena_boundaries` | [x] |
| 51 | `stbds_strreset` | `a == NULL` | `UB/CRASH` (unconditional `a->storage` read) | `err51_strreset_null_same_fault` (out of process) | [x] |
| 52 | `stbds_hash_string` | empty string `""` | loop body never runs; avalanche applied to `seed` alone | `err52_53_hash_string_boundaries` | [x] |
| 53 | `stbds_hash_string` | bytes `>= 0x80` (`(unsigned char)` cast) | added as 128..255, **not** sign-extended | `err52_53_hash_string_boundaries` | [x] |
| 54 | `stbds_hash_bytes` | `len == 0` | tail `data = 0 << 56`, no byte loaded; returns a seed-only hash | `err54_55_56_57_58_hash_bytes_boundaries` | [x] |
| 55 | `stbds_hash_bytes` | `len` in `1..=7` (partial tail, all fall-through cases) | signed-`int` shift quirks at `d[3]<<24`, `d[2]<<16`, `d[1]<<8` sign-extend into `size_t` | `err54_55_56_57_58_hash_bytes_boundaries` | [x] |
| 56 | `stbds_hash_bytes` | tail byte `d[3] >= 0x80` | `(d[3] << 24)` overflows `int` → gcc yields negative → sign-extends to `0xFFFFFFFF_8.......` | `err54_55_56_57_58_hash_bytes_boundaries` | [x] |
| 57 | `stbds_hash_bytes` | body block byte `d[3] >= 0x80` (`len >= 8`) | same sign-extension inside the 8-byte loop | `err54_55_56_57_58_hash_bytes_boundaries` | [x] |
| 58 | `stbds_hash_bytes` | `p == NULL, len == 0` | no dereference — must return the same value as C, no Rust panic | `err54_55_56_57_58_hash_bytes_boundaries` | [x] |
| 59 | `stbds_rand_seed` | `seed == 0` / `SIZE_MAX` | accepted; next `make_hash_index` uses it and evolves `seed*a+b` with wrap-around | `err59_rand_seed_extremes` | [x] |
| 60 | `strkey` | `n == INT_MIN` / `INT_MAX` (widest `%d` output) | `"test_-2147483648"` / `"test_2147483647"` into the 256-byte static buffer | `err60_strkey_extremes` | [x] |
| 61 | `arr_ins` (L953) | `assert(arr[i] == num)` | must hold for every `num`, incl. `0`, `INT_MIN`, `INT_MAX` | `err61_62_63_arr_ins_asserts` | [x] |
| 62 | `arr_ins` (L955) | `assert(arr[4] == 4)` for `i < 4` | must hold for every `num` (fails if the memmove length is wrong) | `err61_62_63_arr_ins_asserts` | [x] |
| 63 | `arr_ins` | `num == 4` (aliases the sentinel checked by row 62) | both asserts still pass — no false negative | `err61_62_63_arr_ins_asserts` | [x] |

## Test locations

* In-process rows → `tests/phase_c_errors.rs` (29 tests).
* Process-terminating rows (6, 7, 16, 35, 36, 49, 51) → `tests/phase_c_crash.rs`.
  Each scenario is run out of process against ONE library at a time and the
  terminating **signal** and **exit code** are compared. Observed, matching:

  | scenario | ERRORS.md row | C | Rust |
  |----------|---------------|---|------|
  | `arrfreef_null`         | 7     | SIGSEGV (139) | SIGSEGV (139) |
  | `getkeyts_null_temp`    | 16    | SIGSEGV (139) | SIGSEGV (139) |
  | `stralloc_null_arena`   | 49    | SIGSEGV (139) | SIGSEGV (139) |
  | `strreset_null`         | 51    | SIGSEGV (139) | SIGSEGV (139) |
  | `overflow_wrap_es4`     | 6     | exit 0, identical header | exit 0, identical header |
  | `overflow_wrap_es16`    | 6     | SIGABRT (134, glibc heap check) | SIGABRT (134) |
  | `overflow_realloc_fail` | 6     | SIGSEGV (139) | SIGSEGV (139) |
  | `hmdel_mode2_middle`    | 35/36 | SIGABRT (134, assert) | SIGABRT (134) |
  | `hmdel_mode1000_middle` | 35/36 | SIGABRT (134, assert) | SIGABRT (134) |

## Harness validation (mutation testing)

To prove the suite is not vacuous, four deliberate bugs were injected into
`src/lib.rs` one at a time and each was caught:

| injected bug | caught by |
|--------------|-----------|
| refresh `temp_key` in `hmput_key`'s wrapped probe half (C does not) | `row70_long_random_streams_all_string_modes` — "temp_key refresh differs (C wrote=false RUST wrote=true)" |
| `tombstone_count_threshold = (n>>3) + (n>>5)` instead of `(n>>4)` | all 13 `phase_b_map_binary` tests + 8 `phase_c_errors` tests |
| `hash += 3` instead of `hash += 2` in the `hash < 2` fixup | `phase_b_map_*`, `phase_c_errors` |
| dropped the `(d[3] << 24)` int sign-extension in the siphash tail | `row02`, `row04`, `row05`, `row06` of `phase_b_hash` |

`src/lib.rs` was restored bit-identically afterwards (verified with `diff`).
