# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping for **every**
`return` that is not the normal success return, **every** `STBDS_ASSERT`,
**every** null check, **every** explicit range / boundary comparison, and
**every** min/max constant.

This library has **no error enum and no error-return macro** (no
`RETURN_ERROR`, no `errno`). It rejects/short-circuits in exactly three ways:

* returning a **sentinel** (`NULL` / `-1` / an unchanged pointer),
* writing a **sentinel into an out-parameter** (`*temp = -1`),
* **aborting via `assert()`** — and because `c_src/CMakeLists.txt` sets no
  `CMAKE_BUILD_TYPE`, `-DNDEBUG` is *not* passed, so `__assert_fail` is a live
  import of the C `.so` (confirmed by `nm -D --undefined-only`). Assert rows
  are therefore "C aborts the process".

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `stbds_arrgrowf` (l.286) | `min_cap <= stbds_arrcap(a)` after `min_cap = max(min_cap, arrlen(a)+addlen)` — i.e. nothing to do (incl. `a=NULL, elemsize=0/any, addlen=0, min_cap=0`) | returns the **input pointer `a` unchanged** (may be `NULL`); no allocation, header untouched |
| E2 | `stbds_arrgrowf` (l.280) | `a != NULL` and `addlen` so large that `arrlen(a)+addlen` wraps `size_t` | C computes with wrapping `size_t`; no check, no rejection — result must match bit-for-bit (typically a tiny/huge `min_cap`, possibly a failed `realloc`) |
| E3 | `stbds_arrfreef` (l.314) | `a == NULL` → `free((stbds_array_header*)NULL - 1)` = `free((void*)-32)` | **no check at all**; undefined behaviour / glibc abort. Documented, deliberately NOT exercised (would abort both libs identically but destroys the test process) |
| E4 | `stbds_hmfree_func` (l.573) | `a == NULL` | returns immediately, no free, no crash |
| E5 | `stbds_hmfree_func` (l.574) | `a != NULL` but `stbds_header(a)->hash_table == NULL` | skips the strdup-scan and `stbds_strreset`, still `free(hash_table)` (= `free(NULL)`, a no-op) then `free(header)` |
| E6 | `stbds_hmfree_func` (l.575) | `hash_table->string.mode != STBDS_SH_STRDUP` (0,1,3,…) | does **not** free the per-element key pointers (only `strreset` + the two frees) |
| E7 | `stbds_hm_find_slot` (l.610, l.621) | probed bucket slot has `hash == STBDS_HASH_EMPTY (0)` ⇒ key is absent | returns `-1` |
| E8 | `stbds_hmget_key_ts` (l.634-639) | `a == NULL` | allocates a 1-element zeroed array, sets `length=1`, writes `*temp = STBDS_INDEX_EMPTY (-1)`, returns `STBDS_ARR_TO_HASH(new,elemsize)` (non-NULL) |
| E9 | `stbds_hmget_key_ts` (l.644-645) | `a != NULL` but `hash_table == NULL` (e.g. array made only by `stbds_hmput_default`) | `*temp = -1`, returns `a` unchanged; **`key` is never dereferenced** (so a NULL `key` is accepted here) |
| E10 | `stbds_hmget_key_ts` (l.648-649) | key not present (`stbds_hm_find_slot` < 0) | `*temp = STBDS_INDEX_EMPTY (-1)`, returns `a` unchanged |
| E11 | `stbds_hmget_key` (l.663) | any of E8/E9/E10 | additionally stores that same `temp` into `stbds_header(STBDS_HASH_TO_ARR(p,elemsize))->temp`, so `stbds_temp` reads back `-1` |
| E12 | `stbds_hmdel_key` (l.809-810) | `a == NULL` | returns `0` (**NULL**) — the only NULL-returning path in the library |
| E13 | `stbds_hmdel_key` (l.814-817) | `a != NULL` but `hash_table == NULL` | sets `stbds_temp(raw_a) = 0` then returns `a` unchanged (deletion reported as "0 removed") |
| E14 | `stbds_hmdel_key` (l.821-822) | key not present (`find_slot` < 0) | `stbds_temp(raw_a)` stays `0`, returns `a` unchanged; `used_count`/`tombstone_count`/`length` untouched |
| E15 | `stbds_hmdel_key` (l.836) | `mode == 2` (`STBDS_HM_PTR_TO_STRING`, an out-of-`STBDS_HM_*`-range-for-this-branch value) | `mode >= STBDS_HM_STRING` so `find_slot` **hashes/compares as a string**, but `mode == STBDS_HM_STRING` is false so the strdup key is **not** freed and the re-find at l.845 uses the **raw-bytes** branch. This asymmetry must be reproduced exactly |
| E16 | `stbds_hmdel_key` (l.828) | `assert(slot < (ptrdiff_t) table->slot_count)` | C aborts if violated (unreachable via the public API — `find_slot` masks `pos` with `slot_count-1`) |
| E17 | `stbds_hmdel_key` (l.832) | `assert(table->used_count >= 0)` on a `size_t` | tautology; never fires. Row kept because it is a literal `STBDS_ASSERT` in the source |
| E18 | `stbds_hmdel_key` (l.846) | `assert(slot >= 0)` — the moved-in final element cannot be found | C aborts. Reachable if the caller corrupted the table |
| E19 | `stbds_hmdel_key` (l.849) | `assert(b->index[i] == final_index)` | C aborts on a corrupt table |
| E20 | `stbds_hmput_key` (l.778) | `assert((size_t) i+1 <= stbds_arrcap(a))` after the conditional grow | C aborts if `realloc` failed |
| E21 | `stbds_make_hash_index` (l.401) | `assert(used_count_threshold + tombstone_count_threshold < slot_count)`. With `slot_count = 8`: `6 + (1+0) = 7 < 8` ✓. With `slot_count < 8` (e.g. 4 → `3+0=3<4` ✓; 1 → `1+0 = 1 < 1` ✗) | C **aborts** for `slot_count == 1` and `slot_count == 2` (`2+0=2<2` ✗). Only reachable through `stbds_shmode_func`/`stbds_hmput_key`, which always pass ≥ 8, so unreachable via the public API |
| E22 | `stbds_make_hash_index` (l.388) | `slot_count == 0` ⇒ allocation of `0*128 + 104 + 63` bytes and `slot_count-1 == SIZE_MAX` masks in `stbds_probe_position` | no check; unreachable via the public API |
| E23 | `stbds_make_hash_index` (l.403) | `ot == NULL` | `memset(&t->string,0,…)`, `t->seed = stbds_hash_seed`, **and the global `stbds_hash_seed` is advanced** by `seed*a+b`. `ot != NULL` copies `string`/`seed` and leaves the global alone. Any divergence here desynchronises every later hash |
| E24 | `stbds_stralloc` (l.913) | `assert(len <= a->remaining)` | C aborts. Reachable if the caller hands in an arena with an inconsistent `remaining`/`storage` pair |
| E25 | `stbds_stralloc` (l.885) | `len > a->remaining` on a **fresh** arena (`storage == NULL`, `remaining == 0`, `block == 0`) | allocates a `512`-byte block (`BLOCKSIZE_MIN << (0>>1)`), `++a->block` |
| E26 | `stbds_stralloc` (l.890) | `blocksize >= STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20)`, i.e. `a->block >= 22` | `a->block` is **not** incremented (saturates) |
| E27 | `stbds_stralloc` (l.893) | `len > blocksize` (a string longer than the whole next block) | takes the "oversize block" path: allocates exactly `len`+hdr, **splices** behind `a->storage` if one exists (`sb->next = storage->next; storage->next = sb;`, `remaining` untouched) or installs it as `a->storage` **with `remaining = 0`**, and returns `sb->storage` directly |
| E28 | `stbds_stralloc` (l.884) | `str` is the empty string `""` (`len == 1`) | still allocates on a fresh arena; returns a pointer to a 1-byte `""` |
| E29 | `stbds_strreset` (l.923-928) | `a->storage == NULL` (already-empty / fresh arena) | loop body never runs; `memset(a,0,24)` — idempotent, no crash |
| E30 | `stbds_hash_bytes` / `stbds_siphash_bytes` (l.522) | `len == 0` (and `p` never dereferenced past `d[0..len)`… except the tail `switch` reads nothing for `len-i == 0`) | returns the seed-only siphash finalisation; `p` may even be a dangling/NULL pointer for `len == 0` |
| E31 | `stbds_siphash_bytes` (l.532) | `len - i` is a value **outside `case 0..7`** — impossible because the main loop guarantees `len-i < 8`; the `switch` has **no `default:`** | the fall-through chain `7→6→…→1→0` must be reproduced (each `case` ORs its byte and falls through), and `len-i == 0` must OR nothing |
| E32 | `stbds_siphash_bytes` (l.523,536) | any input byte with bit 7 set at offset 3 (or 7) of a word | `d[3] << 24` is **`int`** arithmetic that sets the sign bit and is then **sign-extended** to `size_t`. Not an "error" but the classic value-dependent mis-translation; a row is kept so it is explicitly tested |
| E33 | `stbds_hash_string` (l.480) | `str` is the empty string | loop body never runs; `hash = seed` then the avalanche runs |
| E34 | `stbds_hash_string` (l.481) | bytes ≥ 0x80 | added as `(unsigned char)`, i.e. **zero**-extended (unlike E32) |
| E35 | `stbds_hm_find_slot` / `stbds_hmput_key` (l.596, l.719) | the computed hash is `0` or `1` (would collide with `STBDS_HASH_EMPTY` / `STBDS_HASH_DELETED`) | `hash += 2`. **Unreachable by feasible search** — it needs an input whose siphash-2-4 / `stbds_hash_string` output is exactly 0 or 1 (a 2/2^64 event, i.e. inverting the hash). 200 000 randomized probes confirm no such input was found; the guard is byte-identical in both implementations |
| E36 | `stbds_hmput_key` (l.698) | `table == NULL` **or** `used_count >= used_count_threshold` | rebuilds: `slot_count = 8` if fresh else `slot_count*2`. On the fresh path only, `nt->string.mode = (mode >= STBDS_HM_STRING) ? STBDS_SH_DEFAULT : 0` |
| E37 | `stbds_hmput_key` (l.789) | `table->string.mode` is **not** one of `STBDS_SH_STRDUP/ARENA/DEFAULT` (i.e. `0` or `4..255`) | falls into `default:` and `memcpy`s `keysize` raw bytes — so `stbds_shmode_func(elemsize, 0)` and `stbds_shmode_func(elemsize, 4)` behave as *binary* maps even for `mode >= STBDS_HM_STRING` puts |
| E38 | `stbds_shmode_func` (l.803) | `mode` outside `0..255` (e.g. `256`, `-1`, `1000`) | `(unsigned char) mode` **truncates**; `256 → 0`, `-1 → 255`, `1000 → 232`. Must truncate identically |
| E39 | `stbds_hmput_default` (l.669) | `a == NULL` **or** `stbds_header(STBDS_HASH_TO_ARR(a,elemsize))->length == 0` | grows/creates, `length += 1`, zeroes the new element, returns the hash-biased pointer. Otherwise returns `a` untouched |
| E40 | `stbds_hmdel_key` (l.854) | `used_count < used_count_shrink_threshold && slot_count > 8` | **shrinks** to `slot_count>>1` and frees the old table |
| E41 | `stbds_hmdel_key` (l.858) | else-if `tombstone_count > tombstone_count_threshold` | **rebuilds** at the same `slot_count`. The `else if` means a shrink suppresses the rebuild in the same call |
| E42 | `arr_push` (l.950) | `assert(arrlen(NULL) == 0)` | tautology, never fires |
| E43 | `arr_push` (l.951) | `num <= 0` | the `for (i=0; i<num; i+=50)` loop body never executes; the function is a no-op and never allocates |
| E44 | `arr_push` (l.951) | `num == INT_MAX` / very large | `i += 50` on a signed `int`; `i` stays < `num` so no overflow, but the run time is O(num²/50). Behaviour = no observable output; only used to confirm both return |
| E45 | `stbds_is_key_equal` (l.560) | `mode >= STBDS_HM_STRING` with a **negative** `mode` (e.g. `-1`) | `-1 >= 1` is false ⇒ takes the **memcmp** branch. Negative modes are valid `int`s across the FFI and must behave as binary mode |
| E46 | `stbds_is_key_equal` (l.561) | `keysize == 0` in binary mode | `memcmp(...,0) == 0` ⇒ **every** key compares equal |
| E47 | `stbds_log2` (l.375) | `slot_count == 0` | `while (0 > 1)` never runs ⇒ returns `0` |
| E48 | `stbds_probe_position` (l.371) | `slot_count == 0` | `hash & (size_t)-1` = `hash`, then indexing `storage[hash>>3]` is out of bounds. No check; unreachable via the public API |

## Row -> test mapping (all rows accounted for)

Executed differential tests live in `tests/errors.rs`; run them with
`cargo build --release && cargo test --release --test errors`.

| row | status | covering test |
|-----|--------|---------------|
| E1  | PASS | `e1_arrgrowf_early_return_is_the_input_pointer` |
| E2  | PASS | `e2_arrgrowf_addlen_and_mincap_wraparound` (wrapped `min_len`, and `elemsize*min_cap` wrapping to 0 so `realloc` gets only the header) |
| E3  | documented UB | `e3_arrfreef_null_is_documented_ub` — `free((void*)-32)` aborts the process; identical arithmetic in both, deliberately not executed |
| E4  | PASS | `e4_hmfree_func_null_is_noop` |
| E5  | PASS | `e5_hmfree_func_without_hash_table` |
| E6  | PASS | `e6_hmfree_func_only_frees_keys_for_sh_strdup` (caller key buffers verified byte-intact afterwards) |
| E7  | PASS | `e7_e10_e11_absent_key_reports_minus_one` |
| E8  | PASS | `e8_hmget_key_ts_on_null_array` (all 12 `mode` values x 6 `elemsize`) |
| E9  | PASS | `e9_hmget_on_array_without_hash_table_accepts_null_key` (NULL `key` accepted) |
| E10 | PASS | `e7_e10_e11_absent_key_reports_minus_one` |
| E11 | PASS | `e7_e10_e11_absent_key_reports_minus_one`, `e9_...` (header `temp` == -1) |
| E12 | PASS | `e12_hmdel_key_null_returns_null` (5 x 4 x 4 x 12 argument combinations) |
| E13 | PASS | `e13_hmdel_key_without_hash_table` (`temp` reset to 0 from a poisoned value) |
| E14 | PASS | `e14_hmdel_key_absent_key_is_noop` (length / used_count / tombstone_count unchanged) |
| E15 | PASS | `e15_out_of_range_mode_delete_asymmetry`, `generic_mode_sweep_over_every_entry_point` |
| E16 | unreachable | `e16_e22_e24_e48_unreachable_asserts_documented` — `stbds_hm_find_slot` masks `pos` with `slot_count-1` |
| E17 | tautology | same test — `size_t >= 0` is always true |
| E18 | unreachable*| same test — the only route through the exports is E15's `mode >= 2` + `old_index != final_index` shape, which **aborts the C process**; that shape is therefore excluded and documented in the E15 test |
| E19 | unreachable | same test — needs a table inconsistent with the element array |
| E20 | unreachable | same test — fires only after a failed `realloc`, by which point `stbds_arrgrowf` has already dereferenced `(void*)32` |
| E21 | unreachable | same test — empirically asserts the smallest table either library builds is `slot_count == 8` (`6 + 1 < 8`), and the shrink at l.854 is guarded by `slot_count > 8` |
| E22 | unreachable | same test — `slot_count == 0` cannot be requested through `stbds_shmode_func` / `stbds_hmput_key` |
| E23 | PASS | `e23_global_seed_advance_only_for_fresh_tables` (a map growing 8->1024 consumes the global seed exactly once; the next fresh table sees the identically-advanced value) |
| E24 | unreachable | `e16_e22_e24_e48_unreachable_asserts_documented` — the `else` branch always sets `remaining = blocksize >= len`, and `len > blocksize` returns early |
| E25 | PASS | `e25_e26_e27_e28_e29_stralloc_boundaries` |
| E26 | PASS | same test — `block` saturates at 22, blocksize at `1<<20` |
| E27 | PASS | same test — both the empty-arena (`remaining = 0`) and the splice (`remaining` preserved) shapes |
| E28 | PASS | same test — `""` on fresh and warm arenas |
| E29 | PASS | same test — idempotent on fresh / already-reset arenas |
| E30 | PASS | `e30_hash_bytes_zero_length_never_touches_the_pointer` (NULL and an unmapped pointer) |
| E31 | PASS | `e31_hash_bytes_tail_switch_fallthrough` (every `len % 8` x 5 base lengths x 64 random + 5 extremal buffers x 5 seeds) |
| E32 | PASS | `e32_hash_bytes_int_sign_extension` (every single-byte position for `len` 1..=24 x {0x80,0xFF,0x7F,0x01} x 5 seeds) |
| E33 | PASS | `e33_e34_hash_string_boundaries` |
| E34 | PASS | same test — bytes 0x80..0xFF zero-extended |
| E35 | unreachable | `e35_hash_below_two_is_unreachable_by_search` — 200 000 randomized probes, no hash < 2 found (see the row above) |
| E36 | PASS | `e36_table_rebuild_thresholds` (grow sequence asserted to be exactly 8,16,...,1024) |
| E37 | PASS | `e37_out_of_enum_string_mode_falls_into_memcpy` (`string.mode` in {0,4,5,100,255} x `mode` in {1,2,1000}) |
| E38 | PASS | `e38_shmode_func_mode_truncation_full_sweep` (610 `int` values, incl. 256/-1/1000/INT_MIN/INT_MAX) |
| E39 | PASS | `e39_hmput_default_guard` (incl. forcing `length` back to 0 to re-arm the guard) |
| E40 | PASS | `e40_e41_shrink_and_rebuild_branches` (shrink asserted to halve) |
| E41 | PASS | same test (rebuild asserted to keep `slot_count`; the `else if` precedence is observed) |
| E42 | tautology | `e42_e43_e44_arr_push_boundaries` — `arrlen(NULL) == 0` always holds |
| E43 | PASS | same test (`num` <= 0, incl. `INT_MIN`) |
| E44 | PASS | same test up to `num = 5000` (~250k pushes). `INT_MAX` is quadratic **and** overflows the signed `int` `i += 50` (UB in the C original), so it is not a meaningful comparison |
| E45 | PASS | `e45_negative_mode_takes_the_memcmp_branch`, `generic_mode_sweep_over_every_entry_point` |
| E46 | PASS | `e46_keysize_zero_makes_every_key_equal`, `c35_keysize_zero_all_keys_equal` |
| E47 | PASS | `e47_log2_zero_documented` (+ every `slot_count_log2` compared in E36/E40) |
| E48 | unreachable | `e16_e22_e24_e48_unreachable_asserts_documented` |

Plus the generic FFI-boundary tests demanded of any C API:

| area | test |
|------|------|
| out-of-range **enum** `mode` on every entry point (`INT_MIN`, -1000, -2, -1, 0..4, 1000, `INT_MAX`) | `generic_mode_sweep_over_every_entry_point` |
| out-of-range **enum** `string.mode` (0..255 truncation) | `e38_shmode_func_mode_truncation_full_sweep`, `e37_...` |
| NULL pointers (array, key, arena, hash buffer) | `e4`, `e8`, `e9`, `e12`, `e13`, `e30`, `e25_...` |
| zero lengths (`elemsize == 0`, `keysize == 0`, `len == 0`) | `generic_zero_elemsize_and_zero_keysize`, `e46`, `e30` |
| oversized / wrapping lengths | `e2_arrgrowf_addlen_and_mincap_wraparound`, `generic_oversized_keysize_and_keyoffset` |
| one-past-valid `keyoffset` | `generic_oversized_keysize_and_keyoffset` |

Rows marked *unreachable* / *tautology* are asserts on internal invariants that
cannot be provoked through the exported API without fabricating a corrupt
`stbds_hash_index`; each row states its reachability argument, and
`e16_e22_e24_e48_unreachable_asserts_documented` checks the numeric premises of
those arguments (`slot_count == 8`, thresholds `6` / `1` / `0`) against **both**
libraries rather than taking them on trust.

## Suite validation (mutation check)

To prove these tests are not vacuous, 19 single-line mutations were injected
into `src/lib.rs`, the `.so` rebuilt, and the suite re-run:

* **17 of 17 non-equivalent mutations were caught**, e.g. removing the `int`
  sign-extension in the siphash word assembly, `hash * 21` -> `* 23`,
  `used_count_threshold` / `tombstone_count_threshold` formula changes,
  `stbds_shmode_func` truncation, the `hmput_key` bucket `index = i-1`
  off-by-one, `hmdel_key`'s `length -= 1` and `temp` values,
  `hmget_key_ts`'s `*temp = -1`, the `stralloc` blocksize shift and the
  `1<<20` saturation.
* The 2 uncaught mutations are **provably equivalent mutants**, not coverage
  gaps: `} else if min_cap < 4 {` -> `< 5` (the branch body assigns `4`
  either way, so `min_cap == 4` yields `4` in both), and
  `stbds_hmdel_key`'s `ptr::null_mut()` -> `a` (that arm only runs inside
  `if a.is_null()`, so `a` *is* NULL).

The check also uncovered a real infrastructure flaw: `cargo test` does **not**
rebuild a `crate-type = ["cdylib"]` artifact, so the suite had been loading a
stale `.so`. `tests/common/mod.rs` now refuses to run if the `.so` is older
than anything under `src/` or `Cargo.toml`, and `run_all.sh` always builds
before testing.
